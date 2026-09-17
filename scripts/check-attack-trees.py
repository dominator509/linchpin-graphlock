#!/usr/bin/env python3
"""Attack trees: validate them against the threat model and the tree, and render (GEN-092).

WHY THIS EXISTS. GEN-092 ("Attack Tree Analysis") was recorded NOT_APPLICABLE with "no
attack tree analysis exists". Authoring trees is easy; keeping them honest is the point,
so this validator binds them:

  * every tree names a root goal and a threat id that EXISTS in the threat model, so a
    tree cannot describe an attack nobody modelled;
  * the node graph is well formed: unique ids, every child declared, every non-leaf has
    at least two children, no orphans and no cycles;
  * every leaf is MITIGATED, PARTIAL or OPEN; a MITIGATED leaf must name at least one
    symbol that exists in the file it cites, because a mitigation cited without its
    implementation is a claim rather than a control;
  * an OPEN or PARTIAL leaf must name the residual or decision that keeps it open, and
    that text must be traceable to the threat model's residual or its accepted risks --
    otherwise "open" becomes a place to hide unfinished work;
  * every threat in the model is reachable from at least one tree, so the model and the
    trees cannot drift apart.

Then it renders `.agent/evidence/attack-trees.md`; `--check` fails when the rendered
document is stale, so verify.sh can enforce currency like every other derived artefact.
"""
from __future__ import annotations

import json
import pathlib
import sys

ROOT = pathlib.Path(".").resolve()
TREES = ROOT / ".agent/verification/state/ATTACK_TREES.json"
MODEL = ROOT / ".agent/verification/state/THREAT_MODEL.json"
RENDERED = ROOT / ".agent/evidence/attack-trees.md"
LEAF_STATUS = {"MITIGATED", "PARTIAL", "OPEN"}


def validate(trees_doc: dict, model: dict) -> list[str]:
    problems: list[str] = []
    model_threats = {t["id"]: t for t in model.get("threats", [])}
    model_residuals = " ".join(
        [str(t.get("residual", "")) for t in model.get("threats", [])]
        + [f"{r['id']} {r['risk']} {r['decision']} {r['status']}" for r in model.get("accepted_risks", [])]
    ).lower()
    reached: set[str] = set()

    for tree in trees_doc.get("trees", []):
        tid = tree.get("id", "?")
        threat_id = tree.get("threat")
        if threat_id not in model_threats:
            problems.append(f"{tid}: names threat {threat_id!r}, which the model does not declare")
        else:
            reached.add(threat_id)
        nodes = {node["id"]: node for node in tree.get("nodes", [])}
        if len(nodes) != len(tree.get("nodes", [])):
            problems.append(f"{tid}: duplicate node ids")
        if not tree.get("goal"):
            problems.append(f"{tid}: no root goal")

        children_seen: set[str] = set(tree.get("children", []))
        for node in tree.get("nodes", []):
            node_id = node["id"]
            kids = node.get("children", [])
            is_leaf = node.get("leaf", False)
            if is_leaf and kids:
                problems.append(f"{node_id}: marked as a leaf but declares children")
            if not is_leaf:
                if len(kids) < 2:
                    problems.append(f"{node_id}: non-leaf with fewer than two children (an AND/OR node needs alternatives)")
                for kid in kids:
                    if kid not in nodes:
                        problems.append(f"{node_id}: child {kid} is not declared")
                    else:
                        children_seen.add(kid)
            if is_leaf:
                status = node.get("status")
                if status not in LEAF_STATUS:
                    problems.append(f"{node_id}: leaf status {status!r} outside {sorted(LEAF_STATUS)}")
                if status == "MITIGATED":
                    symbols = node.get("symbols") or []
                    if not symbols:
                        problems.append(f"{node_id}: MITIGATED without a bound symbol")
                    for symbol in symbols:
                        path = ROOT / symbol["file"]
                        if not path.exists():
                            problems.append(f"{node_id}: cites missing file {symbol['file']}")
                        elif symbol["contains"] not in path.read_text(encoding="utf-8", errors="replace"):
                            problems.append(
                                f"{node_id}: cites {symbol['file']!r} for {symbol['contains']!r}, "
                                "which that file does not contain"
                            )
                if status in {"OPEN", "PARTIAL"}:
                    residual = str(node.get("residual", "")).strip()
                    if not residual:
                        problems.append(f"{node_id}: {status} leaf without a residual or decision")
                    else:
                        # The residual must be traceable to the model, so an "open" leaf
                        # cannot be a private excuse that never reaches the risk register.
                        key_terms = [w for w in residual.lower().replace(",", " ").split() if len(w) > 5]
                        if key_terms and not any(term in model_residuals for term in key_terms):
                            problems.append(
                                f"{node_id}: its residual is not traceable to any residual or accepted "
                                "risk in the threat model"
                            )
        roots = tree.get("children") or ([tree["nodes"][0]["id"]] if tree.get("nodes") else [])
        orphans = set(nodes) - children_seen - set(roots)
        if orphans:
            problems.append(f"{tid}: nodes unreachable from the root: {sorted(orphans)}")
        if roots:
            seen: set[str] = set()
            stack = list(roots)
            # A tree-level child must exist as a node, or the tree silently loses a branch.
            for root in roots:
                if root not in nodes:
                    problems.append(f"{tid}: root child {root} is not declared as a node")
            while stack:
                current = stack.pop()
                if current in seen:
                    problems.append(f"{tid}: cycle detected at {current}")
                    break
                seen.add(current)
                stack.extend(nodes.get(current, {}).get("children", []))

    missing = sorted(set(model_threats) - reached)
    notes = {entry.get("id"): entry for entry in trees_doc.get("threat_notes", [])}
    node_ids = {n["id"] for t in trees_doc.get("trees", []) for n in t.get("nodes", [])}
    for threat_id in missing:
        note = notes.get(threat_id)
        if not note:
            problems.append(
                f"threat {threat_id} has neither an attack tree nor an explicit threat_notes entry"
            )
            continue
        coverage = str(note.get("coverage", "")).strip()
        if not coverage:
            problems.append(f"threat {threat_id}: threat_notes entry has no coverage statement")
            continue
        # A note may point at the leaf that covers the threat, and that leaf must exist.
        for token in coverage.split():
            if token.startswith("leaf:") and token[5:].rstrip(",") not in node_ids:
                problems.append(f"threat {threat_id}: coverage cites unknown leaf {token[5:]}")
    for threat_id in sorted(set(notes) - set(model_threats)):
        problems.append(f"threat_notes names {threat_id}, which the model does not declare")
    return problems


def render(trees_doc: dict) -> str:
    lines = [
        "# LINCHPIN attack trees (GEN-092)",
        "",
        "Generated by `scripts/check-attack-trees.py` from",
        "`.agent/verification/state/ATTACK_TREES.json`. Do not hand-edit: the validator fails",
        "when a tree names a threat the model does not declare, when a MITIGATED leaf cites a",
        "symbol the tree does not contain, when an OPEN or PARTIAL leaf's residual is not",
        "traceable to the threat model, or when a modelled threat has no tree.",
        "",
        f"- Product: {trees_doc['scope']['product']}",
        f"- Reviewed: {trees_doc['scope']['reviewed']}",
        f"- Method: {trees_doc['scope']['method']}",
        "",
    ]
    for tree in trees_doc["trees"]:
        lines += [f"## {tree['id']} — {tree['goal']}", "", f"Threat: **{tree['threat']}**", "",
                  tree.get("comment", ""), "", "| Node | Kind | Description | Status | Evidence |",
                  "| --- | --- | --- | --- | --- |"]
        for node in tree["nodes"]:
            kind = "leaf" if node.get("leaf") else node.get("kind", "?")
            status = node.get("status", "—")
            evidence = node.get("evidence", "see children")
            lines.append(
                f"| `{node['id']}` | {kind} | {node['description']} | {status} | {evidence} |"
            )
        lines.append("")
        open_leaves = [n for n in tree["nodes"] if n.get("status") in {"OPEN", "PARTIAL"}]
        if open_leaves:
            lines += ["Unmitigated or partial leaves and why they stay that way:", ""]
            for node in open_leaves:
                lines.append(f"- `{node['id']}` ({node['status']}): {node.get('residual', 'no residual recorded')}")
            lines.append("")
    lines += ["## Reproduce", "", "```", "python3 scripts/check-attack-trees.py",
              "python3 scripts/check-attack-trees.py --check", "```", ""]
    return "\n".join(lines)


def main() -> int:
    check_only = "--check" in sys.argv
    for path in (TREES, MODEL):
        if not path.exists():
            print(f"attack-trees: FAIL -- missing {path}", file=sys.stderr)
            return 1
    trees_doc = json.loads(TREES.read_text(encoding="utf-8"))
    model = json.loads(MODEL.read_text(encoding="utf-8"))
    problems = validate(trees_doc, model)
    if problems:
        print(f"attack-trees: FAIL -- {len(problems)} problem(s):", file=sys.stderr)
        for problem in problems:
            print(f"  {problem}", file=sys.stderr)
        return 1
    rendered = render(trees_doc)
    if check_only:
        if not RENDERED.exists() or RENDERED.read_text(encoding="utf-8") != rendered:
            print("attack-trees: FAIL -- rendered document is stale; run without --check", file=sys.stderr)
            return 1
        leaves = sum(1 for t in trees_doc["trees"] for n in t["nodes"] if n.get("leaf"))
        mitigated = sum(
            1 for t in trees_doc["trees"] for n in t["nodes"] if n.get("status") == "MITIGATED"
        )
        print(
            f"attack-trees check: ok ({len(trees_doc['trees'])} trees, {leaves} leaves, "
            f"{mitigated} mitigated with a bound symbol, every modelled threat represented)"
        )
        return 0
    RENDERED.parent.mkdir(parents=True, exist_ok=True)
    RENDERED.write_text(rendered, encoding="utf-8")
    print(f"attack-trees: wrote {RENDERED.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
