# Architecture

The browser is split into small Rust crates so platform UI and web-engine logic do
not collapse into one dependency knot.

Current crates:

- op_browser: entry point, native-event routing and current navigation worker/channels.
- op_browser_core: UI-independent canonical tab/lifecycle/discard policy state.
- op_engine: renderer subsystem orchestration.
- op_platform_win: Win32 integration and current temporary GDI backend.
- op_dom: document/node storage and element attributes.
- op_html: HTML tokenizer and tree builder.
- op_css: CSS/style subsystem.
- op_layout: platform-neutral text/layout geometry.
- op_image: validated raster buffers and Windows WIC infrastructure codec adapter.
- op_paint: platform-neutral display list.
- op_js: original ECMAScript lexer/parser/bytecode/runtime foundation plus Test262 parse probe.
- op_net: source loading, initial owned HTTP URL parsing, bounded WinHTTP transport,
  response validation and native request filtering; cache/cookies remain future work.

The first visible renderer path is live:

HTML -> tokenizer -> tree builder -> DOM -> layout -> display list -> WM_PAINT -> pixels.

GDI drawing is isolated inside op_platform_win; op_engine::text supplies GDI font
metrics through the layout interface. GDI is not responsible for HTML,
CSS, layout, or paint decisions. That means the Windows graphics backend can later be
replaced with DirectWrite/Direct2D/DirectComposition without rewriting the web engine.

The current tree builder and layout are early subsets, not complete WHATWG/CSS
implementations. Compatibility work will progressively replace subset behavior with
specification-defined algorithms.

Positioned output carries a `PaintKey` with signed CSS z-index and final DOM
preorder. The original CSS subsystem computes z-index and layout tags paint
records; `PaintGroup` links positioned atomic block contexts through ancestors.
The platform-neutral painter traverses child groups iteratively so a high-z
descendant cannot escape its lower-z parent. Negative root groups precede normal
block painting, while nested negative groups follow their parent's background.
Relative inline elements now stamp their decoration fragments, text and
images with the nearest positioned inline paint key, including split lines.
An explicit inline z-index is an atomic parent context; auto-z does not trap
explicitly stacked descendants. PaintGroup also records inline ownership so
the inline ancestor background precedes its atomic inline-block descendants.
Independent inline formatting contexts reset outer inline-arena indices.
Position-relative block and inline-block owners with z-index:auto now place
their own ink into the common z=0 source order even when they have stacked
children. Because their children keep independent PaintKeys and auto-z does
not establish an ancestor context, a positive child can paint over later
siblings and a negative child can remain behind the parent's background.
Table-part positioning now resolves structural ancestors for each grid cell
and shifts paint output, not its flow slot, for relative rows, sections, and
cells. Position-relative row/section backgrounds have their own paint groups.
An absolute cell descendant can use a positioned table ancestor as its
containing block. Auto table columns use content-derived preferred widths
(including pixel-sized descendant blocks), rather than always filling the
containing block. Empty absolute-only row tracks suppress stray section pixels.
The auto-width table wrapper now shares its preferred/minimum intrinsic width
decision with column layout before calculating backgrounds, borders or captions;
horizontal border-spacing and box extras contribute to the final width. Explicit
table widths stay authoritative; fixed layout without an explicit width still
follows the auto-width intrinsic sizing path. Empty or complex spanned tables
and multi-row group backgrounds remain approximate. Rowspans now constrain the
sum of covered row heights rather than inflate the first row: after initially
measuring non-spanning cells, the final covered row absorbs missing height,
and later cell paint ranges are translated to updated row origins. This
handles overlapping rowspans and vertical border-spacing, but exact CSS
row-height distribution and complex row-group backgrounds remain partial.
The latest limited static compatibility slice suppresses raw child text in
select elements; it recognizes non-rendering SVG defs, display:contents
restrictions on SVG text/root and basic SVG use references to text definitions.
These rules currently re-use the inline text renderer, rather than a complete
SVG viewport and glyph positioning engine. The CSS color path now protects
near-black OKLab/OKLCH output from artificial chromatic clipping artifacts
while retaining bright wide-gamut conversion until full gamut mapping is
implemented consistently across color spaces.
Group opacity and a single CSS invert() filter are now computed as
non-inherited values, then represented as nested PaintGroup layers with
BeginLayer/EndLayer display-list commands. The Win32 GDI backend paints
each group to bounded black/white offscreen DIBs, recovers premultiplied
alpha from the paired images, inverts RGB within the group when requested,
and composites the group once. Native UI and WPT both use this path.
Pixel budgets and depth limits allow a fallback to visible content if
effects cannot be rendered safely. This is not a full CSS filter or
transparency-color-management implementation.
The renderer now recognizes a first single-URL CSS background-image layer
per eligible block/table box, uses the computed cascade to track its
origin stylesheet, resolves it through the existing budgeted image
resource loader, and paints repeating intrinsic-size raster tiles clipped
to the background border box. Windows WIC decodes embedded ICC profiles
through a color-context transform into the output sRGB surface; PNGs
without a profile keep the original path. Link matching treats an empty
href as a visited self-navigation to the already active document, but
arbitrary navigation-history styling is still intentionally absent.
These slices raised frozen Static v1 to 194/200 (97.00%). Named
CSS Color 5 @color-profile handling now extracts bounded ICC declarations
from inline/linked CSS, resolves their URLs through the same filtered
resource-loader infrastructure, and resolves 3-component color(--profile ...)
tokens at author-style preparation time using WIC's ICC to sRGB
transform. Repeated conversions are memoized within the prepared page;
quoted CSS strings, unknown profiles and other color syntaxes are not
rewritten. Static v1 reaches 195/200 (97.50%). This is an initial
custom RGB profile path, not full CSS Color 5 color management.
The CSS text pipeline now supports a first ::first-line/:first-line
fragment styling slice: selector matching and computed fragment pseudo
styles feed optional first-line ink data to the inline line assembler.
The first actual flushed line receives its inherited pseudo color and
background; future lines remain unaffected. The background ink uses
the same font-metrics height and baseline as an ordinary inline box
rather than the full line-height. The frozen strict Static score reaches
196/200 (98.00%); dynamic font/gradient/shadow/currentcolor first-line
corner cases remain partial.
Currentcolor-dependent background-color and border-side provenance now
survive from computed styles into inline box decorations. ::first-line
resolves dependent backgrounds and borders at paint time for descendants
inheriting the host color while preserving explicit child colors and later
line fragments. Strict WPT Static v1 improves 196/200 -> 197/200 (98.50%);
XYZ precision and two pinned Rec.2020 reference discrepancies remain.
The user-agent default font size is now 16 CSS px instead of 18,
matching common desktop browser defaults. The pinned WPT probe
preserves the strict exact-color score and optionally reports source-
authored fuzzy allowances with --report-wpt-fuzzy. That separate report
reads HTML metadata through the own HTML tokenizer and checks both
maximum per-channel difference and total pixels against inclusive
ranges. With xyz-003's original 0-1 / 0-18432 metadata and the corrected
font-size geometry, WPT-metadata Static is 198/200 (99%) while strict
Static remains 197/200 (98.5%). Rec.2020 reference mismatches remain.
This is not full CSS stacking: additional context triggers, other auto-z
paint-phase details and exact interleaving remain future work. Win32 drawing
never decides paint order.

External HTML travels from address input through a worker-owned Engine and op_net
into the same original renderer. Structural containers preserve nested heading /
paragraph defaults. Windows WinHTTP supplies HTTP/TLS/proxy/framing/decompression
only; it is an OS infrastructure API, not a browser engine. TLS certificate checks
remain enabled. The UI owns window handles and updates them only on its own thread.

Layout preserves inline href metadata as LinkSpan byte ranges on each wrapped line.
Paint commands carry these ranges to GDI, which measures LinkRegion bounds for
native cursor/click hit testing. Engine tracks the last loaded document address
separately from history requests so link bases follow redirects during reload.

Display-list, scroll and measured link-region storage are currently process-global
because M1 still has one browser window and one worker-owned renderer.

The target process model is fixed by ADR-0002: the browser process owns windows, tabs,
lifecycle policy, permissions and renderer supervision; renderer processes own
Engine/DOM/CSS/layout/paint/JavaScript state. The first migration target is one renderer
process per active tab, followed by sandboxing and only then measured renderer sharing.
The current worker is a staging implementation, not the final ownership model.

Document decoding is owned by op_net::encoding: compact Windows-1251/1252 tables,
strict Unicode conversion, charset aliases and a bounded byte-level meta prescan.
HTTP, local-file and data loaders pass decoded Unicode to the original HTML parser.
op_html::references consumes all standard named and numeric character references inside
tokenizer text/attribute states; initial raw-text/RCDATA handling preserves the
context rules. Its Characters result carries one or two Unicode scalars. A sorted
eight-byte NamedEntry table and packed strings replace per-name pointers; prefix
ranges limit lookup to 31 input characters without allocation. A Python standard
library tool generates the table offline from pinned WHATWG data; Cargo builds use
the committed Rust output. No ready-made parsing/browser engine is involved.

Raster image subresources are coordinated by op_engine::images and loaded as binary
bytes by op_net::images. The new op_image crate uses targeted Windows WIC/COM bindings
to decode supported Microsoft raster formats into premultiplied BGRA. No codec
object crosses the worker/UI boundary. Arc pixels reach original ImageBox layout
and Image paint commands; a transient GDI DIB/DC draws them with alpha.
Image rectangles inherit anchor hrefs and use existing native hit testing.
See [Image Loading](Image-Loading.md) for budgets, source policy and current limits.

Layout now accepts a TextMeasurer interface. op_engine::text owns a per-render GDI
font/DC cache on Windows and returns numeric TextMetrics; no OS handle crosses
into layout or the UI result. op_layout::flow groups inline siblings around blocks,
and op_layout::inline constructs measured lines with a shared text/image baseline.
LayoutTree::order preserves TextBox/ImageBox source order in painting. Engine and
native painter share op_paint::TEXT_FONT_FAMILY; portable layout uses approximate
metrics. See [Inline Layout](Inline-Layout.md) for the supported formatting subset.

Engine retains one PreparedDocument after a successful navigation, containing DOM,
effective address/MIME and Arc-shared image resources. Reflow borrows this snapshot
on the worker and performs no loading or history changes. Resize debounce and
viewport-tagged results prevent stale-width presentation; native present_reflow
preserves address edits/scroll bounds and rebuilds link regions. The worker and UI
share a short GDI text gate for font creation, use and cleanup. See
[Page Reflow](Page-Reflow.md) for lifetime/costs and verification.

Request filtering is owned by op_net and is applied before current document, stylesheet
and image loads. The initial Adblock-style subset supports host/wildcard network rules,
exceptions, resource types and per-site allowlisting; see [Request Filtering](Request-Filtering.md).

The first op_js execution slice is deliberately independent of DOM scripting: lexer -> AST
-> bytecode -> VM, with a parse-only Test262 probe. See [JavaScript Engine](JavaScript-Engine.md).

For text, ADR-0003 permits DirectWrite as a focused Windows shaping/raster infrastructure
service behind the platform-neutral text interface. OPBrowser still owns CSS font policy,
line breaking, layout and paint semantics.
