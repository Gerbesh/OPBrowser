# OPBrowser Code Slices

Last updated: 2026-10-09

A code slice is an end-to-end path through the architecture that produces one
observable capability. This prevents isolated subsystems from becoming impressive
piles of code that never form a browser.

**Connected Code Slicer:** [GENERATED_CODE_SLICES.md](GENERATED_CODE_SLICES.md)
is regenerated from a curated list of feature paths in
`tools/code_slices.json`. `python tools/code_intelligence.py --check`
verifies that every referenced Rust file and symbol still exists,
that every slice is valid, and that generated docs and Wiki links are
current. The same validator runs in CI. This is a **source-anchor
feature-flow checker**, not a whole-program AST/data-flow slicer; these
hand-maintained architectural narratives remain the deeper explanation.
See also [Generated Code Graph](GENERATED_CODE_GRAPH.md).

## S29 - live tag lookup and pinned multi-test WPT DOM (M4.25)

Status: IMPLEMENTED bounded original VM/DOM/native paint slice.
Original WPT DOM fixture bodies reach the original JS engine
through the pinned source runner and synchronous shim with
sticky failures and per-file expected test count. Live
Document/Element.getElementsByTagName and hasAttribute(s)
query the authoritative native DOM and reflect timer changes.
Two native page tests cover retained collections and paint.
Scope: ten manually selected attempted WPT files, three
explicit SKIPs, NOT broad DOM conformance.

## S28 - Element.children and expanded WPT DOM fixtures to pixels (M4.24)

Status: IMPLEMENTED bounded HTMLCollection and seven original WPT
fixtures. Native JS reads live element-only child collections and
passes updates through authoritative op_dom and retained paint.
The pinned WPT runner executes unchanged fixture assertions
with a limited synchronous shim, reporting seven attempted PASS
and three explicit SKIP from a manually scoped fixed selection.

## S27 - live Node state and pinned upstream WPT -> native pixels (M4.23)

Status: IMPLEMENTED narrow WPT smoke plus original DOM changes.

The WPT pinned test source is loaded via git show by
op_engine::wpt_dom_probe, executing a fixed upstream
Node-childNodes-cache fixture through the original JS interpreter
and native DOM and paint. The harness adapter supports exactly
one synchronous fixture; three other selected WPT cases are
explicitly skipped. Runtime Node properties, classList and
style declaration parsing remain separately covered by three
native page integration tests. Do not extrapolate broad WPT
compatibility from this slice.

## S26 - live NodeList / classList / style -> original DOM repaint (M4.22)

Status: IMPLEMENTED bounded dynamic DOM/CSSOM bridge.

JS-held Element.childNodes, classList and style objects have
stable identity and query/update native staged state on each
operation. parentNode/firstChild/lastChild, replaceChild/remove
and CSS property reflection connect through VM DomOperation,
op_engine replay, op_dom mutations and CSS recascade to native
paint. Child order, attributes and Text nodes are synchronized
after each script/timer commit, preserving object references
across synthetic-to-physical NodeId binding.

Nine integration tests cover live collection changes, timer
callbacks, CSS visibility and node replacement. Full Node/DOM/
CSSOM spec and WPT compliance are not claimed.

## S25 - DOM removal, reinsert, Text and attributes -> pixels (M4.21)

Status: IMPLEMENTED bounded original VM through native DOM.
Original-JS methods queue CreateText, InsertBefore, RemoveChild
and attribute operations. The engine commits these to op_dom,
recollects CSS (retaining linked sheets), recomputes styles,
then paints the real modified tree. Nine integration tests
cover subtree IDs, stable handles, timers and visible pixels.
Broad DOM/CSSOM compliance remains incomplete.

## S24 - document.createElement -> appendChild -> native pixels (M4.20)

Status: IMPLEMENTED bounded initial dynamic DOM slice.
Inline page JS or a timer/click callback creates a detached native
DOM handle, assigns id/textContent and calls appendChild; the
original JS runtime enqueues DomOperation in execution order.
op_engine::scripts::apply_dom_operations binds synthetic to real
NodeIds, updates the op_dom authoritative tree and preserves JS
object identity. Full style computation, layout and native paint
then see the newly appended node. Event listener aliases also work
on the committed nodes. An integration test verifies a real click
updates the new element on screen.

Limitations: no broad DOM attributes, DOM removal/insertion,
MutationObserver or complete synchronous DOM error semantics.

## S23 - Native String/Array methods -> DOM text -> pixels (M4.19)

Status: IMPLEMENTED in original JS VM and engine paint path.
Prototype String.charAt and Array.push/pop execute inside op_js;
SyntaxError is an owned prototype. Page script stores computed values
in existing DOM textContent; op_engine and op_paint render them as
visible native text. Test262 runtime v2 improved from 65/179 to
78/179 attempted, while 110 SKIPs remain explicit. Dynamic node
insertion is not part of this slice.

## S22 - typeof/conditional/standards -> DOM -> pixels and Test262 v2 (M4.18)

Status: IMPLEMENTED, pinned broad v2 runtime benchmark.

External Test262 v2 manifest -> Test262 runtime probe -> op_js lexer and
parser/bytecode -> native VM typeof and conditional branch -> fixture
pass/fail/skip and JSON summary. The same parser/VM executes original
inline page scripts through op_engine -> DOM textContent mutation ->
native retained layout/paint. The integration test checks the rendered
text, not just a headless interpreter variable.

Wide manifest baseline 39/179 -> 65/179 attempted; 110 explicit SKIP.
The older 91-case runtime subset stays 82/91. These are limited
classic-script compatibility measurements, not a modern Web score.

## S21 - Primitive boxing and ToPrimitive to pinned Test262 (M4.17)

Status: IMPLEMENTED bounded classic-script semantics.

JS new Boolean/Number/String, Object(value) -> JsRuntime::box_primitive
-> VM private boxed_values map -> prototype valueOf/toString methods.
For +, equality and relational operators: Instruction::Binary ->
JsRuntime::binary_with_coercion -> JsRuntime::coerce_to_primitive ->
JsRuntime::call_value (user hooks) -> apply_binary; native instanceof
walks prototype chain and never calls ToPrimitive. Unary void and
global sloppy assignment complete selected evaluation-order fixtures.

The same pinned 91-Test262-runtime v1 fixtures pass 82/91, formerly
59/91. This is not a whole JavaScript-engine score. Eval, Date and
Symbol remain absent, as do strict-mode and modern ES features.

## S20 - Pinned Test262 runtime case -> own VM -> machine-readable verdict (M4.16)

Status: IMPLEMENTED narrow executable classic-script sample.

Test262 fixture -> committed 91-path manifest -> metadata skip/negative
phase handling -> Test262Error/assert self-hosted bootstrap ->
JsRuntime::eval_script -> ordinary bytecode runtime / standard globals ->
pass / fail with exact failure cause or explicit SKIP ->
case-by-case JSON and tools/compatibility.ps1 independent percentage.

Numeric global calls and 0x/0b/0o coercions exercise original VM
builtins instead of delegating evaluation to an external engine.
Script-mode-only, no full Test262 harness, strict, async or modules.
This same-crate benchmark is documented manually: the generated Code Slicer
intentionally requires cross-crate feature paths.

## S19 - strict JSON response -> Promise -> native pixels (M4.15)

Status: IMPLEMENTED bounded strict JSON and array-like combinators.

Page fetch(data.json) -> Engine::dispatch_text_requests -> validated
GET completion -> Engine::tick_timers -> op_js::TextResponse ->
Response.json -> JsonParse -> op_js::json::parse -> json_to_value ->
Promise microtask reactions -> DomTextMutation ->
Document::set_text_content -> native Win32 repaint.

Real VM and engine tests cover Unicode, invalid JSON, delayed Promise
resolution, circular stringify errors and JSON fields painted after GET.
No reviver/toJSON, true iterator combinators, streams, cookies or CORS.

## S18 - Request Headers → vetted WinHTTP GET → checked redirects (M4.14)

Status: **IMPLEMENTED** for bounded same-origin GET text.

~~~text
page Request + Headers constructor/append/set → native header budget
  → fetch(Request, RequestInit) → opFetchText queued request options
  → Engine::dispatch_text_requests → NetworkContext::load_text_response_for_page_with_options
  → http::load_text_with_options → windows::load_with_headers
  → WinHttpAddRequestHeaders (restricted safe fields)
  → status 3xx: resolve_link(Location) + same_origin check BEFORE next GET
  → up to 5 hops / redirect:error short-circuit
  → detached Response metadata → Promise microtask → DOM/native paint
~~~

Validated in VM tests and local WinHTTP/engine tests including a
foreign-origin redirect whose destination server never receives
a connection. No CORS, streams, POST, cookie credentials or abort.

## S17 - WinHTTP HTTP error and redirect metadata → Response → pixels (M4.13)

Status: **IMPLEMENTED** for bounded same-origin GET text bodies.

~~~text
WinHTTP text request → HTTP status, reason, final URL, headers
  → op_net::LoadedTextResponse → NetworkContext::load_text_response_for_page
  → Engine::dispatch_text_requests (generation-tagged worker completion)
  → Engine::tick_timers → op_js::TextResponse
  → JsRuntime::complete_text_response_request
  → ObjectKind::Headers (case-insensitive get/has; no Set-Cookie)
  → self-hosted Response.status / ok / statusText / url / redirected
  → Response.text() Promise → FIFO microtask reaction → DOM text
  → CSS/layout/native repaint
~~~

WinHTTP and engine tests verify status 201 redirects, 404/500 response
bodies and fields, sensitive header filtering and native repaint. Legacy
opFetchText still reports HTTP errors as callback errors. No streaming,
CORS, POST, credentials, public Headers constructor or abort.

## S16 - Promise/fetch text GET → microtask reactions → native paint (M4.12)

Status: **IMPLEMENTED** as a bounded, same-origin text GET subset.

```text
page script: fetch(url).then(function(response) { return response.text(); }).then(handler)
  -> self-hosted async_fetch.js + async_promise.js in original op_js VM
  -> OpFetchText -> JsRuntime::take_text_requests
  -> Engine::dispatch_text_requests -> NetworkContext::load_text_for_page
  -> WinHTTP/local read on bounded worker + generation-tagged completion
  -> Engine::tick_timers -> JsRuntime::complete_text_request
  -> Promise resolution -> JsRuntime::drain_microtasks (FIFO)
  -> user handler -> DomTextMutation -> Document::set_text_content
  -> compute_styles -> render -> Win32 pixels
```

VM tests verify Promise chaining/FIFO/rejection/Response consumption;
`crates/op_engine/tests/scripts.rs` exercises end-to-end local file
resolution and rejected cross-origin GET. The generated S16 anchors this
path to concrete Rust functions. The Response metadata is synthetic;
this does not yet implement cross-origin CORS, response status/headers,
streaming or POST.

## S7 - Classic inline JS → DOM → native pixels (M4.1)

Status: first bounded end-to-end slice **IMPLEMENTED**.

```text
op_html::parse_document
  -> op_engine::scripts::execute_inline (max 16 classic scripts)
  -> op_js::JsRuntime::eval_script (shared page runtime)
  -> document.getElementById / element.textContent (owned JS builtins)
  -> Vec<DomTextMutation> (bounded, detached)
  -> op_dom::Document::set_text_content
  -> op_css::compute_styles + op_layout
  -> op_paint::build_display_list -> Win32 GDI pixels
```

Verified by the data URL navigation integration test, retained reflow
test and `examples/js/dom-text.html`. Arbitrary JS errors do not
invalidate navigation. CSS/layout is recalculated after mutations, but
scripts currently execute as a one-time phase after full HTML parsing:
not parser-blocking; external scripts, events, DOM creation and other Web
APIs remain unimplemented. Generated code slice S7 is source-anchor
checked under `docs/GENERATED_CODE_SLICES.md`.

## S8 - External JS resource → VM → DOM (M4.2)

Status: **IMPLEMENTED** at initial bounded/same-origin level.

```text
<script src="relative.js"> in parsed DOM
  -> op_engine::scripts::execute_for_page
  -> op_net::resolve_script_source
  -> NetworkContext::load_script_for_page
  -> RequestFilter::check(ResourceType::Script)
  -> local file or WinHTTP HTTP(S) JS download + decoding
  -> shared JsRuntime::eval_script
  -> DomTextMutation -> Document::set_text_content
  -> computed styles, retained layout, display list, GDI
```

Verified against real local paths and WinHTTP loopback traffic, plus
source-order inline/external/inline, request blocking, missing resources
and cross-origin rejection. Browser-like parser-blocking semantics,
`async`, `defer`, SRI/CSP, cross-origin script loading and event
dispatch remain unsupported. The `S8` generated source slice is
checked by Code Slicer in CI. Sample:
`examples/js/external.html` and sibling `external.js`.

## S9 - Native click → DOM bubbling → JS callback → pixels (M4.4)

Status: **IMPLEMENTED** for id-bearing block regions with bubbling
and handler removal by function identity.

```text
WM_LBUTTONUP / client coordinates
  -> Win32 toolbar/scroll conversion
  -> NavigationEvent::Click on browser worker
  -> smallest op_layout::ClickRegion target (listener-independent)
  -> op_dom::Node::parent chain
  -> retained JsRuntime::dispatch_dom_click_path
  -> target/ancestor addEventListener / onclick callbacks
  -> bounded DomTextMutation → Document::set_text_content
  -> recompute_styles / reflow / paint without navigating
```

Retained VM closures preserve counters across clicks. Tests cover
bubbling, target/currentTarget/eventPhase and removal/duplicate
registration. Demos: `examples/js/click.html` and
`examples/js/bubble.html`. Capture, cancellation, keyboard,
listener options and full DOM hit-testing remain unsupported.

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
declaration-free source. Tree construction now advances through initial/before-html/
before-head/in-head/after-head/text/in-body/after-body/after-after-body states, synthesizes
omitted html/head/body, keeps title/style/script/noframes text under head, reparents permitted
head-only tokens seen after head, merges duplicate html/body attributes without overwriting
existing ones, and treats a slash on ordinary non-void HTML start tags as non-closing.
In-body recovery now has normal/list-item/button scope boundaries plus implied end tags:
block starts close open paragraphs, repeated li/dd/dt items close predecessors, heading
starts/ends recover across mismatched heading names, nested buttons close the prior button,
generic end tags cannot cross special elements, </br> becomes br and legacy <image> becomes
img. Head-only metadata/raw-text tokens encountered while in body are attached back to the
stored head without losing the body stack. </body>/</html> now switch parser state without
popping that stack; after-body comments attach to html, after-after-body comments attach to
Document, whitespace/html tokens delegate through in-body, and unexpected trailing content
re-enters in-body for recovery. The in-body formatting path now keeps active formatting
entries for a/b/big/code/em/font/i/nobr/s/small/strike/strong/tt/u, reconstructs entries
that have fallen off the open-element stack, caps identical entries with the Noah's Ark rule
and runs bounded adoption-agency recovery for formatting end tags. Misnested formatting can
clone/reparent nodes around a furthest special block; repeated anchors/nobr are recovered and
applet/marquee/object markers prevent inner formatting from leaking outward. Table parsing now
covers in-table/text/caption/column-group/table-body/row/cell states, implicit tbody/tr
creation, table-scope closure and cell formatting markers. Non-whitespace character runs and
misnested elements in table contexts use foster parenting before the last open table through
op_dom sibling insertion, while whitespace table text stays in table context. Head text tokens
encountered in tables return to the original table mode after text parsing. That DOM now feeds
an initial CSS table formatting context: native table tags receive table display roles, captions
flow above the grid, row groups supply rows, colspan/rowspan occupy multiple tracks, cells share
row geometry instead of ordinary vertical block flow, and cell padding/backgrounds/borders plus
header boldness reach native paint. Track widths now use measured cell text/images and CSS
width/min/max preferences plus col/colgroup width hints; colspan deficits are shared across its
tracks. border-spacing is inherited and controls real horizontal/vertical gaps. collapse mode
removes those gaps and resolves shared cell-edge conflicts per grid segment so only one winning
border reaches paint. Table-cell vertical-align now supports baseline/top/middle/bottom: after
row/span heights are resolved, text, images and nested decorations move together while the cell
border box stays fixed; baseline cells align their first line across the row. CSS table fixup now
covers both directions for the core row/cell structure: missing children inside table roots/row
groups become layout-only rows/cells, while consecutive orphan table-internal siblings in normal
flow are grouped under one anonymous block table and then use the same grid formatter. Orphan
captions remain in that wrapper and orphan columns still contribute width hints. Inherited text
style survives these layout-only wrappers. Auto table sizing now retains percentage constraints
from columns and cells while content still supplies min/max preferences. table-layout:fixed is
implemented for explicit-width tables using col/colgroup hints, then first-row widths, then equal
remaining space; later rows cannot resize the tracks. caption-side:top/bottom is computed and
captions now sit outside the table border/background in wrapper flow. display:inline-table creates
a real atomic inline formatting object: auto width shrink-fits against intrinsic tracks, authored
margins/padding/borders survive, and nested text/images/order/link identity are moved into final
line output as a unit. Atomic baseline/top/middle/bottom vertical-align now participates in line
ascent/descent and moves the complete retained table output together. Mode-specific layout quirks,
processing instructions, template/frameset modes, foreign-content context, complete CSS Tables
overconstraint/min-width/percentage edge rules, non-cell collapsed border precedence and deeper
colgroup repair remain later; general inline sub/super/text-top/text-bottom/length/% alignment is
still unsupported.

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

Initial positioned-layout slice:

```text
computed position + inset edges
  -> normal-flow block geometry / positioned-ancestor stack
  -> relative: retain flow slot, translate newly emitted decoration/text/image output
  -> absolute: remove from normal flow, resolve against nearest positioned ancestor
  -> fixed: remove from normal flow, resolve against viewport
  -> existing BoxDecoration/TextBox/ImageBox
  -> normal display-list / native paint
```

The positioned path supports `position:static|relative|absolute|fixed`. Relative offsets move
visual output while preserving the original flow contribution. Absolute/fixed blocks do not advance
normal flow. Absolute positioning uses the nearest positioned ancestor's initial padding-box geometry;
fixed positioning uses the real viewport width and height supplied by `op_engine`. Insets on all four
sides accept px/percentage values when their containing-block axis is definite. Bottom-only placement
is resolved after layout, while opposing left/right or top/bottom insets can stretch an auto-sized
absolute/fixed block. Other positioned `width:auto` cases use the initial shrink-to-fit path. Inline
absolute/fixed boxes leave a zero-width marker in the line so auto-inset static position starts at the
actual inline cursor without consuming width. Horizontal inline margins now affect inline advance,
including negative values, while visual decoration excludes those margins. Initial
`display:inline-block` packages a local block/BFC result into one atomic inline box with shrink-to-fit
auto width. Block children inside an inline now split the parent-linked inline-box path into
continuation nodes: ending/continuing fragments suppress the appropriate logical edge, intermediate
empty fragments preserve line height only when CSS2 requires them, and later descendants resolve to
the newest continuation. Relative inline offsets are carried to split block/float output without
altering normal-flow geometry. Definite block heights, including min/max-clamped explicit heights,
propagate as the percentage-height basis for direct descendants. Auto-height blocks are deliberately
not made definite merely by a min-height clamp. For absolute/fixed non-replaced blocks, the
positioning path now solves both horizontal and vertical auto margins against the space remaining
after opposing insets and used border-box size. It handles a single auto margin, equal auto margins,
negative horizontal free space according to LTR/RTL, negative vertical free space on the bottom, and
overconstrained horizontal fixed margins by preferring left in LTR/right in RTL. A missing inset
resolves auto margins to zero instead of centering against the viewport. Relative inline elements
now retain box-stack identity even with no visible decoration. Lines emits per-fragment measured
border/padding geometry, tracks their relative visual offsets without consuming flow space and
identifies the nearest positioned inline ancestor for absolute descendants. The flow context now
accumulates fragments across separate formatting runs, including block children that split an inline.
It defers absolute inline descendants until the complete first/last padding-edge rectangle (LTR/RTL)
is known. Pending positioned children retain their hypothetical static coordinates, containing
context fallback and flow width; relative block translations also shift pending positions and
fragment bounds. Independently formatted inline-block/table/flex/floated subtrees drain their own
deferred queues before being moved into the parent. A deferred positioned subtree can itself enqueue
more deferred descendants, which are drained in order without affecting normal-flow placement.
RTL absolute/fixed block-level children with both horizontal insets auto now anchor against the
hypothetical flow width rather than the viewport width. Fixed descendants with explicit insets still
use the viewport, and blocks inside unpositioned inline ancestors keep legacy static-position flow.
Complex/replaced positioned constraints, bidi/vertical writing, multicol, sticky and stacking remain.
CSS2 inline content rectangles now use the element's font metrics and own padding/borders, not
its enclosing line-height and ancestor padding. The line strut still determines half-leading and
normal-flow advance. `Lines` exposes the last line baseline so atomic inline-block boxes align
by their last in-flow line, falling back to the bottom border if they contain no line boxes.
`BoxDecoration` now marks Block versus Inline paint phase; `op_paint` renders all block backgrounds
before inline backgrounds and before text/images, avoiding overwrites where inline ink exceeds the
following block's top. This is not full positioned stacking/z-index support. Frozen WPT Positioning
v1 increased 36/100 -> 38/100 (38.00%) with WPT Static holding 187/200 (93.50%).

A follow-up first positioned foreground paint phase extends `DecorationPaintLayer` with
`PositionedBlock/PositionedInline` and `LayoutItem` with positioned text/image variants.
`Context::positioned_element` tags its own output ranges, including nested atomic output,
and `op_paint` emits normal block and inline backgrounds plus normal text/images before
positioned backgrounds and foreground text/images. Relative blocks without nested positioned
children are promoted as a foreground group (including relative table captions that must
occlude previously painted absolute siblings). Relative blocks containing positioned
descendants are deliberately not promoted wholesale, since that would place their ordinary
text on top of their absolute children. This prevents foreground paint reversals but does
not provide CSS source-order stacking contexts, negative z-index or interleaved sibling
foreground paint groups. Pinned WPT Static and Positioning remain 187/200 and 38/100.

The first z-index slice computes a non-inherited signed integer or auto value,
passes PaintKey to positioned block/inline decorations and text/images, and
preserves nested positioned output keys while promoting outer block ranges.
The following atomic-context slice enumerates final DOM preorder after HTML
reparenting, then derives PaintGroup parents from positioned non-auto z-index
ancestors and fixed-position ancestors. Even a paintless ancestor isolates its
children. The painter uses an iterative depth-first context traversal: negative
root groups precede in-flow block backgrounds, while negative child groups come
after their atomic parent's block background but before its inline foreground.
Sibling groups are sorted by local (z-index, DOM order). This fixes large child-z
escaping a low-z parent and foster-parent source-order ties, without recursion.
The following positioned-inline paint slice attaches a PaintKey to
InlineBoxStyle for relative inline elements. InlineBoxes walks its parent
arena path to find the innermost positioned key; Lines emits fragment
backgrounds, text, and images into that group across wraps. Nested
positioned spans preserve their own keys, and a relative span with z-index:auto
does not isolate explicitly stacked descendants. Atomically formatted inline-block
descendants join the enclosing inline group only for their ordinary outputs,
retaining their independently positioned keys. Local table/block/flex inline
formatters clear caller-owned inline arena indices before entering new contexts;
this also fixes an index-out-of-bounds panic in nested inline-block content.
PaintGroup.inline_owner keeps the inline ancestor background behind its atomic
block descendants. Relative block paint promotion no longer depends on the
absence of nested positioned outputs: Context::block tags only the parent's
ordinary decoration and foreground with a zero-level PaintKey even for z-index:auto.
Because LayoutItem::positioned and decoration marking retain existing positioned
keys, nested explicit-z children remain independent participants in the nearest
real atomic ancestor context. This corrects same-level source ordering of the
parent's own ink relative to earlier positioned siblings for both block and
inline-block while preserving positive and negative child ordering. Explicit-z
inline-block context isolation and reflow are regression-tested. Full CSS
stacking remains incomplete: additional context triggers, auto-z interleaving
with other CSS paint phases, and some inline/block decoration ordering remain
coarse.

The next table-positioning slice adds intrinsic pixel-width hints for block
descendants in table cells, letting auto table columns shrink to the available-
capped preferred width. After grid layout, table cells collect structural DOM
ancestors (td/tr/tbody/thead/tfoot), accumulate relative offsets without moving
normal-flow row slots, and translate their backgrounds/text/images and positioned
descendant outputs. Positioned row/section backgrounds are emitted as paint groups
covering each occupied row; a grid track that is only 1px high due to an absolute
descendant does not emit an incorrect one-pixel row-group background. Cell
backgrounds with position:relative receive their own positioned paint key.
During cell layout the nearest relative table ancestor contributes the absolute
containing-block origin, with the later relative visual translation preserved.
This closes 15 more frozen Positioning WPT tests, from 38/100 to 53/100.
The subsequent auto-wrapper pass computes a single intrinsic width choice
before constructing the table's outer border/background and captions. This
makes its wrapper width agree with the column tracks, instead of previously
letting a 60px content grid occupy an accidental 300px painted wrapper.
The size includes outer and inter-column border-spacing, horizontal padding
and border, and respects the existing box-sizing/min-width and explicit-width
path. A width:auto table with table-layout:fixed still uses the auto algorithm
rather than silently switching to fixed tracks when the internal preferred
width becomes definite. Six engine regressions cover these cases. Frozen WPT
Static/Positioning remain 187/200 and 53/100 with zero render errors.
The multi-row table span pass no longer charges the full height of rowspan
cells to their starting rows. Ordinary one-row cell heights and baselines
establish initial row tracks; spanning cells then request any remaining
height across their entire row interval, including internal border-spacing.
The final covered row absorbs each deficit in increasing span-end order.
Since cells are measured before span-height reconciliation, the renderer
records initial row origins and translates the decorations/text/images of
later rows to match final track positions. A rowspan cell's decoration height
is then set from its covered tracks, and vertical alignment uses that span
height. Engine regressions cover a two-row span, overlapping two-/three-row
spans with a following row, vertical border-spacing, and a colspan intrinsic
width control. Frozen WPT Static/Positioning stay 187/200 and 53/100, with
zero render errors.
Limitations: redistribution across rows follows a simple last-track strategy
rather than full browser-compatible row-height rules; row-group paint beyond
basic spans is approximate, and percent-based absolute containing heights
remain incomplete.

The WPT Static v1 191/200 pass adds three independent rendering slices.
CSS Color 4 parsing now routes absolute OKLab/OKLCH colors through a small
near-black output correction: if the corresponding achromatic color would
round to black in the 8-bit SDR framebuffer, an out-of-gamut source chroma
is reduced by binary search at fixed lightness/hue before quantization.
Brighter wide-gamut colors retain the existing direct conversion until
general gamut mapping is consistently implemented across color spaces.
In HTML form layout, raw direct text children of select are not emitted as
page content; option elements keep their own visibility rules.
For the current text-only SVG slice, defs never directly paint, SVG text with
display:contents suppresses its text, outermost SVG with display:contents
has no paint context, nested SVG/g containers remain traversable, and use
can expand text node children from an id-referenced SVG text source.
This is still not complete SVG layout, hit testing, or coordinate/raster support.
Engine and color regressions preserve the narrow behavior. Frozen Static
passes 191/200 from 187/200; Positioning stays at 53/100.

The following CSS effects slice introduces non-inherited opacity and a
single filter:invert() expression in computed-style resolution. Layout
marks opacity/filter owners as paint groups, including static block owners
and positioned children; their nested parent relationships are retained
through collect_paint_groups. The display list emits BeginLayer/EndLayer
around group paint commands while retaining the existing iterative
negative/background/foreground/positive stacking phases.
Win32's shared GDI command painter creates paired offscreen surfaces,
renders each group's contents onto black and white, estimates per-pixel
coverage from the difference, applies CSS invert to premultiplied RGB,
then composites the entire group exactly once with the declared opacity.
The same path drives native window painting and WPT headless reftests.
The layer area and depth are bounded; oversized or failed allocations
fall back to visible unfiltered content rather than erasing the group.
Windows tests enforce uniform opacity over overlapping rectangles and
nested invert/opacity behavior; CSS tests check cascade/non-inheritance.
This raises Static to 192/200 and leaves Positioning 53/100 with zero
render errors. Remaining limits include other CSS filter functions,
precise shadow/gradient handling, other opacity-bearing inline/table
contexts and complete alpha/color-space precision.

The image-background and ICC slice adds a per-node optional CSS URL
associated with its declaration's source stylesheet node, without adding
owned strings to the Copy ComputedStyle. CSS background and background-image
candidates participate in the same important/inline/specificity/source-order
cascade; none/initial reset, explicit inherit, and background shorthand
reset are supported for this single image. The image worker resolves URLs
against the linked stylesheet address where applicable, through the existing
bounded loader, and caches the decoded resource per DOM node. Layout copies
the image Arc to block, table or cell BoxDecoration ink without changing
normal flow. The display-list BackgroundImage command paints between block
background color and its border; native WIC/GDI uses intrinsic-size tiles
under a saved rectangle clip, bounded to 1024 tiles per decoration.
The WIC decoder now retrieves PNG ICC context count, constructs the
required IWICColorContext objects, fetches embedded profiles, constructs
an sRGB destination context and uses IWICColorTransform before PBGRA
conversion. Unprofiled images keep the existing WIC format conversion.
Together with the self-document href="" visited-link correction,
the unchanged exact-pixel WPT Static v1 moves 192 -> 194/200; Positioning
remains 53/100. CSS background gradients, placement, sizing,
repeat controls, inline decorations and multi-layer backgrounds are still
incomplete; arbitrary history-linked :visited remains unsupported.

The named CSS ICC profile slice extends the previous color-managed PNG work:
op_css::parser::parse_color_profiles scans CSS tokens for the bounded
@color-profile --name { src:url(...) } form, including quoted and unquoted
URLs. Normal stylesheet parsing recognizes the at-rule without erroneously
treating it as a selector. The style worker walks inline and linked sheets
with a capped document traversal, up to eight network requests and a 1 MiB
aggregate profile budget. Relative URLs use the effective stylesheet base
and the normal filtered resource path. After matching the author cascade,
StyleMap::resolve_custom_profile_colors scans declaration token sequences,
preserving strings and invalid functions, and replaces valid
color(--name R G B) tokens with ICC-converted sRGB hex. Windows op_image
creates a one-pixel WIC color-transform source/destination context for
arbitrary source ICC bytes; the engine caches repeated (profile,RGB)
conversions per document preparation. This path handles numeric/percentage
RGB input for 3-channel ICC profiles, but does not implement general
color-profile alpha, CMYK/device channels or broad CSS Color 5 syntax.
Frozen Static improves to 195/200, Positioning unchanged at 53/100.

The first-line fragment slice adds FirstLine to the CSS pseudo-element grammar
(both ::first-line and legacy :first-line), selector matching and computed
fragment pseudo-style pipeline, without pretending a fragment creates
generated content. For a block's final inline sequence, flow::Context
passes the host's computed first-line text and background colors into
inline::Lines. The real first line flush uses the pseudo text color for
runs inheriting the host color, preserving descendant explicit colors.
The first line background is painted in the inline decoration phase
behind its text, with the exact ascent/descent glyph metrics box also
used by regular inline spans. Subsequent automatic lines and explicit
breaks retain the original colors. Two engine tests verify that a second
line after br remains unmodified, and that first-line background height
equals the height of an equivalent painted inline span. This brings the
unchanged strict WPT Static v1 suite to 196/200 (98.00%) by passing
selectors/first-line-bidi-002. Remaining limitations: partial first-line
inherited currentcolor resolution for descendants' borders/backgrounds,
font reflow under first-line pseudo font changes, and unsupported
gradients/shadows/filter chains.

The late first-line currentcolor fix preserves computed background-color
relative identity and per-side border-color dependency flags through
InlineBoxStyle. The first line renderer replaces only dependent background
and border ink when a descendant inherits the host color overridden by
::first-line; explicitly red backgrounds/borders and separately blue text
remain unchanged, as do subsequent lines. CSS currentcolor-003 now matches
its frozen reference pixel-for-pixel. Static v1 reaches 197/200 (98.50%)
with exact comparison intact; general gradient, shadow and outline effects
are still incomplete.

The frozen WPT measurement now optionally shows a second compliance result
from unmodified test-supplied fuzzy metadata. The original wpt_probe
default still compares RGB channels and every pixel exactly; existing
JSON CI percent and manifests remain unchanged. With --report-wpt-fuzzy,
the HTML tokenizer scans meta name=fuzzy entries, including the
reference-specific override form, then requires observed maximum channel
error AND total differing pixels to fall inside their author-declared
inclusive ranges. Unmarked tests still use the strict outcome.
Default 'medium' text changes from OPBrowser's previous 18px to the
conventional 16 CSS pixels, correcting the shape of em-based WPT fixtures.
Corresponding unit assertions now validate rem and line/inline metrics
at 16px. With xyz-003 reporting one blue byte over 18432 pixels (both
allowed by its own metadata), optional WPT metadata-aware Static v1
reports 198/200 (99.00%); the primary strict metric still reports
197/200 (98.50%), and Positioning stays 53/100. These separate numbers
must not be conflated or used to claim 100% pixel-exact fidelity.

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

Status: IN PROGRESS at the standalone language-engine stage; page scripting remains planned.

Current executable slice:

```text
JavaScript source
  -> op_js lexer
  -> AST parser
  -> bytecode compiler
  -> stack VM
  -> primitive or ObjectId completion value
  -> runtime-owned lexical environment chain -> global/function/block bindings
  -> closure capture -> function call/return + this/arguments
  -> constructor allocation -> function.prototype -> new instance
  -> abrupt completion -> return / explicit throw / runtime Error object / break / continue
  -> try/catch/finally
  -> runtime-owned object heap -> own properties -> prototype chain
```

The implemented subset covers scalar literals, comma-separated let/const/var declarations,
assignment, unary/arithmetic/comparison/equality operators, string concatenation, short-circuit
logical operators, blocks, if/else, while and break/continue. It now also has reference-identity
objects, object/array literals, dot/computed property access and writes, array holes/length growth,
and an initial prototype chain including object-literal/__proto__ behavior and cycle rejection.
Control flow is compiled to patched jumps and executed by an instruction-pointer VM with bounded
instruction, object, lexical-environment and call-depth budgets. Bindings resolve through
global/function/block lexical environments: let/const use the current block, var targets the nearest
function/global environment, and function objects capture an environment for closures that survive
scope exit. Initial function declarations are hoisted within each compiled statement list. C-style
for, do/while, switch fallthrough, ++/--, explicit throw and try/catch/finally now execute, including
finally before return/throw/break/continue and control transfer back into an enclosing loop/switch.

Method calls now preserve the base object as the receiver, bare non-strict calls use the runtime
global object for this, and user functions receive an array-like arguments object. Each user
function owns a prototype object with a constructor backlink; new allocates an instance using that
prototype, binds it as this, and implements the object-return/primitive-return constructor rule.
Error, TypeError and ReferenceError exist as initial built-in constructors/prototypes. Runtime
Reference/Type failures that occur under try regions are materialized as catchable JavaScript error
objects, while execution-limit failures stay engine-level and deliberately cannot be caught.

A for(let) loop still has one lexical loop scope rather than the spec's fresh per-iteration binding
used by closures. Function calls still recurse through the native Rust stack, so the temporary
call-depth guard is 64 until explicit VM call frames replace native recursion.
Arrow/default/rest/destructuring forms, labels and for-in/of remain later work. The parse-only
Test262 probe progressed 364 -> 391 -> 408 -> 504 -> 508 -> 523 passed expectations, currently
523/1983 (26.37%) on the unchanged manifest; it deliberately does not claim runtime conformance.

Planned continuation:

```text
<script> discovery
  -> op_js runtime / realm + objects/functions/exceptions
  -> Web IDL bindings
  -> DOM mutation/events + event loop
  -> style/layout invalidation
  -> repaint
```

## S5 - Managed background tab

Status: IN PROGRESS at the browser-core policy stage; native multi-tab UI/process release is not connected yet.

```text
op_browser_core::TabManager
  -> TabId + canonical active tab
  -> active/background/throttled/frozen/discarded/restoring lifecycle
  -> protection flags (audio/capture/transfer/unsaved-form/pinned)
  -> estimated private-byte accounting input
  -> automatic_discard_candidate()
  -> retained address + scroll restore state
  -> activation of discarded tab requests restoring state
```

The next product slice connects this model to native tab UI and one renderer process per
active tab, then uses real process memory/pressure signals to terminate and recreate
renderers. Current tests cover activation/close invariants, protected tabs, candidate
selection and restore transitions.

## S6 - Native content blocking

Status: IN PROGRESS with network filtering active on current document/CSS/image paths.

```text
Engine document / stylesheet / image load
  -> op_net::NetworkContext
  -> RequestFilter::check(url, ResourceType, top-level URL)
      -> Allow -> existing file/data/HTTP(S) loader
      -> Block -> LoadError::BlockedRequest { url, rule }
  -> checked/allowed/blocked counters
```

The initial rule subset accepts `||host^`, wildcard patterns, `@@` exceptions, document /
stylesheet / image / script resource options and a per-site allowlist. Stylesheet/image
block failures remain nonfatal to the document, matching the existing subresource failure
policy. The current matcher is linear and is not yet suitable for EasyList-scale lists.
Subscriptions, domain/third-party options, scalable indexing, cosmetic filtering and UI
statistics/settings remain planned.

## S7 - Versioned external conformance measurement

Status: COMPLETE for the initial static/parser v1 measurement slice.

```text
pinned WPT/Test262 revisions + committed manifests
  -> CI/local compatibility runner
  -> Test262 classic-script parse expectations
  -> WPT test/reference Engine::render_source
  -> ordinary DisplayList
  -> offscreen Win32 GDI surface using shared paint_command
  -> stable pass/total percentages
  -> JSON/artifact
  -> public metrics branch + README badges
```

The initial baselines are WPT Static v1 86/200 (43.00%) and Test262 Parser v1
364/1983 executable scripts (18.36%, with 17 module entries skipped). The first measured
CSS Color 4 implementation pass moved the unchanged WPT Static v1 manifest to 126/200
(63.00%); the following system-color/@supports and selector-semantics pass moved it to
151/200 (75.50%), the deferred-color/display-contents/nth-grammar pass moved it to
172/200 (86.00%), the relational-selector/empty-inline pass moved it to 179/200
(89.50%), structural table `display:contents` moved it to 181/200 (90.50%), and the
grapheme-aware `::first-letter` fragment-pseudo pass moved it to 182/200 (91.00%),
self-collapsing block-in-inline margin propagation moved it to 183/200 (91.50%), and the
initial flow-root/BFC/float/visibility pass moved it to 185/200 (92.50%), and the initial
flex formatting/display-contents pass moved it to 187/200 (93.50%).
These numbers name their subsets explicitly and are not full browser or ECMAScript conformance
scores. Future scope changes require a new manifest version so agents
can compare before/after results without moving the denominator underneath themselves.

## S8 - Initial flex formatting context

Status: COMPLETE for the first default row slice.

```text
display:flex / inline-flex
  -> flex formatting dispatch
  -> element and anonymous text items
  -> display:contents flattening
  -> intrinsic item sizing
  -> single-line horizontal row
  -> block flex or atomic inline-flex
  -> LayoutTree / DisplayList / Win32 paint
```

The first slice implements default single-line row behavior. `display:contents` contributes no
principal box and recursively exposes its children while preserving inherited text style.
`inline-flex` uses the same nested row layout as a shrink-to-content atomic inline object.
Direction variants, wrapping, flexible lengths, ordering, alignment and gaps remain follow-up work.

The two pinned flex/display-contents reftests moved from 0/2 to 2/2, raising WPT Static v1 from
185/200 (92.50%) to 187/200 (93.50%) on the unchanged manifest.

## Rule

When adding a major feature, either extend an existing slice or add a new slice.
A subsystem is not considered product-progress until it participates in an end-to-end
slice.

## S10 - Closing script to retained parser-state DOM (M4.6, current)

Status: IMPLEMENTED for bounded classic scripts.

```text
op_html::parse_document_with_script_hook
  -> script close and text-buffer flush
  -> op_engine::scripts::ParserScriptRunner::execute
  -> optional filtered same-origin op_net script fetch
  -> JsRuntime::install_dom_snapshot / refresh_dom_snapshot
  -> JsRuntime::eval_script
  -> Document::set_text_content before later markup
  -> final snapshot refresh for retained event handlers
  -> CSS / layout / paint
```

Tests assert that a script cannot find future IDs, later scripts share
globals and can find newly parsed nodes, and a click listener installed
early can access elements parsed after that script. This path supersedes
the post-parse timing described in the historical S7 and S8 sections.
Tokenization is eager; document.write and async/defer are not implemented.

## S11 - External defer/async script to retained DOM (M4.7)

Status: IMPLEMENTED as bounded initial-load scheduling, not a browser
event loop.

    script close in HTML parser
      -> ParserScriptRunner::prepare_fetch
      -> scoped worker and filtered NetworkContext::load_script_for_page
      -> CompletedScript channel
      -> ParserScriptRunner::drain_ready / complete
      -> async completion-order execution at parser polls
      -> deferred source-order execution after DOM construction
      -> JsRuntime::eval_script / Document::set_text_content
      -> CSS/layout/paint

Local-file and WinHTTP tests cover deferred ordering, async completion
ordering, inline flags and retained DOM. There is no post-presentation
event loop, module support, SRI, document.write or DOMContentLoaded.

## S12 - HTML parser completion → document lifecycle events (M4.8)

Status: initial lifecycle dispatch implemented.

    parse_and_execute -> ParserScriptRunner::advance_state
      -> JsRuntime::set_document_ready_state
      -> JsRuntime::dispatch_lifecycle_event
      -> DomTextMutation -> Document::set_text_content -> paint

Tests verify interactive before deferred execution, DOMContentLoaded
after deferred scripts, complete/load after initial async workers.
An independent browser task/event loop is not yet available.

## S12 - Page lifecycle events to native pixels (M4.8)

Status: IMPLEMENTED for initial-load-only lifecycle events.

    HTML parser completes
      -> ParserScriptRunner::advance_state interactive
      -> JsRuntime::set_document_ready_state
      -> document readystatechange
      -> deferred classic scripts
      -> JsRuntime::dispatch_lifecycle_event DOMContentLoaded
      -> pending initial async completion
      -> ParserScriptRunner::advance_state complete
      -> document readystatechange + window load
      -> DOM text mutation -> CSS/layout/paint

Tests cover readyState immutability, event fields/receivers, property
handlers and deferred registration. There is no independent event loop.

## S13 - JS timeout to live page pixels (M4.9)

Status: IMPLEMENTED for bounded one-shot function timers.

    setTimeout(fn, delay, ...args) in op_js::JsRuntime
      -> PendingTimer (Instant deadline, numeric ID)
      -> Engine::next_timer_wait
      -> op_browser worker recv_timeout and Engine::tick_timers
      -> JsRuntime::run_due_timers (bounded JS callback execution)
      -> DomTextMutation / Document::set_text_content
      -> style recompute and retained render
      -> UI Poll receives reflow / Win32 GDI

A real click can schedule a callback after the page has been painted.
Timer handles are page-owned and are discarded on navigation. Verified
by tests for callback arguments, cancellation, deadlines, errors and
native click-to-timer-to-display-list repaint. Does not include
setInterval, Promise microtasks, string evaluation, background tab
throttling or post-presentation async network resource completion.

## S14 - JS interval and microtask checkpoints to repaint (M4.10)

Status: bounded setInterval/clearInterval and queueMicrotask supported.

    setInterval(fn, ms) / setTimeout(fn, ms)
      -> PendingTimer interval: Option<Duration>
      -> Engine::next_timer_wait / op_browser recv_timeout
      -> JsRuntime::run_due_timers (single page worker)
      -> callable invocation + drain_microtasks FIFO checkpoint
      -> DomTextMutation -> Engine::tick_timers
      -> computed styles -> display-list native repaint

    queueMicrotask(fn)
      -> VecDeque<JsValue>
      -> script / event / timer checkpoint
      -> callback FIFO, bounded drain
      -> DOM update before next macrotask

The page caps timer IDs, interval rate and microtask queue length/total
enqueues. Intervals may clear themselves while executing. The source
queue still has no Promise jobs, MutationObserver, fetch completion
tasks, or full browser event loop prioritization.

## S15 - JS text request to post-presentation native pixels (M4.11)

Status: IMPLEMENTED as a nonstandard bounded callback API, not fetch().

    opFetchText(url, callback) in op_js::JsRuntime
      -> TextRequest {id, url} queue in VM
      -> Engine::dispatch_text_requests
      -> resolve_script_source and NetworkContext clone
      -> NetworkContext::load_text_for_page on capped IO worker
      -> same-origin / ResourceType::Other filter / text MIME / byte limits
      -> mpsc NetworkCompletion {generation, id, result}
      -> Engine::tick_timers on page worker
      -> JsRuntime::complete_text_request callback + microtasks
      -> DomTextMutation and computed styles
      -> native paint through existing unsolicited reflow

Failures reach the callback as error strings. Old page generations are
discarded. This is a narrow networking bridge without standard fetch,
Promise/Response, credential policy, CORS, abort or response streaming.
