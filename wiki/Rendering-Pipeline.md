# Rendering Pipeline

OPBrowser has a complete initial local/data/external static-document rendering slice.

## Current pipeline

```text
HTTP(S) URL / filesystem path / file: URL / data:text/html URL
  -> op_net::NetworkContext::load_document
  -> LoadedDocument
  -> Engine::navigate / render_source (navigation worker)
  -> op_html::Tokenizer
  -> op_html::parse_document
  -> op_dom::Document
  -> op_layout::layout_document
  -> LayoutTree
  -> op_paint::build_display_list
  -> DisplayList
  -> UI result channel -> op_platform_win::NativeBrowserWindow::present
  -> WM_PAINT
  -> Win32 GDI
```

## Why GDI is acceptable here

GDI is only the temporary Windows pixel-output backend. It does not parse HTML, run
CSS, create layout, or decide what should be painted. Those decisions are owned by
OPBrowser crates.

The planned Windows rendering evolution is:

GDI bootstrap -> DirectWrite text -> Direct2D/Direct3D composition ->
DirectComposition where useful.

## Current source subset

The source loader currently accepts:

- direct/relative filesystem paths;
- Windows drive paths without mistaking C: for a URI scheme;
- file: URLs with percent decoding;
- data:text/html URLs using percent-encoded UTF-8;
- data:text/html;base64 URLs.
- HTTP/HTTPS UTF-8 HTML via the system WinHTTP transport/TLS/proxy API.

Network loads validate status, media type and charset, limit decoded HTML to 2 MiB,
and follow at most five redirects. See [Document Source Loading](Document-Source-Loading.md)
for the precise initial URL/encoding/timeout limits.

## Current layout subset

The M1 layout layer currently provides:

- body-root selection;
- hidden head/style/script content filtering;
- basic h1/h2/h3/p defaults;
- block traversal inside div/main/section and other structural containers;
- approximate word wrapping;
- vertical text flow;
- platform-neutral text boxes.
- UTF-8 href spans across nested inline labels, whitespace normalization and wrapping.

Text paint commands carry LinkSpan byte ranges. The native painter draws linked
ranges blue/underlined and measures their glyph bounds with GDI; cursor and click
hit testing use these regions in document coordinates with the toolbar/scroll
offset applied. Page replacement clears stale regions. Address resolution remains
in op_net/Engine rather than the graphics backend.

This is deliberately small. It exists to prove subsystem boundaries and the complete
path to pixels before expanding CSS/layout complexity.

## Paint smoke verification

NativeBrowserWindow::create calls UpdateWindow after ShowWindow. The WM_PAINT handler
sets an atomic painted-once flag. The --smoke-test mode fails if that flag is not set.

The repository also smoke-tests examples\hello.html and an HTML data URL through
the source-to-pixels path. --navigation-smoke-test uses queued native Enter input,
worker navigation and display-list replacement, then checks real replacement
painting. Passing https://example.com additionally verifies external HTTPS without
making normal CI tests depend on public network access.

--link-smoke-test loads examples/navigation/index.html, queues a native click on its
first visible link, resolves the relative local URL on the worker and checks the
destination painting/history. It runs in CI using only repository fixtures.
