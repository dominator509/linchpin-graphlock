#!/usr/bin/env python3
"""Generate correctly-formatted Windows .ico and Apple .icns icon files from a source PNG.

GraphLock context: EP-009/M1 requires a real frozen Windows artifact. The repository
previously shipped five byte-identical PNG copies (including icon.ico), which caused
tauri-winres / RC.EXE to fail with:
    RC2175 : resource file ...\\icons\\icon.ico is not in 3.00 format
This script performs a real, dependency-free format conversion (stdlib only):
  - .ico : ICONDIR/ICONDIRENTRY container with PNG-compressed entries (Vista+ format)
  - .icns: ICNS container with ic07..ic10 PNG element types
Both are validated by re-parsing the produced bytes before the script reports success.

Usage: python3 scripts/generate-icons.py <source.png> <outdir>
"""
from __future__ import annotations

import struct
import sys
import zlib
from pathlib import Path

PNG_MAGIC = b"\x89PNG\r\n\x1a\n"
ICO_SIZES = [16, 24, 32, 48, 64, 128, 256]
ICNS_TYPES = {16: b"icp4", 32: b"icp5", 64: b"icp6", 128: b"ic07", 256: b"ic08"}


def read_png(path: Path):
    """Minimal PNG decoder -> (width, height, rgba_rows). Stdlib only."""
    data = path.read_bytes()
    if not data.startswith(PNG_MAGIC):
        raise SystemExit(f"not a PNG: {path}")
    pos, idat, w, h, bitdepth, ctype, interlace = 8, bytearray(), 0, 0, 0, 0, 0
    while pos < len(data):
        (length,) = struct.unpack(">I", data[pos : pos + 4])
        ctag = data[pos + 4 : pos + 8]
        body = data[pos + 8 : pos + 8 + length]
        if ctag == b"IHDR":
            w, h, bitdepth, ctype, _, _, interlace = struct.unpack(">IIBBBBB", body)
        elif ctag == b"IDAT":
            idat += body
        elif ctag == b"IEND":
            break
        pos += 12 + length
    if bitdepth != 8:
        raise SystemExit(f"only 8-bit PNG supported, got {bitdepth}")
    channels = {0: 1, 2: 3, 3: 1, 4: 2, 6: 4}.get(ctype)
    if channels is None:
        raise SystemExit(f"unsupported PNG color type {ctype}")
    if interlace:
        raise SystemExit("interlaced PNG not supported")
    raw = zlib.decompress(bytes(idat))
    stride = w * channels
    out, prev = [], bytearray(stride)
    p = 0
    for _ in range(h):
        ft = raw[p]
        p += 1
        line = bytearray(raw[p : p + stride])
        p += stride
        for i in range(stride):
            a = line[i - channels] if i >= channels else 0
            b = prev[i]
            c = prev[i - channels] if i >= channels else 0
            x = line[i]
            if ft == 1:
                x = (x + a) & 0xFF
            elif ft == 2:
                x = (x + b) & 0xFF
            elif ft == 3:
                x = (x + ((a + b) >> 1)) & 0xFF
            elif ft == 4:
                pa, pb, pc = abs(b - c), abs(a - c), abs(a + b - 2 * c)
                pr = a if (pa <= pb and pa <= pc) else (b if pb <= pc else c)
                x = (x + pr) & 0xFF
            line[i] = x
        out.append(bytes(line))
        prev = line
    return w, h, channels, out


def to_rgba(w, h, channels, rows):
    """Normalize decoded rows to RGBA8."""
    if channels == 4:
        return rows
    res = []
    for row in rows:
        px = bytearray()
        for i in range(0, len(row), channels):
            if channels == 3:
                px += row[i : i + 3] + b"\xff"
            elif channels == 1:
                px += bytes([row[i]] * 3) + b"\xff"
            elif channels == 2:
                px += bytes([row[i]] * 3) + bytes([row[i + 1]])
            else:
                raise SystemExit("unsupported channel count")
        res.append(bytes(px))
    return res


def encode_png(w, h, rgba_rows):
    """Encode RGBA rows to a real PNG byte stream (stdlib zlib)."""
    raw = b"".join(b"\x00" + r for r in rgba_rows)
    def chunk(tag, body):
        return (
            struct.pack(">I", len(body))
            + tag
            + body
            + struct.pack(">I", zlib.crc32(tag + body) & 0xFFFFFFFF)
        )
    return (
        PNG_MAGIC
        + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )


def box_resize(w, h, rgba_rows, nw, nh):
    """Box-filter downscale to (nw, nh). Uses premultiplied alpha for correctness."""
    out = []
    for y in range(nh):
        y0, y1 = (y * h) // nh, max(((y + 1) * h) // nh, (y * h) // nh + 1)
        row = bytearray()
        for x in range(nw):
            x0, x1 = (x * w) // nw, max(((x + 1) * w) // nw, (x * w) // nw + 1)
            ar = ag = ab = aa = n = 0
            for yy in range(y0, min(y1, h)):
                src = rgba_rows[yy]
                for xx in range(x0, min(x1, w)):
                    r, g, b, a = src[xx * 4 : xx * 4 + 4]
                    ar += r * a
                    ag += g * a
                    ab += b * a
                    aa += a
                    n += 1
            if aa:
                row += bytes([ar // aa, ag // aa, ab // aa, aa // n])
            else:
                row += b"\x00\x00\x00\x00"
        out.append(bytes(row))
    return out


def build_ico(images):
    """ICONDIR + ICONDIRENTRY[] + PNG payloads (width/height 0 means 256)."""
    n = len(images)
    header = struct.pack("<HHH", 0, 1, n)
    offset = 6 + 16 * n
    entries, blobs = b"", b""
    for size, png in images:
        dim = 0 if size >= 256 else size
        entries += struct.pack(
            "<BBBBHHII", dim, dim, 0, 0, 1, 32, len(png), offset
        )
        blobs += png
        offset += len(png)
    return header + entries + blobs


def build_icns(images):
    """ICNS container: magic + total length + (type, length, PNG) elements."""
    body = b""
    for size, png in images:
        tag = ICNS_TYPES.get(size)
        if not tag:
            continue
        body += tag + struct.pack(">I", len(png) + 8) + png
    return b"icns" + struct.pack(">I", len(body) + 8) + body


def verify_ico(blob, expect_sizes):
    """Re-parse the produced ICO to prove structural correctness."""
    reserved, itype, count = struct.unpack("<HHH", blob[:6])
    assert reserved == 0 and itype == 1, "bad ICONDIR"
    assert count == len(expect_sizes), f"entry count {count} != {len(expect_sizes)}"
    seen = []
    for i in range(count):
        e = blob[6 + 16 * i : 6 + 16 * (i + 1)]
        w, h, _, _, planes, bpp, length, offset = struct.unpack("<BBBBHHII", e)
        payload = blob[offset : offset + length]
        assert payload.startswith(PNG_MAGIC), f"entry {i} is not PNG payload"
        assert struct.unpack(">I", payload[8:12])[0] > 0
        seen.append(256 if w == 0 else w)
    assert seen == expect_sizes, f"size mismatch {seen}"
    return True


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: generate-icons.py <source.png> <outdir>", file=sys.stderr)
        return 2
    src, outdir = Path(sys.argv[1]), Path(sys.argv[2])
    w, h, channels, rows = read_png(src)
    rgba = to_rgba(w, h, channels, rows)
    print(f"source: {src} {w}x{h} channels={channels}")
    scaled = [(s, encode_png(s, s, box_resize(w, h, rgba, s, s))) for s in ICO_SIZES]

    ico = build_ico(scaled)
    if not verify_ico(ico, ICO_SIZES):
        raise SystemExit("ICO self-verification failed")
    icns = build_icns(scaled)
    assert icns[:4] == b"icns" and struct.unpack(">I", icns[4:8])[0] == len(icns)

    outdir.mkdir(parents=True, exist_ok=True)
    (outdir / "icon.ico").write_bytes(ico)
    (outdir / "icon.icns").write_bytes(icns)
    for s in (32, 128, 256):
        (outdir / f"{s}x{s}.png").write_bytes(dict(scaled)[s])
    (outdir / "128x128@2x.png").write_bytes(dict(scaled)[256])
    print(f"wrote icon.ico ({len(ico)} bytes, {len(ICO_SIZES)} sizes {ICO_SIZES})")
    print(f"wrote icon.icns ({len(icns)} bytes)")
    print("ICO verification: ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
