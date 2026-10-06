#!/usr/bin/env python3
"""Build deterministic compatibility manifests from pinned upstream checkouts.

This tool is intentionally not run during normal CI. It is used when a manifest
version is deliberately refreshed to a new upstream revision.
"""

from __future__ import annotations

import argparse
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import urlsplit


TEST262_COUNT = 2000
WPT_COUNT = 200
WPT_PREFIXES = (
    "css/CSS2/box-display",
    "css/CSS2/box-model",
    "css/css-box",
    "css/css-color",
    "css/css-display",
    "css/selectors",
)


class MatchLinkParser(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.match_href: str | None = None

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        if tag.lower() != "link" or self.match_href is not None:
            return
        values = {name.lower(): value for name, value in attrs}
        rel = (values.get("rel") or "").lower().split()
        href = values.get("href")
        if "match" in rel and href:
            self.match_href = href


def evenly_spaced_indices(length: int, count: int) -> list[int]:
    if length <= count:
        return list(range(length))
    return [((2 * index + 1) * length) // (2 * count) for index in range(count)]


def build_test262(root: Path) -> list[str]:
    language = root / "language"
    candidates = [
        path.relative_to(root).as_posix()
        for path in sorted(language.rglob("*.js"))
        if "module-code" not in path.relative_to(root).as_posix()
    ]
    return [candidates[index] for index in evenly_spaced_indices(len(candidates), TEST262_COUNT)]


def static_wpt_source(source: str) -> bool:
    lowered = source.lower()
    return (
        "<script" not in lowered
        and "testharness" not in lowered
        and "reftest-wait" not in lowered
        and 'rel="mismatch"' not in lowered
        and "rel='mismatch'" not in lowered
    )


def find_match_href(source: str) -> str | None:
    parser = MatchLinkParser()
    parser.feed(source)
    return parser.match_href


def build_wpt(root: Path) -> list[tuple[str, str]]:
    candidates: list[tuple[str, str]] = []
    for prefix in WPT_PREFIXES:
        directory = root / Path(prefix)
        if not directory.exists():
            continue
        for path in sorted(directory.rglob("*.html")):
            source = path.read_text(encoding="utf-8", errors="replace")
            if not static_wpt_source(source):
                continue
            href = find_match_href(source)
            if not href:
                continue
            parsed = urlsplit(href)
            if parsed.scheme or parsed.netloc or href.startswith("/") or parsed.query or parsed.fragment:
                continue
            reference = (path.parent / parsed.path).resolve()
            try:
                reference_relative = reference.relative_to(root.resolve()).as_posix()
            except ValueError:
                continue
            if reference.suffix.lower() != ".html" or not reference.is_file():
                continue
            reference_source = reference.read_text(encoding="utf-8", errors="replace")
            if not static_wpt_source(reference_source):
                continue
            candidates.append((path.relative_to(root).as_posix(), reference_relative))

    candidates.sort()
    return [candidates[index] for index in evenly_spaced_indices(len(candidates), WPT_COUNT)]


def write_lines(path: Path, header: list[str], lines: list[str]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    text = "\n".join([*(f"# {line}" for line in header), *lines, ""])
    path.write_text(text, encoding="utf-8", newline="\n")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--test262-root", type=Path, required=True, help="Path to Test262 test/")
    parser.add_argument("--wpt-root", type=Path, required=True, help="Path to WPT checkout root")
    parser.add_argument("--output", type=Path, default=Path("compat"))
    parser.add_argument("--test262-revision", required=True)
    parser.add_argument("--wpt-revision", required=True)
    args = parser.parse_args()

    test262 = build_test262(args.test262_root.resolve())
    wpt = build_wpt(args.wpt_root.resolve())
    if len(test262) != TEST262_COUNT:
        raise SystemExit(f"expected {TEST262_COUNT} Test262 entries, found {len(test262)}")
    if len(wpt) != WPT_COUNT:
        raise SystemExit(f"expected {WPT_COUNT} WPT entries, found {len(wpt)}")

    write_lines(args.output / "test262-parser-v1.txt", ["OPBrowser Test262 parser subset v1", f"upstream={args.test262_revision}", f"entries={len(test262)}", "scope=test/language deterministic even-spaced sample; module tests skipped by runner"], test262)
    write_lines(args.output / "wpt-static-v1.tsv", ["OPBrowser WPT static reftest subset v1", f"upstream={args.wpt_revision}", f"entries={len(wpt)}", "format=test-path<TAB>reference-path", "scope=static HTML reftests without script/testharness/reftest-wait"], [f"{test}\t{reference}" for test, reference in wpt])
    print(f"Test262 parser v1: {len(test262)} entries")
    print(f"WPT static v1: {len(wpt)} entries")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
