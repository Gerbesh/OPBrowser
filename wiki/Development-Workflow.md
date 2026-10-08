# Development Workflow

Every coherent update follows the same loop:

1. implement one bounded change;
2. update the project plan/status if needed;
3. append the change to the development log;
4. update code graph/code slices when architecture changes;
5. update relevant wiki pages;
6. run rustfmt, Clippy, tests, and startup smoke tests;
7. create one descriptive Git commit;
8. push it to the public repository.

Red builds are not considered completed updates.

See [AGENTS.md](https://github.com/Gerbesh/OPBrowser/blob/main/AGENTS.md)
for the exact repository rules.

## Code Graph, Code Slicer and Wiki freshness

```powershell
python tools/code_intelligence.py --write # update graph and curated slices
python tools/code_intelligence.py --check # CI verification + Wiki link check
```

The graph reads all workspace Cargo manifests; slice definitions in
`tools/code_slices.json` must point to actual Rust files and symbols.
Two reproducible Markdown files are tracked in `docs/`. The verifier
will reject stale reports and broken relative Wiki links.
Update `wiki/Current-Status.md` and `wiki/Compatibility-Strategy.md`
when measured progress changes. Classify legacy reference mismatches in
`docs/KNOWN_TEST_DIVERGENCES.md` without removing them from metrics.

GitHub's separate Wiki repository is now initialized and published
at [OPBrowser Wiki](https://github.com/Gerbesh/OPBrowser/wiki).
The `wiki/` directory in the main repository remains the canonical
version-controlled source. To synchronize it, clone
`https://github.com/Gerbesh/OPBrowser.wiki.git` separately and run
`python tools/publish_wiki.py --target PATH_TO_WIKI_CHECKOUT --write`,
then commit and push **inside the Wiki checkout**, not the main
repository. The publisher converts local page links to GitHub Wiki URLs
and leaves the separate Git history intact.

The test suite includes bounded loopback HTTP fixtures and native control dispatch /
repaint coverage. `cargo run -p op_browser -- --navigation-smoke-test` additionally
checks Enter input -> command channel -> navigation worker -> result channel ->
native repaint using an offline data URL; run it locally when navigation behavior changes.
`cargo run -p op_browser -- --link-smoke-test` additionally verifies a native link
click, relative-file loading, history and destination repaint using local fixtures.
External network verification is separate:
`cargo run -p op_browser -- --navigation-smoke-test https://example.com`.

The complete named-reference source is pinned in crates/op_html/data/entities.tsv.
`python tools/generate_html_entities.py --check` verifies the committed compact Rust
table locally when the generated data changes; normal builds do not run a generator or access the network. Exhaustive
Cargo tests cover every source spelling in text/attributes/RCDATA and prefix recovery.
`cargo run -p op_browser -- --link-smoke-test examples/encoding/named-references.html`
checks Unicode text and decoded links through native painting and click dispatch.
See [HTML Text Decoding](HTML-Text-Decoding.md) for provenance and supported rules.

CSS named colors are pinned in crates/op_css/data/named-colors.tsv. Run
`python tools/generate_css_named_colors.py --check` locally after changing the table representation. The Python standard-library generator validates count/aliases and
regenerates packed Rust data without network access. Normal builds need no Python. Cargo
tests exhaust every name/case variant and verify the six-byte record/static-data budget.
See [CSS Syntax Foundation](CSS-Syntax-Foundation.md).

## Image checks

Image fixtures in examples/images use only generated color pixels. Normal tests
need no image generator or external service. Optional regeneration uses Windows
System.Drawing via tools/generate_image_fixtures.ps1. Run `--image-smoke-test`
and `--link-smoke-test examples/images/index.html` locally when image behavior changes.
Codec, loopback, resource-budget, dimension and actual GDI pixel tests cover the
initial [Image Loading](Image-Loading.md) slice.

The local verification set also includes image paint and image-link smokes against
`examples/images/inline.html` when that path changes. Injected metrics verify wrapping, baseline alignment,
HTML whitespace/NBSP, br breaks, sibling/block grouping and Unicode link ranges.
Windows engine tests verify positions against real GDI extents and source-order
painting. The font adapter reuses windows-sys; no additional package install is
needed. See [Inline Layout](Inline-Layout.md).

`cargo run -p op_browser -- --resize-smoke-test` exercises real window resizing,
including a size change while reflow is in flight, final wrapping/image paint and
a native click after the geometry update. It is offline and bounded by a watchdog.
Engine tests remove loaded files before reflow, verify Arc reuse/history rollback,
and stress concurrent font realization/measurement. Native tests cover debounce,
scroll clamps, address edits and stale hit regions. See [Page Reflow](Page-Reflow.md).

## Current GitHub Actions status

GitHub Actions is enabled for pushes to `main`, pull requests targeting `main`, and manual
dispatch. The normal Windows job runs rustfmt, generated-data checks, warning-free Clippy, the
workspace test suite, native smoke tests and an optimized release build. Concurrency cancellation
prevents superseded commits from continuing to consume runner time.

A separate Windows compatibility job fetches the exact Test262/WPT revisions pinned in
`compat/upstream.env`, runs Test262 Parser v1 and WPT Static v1, and uploads their JSON/badge
data plus a Markdown summary. After successful pushes to `main`, a small write-enabled publish
job updates the `metrics` branch consumed by the README badges. PR/build jobs remain read-only;
write permission is not handed to arbitrary compatibility code.

The initial local external baselines are WPT Static v1 86/200 (43.00%) and Test262 Parser v1
364/1983 executable scripts (18.36%, 17 module entries skipped). Run the same measurement locally
with `.\tools\compatibility.ps1 -ExternalOnly -Test262Path ... -WptPath ...`.

CI supplements rather than replaces local verification: every coherent update still requires
the relevant local checks before commit/push so the public repository is not used as a rather
expensive syntax checker.
