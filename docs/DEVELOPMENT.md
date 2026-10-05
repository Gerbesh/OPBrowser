# OPBrowser development environment

## Installed toolchain

Development host: Windows 11, x86-64.

Installed and verified:

- Rustup 1.29.1
- rustc 1.99.0, stable x86_64-pc-windows-msvc
- Cargo 1.99.0
- rustfmt
- Clippy
- rust-src
- Visual Studio Build Tools 2022 17.14.x with the C++ build tools workload
- Windows SDK installed through the Visual Studio C++ workload
- CMake 4.4.3
- Ninja 1.13.2
- Git 2.54.0
- Python 3.12.10

CMake is installed at:

C:\Program Files\CMake\bin\cmake.exe

Ninja is installed by WinGet at:

C:\Users\gerbe\AppData\Local\Microsoft\WinGet\Packages\Ninja-build.Ninja_Microsoft.Winget.Source_8wekyb3d8bbwe\ninja.exe

A newly opened terminal should receive updated WinGet/PATH environment state. The
project itself currently does not require CMake or Ninja for the Rust-only crates,
but they are installed now for future native dependencies and graphics tooling.

## Workspace crates

- op_browser: executable and browser-process bootstrap
- op_engine: engine orchestration
- op_dom: DOM data model
- op_html: HTML tokenizer/parser work
- op_css: CSS engine
- op_js: ECMAScript engine
- op_net: networking
- op_image: bounded raster image buffers and Windows WIC codec adapter
- op_layout: original text/image layout
- op_paint: platform-neutral text/image display list
- op_platform_win: Win32 platform integration

## Common commands

Format:

    cargo fmt --all

Check:

    cargo check --workspace

Lint:

    cargo clippy --workspace --all-targets -- -D warnings

Test:

    cargo test --workspace

Build debug browser:

    cargo build -p op_browser

Build release browser:

    cargo build -p op_browser --release

Run the browser:

    cargo run -p op_browser

Run the non-interactive Win32 startup smoke test:

    cargo run -p op_browser -- --smoke-test

Run the asynchronous address-input/worker/repaint smoke test (offline data URL):

    cargo run -p op_browser -- --navigation-smoke-test

Run the offline hyperlink-click/relative-file/worker/repaint smoke test:

    cargo run -p op_browser -- --link-smoke-test

Open the hyperlink example interactively:

    cargo run -p op_browser -- examples/navigation/index.html

Verify declared Windows-1251 decoding and native repaint:

    cargo run -p op_browser -- --navigation-smoke-test examples/encoding/windows-1251.html

Verify links from the same legacy-encoded page:

    cargo run -p op_browser -- --link-smoke-test examples/encoding/windows-1251.html

The fixture intentionally contains Windows-1251 bytes, not UTF-8. Preserve its
declared encoding when editing it; all Rust source and documentation remain UTF-8.

Verify the full named-reference page and its decoded Unicode query link:

    cargo run -p op_browser -- --link-smoke-test examples/encoding/named-references.html

The named-reference table is checked into Rust source; normal Cargo builds need
neither Python nor internet access. When changing the table representation, run:

    python tools/generate_html_entities.py
    python tools/generate_html_entities.py --check

The script uses only the Python 3 standard library and the pinned
crates/op_html/data/entities.tsv snapshot. CI checks generated output, and Cargo
tests independently decode all 2231 snapshot spellings through tokenizer states.
For an intentional source refresh, download https://html.spec.whatwg.org/entities.json
to a local file, then use `python tools/generate_html_entities.py --import-json PATH`.
The import validates codepoints against source characters and retains the source
SHA-256. Preserve third_party/WHATWG-HTML-LICENSE.txt with derived data and binaries.

Open an external site interactively:

    cargo run -p op_browser -- https://example.com

Verify external HTTPS through the same native input and worker path:

    cargo run -p op_browser -- --navigation-smoke-test https://example.com

Network tests use loopback TCP fixtures and do not require external access. WinHTTP
bindings reuse the existing windows-sys dependency; no new third-party package or
ready-made browser/JavaScript engine is added.

Debug executable:

    target\debug\op_browser.exe

Release executable:

    target\release\op_browser.exe

Rebuild the runnable release after a user-visible engine change:

    cargo build -p op_browser --release

CSS rendering example:

    cargo run -p op_browser -- examples/css/index.html
    target\release\op_browser.exe examples\css\index.html

The CSS example links `examples/css/theme.css`, so it exercises the external local
stylesheet loader, document-order cascade, nested block margin/padding/background/border
geometry and native paint path. The built-in start page also renders one box-model panel.

## Image support

Image checks and example:

    cargo run -p op_browser -- examples/images/index.html
    cargo run -p op_browser -- --image-smoke-test
    cargo run -p op_browser -- --link-smoke-test examples/images/index.html
    cargo run -p op_browser -- examples/images/inline.html
    cargo run -p op_browser -- --image-smoke-test examples/images/inline.html
    cargo run -p op_browser -- --link-smoke-test examples/images/inline.html

The image smoke requires a successful raster paint in WM_PAINT, not just window
startup. Image-link smoke clicks the linked PNG and paints its destination.
Color/premultiplied-alpha tests draw into real GDI DIB surfaces and inspect pixels.
Fixtures are checked in; regenerate offline with `tools/generate_image_fixtures.ps1`
(Windows System.Drawing). They use no external pictures or services.

The windows 0.62.2 binding is limited to WIC and COM feature sets in op_image.
Its 14 added packages include binding/core support and build-time procedural macros;
the codec itself remains part of Windows. Image CPU/OS working memory are separate
from OPBrowser's explicit raster/encoded budgets; keep these limits measurable.

Mixed-line layout tests use injected deterministic text metrics. On Windows,
op_engine::text measures Segoe UI with GDI and caches fonts/DCs for one render;
objects are released before the page crosses the worker/UI boundary. Native metric
tests check exact image positions/baselines and source-order paint commands. Other
platforms and standalone layout helpers use approximate metrics. This reuses the
existing windows-sys binding and adds no dependency packages. See
[Inline Layout](../wiki/Inline-Layout.md) for the current limitations.

## Resize reflow

    cargo run -p op_browser -- examples/navigation/resize.html
    cargo run -p op_browser -- --resize-smoke-test

The offline resize smoke changes the real client size, changes it again while the
worker has an older request, verifies final wrapping/image paint and clicks the
reflowed image link. A watchdog prevents a broken resize path from hanging CI.
Engine regression tests remove source files before reflow and verify shared pixels,
history/base preservation and failed-load rollback. Native tests verify debounce,
scroll bounds and address-edit/hit-region preservation. See
[Page Reflow](../wiki/Page-Reflow.md) for ownership and memory costs.

The Windows metric adapter and painter share a short GDI text mutex. Parallel tests
exposed transient implausible extents on this installation; synchronized font
creation/extents/drawing/cleanup removes the observed instability. The OS cause is
not established. Concurrent layout tests compare 128 renders against a baseline.
No networking or original layout algorithm runs under the gate.

## Dependency rule

Do not add a browser engine, rendering engine, or ready-made JavaScript engine as a
dependency. OS APIs, generated Windows bindings, codecs, TLS, Unicode data and other
infrastructure libraries are allowed only when they do not replace OPBrowser's own
HTML, CSS, DOM, layout, rendering, JavaScript, or Web API implementation.
