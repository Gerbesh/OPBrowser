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
- inherited anchor hrefs on image rectangles;
- an initial table formatting context with captions, row groups, equal-width column tracks,
  2D cell placement, colspan/rowspan occupancy and cell box decoration.

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
`:is/:where/:not/:nth-child/:nth-last-child` resolve through the same matcher/specificity path.
Forgiving is/where recovery keeps valid branches; strict nth of-filters index matching siblings
in either direction and contribute maximum-filter specificity. Typed of-type pseudos
reuse sibling indexing with candidate-tag filtering. Terminal
`::before`/`::after` declarations are collected in separate `(NodeId, PseudoElement)` buckets.
Before normal value parsing, custom-property winners build inherited per-target token maps and
bounded `var()` substitution resolves references/fallbacks after dependency analysis; the computed map retains those
custom maps plus generated pseudo styles alongside host styles. During the same parent-first
walk, `counter-reset`/`counter-set`/`counter-increment` maintain scoped value stacks; `attr()`
reads the originating DOM element and `counter()`/`counters()` materialize final pseudo text.
Inherited `quotes` pairs also materialize quote commands here, using one document-order depth.
Hidden subtrees and absent/hidden pseudos do not mutate generated state; `<q>` has UA defaults.
Generated block pseudos use the same BlockContent sizing and decoration path as element blocks,
including empty generated boxes. Definite heights constrain boxes even when text overflows.
Custom values now resolve via an iterative dependency graph with fallback edges and exact
cycle components. Token/byte/depth budgets limit expansion and retained custom storage.
Computed declarations retain a value_from_var flag: invalid computed winners become unset
per supported property/component while preserving cascade priority. Malformed var() syntax
is rejected before author collection; literal invalid values remain parse-time exclusions.
Empty generated/DOM inline boxes carry edge/font metadata through the line formatter and
emit BoxDecoration without creating TextBox glyphs or artificial link ranges.
Inheritance produces per-node styles for display, text properties and the initial block box-model properties. PreparedDocument retains author/computed style data beside DOM/images; resize
reflow neither refetches nor reparses external CSS.

op_layout consumes that map for display:none/block/inline plus table/table-caption/
table-column-group/table-column/table-header-group/table-row-group/table-footer-group/table-row/
table-cell, mixed inline text runs, line-height/text-align/white-space geometry, font
style/decorations, text transforms, letter/word spacing and the expanded
[block box model](CSS-Box-Model.md). Native table roles enter a dedicated grid formatter.
It measures cell text/images for min/max track preferences, honors cell/col/colgroup width hints,
distributes the available table width across those preferences, applies horizontal/vertical
border-spacing, performs initial per-segment collapsed cell-border conflict resolution and then
repositions each cell's nested output for baseline/top/middle/bottom vertical-align after final
row heights are known. Anonymous table fixup works in both core directions without mutating the
DOM: improper table/row-group children gain layout-only rows, non-cell row children gain
layout-only cells, and consecutive orphan table-internal siblings found in normal flow are grouped
under one anonymous block table. Before grid construction, structural `display:contents` wrappers
that expose only table-internal boxes are expanded into that fixup stream; non-structural contents
wrappers remain in the collection path so their inherited text style is not lost. Orphan captions stay with that repaired wrapper and orphan
columns still feed track width hints. Real and anonymous table roots share the same table_box/grid
implementation. Auto track sizing now records percentage constraints from col/colgroup/cell widths
alongside measured content preferences. Explicit-width table-layout:fixed skips late intrinsic
content negotiation: col/colgroup widths win first, first-row explicit cell widths fill unresolved
tracks next, and remaining track space is distributed afterward. caption-side:top/bottom is
computed through the cascade and captions are wrapper siblings of the table border box rather than
being painted inside its background. display:inline-table runs the same formatter in a local
Context, packages the result as one InlineAtomic object and shrink-fits auto width against
intrinsic tracks. InlineAtomic carries vertical-align: baseline uses the first-row baseline,
top/bottom align the complete atom to the line box and middle centers it around the parent text
middle approximation; tall top/bottom atoms enlarge line height as needed. Nested decorations/
text/images/order are translated together at the aligned position, including inherited outer link
identity. Complete CSS Tables overconstraint/min-width/percentage edge algorithms, deeper colgroup
repair and non-cell collapsed-border precedence remain later. General inline vertical-align values
sub/super/text-top/text-bottom/length/% are still unsupported. Transformed text is
measured before line placement; op_paint carries italic/decoration/spacing metadata and Win32
uses matching text advances while drawing measured decoration/link segments. It resolves used widths/min/max/auto margins,
box-sizing, padding, independent border edges, fixed height constraints and sibling margin
collapse. Zero-height self-collapsing block subtrees can keep a collapsed adjoining-margin set
pending through undecorated inline/`display:contents` wrappers and empty parents, so block-in-inline
does not turn a 30px/40px collapsed pair into 70px of flow height. The normal path then emits
BoxDecoration records. The block path also tracks active float rectangles separately from normal
flow. Flow-root creates a local float scope, expands to contained float bottoms and restores the
outer scope; clear consults that active set before block placement. Floated tables continue through
the table formatter instead of generic block layout. Positioned layout tracks positioned ancestor
geometry in the same Context: relative boxes keep their normal-flow slot and translate retained
output, absolute boxes leave flow and resolve against the nearest positioned ancestor padding box,
and fixed boxes resolve against the viewport width/height. All four insets accept px/percentage
values when their axis is definite; bottom-only placement, opposing-inset auto stretching,
shrink-to-fit positioned auto widths and direct percentage-height propagation from definite block
heights are implemented. Non-replaced positioned block margin equations now distribute horizontal
and vertical auto margins between definite opposing insets, preserve negative free-space cases
and apply LTR/RTL horizontal overconstraint anchoring. Inline positioned boxes can retain a
zero-width static-position marker at the real line cursor. Horizontal inline margins, including negative values, participate in advance;
initial `display:inline-block` runs a local block/BFC layout and enters the line as one atomic box.
Block children split active inline decoration paths into continuation nodes, suppressing opposite
logical fragment edges according to computed LTR/RTL direction. Required zero-width intermediate
fragments retain line height, later descendants attach to the newest continuation, and relative
inline visual offsets are carried onto split block/float output without changing flow geometry.
Large finite CSS lengths survive parsing and are bounded when converted to used integer geometry.
Sticky positioning, full inline containing-block rectangles, complete bidi/vertical writing,
stacking/z-index, multicol and the remaining abspos constraint equations remain later. The inline formatter
injects already-resolved
generated before/after text (strings, attributes or counters) around real DOM children and emits the same BoxDecoration shape for
padded/background/bordered host or pseudo text fragments, so block and inline boxes share one
platform-neutral paint path. `visibility:hidden` is retained as layout metadata on text/image
boxes so hidden content still measures and reserves space, while op_paint omits its text/image
commands and transparent box decoration colors produce no pixels. op_paint expands visible boxes
into side-specific FillRect commands before text/images; Win32 remains only the native drawing
backend.

## Paint smoke verification

CSS-generated URLs follow ComputedPseudoStyle ordered Text/Image items into the shared
image resource loader. Effective stylesheet bases and consuming declaration provenance
resolve URLs; PageImages stores generated host/pseudo/item-index Arc pixels beside DOM images.
They enter inline baseline/wrapping or block content, then the same ImageBox/paint/GDI path,
including inherited anchor href and retained reflow. Generated images that fail load add no
inline image; surrounding generated text still paints.

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
