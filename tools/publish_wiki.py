#!/usr/bin/env python3
"""Synchronize tracked wiki/*.md to a checked-out GitHub Wiki repository.

Only writes Markdown pages. Does not run git, delete remote pages, or touch
the separate wiki history. GitHub Wiki is a different Git repository.
"""
from __future__ import annotations

import argparse
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "wiki"
MAIN = "https://github.com/Gerbesh/OPBrowser/blob/main/"
WIKI = "https://github.com/Gerbesh/OPBrowser/wiki/"
LINK = re.compile(r"(?<!!)\]\(([^)]+)\)")


def convert(content: str) -> str:
    """Convert repository-relative Markdown links into published GitHub links."""
    def replace(match: re.Match[str]) -> str:
        target = match.group(1)
        path, separator, fragment = target.partition("#")
        suffix = separator + fragment if separator else ""
        if path.startswith("../"):
            normalized = (SOURCE / path).resolve()
            if not normalized.is_relative_to(ROOT) or not normalized.is_file():
                raise ValueError(f"Invalid linked repository file: {target}")
            return "](" + MAIN + normalized.relative_to(ROOT).as_posix() + suffix + ")"
        if path.endswith(".md") and not path.startswith(("https://", "http://", "/")):
            page = SOURCE / path
            if not page.is_file() or page.parent.resolve() != SOURCE.resolve():
                raise ValueError(f"Missing wiki page: {target}")
            return "](" + WIKI + page.stem + suffix + ")"
        return match.group(0)
    return LINK.sub(replace, content)


def render() -> dict[str, str]:
    return {
        path.name: convert(path.read_text(encoding="utf-8"))
        for path in sorted(SOURCE.glob("*.md"))
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", type=Path, required=True,
                        help="existing local checkout of OPBrowser.wiki.git")
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--write", action="store_true")
    mode.add_argument("--check", action="store_true")
    args = parser.parse_args()
    target = args.target.resolve()
    if not target.is_dir() or not (target / ".git").is_dir():
        parser.error("target must be an existing Git checkout of the Wiki repository")
    if target == ROOT or target == SOURCE:
        parser.error("target cannot be the main workspace")
    files = render()
    stale = []
    for name, content in files.items():
        output = target / name
        if args.write:
            output.write_text(content, encoding="utf-8", newline="\n")
        elif not output.is_file() or output.read_text(encoding="utf-8") != content:
            stale.append(name)
    if stale:
        print("Wiki pages not synchronized: " + ", ".join(stale), file=sys.stderr)
        return 1
    print(f"{'Synchronized' if args.write else 'Verified'} {len(files)} Wiki pages")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
