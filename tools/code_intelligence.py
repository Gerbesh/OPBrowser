#!/usr/bin/env python3
"""OPBrowser Code Graph + Code Slicer: deterministic source-backed documentation."""
import argparse
import json
from pathlib import Path
import re
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]
GRAPH = Path("docs/GENERATED_CODE_GRAPH.md")
SLICES = Path("docs/GENERATED_CODE_SLICES.md")


def crates():
    workspace = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    found = {}
    for member in workspace["workspace"]["members"]:
        manifest = tomllib.loads((ROOT / member / "Cargo.toml").read_text(encoding="utf-8"))
        name = manifest["package"]["name"]
        if name in found:
            raise ValueError(f"Duplicate crate {name}")
        found[name] = (member, manifest)
    return found


def graph_report(found):
    links = set()
    for name, (_, manifest) in found.items():
        sections = [manifest.get(kind, {}) for kind in ("dependencies", "dev-dependencies", "build-dependencies")]
        for target in manifest.get("target", {}).values():
            sections += [target.get(kind, {}) for kind in ("dependencies", "dev-dependencies", "build-dependencies")]
        for section in sections:
            for alias, spec in section.items():
                if isinstance(spec, dict) and "path" in spec:
                    dependency = spec.get("package", alias)
                    if dependency not in found:
                        raise ValueError(f"Unknown local dependency: {name} -> {dependency}")
                    links.add((name, dependency))
    rows = ["# Generated Code Graph", "",
            "> Generated from Cargo manifests using `python tools/code_intelligence.py --write`.",
            "> Edges represent local crate dependencies, not Rust function calls.", "",
            f"**{len(found)} crates; {len(links)} local dependency edges.**", "",
            "```mermaid", "graph LR"]
    rows += [f'    {name}["{name}"]' for name in sorted(found)]
    rows += [f"    {source} --> {target}" for source, target in sorted(links)]
    rows += ["```", "", "## Sources", ""]
    rows += [f"- [`{name}`](../{member}/Cargo.toml)" for name, (member, _) in sorted(found.items())]
    rows += ["", "See [CODE_GRAPH.md](CODE_GRAPH.md) for architectural decisions and runtime ownership.", ""]
    return "\n".join(rows)


def slices_report(found):
    manifest = json.loads((ROOT / "tools/code_slices.json").read_text(encoding="utf-8"))
    if manifest.get("schema_version") != 1:
        raise ValueError("Unsupported code slice manifest")
    ids = set()
    rows = ["# Generated Code Slices", "",
            "> Generated from `tools/code_slices.json` and validated against live Rust files.",
            "> This is a curated vertical feature map, **not** an AST/call/data-flow slicer.", ""]
    for spec in manifest["slices"]:
        if spec["id"] in ids:
            raise ValueError(f"Duplicate slice: {spec['id']}")
        ids.add(spec["id"])
        if len(spec["steps"]) < 2 or len({
            step["path"].split("/")[1] for step in spec["steps"]
        }) < 2:
            raise ValueError(f"Slice must cross at least two crates: {spec['id']}")
        rows += [f"## {spec['id']} — {spec['title']}", "",
                 f"Status: **{spec['status']}**.", "", "```text"]
        verified = []
        for step in spec["steps"]:
            path = step["path"]
            symbol = step["symbol"]
            resolved = (ROOT / path).resolve()
            if not resolved.is_relative_to(ROOT) or not path.startswith("crates/") or not resolved.is_file():
                raise ValueError(f"Missing/out-of-workspace source: {path}")
            if path.split("/")[1] not in found:
                raise ValueError(f"Unknown crate for {path}")
            content = resolved.read_text(encoding="utf-8").splitlines()
            matching = [i for i, line in enumerate(content, 1) if
                        re.search(r"(?<![\w])" + re.escape(symbol) + r"(?![\w])", line)]
            if not matching:
                raise ValueError(f"Stale slice anchor: {path}: {symbol}")
            rows.append(f"{path.split('/')[1]}::{symbol}")
            verified.append(f"- [`{path.split('/')[1]}::{symbol}`](../{path}#L{matching[0]})")
        rows += ["```", "", *verified, "", spec["limitation"], ""]
    rows += ["See [CODE_SLICES.md](CODE_SLICES.md) for full engineering notes.", ""]
    return "\n".join(rows)


def wiki_link_errors():
    errors = []
    for page in sorted((ROOT / "wiki").glob("*.md")):
        for target in re.findall(r"(?<!!)\[[^\]]+\]\(([^)]+)\)",
                                 page.read_text(encoding="utf-8")):
            target = target.split("#", 1)[0].strip()
            if not target or target.startswith(("https://", "http://", "mailto:")):
                continue
            dest = (page.parent / target).resolve()
            if not dest.is_relative_to(ROOT) or not dest.is_file():
                errors.append(f"{page.relative_to(ROOT)} -> {target}")
    return errors


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    operation = parser.add_mutually_exclusive_group(required=True)
    operation.add_argument("--write", action="store_true", help="Generate both Markdown reports")
    operation.add_argument("--check", action="store_true", help="Verify reports and wiki links")
    args = parser.parse_args()
    found = crates()
    reports = {GRAPH: graph_report(found), SLICES: slices_report(found)}
    errors = []
    for name, report in reports.items():
        path = ROOT / name
        if args.write:
            path.write_text(report, encoding="utf-8", newline="\n")
            print(f"WRITE {name}")
        elif not path.is_file() or path.read_text(encoding="utf-8") != report:
            errors.append(f"STALE {name}: run python tools/code_intelligence.py --write")
        else:
            print(f"OK {name}")
    errors += ["BROKEN WIKI LINK " + link for link in wiki_link_errors()]
    for error in errors:
        print(error, file=sys.stderr)
    return 1 if errors else 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, KeyError, ValueError, tomllib.TOMLDecodeError) as err:
        print(f"Code intelligence error: {err}", file=sys.stderr)
        sys.exit(2)
