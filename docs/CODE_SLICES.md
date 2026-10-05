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

Failed/blocked/over-budget images produce alt text (or `[image]` without alt) and
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

Status: IN PROGRESS. Author style collection/matching is connected to loaded DOM;
computed styles are not connected to pixels yet.

Current path:

```text
HTML
  -> op_html DOM
  -> collect <style> text + style="" declarations
  -> op_css tokenizer/parser
  -> supported Selector AST + Specificity
  -> right-to-left selector matching against op_dom
  -> per-NodeId StyleMap<MatchedDeclaration>
  -> retained StyleCollection in PreparedDocument
  -> resize reflow reuses retained DOM/images/styles
```

Matched declarations retain specificity, source order, !important and whether they
came from a stylesheet or inline attribute. If multiple selectors from one selector
list match the same element, the declaration is stored once with the highest matching
specificity. Parser errors are retained with the source NodeId. Non-CSS style types
are skipped. At-rules and unsupported selector forms remain explicit limitations.

Planned next path:

```text
StyleMap candidates
  -> cascade + inheritance
  -> computed style
  -> CSS-aware layout
  -> paint
```

Linked stylesheet loading remains later work after the local author-style/cascade
path is established.

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
