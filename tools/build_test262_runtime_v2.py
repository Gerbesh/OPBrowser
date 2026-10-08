#!/usr/bin/env python3
"""Create a deterministic Test262 runtime v2 sample from a *pinned* upstream tree.

Selection is deliberately blind to test contents, metadata, and pass/fail results.
Choose twelve evenly spaced sorted JavaScript paths per declared feature family.
Do NOT regenerate a published v2 manifest to make the percentage rise.
"""
import argparse
import subprocess
from pathlib import Path

TEST262_REVISION = "c8c798898646638cd0c24879f8e0374e847e7d74"
PER_FAMILY = 12
FAMILIES = (
    "language/expressions/array",
    "language/expressions/object",
    "language/expressions/conditional",
    "language/expressions/logical-and",
    "language/expressions/logical-or",
    "language/expressions/instanceof",
    "language/expressions/typeof",
    "language/statements/if",
    "language/statements/for",
    "language/statements/while",
    "language/statements/try",
    "language/statements/switch",
    "language/statements/function",
    "built-ins/Array/isArray",
    "built-ins/Array/of",
    "built-ins/Number/isFinite",
    "built-ins/Number/isNaN",
    "built-ins/String/prototype/charAt",
    "built-ins/JSON/parse",
    "built-ins/JSON/stringify",
    "built-ins/Promise/all",
    "built-ins/Promise/race",
    "built-ins/Object/keys",
    "built-ins/Object/assign",
    "built-ins/Boolean/prototype/valueOf",
)

def deterministic_pick(files, count):
    if len(files) <= count:
        return files
    # Midpoint stratification, independent of runtime feature support.
    return [files[((2 * i + 1) * len(files)) // (2 * count)] for i in range(count)]

def generate(root: Path) -> str:
    git_root = root.parent
    revision = subprocess.check_output(
        ["git", "-C", str(git_root), "rev-parse", "HEAD"], text=True
    ).strip()
    if revision != TEST262_REVISION:
        raise RuntimeError(f"expected Test262 {TEST262_REVISION}, found {revision}")
    entries = []
    summary = []
    for family in FAMILIES:
        directory = root / family
        if not directory.is_dir():
            raise RuntimeError(f"missing pinned fixture directory: {directory}")
        candidates = sorted(p.relative_to(root).as_posix()
                            for p in directory.rglob("*.js") if p.is_file())
        if not candidates:
            raise RuntimeError(f"no fixtures: {family}")
        chosen = deterministic_pick(candidates, PER_FAMILY)
        entries.extend(chosen)
        summary.append((family, len(candidates), len(chosen)))
    if len(entries) != len(set(entries)):
        raise RuntimeError("overlapping fixture groups")
    lines = [
        "# OPBrowser Test262 runtime broad v2: blind midpoint sample",
        f"# upstream={TEST262_REVISION}",
        f"# entries={len(entries)}",
        f"# families={len(FAMILIES)}",
        f"# per_family_max={PER_FAMILY}",
        "# scope=25 named language/builtin feature families; recursive JS file paths",
        "# selection=sorted paths, indices floor((2*i+1)*N/(2*k)), k=min(12,N)",
        "# no content inspection or filtering by implementation support, skip metadata or test result",
    ]
    for family, total, picked in summary:
        lines.append(f"# family={family} total={total} sampled={picked}")
    return "\n".join(lines + entries) + "\n"

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, required=True,
                        help="Pinned Test262/test folder with complete listed family subtrees")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    manifest = generate(args.root.resolve())
    if args.check:
        if not args.output.is_file() or args.output.read_text(encoding="utf-8") != manifest:
            raise SystemExit("Test262 v2 manifest differs from committed deterministic selection")
        print("Verified Test262 runtime v2 manifest: "+str(len(FAMILIES))+" families")
    else:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(manifest, encoding="utf-8", newline="\n")
        print("Wrote Test262 v2 manifest:", len(FAMILIES), "families,",
              sum(not line.startswith("#") for line in manifest.splitlines()), "tests")

if __name__ == "__main__":
    main()
