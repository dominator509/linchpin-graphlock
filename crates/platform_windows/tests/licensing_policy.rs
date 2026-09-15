//! Licensing, foundation and pinning policy acceptance tests.
//!
//! SPEC-010 groups Foundation, Platform and Licensing, so the policy that
//! REQ-LIC-001/002/003 and REQ-FOUND-001 state is asserted here rather than
//! described in prose. Each test reads the real repository artefact the
//! requirement names -- `LICENSE_ALLOWLIST.md`, `deny.toml`, the CycloneDX SBOM,
//! `Cargo.toml` and `Cargo.lock` -- so it fails when the policy is violated
//! rather than when a copy of the policy drifts.
//!
//! These are acceptance tests, not unit tests: they deliberately reach outside
//! the crate to the repository, because the requirement is about the
//! repository.

use std::path::{Path, PathBuf};

/// Repository root, derived from this crate's manifest directory.
fn repo_root() -> PathBuf {
    // <root>/crates/platform_windows
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate lives two levels below the repository root")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    let p = repo_root().join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("cannot read {}: {e}", p.display()))
}

/// Extract the quoted entries of a TOML array assigned to `key`.
///
/// Comment lines are removed BEFORE splitting on commas. Without that, a
/// multi-line comment inside the array leaks its fragments into the result: an
/// earlier version of this parser returned entries like `"cssparser-macros
/// 0.6.1"` from a comment, and the comparison it fed then reported a false
/// divergence between two allowlists that in fact agree.
fn toml_array(source: &str, key: &str, section: Option<&str>) -> Vec<String> {
    // Optionally scope to a section, so `allow =` inside `[licenses]` is not
    // confused with the same token appearing in prose elsewhere in the file.
    let scoped = match section {
        Some(name) => {
            let header = format!("[{name}]");
            let start = source
                .find(&header)
                .unwrap_or_else(|| panic!("section {header} not found"));
            let rest = &source[start + header.len()..];
            let end = rest.find("\n[").map(|i| i + 1).unwrap_or(rest.len());
            &rest[..end]
        }
        None => source,
    };

    // Drop comment lines first.
    let decommented: String = scoped
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");

    let start = decommented
        .find(key)
        .unwrap_or_else(|| panic!("{key} not found"));
    let open = decommented[start..]
        .find('[')
        .map(|i| start + i)
        .expect("array open bracket");
    let close = decommented[open..]
        .find(']')
        .map(|i| open + i)
        .expect("array close bracket");

    decommented[open + 1..close]
        .split(',')
        .map(|raw| raw.trim().trim_matches('"').trim().to_string())
        .filter(|t| !t.is_empty())
        .collect()
}

/// Remove double-quoted spans, so a token inside an `echo` message is not
/// mistaken for an invocation.
fn strip_double_quoted(line: &str) -> String {
    let mut out = String::new();
    let mut in_quotes = false;
    for ch in line.chars() {
        if ch == '"' {
            in_quotes = !in_quotes;
            continue;
        }
        if !in_quotes {
            out.push(ch);
        }
    }
    out
}

/// covers: REQ-LIC-001
/// "Redistributed core uses the permissive allowlist in `LICENSE_ALLOWLIST.md`:
/// MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Zlib." The forbidden
/// families must not be allowlisted without their own ADR.
#[test]
fn test_redistributed_core_uses_the_permissive_allowlist() {
    let deny = read("deny.toml");
    let allowed = toml_array(&deny, "allow =", Some("licenses"));

    for required in [
        "MIT",
        "Apache-2.0",
        "BSD-2-Clause",
        "BSD-3-Clause",
        "ISC",
        "Zlib",
    ] {
        assert!(
            allowed.iter().any(|a| a == required),
            "the permissive allowlist must contain {required}; found {allowed:?}"
        );
    }

    // "GPL, LGPL ..., AGPL, SSPL, BSL, FSL and custom/source-available licenses
    // ... are not allowed by default."
    for forbidden in [
        "GPL-2.0",
        "GPL-3.0",
        "GPL-2.0-only",
        "GPL-3.0-only",
        "LGPL-2.1",
        "LGPL-3.0",
        "AGPL-3.0",
        "SSPL-1.0",
        "BUSL-1.1",
        "BSL-1.1",
        "FSL-1.1",
    ] {
        assert!(
            !allowed.iter().any(|a| a == forbidden),
            "{forbidden} is not allowed by default but appears in deny.toml allow: {allowed:?}"
        );
    }

    // MPL-2.0 is permitted only because ADR-002 recorded the file-level review.
    if allowed.iter().any(|a| a == "MPL-2.0") {
        let decisions = read("DECISIONS.md");
        assert!(
            decisions.contains("ADR-002") && decisions.contains("ACCEPTED"),
            "MPL-2.0 is allowlisted, so ADR-002 must be recorded as accepted"
        );
        let adr = read(".agent/evidence/ADR-002-mpl-dependencies.md");
        assert!(
            adr.contains("htmlescape"),
            "the ADR-002 review must name every MPL crate it covers"
        );
    }
}

/// covers: REQ-LIC-002
/// "Every dependency addition records SPDX ID, version, source URL ...
/// `cargo-deny`, JavaScript license scanning, Python license inventory and SBOM
/// generation must agree before release."
#[test]
fn test_license_scanners_agree_and_the_sbom_records_spdx_and_source() {
    let deny = read("deny.toml");
    let deny_allow = toml_array(&deny, "allow =", Some("licenses"));

    // The SBOM generator carries its own allowlist. REQ-LIC-002 requires the
    // scanners to AGREE, so the two sets must match; a divergence is exactly the
    // drift the requirement forbids.
    let generator = read("scripts/generate-sbom.py");
    let gen_allow = {
        let start = generator
            .find("ALLOWED = {")
            .expect("ALLOWED set in generate-sbom.py");
        let open = generator[start..].find('{').unwrap() + start;
        let close = generator[open..].find('}').unwrap() + open;
        let body = &generator[open + 1..close];
        let mut v: Vec<String> = body
            .lines()
            .filter_map(|l| {
                let t = l.trim().trim_end_matches(',');
                let t = t.split('#').next().unwrap_or("").trim();
                let t = t.trim_matches('"');
                if t.is_empty() {
                    None
                } else {
                    Some(t.to_string())
                }
            })
            .collect();
        v.sort();
        v
    };
    let mut deny_sorted = deny_allow.clone();
    deny_sorted.sort();
    assert_eq!(
        gen_allow, deny_sorted,
        "cargo-deny's allowlist and the SBOM generator's allowlist must agree"
    );

    // The SBOM itself must exist and give every component a licence and a
    // resolvable source.
    let sbom: serde_json::Value =
        serde_json::from_str(&read(".agent/evidence/sbom/linchpin.cdx.json"))
            .expect("SBOM is valid JSON");
    let components = sbom["components"]
        .as_array()
        .expect("SBOM has a components array");
    assert!(
        !components.is_empty(),
        "an SBOM with no components proves nothing"
    );

    let mut missing_license = Vec::new();
    let mut missing_source = Vec::new();
    for c in components {
        let name = c["name"].as_str().unwrap_or("<unnamed>");
        if c["licenses"]
            .as_array()
            .map(|a| a.is_empty())
            .unwrap_or(true)
        {
            missing_license.push(name.to_string());
        }
        if c["purl"].as_str().unwrap_or("").is_empty() {
            missing_source.push(name.to_string());
        }
    }
    assert!(
        missing_license.is_empty(),
        "components with no recorded licence: {missing_license:?}"
    );
    assert!(
        missing_source.is_empty(),
        "components with no source (purl): {missing_source:?}"
    );

    // Notices must be produced alongside the machine-readable SBOM.
    let notices = read(".agent/evidence/sbom/THIRD_PARTY_NOTICES.md");
    assert!(
        notices.contains("MPL-2.0"),
        "the notices must carry the reviewed MPL-2.0 permission"
    );
    assert!(
        notices.contains("Artifact identity"),
        "the notices must record artifact identity, not only licences"
    );
}

/// covers: REQ-LIC-003
/// "Dependency versions are pinned exactly and never floated."
///
/// The mechanism that prevents floating in this repository is the committed
/// lockfile plus `--locked` on every build, which makes cargo FAIL rather than
/// silently re-resolve. That is what this asserts, and it asserts it across
/// every invocation rather than one.
#[test]
fn test_dependency_versions_are_pinned_and_never_floated() {
    let root = repo_root();
    assert!(
        root.join("Cargo.lock").is_file(),
        "a committed Cargo.lock is required to pin versions"
    );
    assert!(
        root.join("pnpm-lock.yaml").is_file(),
        "a committed pnpm lockfile is required to pin JavaScript versions"
    );

    // No manifest may declare a wildcard requirement.
    let manifests = [
        "Cargo.toml",
        "apps/desktop/src-tauri/Cargo.toml",
        "crates/platform_windows/Cargo.toml",
        "crates/research/Cargo.toml",
        "crates/domain/Cargo.toml",
    ];
    for m in manifests {
        let text = read(m);
        for line in text.lines() {
            let t = line.trim();
            if t.starts_with('#') {
                continue;
            }
            assert!(
                !t.contains("version = \"*\""),
                "{m} declares a wildcard dependency version: {t}"
            );
        }
    }

    // Every cargo build/test/check/clippy invocation in scripts/ must pin the
    // lock, including the tauri build passthrough.
    let scripts = root.join("scripts");
    let mut offenders = Vec::new();
    let mut checked = 0;
    for entry in std::fs::read_dir(&scripts).expect("scripts dir") {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("sh") {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        for line in text.lines() {
            let t = line.trim();
            // Skip comments: they discuss cargo commands without running them.
            if t.starts_with('#') || t.is_empty() {
                continue;
            }
            // Strip quoted text before looking for a command, so the phrase
            // "tauri build" inside an `echo` failure message is not counted as an
            // invocation. An earlier revision flagged exactly that.
            let code = strip_double_quoted(t);
            let is_cargo = code.contains("cargo build")
                || code.contains("cargo test")
                || code.contains("cargo check")
                || code.contains("cargo clippy")
                || code.contains("tauri build");
            if !is_cargo {
                continue;
            }
            checked += 1;
            let pinned = code.contains("--locked")
                || code.contains("--frozen")
                || code.contains("--offline");
            if !pinned {
                offenders.push(format!(
                    "{}: {}",
                    path.file_name().unwrap().to_string_lossy(),
                    t
                ));
            }
        }
    }
    assert!(checked > 0, "no build invocations were inspected");
    assert!(
        offenders.is_empty(),
        "build invocations that do not pin the lockfile: {offenders:#?}"
    );
}

/// covers: REQ-FOUND-001
/// "The product is a local-first desktop application on a Tauri 2.x foundation
/// with a Rust workspace of [11] crates ... plus a React desktop client.
/// Workspace membership and crate boundaries are declared in `Cargo.toml`."
#[test]
fn test_workspace_membership_and_tauri_2x_foundation() {
    let cargo = read("Cargo.toml");
    let members = toml_array(&cargo, "members =", Some("workspace"));

    for required in [
        "crates/domain",
        "crates/application",
        "crates/storage",
        "crates/evidence",
        "crates/research",
        "crates/patent",
        "crates/commercialization",
        "crates/provider_transport",
        "crates/mcp_hub",
        "crates/crash_reporter",
        "crates/platform_windows",
        "apps/desktop/src-tauri",
    ] {
        assert!(
            members.iter().any(|m| m == required),
            "workspace membership must declare {required}; found {members:?}"
        );
    }

    // Tauri 2.x foundation.
    let desktop = read("apps/desktop/src-tauri/Cargo.toml");
    let tauri_line = desktop
        .lines()
        .find(|l| l.trim_start().starts_with("tauri ="))
        .expect("a tauri dependency");
    assert!(
        tauri_line.contains("2."),
        "the foundation must be Tauri 2.x, found: {tauri_line}"
    );

    // React desktop client, declared as the frontend.
    let desktop_pkg = read("apps/desktop/package.json");
    assert!(
        desktop_pkg.contains("\"react\""),
        "the desktop client must be a React application"
    );
    let conf = read("apps/desktop/src-tauri/tauri.conf.json");
    assert!(
        conf.contains("frontendDist"),
        "the Tauri config must declare the embedded frontend"
    );

    // Local-first: the shipped build embeds a local frontend directory rather
    // than fetching one, and any URL in the configuration is loopback. NOTE the
    // `devUrl` key is expected to be present: it points at the local `tauri dev`
    // server and is used only by that command, so treating its presence as a
    // violation was wrong in an earlier revision of this test.
    let dist = conf
        .lines()
        .find(|l| l.contains("frontendDist"))
        .expect("frontendDist in the Tauri config");
    assert!(
        dist.contains("../") || dist.contains("./"),
        "frontendDist must be a local relative path, found: {dist}"
    );
    for line in conf.lines() {
        let t = line.trim();
        let Some(pos) = t.find("http://").or_else(|| t.find("https://")) else {
            continue;
        };
        let url = &t[pos..];
        let loopback = url.starts_with("http://localhost")
            || url.starts_with("http://127.0.0.1")
            || url.starts_with("https://schema.tauri.app");
        assert!(
            loopback,
            "the shipped configuration must not name a non-loopback origin: {t}"
        );
    }
}
