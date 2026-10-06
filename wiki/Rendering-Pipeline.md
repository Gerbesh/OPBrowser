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
  -> op_engine::text GDI TextMeasurer
  -> op_layout::layout_document_with_metrics
  -> LayoutTree
  -> op_paint::build_display_list
  -> DisplayList
  -> UI result channel -> op_platform_win::NativeBrowserWindow::present
  -> WM_PAINT
  -> Win32 GDI
```

## Why GDI is acceptable here

GDI supplies font extents and the temporary Windows pixel-output backend. It does not parse HTML, run
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
- data:text/html URLs using percent-encoded document bytes and charset parameters;
- data:text/html;base64 URLs.
- HTTP/HTTPS HTML via the system WinHTTP transport/TLS/proxy API.

Network loads validate status, media type and charset, limit decoded HTML to 2 MiB,
and follow at most five redirects. See [Document Source Loading](Document-Source-Loading.md)
for the precise initial URL/encoding/timeout limits.

All document loaders share owned byte decoding for UTF-8, UTF-16, Windows-1251 and
Windows-1252. BOM/header/early-meta selection happens before HTML tokenization.
The tokenizer decodes the full named-reference table and numeric references in text/attributes while
keeping escaped markup as text; raw-text/RCDATA context preserves script/style
content. One- and two-scalar replacements reach the DOM and UTF-8 link ranges without
being reparsed as markup. See [HTML Text Decoding](HTML-Text-Decoding.md) for supported
rules and the remaining tokenizer/encoding limits.

## Current layout subset

The M1 layout layer currently provides:

- body-root selection;
- hidden head/style/script content filtering;
- basic h1-h6/p/li defaults;
- block traversal inside div/main/section and other structural containers;
- measured word wrapping on Windows, approximate portable fallback;
- adjacent inline nodes grouped around block children;
- platform-neutral text/image boxes in source-order paint sequence;
- UTF-8 href spans across nested inline labels and wrapping;
- HTML whitespace collapsing, preserved NBSP and explicit/repeated br breaks;
- image boxes with intrinsic/HTML dimensions, viewport fitting and alpha pixels;
- mixed text/image lines with shared baselines and inline alt fallback;
- inherited anchor hrefs on image rectangles.

Text paint commands carry LinkSpan byte ranges. The native painter draws linked
ranges blue/underlined and measures their glyph bounds with GDI; cursor and click
hit testing use these regions in document coordinates with the toolbar/scroll
offset applied. Page replacement clears stale regions. Address resolution remains
in op_net/Engine rather than the graphics backend.
Image commands carry shared premultiplied BGRA pixels. A transient GDI surface
draws only visible images with AlphaBlend; the source bitmap/DC are released after
drawing. Clickable rectangles use the same document-coordinate hit regions.
See [Image Loading](Image-Loading.md) for the complete resource-to-pixels path.
See [Inline Layout](Inline-Layout.md) for font measurement, line building and limits.
Successful navigation retains the DOM and shared images for
[Page Reflow](Page-Reflow.md). WM_SIZE triggers a debounced worker layout rebuild;
only results matching the current viewport reach present_reflow and native paint.

The [CSS foundation](CSS-Syntax-Foundation.md) now participates in page preparation.
After HTML parsing, [stylesheet loading](Stylesheet-Loading.md) fetches eligible external
CSS on the navigation worker. Loaded link CSS and embedded style rules are collected in
DOM source order, inline declarations join the author cascade, functional selectors
`:is/:where/:not/:nth-child` resolve through the same matcher/specificity path. Terminal
`::before`/`::after` declarations are collected in separate `(NodeId, PseudoElement)` buckets.
Before normal value parsing, custom-property winners build inherited per-target token maps and
recursive `var()` substitution resolves references/fallbacks; the computed map retains those
custom maps plus generated pseudo styles alongside host styles. Inheritance produces per-node
styles for display, text properties and the initial block box-model properties. PreparedDocument retains author/computed style data beside DOM/images; resize
reflow neither refetches nor reparses external CSS.

op_layout consumes that map for display:none/block/inline, mixed inline text runs,
line-height/text-align/white-space geometry, font style/decorations, text transforms,
letter/word spacing and the expanded [block box model](CSS-Box-Model.md). Transformed text is
measured before line placement; op_paint carries italic/decoration/spacing metadata and Win32
uses matching text advances while drawing measured decoration/link segments. It resolves used widths/min/max/auto margins,
box-sizing, padding, independent border edges, fixed height constraints and sibling margin
collapse, then emits BoxDecoration records. The inline formatter injects generated before/after
quoted strings around real DOM children and emits the same BoxDecoration shape for
padded/background/bordered host or pseudo text fragments, so block and inline boxes share one
platform-neutral paint path. op_paint expands them into side-specific FillRect
commands before text/images; Win32 remains only the native drawing backend.

## Paint smoke verification

NativeBrowserWindow::create calls UpdateWindow after ShowWindow. The WM_PAINT handler
sets an atomic painted-once flag. The --smoke-test mode fails if that flag is not set.
The --image-smoke-test mode additionally requires a successful raster draw after
address input/worker loading. Codec tests check all supported formats and budgets;
GDI surface tests inspect scaled colors and alpha-composited pixels. Native tests
exercise image hit bounds, scroll offsets, click dispatch and stale-region cleanup.

The repository also smoke-tests examples\hello.html and an HTML data URL through
the source-to-pixels path. --navigation-smoke-test uses queued native Enter input,
worker navigation and display-list replacement, then checks real replacement
painting. Passing https://example.com additionally verifies external HTTPS without
making normal CI tests depend on public network access.

--link-smoke-test loads examples/navigation/index.html, queues a native click on its
first visible link, resolves the relative local URL on the worker and checks the
destination painting/history. It runs in CI using only repository fixtures.
The Windows-1251 fixture in examples/encoding verifies legacy bytes -> Cyrillic
pixels through the same native navigation smoke path.
