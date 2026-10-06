# OPBrowser Code Slices

Last updated: 2026-10-06

A code slice is an end-to-end path through the architecture that produces one
observable capability. This prevents isolated subsystems from becoming impressive
piles of code that never form a browser.

## S0 - Native process startup

Status: COMPLETE at M0 level.

```text
op_browser::main
  -> Engine::start
  -> NativeBrowserWindow::create
  -> Win32 CreateWindowExW
  -> NativeBrowserWindow::run_message_loop
```

## S1 - Local/data document to pixels

Status: COMPLETE at initial M1 level.

```text
filesystem path / file: URL / data:text/html URL
  -> NetworkContext::load_document
  -> LoadedDocument
  -> Engine::navigate
  -> HTML tokenizer/tree builder
  -> DOM
  -> layout
  -> display list
  -> Win32 WM_PAINT/GDI
  -> pixels
```

Verified with examples\hello.html and data:text/html startup smoke tests.

HTML comments follow Tokenizer::consume_comment -> Token::Comment -> DOM Comment.
They keep document order and split adjacent DOM text nodes at the real comment boundary,
but CSS :empty ignores them and layout emits no inline/block item for them. Normal, abrupt
and EOF closing recovery has tokenizer coverage; Engine compares exact display lists
against comment-free source before/after reflow. Raw-text/RCDATA comment markers remain
literal text. Doctype declarations follow Tokenizer::consume_doctype -> Token::Doctype ->
the first pre-element DOM DocumentType node; later/in-element doctypes are ignored.
Typed nodes retain lowercase names, missing/empty PUBLIC/SYSTEM identifiers and
force-quirks recovery. The initial insertion phase now classifies the document as
NoQuirks, LimitedQuirks or Quirks using the full WHATWG legacy doctype matrix; missing,
malformed or late doctypes select Quirks, and leading initial ASCII whitespace is ignored.
Unknown <! declarations and HTML CDATA-like declarations become Comment nodes; token/tree
tests verify recovery and Engine tests compare exact rendering/reflow against
declaration-free source. Mode-specific layout quirks, processing instructions and
foreign-content context remain later.

## S2 - Navigation to static page

Status: COMPLETE for address-driven static HTML navigation.

Working core:

```text
Engine::navigate(request)
  -> load + render succeeds
  -> NavigationState::commit_navigation
  -> NavigationEntry

Engine::go_back / go_forward
  -> historical request
  -> reload source
  -> move current_index only after success

Engine::reload
  -> current historical request
  -> reload/render
  -> history unchanged
```

Implemented:

- navigation entries with original request, normalized address and MIME type
- current index
- can_go_back / can_go_forward
- back / forward
- reload
- forward-branch truncation after new navigation
- failed-navigation rollback by commit-after-success design

User-visible path:

```text
address Enter / hyperlink click / Go / Back / Forward / Reload / startup URL
  -> Win32 NavigationEvent
  -> op_browser command channel
  -> worker-owned Engine navigation
  -> op_net HTTP URL parser
  -> WinHTTP GET + TLS + redirect/framing/decompression
  -> bounded bytes -> BOM / charset / early meta -> owned decoding to Unicode
  -> own tokenizer/DOM/block text layout/display list
  -> result channel polled only during loading
  -> NativeBrowserWindow::present + status/address/history controls
  -> WM_PAINT/GDI pixels
```

Ctrl+L focuses/selects the address, F5 reloads, and wheel input scrolls. Failure
preserves the previous display list and history. Native button/Enter dispatch and
replacement painting have an automated Win32 test. Loopback HTTP tests cover GET,
redirects, cookie suppression, gzip, chunked bodies, status/MIME errors, size limits,
redirect loops, rendered text and history preservation after failed navigation,
back and reload. The asynchronous external HTTPS path is manually smoke-tested with
`--navigation-smoke-test https://example.com`; normal CI uses offline/local fixtures.

The hyperlink path preserves UTF-8 href spans through whitespace normalization,
nested inline labels and wrapping. GDI measures painted label bounds; hit testing
excludes ordinary text, toolbar and off-screen points, compensates for scrolling,
and clears stale regions after page replacement. Engine::follow_link uses the last
successful effective URL, including redirects during reload, before committing a
new history entry. Absolute HTTP(S), relative path/root/query/network references,
dot segments and local-file relatives are supported. The offline `--link-smoke-test`
clicks the first link in examples/navigation/index.html and paints its destination;
CI runs this with no external network dependency.

The text decoding slice now accepts Windows-1251/1252 and UTF-16 alongside UTF-8,
with BOM/transport/early-meta precedence. All standard named and numeric references are
consumed in the tokenizer, so `&amp;` in hrefs becomes the actual query separator,
and escaped `<` remains text rather than becoming markup. Raw-text/RCDATA contexts
keep script/style source literal and decode references in title/textarea text.
HTTP regression fixtures and examples/encoding/windows-1251.html verify Cyrillic,
link metadata, loaded bytes -> pixels and subsequent link navigation in CI.
The full named-reference path also handles two-scalar replacements, longest matches
and legacy prefix/attribute ambiguity. All 2231 source spellings are verified in
text, three attribute-value states and RCDATA against the pinned WHATWG snapshot.
examples/encoding/named-references.html verifies exact painted text, Unicode link
byte ranges, decoded query values and subsequent native hyperlink navigation.

Limits: initial URL/encoding subsets; no CSS or scripts yet. Raster images are
supported by the S2a slice below.
Links open in the current window. Fragment links reload the document without anchor
scrolling; HTML base elements, target/download behavior, other legacy encodings and full
WHATWG URL processing remain future work. Resize reflow is covered by S2b below.

## S2a - Image subresource to pixels and link behavior

Status: COMPLETE at initial M1 level.

```text
loaded HTML -> DOM img src (character references already decoded)
  -> op_engine::images visible-node traversal / effective document base
  -> op_net::images relative-source policy / bounded binary HTTP, file or data loading
  -> op_image Microsoft WIC codec / first frame / preallocation pixel limits
  -> Arc<RasterImage> premultiplied BGRA shared across repeated sources
  -> op_engine::text GDI font metrics / portable approximate fallback
  -> op_layout::flow inline grouping around block children
  -> op_layout::inline measured TextBox + ImageBox lines / wrapping / shared baseline
  -> LayoutTree::order / op_paint source-ordered Text and Image commands
  -> op_platform_win transient DIB + GDI AlphaBlend -> pixels
  -> image rectangle / scroll-aware hit test / click -> S2 navigation
```

Failed/blocked/over-budget images produce styled nonempty alt text or transparent replacement
geometry with empty/absent alt, and
allow document navigation/history to succeed. Hidden head/script/style/template
subtrees do not request images. Budgets limit node count, unique attempts, accepted
encoded bytes and decoded pixel storage; caches are local to each rendered page.
The navigation worker loads images serially before publishing the page, keeping
Windows UI operations on the UI thread. See wiki/Image-Loading.md for exact limits.

Verified with color/alpha GDI pixel assertions, all four codec fixtures, loopback
redirect/cache/error tests, data image/node/pixel budgets and native image paint/link
smokes, plus mixed-line metrics, baseline/wrapping, Unicode href byte ranges,
block boundaries, repeated br, HTML whitespace/NBSP and bounded long-word probing.
`examples/images/inline.html` exercises text/image order and linked-image clicks
through native paint/link smokes in CI. Font extents use the same Segoe UI settings
and href-run segmentation as painting; original layout chooses line breaks.
Full CSS inline formatting, shaping/bidi/grapheme line breaking, progressive
results, GIF animation, srcset/picture, EXIF orientation and color management
are future work. See wiki/Inline-Layout.md for the supported subset.

## S2b - Window resize to retained-page reflow and link behavior

Status: COMPLETE at initial M1 level.

```text
WM_SIZE -> toolbar layout / 120 ms debounce -> NavigationEvent::Resize
  -> op_browser command with latest viewport -> worker-owned Engine::reflow
  -> retained PreparedDocument DOM + Arc image resources (no network/file access)
  -> measured original layout -> new DisplayList + requested viewport dimensions
  -> UI checks current viewport, discards stale geometry and requests newest size
  -> present_reflow / preserve address edit / clamp scroll / rebuild link regions
  -> WM_PAINT -> native hyperlink click -> S2 navigation
```

The startup page also retains its DOM without a history entry. Successful
navigation/back/forward/reload replaces the single active snapshot; failed loading
keeps it. Reflow does not mutate history or the effective document base. Back and
reload still fetch their historical source; this is not a back/forward page cache.
Original decoded HTML is dropped after parsing; retained DOM strings have their
own memory cost. Image pixels are Arc-shared with paint commands rather than copied.

Verification deletes loaded HTML/image files before narrowing/restoring the page,
asserts identical restored painting and shared image allocation, and checks failed
reload/navigation rollback and start/back/forward/reload snapshots. Native tests
check debounce, scroll preservation/clamping, address edits and stale hit cleanup.
`--resize-smoke-test` resizes the real window both while idle and while an older
reflow is in flight, verifies final 320-pixel wrapping/raster paint, then clicks
the image link and paints its destination. It has a ten-second watchdog and runs
offline in CI. Concurrent font regression tests exercise the shared GDI text gate.

Limits: full relayout after a 120 ms pause, no incremental DOM invalidation, semantic
scroll anchoring or history page cache. Current single-window ownership still applies.

## S3 - CSS-styled document

Status: IN PROGRESS. Embedded, inline and initial external author CSS plus the first
block-level box model are end-to-end through native pixels; broader CSS remains.

Current path:

```text
HTML
  -> op_html DOM
  -> op_engine stylesheet discovery
       -> op_net local/file/data/HTTP(S) CSS loading
       -> loaded CSS attached to each <link rel=stylesheet> NodeId
       -> effective stylesheet addresses retained after redirects/cache reuse
  -> collect linked + <style> rules in DOM order + style="" declarations
  -> op_css tokenizer/parser
       unquoted Url tokens + quoted url()/String components
       BadUrl/BadString reject whole declarations, including custom/fallback values
  -> Selector AST + Specificity
       type/universal/class/ID + attribute operators
       descendant/child/adjacent/general-sibling combinators
       root/child/empty/link pseudo-classes
       forgiving is/where + strict not + filtered nth-child/nth-last-child(An+B of S)
       first/last/only-of-type + nth-of-type/nth-last-of-type(An+B)
       terminal ::before / ::after pseudo-element targets
  -> right-to-left selector matching against op_dom
  -> StyleMap host buckets + (NodeId, PseudoElement) author buckets
       MatchedDeclaration.style_node preserves embedded/link/inline provenance
  -> cascade: !important -> inline source -> specificity -> source order
  -> inherited CustomPropertyMap per element/pseudo target
       case-sensitive --name winners -> directed dependency graph/SCC -> bounded var() resolution/fallbacks
  -> substitute var() tokens before supported normal-property value parsing
  -> inheritance + initial/inherit/unset
  -> ComputedStyleMap { display, color, font-size, font-weight/font-style,
                        line-height, text-align, white-space, text-decoration-line,
                        letter-spacing, word-spacing, text-transform,
                        background-color, margin/padding edges, border edges,
                        width/height min/max, box-sizing }
     + ComputedPseudoStyle { inherited host style + pseudo declarations + generated content }
     + CustomPropertyMap snapshots for host/pseudo diagnostics and inheritance
  -> retained in PreparedDocument
  -> op_layout injects ::before before DOM children and ::after after DOM children
  -> op_layout display/block/inline decisions + mixed inline style runs
       + block margin/border/padding content geometry
       + BoxDecoration background/border rectangles
  -> TextBox color/size/weight + BoxDecoration geometry
  -> op_paint text plus background/border FillRect commands
  -> Win32 GDI pixels
  -> resize reflow reuses retained DOM/images/author/computed styles
```

The value subset accepts display inline/block/none, font-size keywords/percent/lengths,
font-weight normal/bold/bolder/lighter and numeric 1-1000 (currently mapped to the native
normal/bold backend), font-style normal/italic/oblique, text-align start/end/left/right/center,
line-height normal/unitless/percent/length, white-space normal/nowrap/pre/pre-wrap/pre-line,
text-decoration-line none/underline/line-through combinations, letter/word spacing lengths,
text-transform none/uppercase/lowercase/capitalize, color-only `background` shorthand,
#RGB(A)/#RRGGBB(AA), all 148 opaque CSS named colors with aliases, transparent,
and legacy/modern rgb()/rgba()/hsl()/hsla(). Functional RGB accepts numeric
or percentage channels plus number/percentage alpha; HSL accepts deg/grad/rad/turn hue,
legacy percentage or modern percentage/number saturation/lightness and alpha. Modern RGB/HSL
accept none components; legacy RGB channels must use uniform number/percentage units. Channels
clamp to the CSS output range. The
global keywords inherit/initial/unset remain shared. color/font-size/font-weight/font-style/line-height/text-align/white-space/letter-spacing/word-spacing/text-transform inherit; display
does not unless explicitly set to inherit. Unsupported/invalid literal values are discarded
before cascade winner selection so a lower-priority valid declaration may still win.
Custom property names remain case-sensitive and inherit by default. Their selected token values
are resolved on the element where they are computed, so an inherited `--frozen:var(--accent)`
does not rebind when a child later overrides `--accent`. `var()` supports nested fallbacks and
directed dependency-cycle invalidation including unused fallback branches, and substitution works inside shorthands, functional
colors, dimensions and generated `content`. A value_from_var candidate retains its cascade
priority when substitution fails, expands to empty or produces the wrong property grammar;
it resolves to unset per affected longhand rather than revealing an older candidate. This
also suppresses invalid generated content and restores inherited quotes/initial counters.
Malformed var() syntax is rejected before cascade, including malformed unused fallbacks.
UA defaults preserve M1 block/hidden behavior and heading typography. Former semantic
heading/paragraph/list spacing now lives in computed margins and goes through the same
block geometry path as author margins. Inline text runs may differ in size, weight/style,
line-height, decoration, spacing, transform and color while sharing a baseline. text-align
offsets each completed line inside its actual content box. white-space controls collapse,
preserved newlines/spaces and soft wrapping. text-transform runs before measurement so
Unicode expansions and link byte ranges stay aligned with the transformed display text.
Non-replaced inline elements with background-color/padding/solid borders now contribute
horizontal fragment width during wrapping/alignment and emit per-line BoxDecoration geometry.
Generated `::before`/`::after` quoted strings enter that same inline item stream. Their
computed style inherits host typography, may override normal supported properties, and box
identity includes the pseudo target so host/before/after fragments cannot accidentally merge.
RGBA text and box colors are currently composited
over the white page background before native painting. Anchors with href receive blue and
underline UA defaults before author cascade. Link glyphs/decorations consume computed text
presentation in GDI, including nested/generated runs; LinkSpan still supplies measured,
scroll-aware click regions independently of presentation.

External stylesheet loads are bounded and nonfatal: up to 32 link nodes, 8 distinct
requests, 1 MiB per stylesheet and 2 MiB decoded CSS per document. The current activation
subset accepts normal screen/all stylesheets and skips alternate/disabled/print links.
Resize reflow reuses retained author/computed styles without refetching CSS.

Block sizing now accepts `margin`/`padding` shorthands and side longhands; margin supports
`auto`, negatives and percentages while padding rejects negatives. Width/height and min/max
accept px/%, em/rem and CSS absolute units; horizontal percentages resolve against the
containing-block width. `box-sizing: content-box|border-box` affects used sizing. Percentage
height values are parsed but remain auto-like when no definite containing height is available.
The current rem conversion uses OPBrowser's initial root font-size baseline rather than a
recomputed author-modified root rem basis.

Borders support `border`, per-side shorthands, `border-width/style/color`, and their side
longhands. Width keywords thin/medium/thick and `currentColor` work; styles are currently
`none` and `solid`. Shorthand and longhand candidates compete through the normal cascade
rather than by hard-coded application order. Painting carries independent widths/colors
for all four sides.

Adjacent sibling block margins collapse using CSS positive/negative margin arithmetic.
Whitespace-only inline text between block siblings no longer creates a line or breaks that
collapse. Parent/child and empty-block margin collapsing are deliberately not implemented
in this slice. Author box geometry applies to ordinary non-replaced block boxes and to initial non-replaced
inline fragments. Inline fragments clone their left/right edge treatment on each wrapped line
in this initial implementation, and vertical padding/borders expand safe line geometry to
avoid paint overlap. Nested decorated stacks and replaced image own boxes now share this
formatter; sliced edge treatment and complete CSS inline vertical positioning remain later.

Selector matching supports attribute existence/equality/token/dash/prefix/suffix/substring
operators with explicit ASCII `i`/`s` flags, adjacent/general sibling combinators that ignore
intervening text nodes, :root/:first-child/:last-child/:only-child/:empty/:link, plus
`:is()`/`:where()`/`:not()` nested selector lists and filtered `:nth-child()`/`:nth-last-child()`.
`:is()`/`:not()` use the maximum valid argument specificity, `:where()` contributes zero,
and nth selectors add one class-level component plus the maximum `of` filter specificity.
Filters count matching element siblings once, with optional reverse indexing. An+B uses
token-aware signed/unsigned integer and n-dimension grammar instead of whitespace concatenation.
Terminal `::before`/`::after` add one type-level specificity
component and are collected into independent pseudo buckets. Pseudo-elements inside
`:is()`/`:where()` are discarded as invalid argument branches. These two functions forgive
unsupported/malformed branches, including empty/all-invalid lists that match nothing.
`:not()` and nth `of` lists reject invalid branches/pseudo-elements. Ordinary top-level
selector lists remain strict; nested functional selectors are bounded to 64 levels.
Typed first/last/only-of-type and nth-of-type/nth-last-of-type filter siblings by the
candidate's HTML tag name and reuse the same indexing/An+B arithmetic. They contribute
one class-level specificity component and do not accept `of` lists or add filter specificity.

Functional colors feed the same computed CssColor path for text, backgrounds, border-color
longhands/lists and border shorthands. A color-only `background` shorthand (including `none`,
transparent and global keywords) competes with `background-color` through the same cascade
key instead of fixed application order. Alpha
still composites against the current white page background in op_paint, so true layered
translucent backgrounds remain later rendering work.

Modern hwb() resolves percentage/number whiteness/blackness, hue units, optional alpha and
none components through the same cascade and text/background/border/pseudo path. White+black
at or above 100% becomes normalized gray, including individual values above 100%. Shared
unquantized HSL channels avoid intermediate byte rounding; hue-unit normalization prevents
large-angle overflow. Missing components become zero for current used-color painting;
preserving missing components for interpolation/serialization, calc() and relative colors
remain later work. CSS/Engine tests check primary examples, invalid-value cascade recovery,
custom-property substitution, native display-list colors and retained reflow.

Named colors follow identifier/escape tokenization -> allocation-free ASCII case-insensitive
op_css::named binary search -> CssColor -> normal cascade/inheritance/var()/pseudo/box paint.
Pinned W3C TSV input and offline generator produce packed names and six-byte RGB records;
all 148 spellings and uppercase variants are exhaustively checked against source values.
Transparent/currentcolor remain special keywords outside the opaque table. Engine coverage
verifies expanded text/background/border names and stable retained reflow.

Predefined color(srgb ...)/color(srgb-linear ...) accepts number/percentage channels,
optional slash alpha and none; the modern component parser is shared with HWB. Linear-light
values use the sRGB transfer curve before final 8-bit CssColor encoding and existing text/
background/border/pseudo paint. Initial channel clipping is explicit; wide-gamut spaces,
perceptual gamut mapping and color-space/missing-component preservation remain later work.
Reference/transfer-boundary, malformed syntax/cascade, var() and Engine reflow tests cover
the two supported spaces without adding dependencies or touching raster decoding.

Modern RGB/HSL now share component/slash-alpha parsing with HWB/color(). Missing channels and
alpha become zero for current painting; numeric HSL saturation/lightness use the 0..100
percentage scale. Legacy comma RGB rejects mixed number/percentage channels and none; comma
HSL keeps percentage-only saturation/lightness. Regressions distinguish parse-time literal
recovery from invalid-at-computed-time var() unset semantics and cover native paint/reflow.

Planned next path:

```text
Rendering/property expansion
  -> sliced inline decoration edges / broader computed values
  -> broader custom-property grammar/registration/animation-taint semantics
  -> additional computed properties
```

Generated content accepts mixed quoted strings, `attr(name)`, `counter()` and `counters()`;
`none`/`normal` suppress the pseudo box. Counter state comes from initial
`counter-reset`/`counter-set`/`counter-increment` parsing with sibling-aware nested scopes and
basic decimal/alpha/roman formatting. Inherited `quotes` auto/none/string pairs feed open/close
and no-open/no-close commands with document-order depth; `<q>` gets UA before/after defaults.
Only the winning emitted content changes depth. Hidden subtrees/absent pseudos do not change
quotes or counters. Pairs repeat at deeper nesting; unmatched closing commands have no effect.
Computed pairs and materialized text/image lists survive retained reflow.
Language-aware automatic quote selection remains later (`auto` currently uses English pairs).
`display:block` generated content shares ordinary BlockContent geometry: width/height/min/max,
box-sizing, percentage sizing, auto/negative margins, padding, borders, backgrounds and sibling
margin collapse. Empty generated block strings still materialize sized/decorated boxes without
inventing text or changing the DOM. Fixed block heights bound flow and decoration while text
may overflow. Empty generated and visually empty ordinary inline boxes carry an EmptyInline item
through the formatter: padding/border width contributes to wrapping/nowrap/alignment, font
and vertical-edge extents contribute to the line, decorations paint without emitting glyphs
or TextBox/LinkSpan commands. Empty descendants without their own box do not duplicate an
inherited decoration.
Visually empty descendants (hidden/empty nodes or only collapsed whitespace) preserve the
host's own EmptyInline frame. A flow block-epoch guard suppresses synthesis across block
boundaries, including zero-height blocks; preserved preformatted spaces remain real text.
Replaced elements do not yet
receive generated pseudos. Broader property/value coverage, `@import`, media queries and CSS
background `url(...)` resources remain later work.

Generated image slice:

```text
content strings / URL tokens / quoted url() / var() / quotes / counters
  -> ComputedPseudoStyle.items: ordered Text / Image(url, style_node)
  -> effective document or consuming stylesheet base (including CSS redirects)
  -> shared DOM/generated worker loader: source policy / cache / budgets / WIC decode
  -> PageImages.generated[(host NodeId,pseudo,item index)] -> Arc RasterImage
  -> intrinsic inline image baseline / atomic wrapping or generated block content
  -> ImageBox + href -> Image paint -> GDI AlphaBlend / measured image hit region
  -> retained Arc resources reused by reflow without requests or decoding
```

Invalid/unavailable URLs in mixed generated lists skip images while surrounding text/boxes remain.
Hidden subtrees/pseudos do not request resources. Ordinary and generated URLs share the
32-candidate, 8-request, 8 MiB encoded and 32 MiB decoded budgets/cache. Gradients/image
modifiers and alternative-text syntax remain future work.
The native generated image/link fixture is examples/css/generated-images.html.

DOM img items carry InlineStyle ancestor indices and a separate own InlineBoxStyle. Padding/solid borders reserve
horizontal edges before atomic line fitting; width shrinking leaves room for those edges.
The border-box bottom aligns to the text baseline and full image/vertical edges expand line
ascent. Background/borders use precise image-box BoxDecoration geometry before raster paint.
Text alignment includes the outer width; nowrap suppresses soft image wrapping. Generated
anonymous image items retain intrinsic geometry; sole-URL pseudos share replaced sizing and
own decoration. Native click regions still cover painted image pixels.

DOM image size slice:

```text
HTML bounded width/height attributes -> computed image size hints
  -> author cascade (including auto/global/invalid-var resets)
  -> CSS width percentages / font-relative lengths / definite heights / box-sizing
  -> replaced::dimensions intrinsic ratio + min/max constraint resolution
  -> content-width/draw-height fitting policy -> decorated atomic inline ImageBox
  -> display list / raster paint / retained reflow
```

Both auto dimensions preserve ratio across compatible min/max bounds; conflicting bounds
may stretch. One explicit side derives the auto side before its limits; two explicit sides
can stretch. Minimum constraints win over smaller maxima. Percentage heights remain auto-like
without containing-height propagation. Existing viewport fitting can shrink below CSS minima;
zero final dimensions suppress drawing, including the previous zero-attribute fallback rule.

ComputedPseudoStyle.replaced_image records exactly one parsed URL before empty text/quote
materialization. Sole-URL inline pseudos use the DOM image resolve_image_size path for CSS
dimensions, box-sizing, ratio constraints and fitting; their own pseudo InlineBoxStyle adds
padding/background/borders and atomic baseline/wrapping geometry. Mixed content images remain
anonymous intrinsic items, even when adjacent text happens to resolve empty. Missing sole-URL
images use transparent zero-natural-size replacements and preserve CSS axes/box edges.

Block DOM/sole-URL images use Context::block_image: intrinsic/CSS dimensions and exact
border-box bounds, auto horizontal margins and adjacent vertical-margin collapse. Percentage
widths use containing width while viewport fitting reserves specified horizontal margins.
The raster starts after padding/border, box height advances following flow directly without
anonymous text-line leading, and anchor href/order/Arc pixels survive paint and reflow.

Nested inline decoration slice:

```text
computed own inline/pseudo boxes -> Flow InlineBoxes arena (parent indices)
  -> one stack index per character/image/empty item; own image box stays separate
  -> cached ancestor edges + iterative common-ancestor width transitions
  -> wrapping / nowrap / alignment with all fragment edges reserved
  -> per-line outer-to-inner decoration placeholders, completed on fragment close
  -> BoxDecoration background/borders before text/raster paint and retained reflow
```

Outer fragments continue around nested boxes and images rather than disappearing for inner
runs. Image fitting reserves ancestor edges without changing CSS percentage bases. Empty and
mixed generated content share the same path; no repeated inherited box or fake glyph is added.
Deterministic coverage checks exact nested text/image/pseudo/empty geometry, wrap boundaries,
percentage fitting, 128 decorated levels, native display-list paint order and stable reflow.

Unavailable replacement slice:

```text
failed/missing resource -> zero natural dimensions, no intrinsic ratio
  -> independent CSS axes/min/max/box-sizing -> optional-raster InlineImage
  -> shared atomic wrap/nowrap/ancestor fragments or exact block size/margins
  -> background/border decorations, no raster allocation/paint/hit region
nonempty DOM alt -> styled text with inherited href -> inline fragments or ImageAlt block
mixed generated failure -> skip anonymous image, retain text/empty pseudo decoration
```

Zero natural axes remain zero when the other CSS axis is specified; padding/borders still
have geometry. Missing/blocked/cache-reused HTTP sources are covered through Engine paint
and retained reflow. Full loading-state/quirks-mode HTML fallback rules remain later work.

## S4 - Scripted page

Status: PLANNED.

```text
<script>
  -> op_js lexer/parser
  -> bytecode
  -> VM
  -> Web IDL bindings
  -> DOM mutation/events
  -> style/layout invalidation
  -> repaint
```

## S5 - Managed background tab

Status: PLANNED.

```text
browser tab
  -> lifecycle scoring
  -> throttle/freeze
  -> discard snapshot
  -> process/RAM release
  -> restore
```

## S6 - Native content blocking

Status: PLANNED.

```text
resource request
  -> request classification
  -> filter engine
  -> allow/block/redirect decision
  -> op_net
  -> page-visible result/statistics
```

## Rule

When adding a major feature, either extend an existing slice or add a new slice.
A subsystem is not considered product-progress until it participates in an end-to-end
slice.
