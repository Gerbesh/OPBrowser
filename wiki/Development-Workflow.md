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

See AGENTS.md for the exact repository rules.

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

GitHub Actions is intentionally disabled for this repository. The previous push/PR
workflow was removed because the account billing state prevented runners from starting
and every push generated a failed workflow notification without executing any checks.

Project verification is local: every coherent update still requires rustfmt, warning-free
Clippy and the workspace test suite, plus the relevant native smoke tests and release build
for the area being changed.
