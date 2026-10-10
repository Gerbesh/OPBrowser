# OPBrowser Project Plan

Last updated: 2026-10-10

## M4.32a - Event.composedPath and legacy propagation alias (2026-10-10)

- DONE added Event.composedPath() to original new Event(), legacy
  document.createEvent(), native click, parser lifecycle and AbortSignal
  abort events. Runtime-owned event_paths snapshots reflect the actual
  target-first Element ancestry, followed by Document and Window only
  for connected nodes. Document events include Window; window and
  AbortSignal events have one target.
- DONE event paths are accessible only while the event dispatches;
  outside dispatch and after cleanup composedPath() returns an empty
  Array rather than exposing stale targets.
- DONE legacy Event.cancelBubble getter/setter reflects the internal
  stopPropagation flag. Setting true prevents later ancestors, setting
  false cannot undo a stop, while remaining handlers on the current
  target are still allowed. Legacy returnValue also applies to native
  click event objects, not just constructed Event instances.
- VERIFIED five original VM tests and two complete original HTML ->
  JavaScript -> DOM -> native paint integrations, including connected
  ancestor ordering, reusing a legacy event, signal/document/window
  events and normal HTML body/html nodes in the path.
- LIMITS: no composed Shadow DOM boundaries/retargeting, immutable
  WebIDL Event descriptors, complete Event constructor options,
  full event listener object interface or general WPT Events
  conformance. Frozen original WPT counts unchanged.
- NEXT M4.32b: more accurate native-versus-programmatic isTrusted,
  composed event options, propagation retargeting for more node kinds,
  and independently pinned upstream Events WPT smoke expansion.

## M4.31 - Stable EventTarget registrations and property handler order (2026-10-10)

- DONE assigned monotonically advancing registration IDs to each original
  Element, document, window and AbortSignal event listener registration.
  Dispatch snapshots now distinguish a removed callback from a subsequent
  registration of the same JS function; the later registration is not
  invoked from the old snapshot.
- DONE preserved listener metadata (once/passive/signal/registration ID)
  when JS-created elements bind to native op_dom NodeIds.
- DONE integrated onclick with normal target-phase callback delivery,
  ordered by registration time rather than always invoking it last.
  Null/removal plus re-registration gets a new slot; stale handler
  snapshots do not accidentally run.
- DONE unified readystatechange/onreadystatechange, window load/onload
  and AbortSignal abort/onabort target-phase callback ordering, with
  preserved property slot on direct reassignment and distinct slot after
  clearing. stopImmediatePropagation and isolated exceptions apply.
- VERIFIED eleven new VM event-order/identity regressions and four native
  JS -> DOM -> paint tests, including virtual-to-physical node binding,
  post-load timers and lifecycle/abort events.
- LIMITS: bounded partial EventTarget; handler-object callbacks,
  standards-level property descriptor behavior, Shadow DOM/retargeting,
  trusted default activation, generic WebIDL and full DOM WPT Events
  conformance remain unimplemented. Frozen external WPT counts unchanged.
- NEXT M4.32: expand independently pinned upstream WPT Events samples,
  refine listener snapshot/at-target behavior and consider active
  WinHTTP cancellation rather than only dropping late JS completions.

## M4.30d - DOMException, composable signals and abort-aware fetch (2026-10-10)

- DONE original VM DOMException constructor and branded instances with
  name/message/legacy numeric code, toString() and guarded data fields.
  Default AbortController.abort()/AbortSignal.abort() reasons are native
  DOMException AbortError objects (name AbortError, code 20), while
  TimeoutError uses code 23. Custom JS reason identity is preserved.
- DONE AbortSignal.timeout(ms) scheduling in the page-owned event loop:
  bounded 0..60000 ms, no additional OS threads. next_timer_wait() and
  run_due_timers() include signal deadlines; timer abort event handlers
  can update the original DOM and drain microtasks.
- DONE AbortSignal.any(signalArray) for bounded array-like inputs (max
  64), first-aborted source reason, cascaded follower abort events and
  immediate removal of listeners registered with composed signals.
  This is not yet a general iterable protocol implementation.
- DONE Request(input,init) carries and clones signal; fetch(url/init)
  uses the signal for Promise rejection with the actual reason.
  opFetchText validates native AbortSignal identity, suppresses
  pre-aborted jobs, tracks outstanding signal-bound callbacks and
  removes queued work/late completions synchronously upon abort.
  A blocking WinHTTP request already executing MAY finish in its
  worker thread; its JavaScript completion is discarded, but this
  does not actively cancel the underlying HTTP operation.
- VERIFIED thirteen new VM tests (5 signal/DOMException, 5 fetch
  cancellation, 3 polish/cross-feature) and one real delayed HTTP
  integration alongside existing native fetch and browser tests.
- LIMITS: full DOMException WebIDL constructor descriptors, arbitrary
  signal deadlines, AbortSignal.any generic iterables, fetch streaming,
  credentials/CORS/non-GET and physical cancellation of running WinHTTP
  are unsupported. Existing frozen WPT/Test262 scores are unchanged.
- NEXT M4.31: stable EventTarget listener registration identities,
  dispatch-time remove/readd conformance and more independently sourced
  WPT Events fixtures, then improve active network cancellation.

## M4.30a - AbortController and signal-driven listener cancellation (2026-10-10)

- DONE original AbortController constructor with persistent signal identity;
  AbortController.abort(reason) is idempotent, records signal.aborted and
  reason and delivers one nonbubbling, noncancelable 'abort' event.
- DONE AbortSignal event target with add/removeEventListener, dispatchEvent,
  onabort property, and native AbortSignal.abort(reason) factory.
  signal.throwIfAborted() throws the stored reason.
- DONE read-only exposed signal.aborted/reason and controller.signal in
  the bounded VM's property assignment path.
- DONE Element/document/window addEventListener options.signal validates
  AbortSignal, rejects invalid objects, skips already-aborted signals,
  preserves first registration options on duplicates, and removes matching
  listeners synchronously on abort, including within an active dispatch.
- DONE seven new VM regressions and two original JS -> DOM -> native-paint
  integrations, including abort from a timer after initial page paint.
- LIMITS: default abort reason currently a string 'AbortError' rather than
  a DOMException; AbortSignal.timeout(), AbortSignal.any(), abort-driven
  cancellation of fetch/timers and complete DOM EventTarget semantics
  remain unsupported. No new original-source WPT/DOM metrics.
- NEXT M4.30b: native DOMException AbortError and AbortSignal.timeout/any,
  then connect AbortSignal to fetch's request cancellation and expand
  original pinned WPT Events coverage.

## M4.29b - Connected Element events through Document/Window (2026-10-10)

- DONE original Element.dispatchEvent(Event), synthetic element.click(), and
  native hit-tested clicks follow the complete bounded connected event path:
  window capture -> document capture -> ancestor captures -> target ->
  ancestor bubbles -> document bubble -> window bubble.
- DONE capture phases still execute for nonbubbling events; bubbling-only
  phases are suppressed when Event.bubbles=false.
- DONE document/window-only native click listeners can trigger dispatch and
  DOM repaint even without an Element listener. Host-side pre-dispatch
  filtering now checks the full attached path.
- DONE original op_js receives actual parser document-root identity from
  op_engine::scripts::sync_dom_tree. Detached and subsequently removed
  nodes do not propagate to global targets; non-element root nodes are not
  mistaken for DOM Elements.
- TESTED five new VM event-path/removal tests, one native click integration
  and one original JS -> native paint custom-event integration. Earlier
  M4.29a and original DOM click regressions remain unchanged.
- LIMITS: intentionally bounded path (64 elements), no Shadow DOM/retargeting,
  no user-gesture/default-activation algorithms, no AbortSignal and no
  full EventTarget/Web Platform Test coverage. Pinned upstream WPT DOM/Events
  v5 manifest and score unchanged; unavailable original upstream checkout
  is not represented as a verified new result.
- NEXT M4.30: original AbortController/AbortSignal listener removal,
  signal state and interoperable option handling, then broaden real WPT
  EventTarget fixtures with independently pinned source revisions.

## M4.29a - Document/Window EventTarget and lifecycle listener options (2026-10-10)

- DONE original document/window addEventListener/removeEventListener accept
  boolean capture or options dictionaries (capture, once, passive).
- DONE host readyState/DOMContentLoaded/window load callbacks share isolated
  exception reporting, stopImmediatePropagation and once removal before call.
  Legacy load event target remains document for compatibility with existing
  page lifecycle tests.
- DONE general document.dispatchEvent(Event) and window.dispatchEvent(Event)
  for bounded custom event types, including document->window capture and
  optional bubbling, cancelation return values, per-dispatch flag reset,
  recursion guard, property handler integration, and microtask draining.
- DONE five VM and two native text-paint regressions; existing lifecycle
  assertions still pass.
- LIMITS: Element-originated events do not yet extend their path to
  document/window; addEventListener signal/AbortSignal is not implemented.
  Listener objects, default browser actions, async WPT harness and complete
  EventTarget algorithm remain future work. Original pinned WPT/Events v5
  baseline is unchanged and not extrapolated to general conformance.
- NEXT M4.29b: unify Element-to-document/window propagation and lifecycle
  event paths, followed by AbortController/signal and a broader frozen
  original-source WPT sample.

## North star

Build an independent, lightweight, privacy-oriented Windows 11 browser with an
original web engine and original ECMAScript engine, while progressively reaching
modern web-platform conformance.

Primary conformance targets:

- HTML5test feature coverage: full target, never faked.
- Web Platform Tests: primary browser-platform conformance signal.
- TC39 Test262: primary ECMAScript conformance signal.

## Status legend

- DONE: implemented and verified at the current milestone level.
- IN PROGRESS: active engineering work.
- NEXT: queued immediately after active work.
- BLOCKED: configured or implemented, but prevented by an external dependency/state.
- LATER: planned but intentionally deferred.

## Documentation, Wiki and code intelligence (2026-10-08)

- DONE reviewed local `wiki/` pages against current engine capabilities and
  external test measurements. Updated the home page, overview, compatibility,
  workflow and added current status/roadmap/reference-divergence guides.
- DONE recognized two pinned legacy Rec.2020 color reference mismatches as
  known, non-blocking WPT failures. They **remain failed** in the exact
  197/200 metric and the metadata-aware 198/200 metric. Never remove them
  from a frozen manifest merely to raise its percentage.
- DONE connected offline Code Graph: derive all local crate edges from
  Cargo manifests and produce `docs/GENERATED_CODE_GRAPH.md`.
- DONE connected curated Code Slicer: validate feature paths and source
  symbols in `tools/code_slices.json`, generating
  `docs/GENERATED_CODE_SLICES.md`. Both reports and Wiki links are
  checked in normal Windows CI by `python tools/code_intelligence.py --check`.
- DONE GitHub Wiki enabled, initialized and published to the distinct
  `Gerbesh/OPBrowser.wiki.git` repository. The main repository's `wiki/`
  source remains canonical; `tools/publish_wiki.py` rewrites links for
  live Wiki pages, including a navigable `_Sidebar.md` index.
  Git histories are independent and must be pushed separately.
- NEXT keep README, Wiki status, generated reports, manual dependency graph,
  Code Slices and known-divergence log synchronized with future changes.

## M4.1 - Page scripting: first vertical slice (2026-10-08)

- DONE first integration of OPBrowser-owned `op_js` into HTML document
  preparation. Classic inline scripts execute once, in DOM order, with one
  shared VM context per loaded page and instruction/heap budgets.
- DONE minimal, detached browser host binding:
  `document.getElementById(id)` returns an element snapshot,
  `element.textContent` gets/sets its text. Changes go through bounded
  DOM mutation records and update real `op_dom::Document` nodes.
- DONE CSS styles and layout are calculated after mutations, and retained
  DOM reflow preserves script-generated content; tests cover initial page,
  real data URL loading, multiple scripts, unknown IDs and length limits.
- DONE manual example `examples/js/dom-text.html` renders changed text
  through the ordinary Win32 browser pipeline.
- LIMITATION no external `script src`, parser-blocking script timing,
  synchronous DOM access before later parser nodes, events or `addEventListener`,
  DOM creation/appendChild, timers, global Web APIs, or HTML5test score.
  Script failures are non-fatal and reported via `active_script_report()`.
- NEXT M4.2 execute external same-origin scripts through the normal filtered
  resource loader; establish script loading order and lifecycle, then
  implement event dispatch and additional DOM APIs with honest test coverage.

## M4.2 - Filtered external classic JS (2026-10-08)

- DONE external `script src` URLs in document order alongside inline
  classic scripts, sharing one `op_js` runtime and its global bindings.
  The resource loader uses `ResourceType::Script` (not an image/CSS
  disguise), local/HTTP(S) source resolution, same-origin validation,
  explicit script MIME/UTF-8 decoding and independent size budgets.
- DONE same-origin restrictions on requested URL and final redirected URL,
  HTTPS-to-HTTP redirect prohibition via WinHTTP, non-fatal load failures,
  bounded 8 external requests / 512 KiB aggregate / 128 KiB per script /
  10-second request-phase deadline, plus existing VM/DOM budgets.
- DONE real WinHTTP loopback test (page, script, inline execution),
  local file sequence/reflow tests, missing external file recovery,
  cross-origin denial and request-filter `$script` rule enforcement.
  Manual example: `examples/js/external.html` with sibling `external.js`.
- LIMITATION classic-only scripts run after full DOM parse, not true
  parser-blocking timing. `async`, `defer` and `integrity` external
  attributes are skipped rather than falsely scheduled/verified.
  Cross-origin redirects are rejected after the transport has fetched
  them, not at each redirect hop. No broad JavaScript Web APIs, DOM
  events, modules, CORS, CSP, document.write, or HTML5test score.
- NEXT M4.3: event dispatch, DOM lifecycle and safe API expansion,
  followed by real parser-blocking/defer/async semantics and
  script-fetch redirect restrictions.

## M4.3 - First real click event integration (2026-10-08)

- DONE persist the original `op_js::JsRuntime` in the active prepared
  document instead of dropping it after the scripts run. Registered
  function closures and global state therefore survive real user clicks.
- DONE initial `element.addEventListener("click", callback)` and
  `element.onclick = callback` / `null` support for element objects
  returned by `document.getElementById`. The event object exposes
  `type`, `target`, and `currentTarget`; `this` is the target.
- DONE Win32 `WM_LBUTTONUP` now yields a document-relative click
  event when not handled as a hyperlink; browser worker dispatches
  on the retained page. Own layout collects hit boxes for id-bearing
  blocks. After listener DOM text changes, recompute CSS and repaint
  while preserving scrolling and session state.
- DONE tests verify retained closures, multiple clicks and `onclick`
  replacement/removal, native layout click coordinates, reflow after
  mutation, error-free WPT baselines, and native demo
  `examples/js/click.html` with sibling `click.js`.
- LIMITATION initial subset has no bubbling, capture, keyboard dispatch,
  browser default actions, `preventDefault`, `stopPropagation`,
  `removeEventListener`, arbitrary inline/flex/table hit regions,
  style mutation APIs or full HTML5test support.
- NEXT expand DOM event targeting and state, expose a real DOMEvent
  dispatch path, then implement parser-blocking/async/defer script
  timing, timers and independent Web APIs.

## M4.4 - Bubbling click events and removable listeners (2026-10-08)

- DONE native hit-testing now chooses the smallest id-bearing element
  under the cursor, even when only an ancestor registered a listener.
  The engine follows attached DOM parents and dispatches to the target,
  then its ancestors, bounded to 64 elements.
- DONE retained JS listeners receive stable `event.target`, changing
  `event.currentTarget`, `eventPhase` (2 target, 3 bubbling) and
  `event.bubbles`. Callback `this` is the current listener's element.
- DONE `element.removeEventListener("click", sameFunction)` removes a
  callback by identity, and duplicate listeners remain deduplicated.
  Engine/VM tests cover ancestry, field values and removed listeners.
- LIMITATION capture, stopPropagation, preventDefault, listener options,
  default actions, keyboard input and full DOM hit-test are unsupported.
- NEXT implement capture/cancellation and HTML script lifecycle
  (parser-blocking classic scripts, async/defer scheduling).

## M4.6 - Parser-blocking classic scripts (2026-10-08)

- DONE HTML tree builder exposes a closing-script callback. Classic inline and
  bounded same-origin external scripts execute when the parser closes their
  script element, before subsequently parsed nodes exist in the DOM.
- DONE one retained page VM survives each parser pause, with DOM snapshots
  refreshed before each execution and once after the final parse so later
  elements are visible to event callbacks. DOM text mutations apply before
  tree construction resumes; CSS/layout run after the complete DOM is built.
- DONE tests verify that early scripts cannot query future IDs, later scripts
  share globals, script source is flushed before callbacks, and listeners
  installed early can modify later DOM after a native click.
- LIMITATION tokenization is eager; this is a tree-builder execution pause,
  not a streaming parser with document.write/reentrant tokenization. External
  fetches block synchronously within bounded budgets. External async/defer
  and integrity remain skipped; no modules, event loop or DOMContentLoaded.
- NEXT add standards-aware deferred/async scheduling and lifecycle hooks,
  then larger DOM/Web API surface.

## M4.7 - Bounded defer and async classic-script scheduling (2026-10-08)

- DONE external classic defer scripts fetch on scoped workers and execute
  after the DOM is constructed, in original document order.
- DONE external classic async scripts fetch concurrently and execute in
  completion order at parser script boundaries or load finalization.
  Inline async/defer attributes are ignored; async wins over defer for
  external scripts with both attributes.
- DONE each in-flight source reserves the existing aggregate byte budget,
  honors script request/deadline limits and same-origin request filtering.
  All JS execution and DOM mutation remain on the engine thread.
- DONE regressions cover defer order and complete DOM, inline attribute
  timing, async loading, and WinHTTP slow/fast completion-order dispatch.
- LIMITATION async execution is polled at script closing tags or before
  initial page presentation, not by an independent browser event loop.
  Navigation waits for worker completion. document.readyState,
  DOMContentLoaded/load, document.write, modules and SRI are still absent.
- NEXT implement document lifecycle state/events, followed by interactive
  asynchronous completion and task scheduling.
## M4.8 - Page readyState and lifecycle events (2026-10-08)

- DONE host-owned read-only document.readyState transitions from "loading"
  during tree construction to "interactive" after HTML parsing, then to
  "complete" after pending initial async script requests finish.
- DONE document.addEventListener/removeEventListener for readystatechange
  and DOMContentLoaded, plus document.onreadystatechange property.
  window.addEventListener/removeEventListener for load and window.onload
  property are available through the original JS VM.
- DONE document readystatechange fires after each state transition;
  DOMContentLoaded fires after all deferred classic scripts but does not
  wait for async scripts; window load fires after initial async completion.
  Lifecycle callbacks run in the single engine VM and their DOM text
  mutations are committed before CSS/layout/paint.
- DONE tests verify lifecycle ordering and event receiver/target, read-only
  readyState, registration/removal and property callbacks, and defer
  registration before DOMContentLoaded dispatch.
- LIMITATION script/module error/load resource events, an independent task
  queue, microtasks, timers, post-presentation async work, document.write,
  broader event propagation and full document lifecycle are unsupported.
  Initial navigation still waits for async work before presenting the page.
- NEXT introduce a page-owned task queue/timers and move async fetch
  completions to interactive worker scheduling; expand DOM APIs.
## M0 - Foundation

Status: DONE at initial level.

- DONE Rust workspace under C:\OPBrowser.
- DONE Native Win32 top-level browser window.
- DONE Separate browser, engine, DOM, HTML, CSS, layout, paint, JS, network, and Windows crates.
- DONE Git repository.
- DONE rustfmt / Clippy / test workflow.
- DONE debug and release builds.
- DONE non-interactive Win32 startup/paint smoke test.
- DONE documented dependency policy.
- DONE project documentation workflow, code graph, code slices, and local wiki.
- DONE public GitHub repository: https://github.com/Gerbesh/OPBrowser
- DONE continuous Git commit/push workflow to public main.
- DONE GitHub Actions Windows CI restored for main pushes, pull requests and manual dispatch.
  Superseded runs are cancelled automatically. Restored run 37527054337 completed successfully
  on a hosted Windows runner, confirming the previous billing lock no longer blocks execution.

## M1 - First static document pipeline

Goal: own bytes -> own HTML parser -> own DOM -> own layout -> own paint -> pixels.

- DONE initial HTML tokenizer state machine.
- DONE owned comment tokenizer states, malformed closing/EOF recovery and invisible
  comment handling through tree construction, CSS :empty and retained painting.
- DONE typed doctype tokens with name/PUBLIC/SYSTEM/force-quirks recovery and bogus
  HTML declarations as invisible comments, preserving raw-text/RCDATA and reflow.
- DONE DOM comment and DocumentType nodes with preserved token data; comments split DOM
  text at their real position while CSS/layout keep them non-rendering and :empty-neutral.
- DONE WHATWG doctype-driven DocumentMode selection for no-quirks, limited-quirks and
  quirks, including the complete legacy public/system identifier compatibility matrix.
- DONE first real tree-construction insertion-mode slice: initial/before-html/before-head/
  in-head/after-head/text/in-body, implicit html/head/body creation, head-token routing,
  duplicate html/body attribute merging and HTML non-void self-closing recovery.
- DONE in-body structural recovery slice: normal/list-item/button scope checks, implied
  end tags, paragraph/block autoclosing, li/dd/dt and heading recovery, nested-button
  recovery, generic end-tag special-boundary handling, </br> and legacy <image> recovery.
- DONE after-body/after-after-body insertion modes: </body>/</html> switch parser state
  without destroying the recovery stack, comments land on html/document as specified,
  whitespace/html tokens delegate to in-body and stray trailing content re-enters in-body.
- DONE active formatting elements/adoption-agency slice: reconstruction for the HTML formatting
  tag set, Noah's Ark three-entry cap, repeated-anchor/nobr recovery, applet/marquee/object
  marker boundaries and furthest-block DOM reparenting for misnested formatting.
- DONE table tree-construction slice: in-table/text/caption/column-group/table-body/row/cell
  insertion modes, implicit tbody/tr recovery, table-scope cleanup, cell formatting markers,
  pending table-text buffering and foster parenting before the last table.
- DONE CSS table sizing/alignment follow-up: table display roles/UA defaults, captions, row groups,
  2D row/cell placement, colspan/rowspan occupancy, content-driven min/max track sizing from
  measured text/images/cell constraints, col/colgroup width hints, inherited border-spacing,
  initial collapsed cell-edge conflict resolution and table-cell baseline/top/middle/bottom
  vertical alignment, all through native paint.
- DONE initial anonymous-table fixup: child-side repair creates missing rows/cells inside table
  roots/row groups, while normal-flow sibling collection groups consecutive orphan table-internal
  boxes under one layout-only anonymous table wrapper. Orphan captions join the repaired wrapper
  and orphan columns contribute track width hints. No synthetic DOM nodes are created and
  whitespace/display:none separators do not split a repaired sibling run.
- DONE initial true inline-table formatting: display:inline-table creates one atomic inline object
  backed by the normal table_box/grid formatter, uses first-row baseline alignment, shrink-to-fit
  auto width, authored box model/margins and transfers nested text/images/link identity into paint.
- DONE atomic inline-table vertical alignment for baseline/top/middle/bottom; line ascent/descent
  expands for tall top/bottom-aligned atoms and final nested paint output follows the aligned box.
- DONE initial advanced table width/caption pass: auto layout retains percentage constraints from
  col/colgroup/cell widths, table-layout:fixed uses column hints then first-row widths before sharing
  remaining space, and caption-side:top/bottom places captions outside the table border/background.
- LATER mode-specific legacy CSS/layout quirks, processing instructions, foreign content/CDATA,
  template/frameset insertion modes, complete script-data escape states and remaining advanced
  table layout: complete CSS Tables overconstraint/min-width/percentage edge algorithms,
  table/row-group/row/column collapsed-border conflict precedence and deeper anonymous colgroup
  repair. General inline vertical-align still needs sub/super/text-top/text-bottom/length/% support.
- DONE initial DOM arena with stable NodeId values, attributes, and parent/child relationships.
- DONE initial HTML tree builder from tokenizer output into op_dom::Document.
- DONE initial document-to-layout pipeline with basic text flow and wrapping.
- DONE initial paint/display-list primitives.
- DONE Win32 GDI backend consuming OPBrowser display-list commands.
- DONE first in-memory HTML page rendered by the complete OPBrowser pipeline.
- DONE smoke test verifies WM_PAINT actually ran.
- DONE local filesystem document loading.
- DONE file: URL loading with percent decoding.
- DONE data:text/html URL loading with percent and base64 decoding.
- DONE startup source argument wired through op_net -> engine -> renderer.
- DONE navigation history core with navigate/back/forward/reload.
- DONE failed navigations leave history unchanged.
- DONE new navigation after Back discards the old forward branch.
- DONE post-startup native address input, Go/Back/Forward/Reload, display replacement.
- DONE initial owned HTTP(S) URL parser (ASCII hosts, ports, IPv6, UTF-8 paths/query).
- DONE bounded HTTP(S) HTML fetching using WinHTTP transport and system TLS/proxy.
- DONE worker-thread loading with visible loading/errors and history rollback.
- DONE mouse-wheel scrolling and block defaults inside structural HTML containers.
- DONE external https://example.com navigation + native repaint verified.
- DONE clickable text hyperlinks with measured hit regions and scroll-aware input.
- DONE relative HTTP(S)/local-file link resolution using the loaded document base.
- DONE link input -> worker -> load -> history -> pixels smoke coverage.
- DONE owned UTF-8/UTF-16/Windows-1251/Windows-1252 document decoding.
- DONE BOM -> transport -> first-1024-byte meta charset selection and label aliases.
- DONE common named and numeric references in HTML text/attributes, including hrefs.
- DONE initial raw-text/RCDATA tokenizer handling to preserve script/style text.
- DONE Windows-1251 source -> Cyrillic pixels -> decoded hyperlink smoke coverage.
- DONE full HTML named-reference table, two-scalar results and longest-match lookup.
- DONE exhaustive named-reference text/attribute/RCDATA tests and native link smoke.
- NEXT broader encoding/URL conformance.
- DONE basic PNG/JPEG/GIF/BMP image subresources from HTTP(S), local files and data URLs.
- DONE bounded worker loading, per-page reuse, alt fallback, dimensions and alpha painting.
- DONE clickable image links with scroll-aware native hit regions and image smoke tests.
- DONE initial mixed text/image lines, measured font extents, baseline alignment and wrapping.
- DONE inline sibling grouping around blocks, HTML whitespace/NBSP and explicit br breaks.
- NEXT progressive image loading, animation and additional formats.
- DONE resize reflow from retained DOM/images without refetch or history mutation.
- DONE debounced resize input, stale-width result suppression, scroll/link-region updates.
- NEXT richer CSS block/inline layout and shaping.

Exit condition: OPBrowser renders a non-trivial local HTML document using only its
own HTML/DOM/layout/paint pipeline. Achieved at the initial M1 level. The current
iteration also opens external HTML sites, including declared Windows-1251 pages,
through the address bar and renders raster images beside measured text; full CSS,
image animation, scripting and modern-site compatibility remain later milestones.

## M2 - CSS foundation

Status: IN PROGRESS.

- DONE initial owned CSS tokenizer with spans, comments, strings/escapes, identifiers,
  hashes, numbers, percentages, dimensions, functions and structural tokens.
- DONE stylesheet and declaration-list parsers with bounded error recovery, !important
  extraction and custom-property name preservation.
- DONE initial selector AST/parser for type, universal, class and ID simple selectors,
  selector lists, descendant/child combinators and specificity calculation.
- DONE collect CSS from embedded style elements and inline style attributes.
- DONE match the supported selector subset against op_dom and retain per-node StyleMap
  candidates with specificity, source order, stylesheet/inline source and parse errors.
- DONE Engine retains the author StyleMap beside DOM/images across resize reflow.
- DONE initial author cascade over !important, inline-vs-stylesheet source, specificity
  and source order; invalid literal supported-property values are ignored before winner choice.
- DONE initial inheritance/global keywords plus ComputedStyleMap for display, color,
  font-size and font-weight, with temporary UA defaults matching the M1 layout baseline.
- DONE Engine retains computed styles across resize reflow.
- DONE feed computed display/font-size/font-weight/color into layout and paint, including
  mixed inline style runs sharing one line/baseline and display:none/block/inline flow.
- DONE render CSS text color through the platform-neutral display list; alpha colors are
  composited against the current white page background.
- DONE load bounded external `<link rel="stylesheet">` resources from local/file/data/HTTP(S)
  through op_net, preserve their DOM source order with embedded styles, and retain the result
  across reflow; failed stylesheet subresources are nonfatal.
- DONE initial block-level box model: `margin`/`padding`, `background-color` and solid/none
  borders reach computed style, layout geometry, display-list fills and native paint.
- DONE expanded block sizing/value layer: margin/padding side longhands, border side and
  width/style/color shorthands/longhands, `width`/`height` + min/max, `box-sizing`, auto
  horizontal margins, negative margins, percentages, em/rem and CSS absolute length units.
- DONE adjacent sibling vertical margin collapsing (positive/negative combinations) and
  block-boundary whitespace suppression; parent/child/empty-block collapsing remains later.
- DONE move the previous temporary heading/paragraph/list spacing into computed UA margins
  so normal flow consumes one box-model spacing path instead of separate semantic offsets.
- DONE expand selector syntax/matching with attribute selectors (`[a]`, =, ~=, |=, ^=,
  $=, *=, i/s flags), adjacent/general sibling combinators and structural pseudo-classes
  :root/:first-child/:last-child/:only-child/:empty/:link.
- DONE expand color values with legacy/modern `rgb()`/`rgba()` and `hsl()`/`hsla()`,
  percentage/alpha channels, hue angle units, clamping, CSS basic named colors plus
  `rebeccapurple`, and functional colors in border shorthand/longhands.
- DONE modern `hwb()` percentage/number whiteness and blackness, gray normalization,
  hue units, alpha and used-value none components through text/background/border/pseudo paint.
- DONE all 148 opaque CSS named colors and aliases from pinned W3C data, allocation-free
  case-insensitive lookup, 2,210-byte static table and reproducible offline generation/CI check.
- DONE `color(srgb ...)` and `color(srgb-linear ...)` with percentage/number channels,
  optional alpha/none, linear-to-encoded transfer and initial 8-bit channel clipping.
- DONE modern RGB/HSL none components and numeric HSL saturation/lightness on the percent
  reference scale; strict legacy RGB uniform channel units and comma-HSL percentage grammar.
- DONE richer initial typography: inherited `text-align` start/end/left/right/center,
  `line-height` normal/number/percent/length with real line-box geometry, and numeric
  `font-weight` 1-1000 plus bolder/lighter mapped onto the current normal/bold backend.
- DONE `font-style` normal/italic/oblique through GDI font realization/measurement, initial
  `text-decoration`/`text-decoration-line` underline + line-through native painting, and
  `white-space` normal/nowrap/pre/pre-wrap/pre-line behavior in the owned line formatter.
- DONE inherited `letter-spacing`/`word-spacing` lengths plus `text-transform`
  none/uppercase/lowercase/capitalize; transformed Unicode text is measured before layout,
  spacing affects wrapping/alignment/link bounds, and Win32 paints matching visual advances.
- DONE initial inline box fragments for non-replaced inline elements: background-color,
  padding and solid per-side borders participate in width/wrapping/alignment, expand safe
  line geometry, survive ordinary nested text styling and emit per-line BoxDecoration paint.
- DONE functional pseudo-classes `:is()`/`:where()`/`:not()` with nested selector lists and
  correct specificity rules, plus `:nth-child(An+B)` over element siblings and a color-only
  `background` shorthand sharing cascade priority with `background-color`.
- DONE terminal `::before`/`::after` pseudo-elements with type specificity, separate author
  cascade buckets and computed pseudo styles; quoted-string `content` enters the normal inline
  formatter with inherited typography plus its own color/background/padding/solid borders.
- DONE inherited case-sensitive CSS custom properties with author cascade/`!important`,
  computed per-element token values, bounded `var(--name, fallback)` substitution, directed cycle
  invalidation and pseudo-element inheritance/overrides before normal property value parsing.
- DONE generated `content` functions for `attr(name)`, `counter()` and `counters()` plus
  initial `counter-reset`/`counter-set`/`counter-increment`, nested counter scopes and decimal,
  decimal-leading-zero, alpha/latin and roman formatting.
- DONE inherited `quotes` auto/none/string pairs, generated open/close/no-open/no-close quote
  commands and UA `<q>` pseudos; nesting follows emitted document-order content, repeats the
  deepest pair and ignores hidden/absent pseudos and subtrees for quote/counter mutation.
- DONE generated `display:block` shares ordinary block sizing/margins/padding/borders,
  including percentage/min/max sizes, auto margins, wrapping and empty decorated block boxes.
- DONE definite block height controls flow/decorations even when text overflows.
- DONE iterative custom-property dependency graph/SCC resolution including unused fallback
  edges and self-cycles, valid empty custom values and bounded expansion/storage/depth.
- DONE invalid-at-computed-value-time var() winners resolve to unset for supported properties,
  including shorthand components/pseudo content/quotes/counters; malformed var() is rejected
  before cascade. Literal invalid values remain parse-time exclusions.
- DONE forgiving is()/where() selector-list recovery, strict not()/of lists, filtered
  nth-child/nth-last-child with maximum-filter specificity and token-aware An+B grammar;
  functional selector nesting is bounded to 64 levels.
- DONE empty generated/DOM inline decoration boxes reserve edges, wrap/align and paint
  without fabricated text commands; nowrap and line extents use the existing formatter.
- DONE empty inline frames with hidden/empty descendants or only collapsed whitespace;
  block boundaries suppress duplicate empty-fragment synthesis and preformatted spaces remain text.
- DONE first/last/only-of-type and nth-of-type/nth-last-of-type share sibling indexing with
  same-tag filtering, token-aware An+B grammar and normal pseudo-class specificity.
- DONE generated `url()` images in ordered before/after text/image lists, stylesheet-relative
  bases including redirects/var() consumers, shared DOM/generated resource budgets/cache,
  intrinsic inline/block image flow, generated image links and retained reflow.
- DONE DOM image CSS width/height/min/max/box-sizing, inherited font-relative and percentage
  width resolution, HTML size hints before author cascade and intrinsic ratio constraints.
- DONE sole-URL inline generated image replacement with CSS width/height/min/max/box-sizing,
  own padding/background/borders, intrinsic ratio constraints and retained image link identity.
- DONE shared block DOM/sole-URL replaced-image geometry with intrinsic/CSS width, auto
  margins, padding/borders, precise box height and adjacent vertical margin collapsing.
- DONE nested decorated-inline stacks with parent-linked arena indices, cumulative edge
  geometry, shared text/image/empty/pseudo fragments and outer-before-inner background paint.
- DONE unavailable sole-URL/empty-or-absent-alt image geometry with zero natural sizes,
  independent CSS axes, transparent atomic/block boxes and no fake raster allocation.
- DONE styled nonempty alt fallback through ordinary inline fragments or block sizing;
  mixed generated failures skip only images while retaining empty pseudo decorations.
- NEXT sliced inline decoration edges and broader computed values.
- DONE DOM image padding/background/solid borders as atomic inline boxes, edge-aware
  width fitting, border-box baseline extents, text alignment and nowrap behavior.
- DONE declaration source-node provenance and retained effective external stylesheet
  addresses across redirects, duplicate link reuse and resize reflow.
- DONE CSS Url/BadUrl tokenization with escapes, punctuation-preserving addresses/data URLs,
  quoted function distinction, bounded bad-URL recovery and declaration rejection.
- DONE computed UA link color/underline defaults and author overrides through native GDI,
  including nested/generated text and retained reflow with unchanged link hit identity.
- LATER language-aware `quotes:auto` (currently deterministic English Unicode pairs).
- LATER broader custom-property grammar/registration/animation-taint behavior,
  relational selectors, sliced inline decoration edges and fuller
  parent/child margin collapsing / definite percentage-height propagation.
- IN PROGRESS readable-static-web priority: computed `position: static|relative|absolute|fixed`
  plus all four inset properties feed layout. Relative boxes preserve normal-flow geometry while
  px/percentage offsets translate their output. Absolute/fixed boxes leave normal flow and now use
  viewport-height-aware or nearest-positioned padding-box geometry, px/percentage insets on both axes,
  bottom-only placement, opposing-inset auto width/height stretching, shrink-to-fit auto widths and
  direct percentage-height resolution from definite containing blocks. Inline absolute/fixed boxes
  with static-positioned axes retain a zero-width marker at the real inline cursor instead of
  flushing the line. Initial `display:inline-block` is atomic, shrink-to-fit and BFC-like; horizontal
  inline margins, including negative margins, participate in advance without painting. Block children
  now split enclosing inline boxes into continuation fragments with CSS2 start/end edge suppression,
  including logical LTR/RTL edges, empty intermediate line fragments and inherited relative visual
  offsets for block/float descendants. Very large finite CSS lengths remain computed and are bounded
  only at used layout geometry. Absolute/fixed non-replaced blocks now also solve horizontal and
  vertical auto margins inside definite opposing insets, including negative available space,
  one-auto-margin cases and direction-dependent horizontal overconstraint precedence.
  Inline containing blocks now record relative inline fragment rectangles, including
  continuations across separate formatting runs and intervening block descendants.
  Absolute descendants are resolved after all fragments are measured; nested deferred
  positioned subtrees are drained in order, with relative visual translations retained.
  RTL/LTR static block offsets without horizontal insets use the hypothetical flow width
  rather than the viewport width. Complete bidi/vertical writing, multicol,
  replaced-element/complex CSS2 positioned constraints and stacking remain NEXT, followed by
  `overflow`, media queries, font faces, background images, border radius and broader flex/grid work.
- LATER broader computed values outside the readable-static-web priority.
- LATER fuller normal flow and CSS inline formatting plus Unicode line breaking.
- NEXT migrate the text backend toward DirectWrite shaping/fallback behind TextMeasurer per ADR-0003.
- DONE versioned WPT Static v1 measurement with 200 pinned HTML/CSS reftests and an initial 86/200 (43.00%) baseline.
- DONE versioned Test262 Parser v1 measurement with 2,000 pinned language paths and an initial 364/1983 (18.36%) executable baseline; module entries are skipped.
- DONE GitHub Actions publishes compatibility artifacts and README badge data after successful main pushes.
- DONE WPT Positioning v1 adds 100 pinned positioning/visual-formatting reftests at the existing
  WPT revision with a deliberately broad initial baseline of 18/100 (18.00%) and zero render errors;
  WPT Static v1 remains frozen at 187/200 (93.50%) for historical comparability.
- DONE opt-in WPT failed-reftest bitmap diagnostics with dependency-free top-down BMP output,
  maximum 12 saved actual/reference pairs per run and unchanged metric semantics.
- DONE CSS Color 4 Lab/LCH/OKLab/OKLCH plus display-p3/display-p3-linear, A98 RGB, ProPhoto RGB, Rec.2020 and XYZ predefined-space conversion to the current 8-bit sRGB paint target; `currentColor` now resolves for color/background/borders.
- DONE first metric-driven WPT pass raised WPT Static v1 from 86/200 (43.00%) to 126/200 (63.00%) without changing the manifest.
- DONE deterministic CSS system colors/deprecated aliases, simple declaration-form `@supports`, Selectors 4 `:lang()` Extended Filtering, inherited `:dir(ltr|rtl)`, and initial `:open`/`:required`/`:optional`/`:visited` semantics.
- DONE second metric-driven pass raised the unchanged WPT Static v1 manifest from 126/200 (63.00%) to 151/200 (75.50%).
- DONE deferred computed color expressions preserve `currentColor` dependencies across inheritance; initial `color-mix()` (`srgb`/`lch`) and relative `from currentColor` forms now reach used sRGB colors.
- DONE basic `display: contents` suppresses the principal box while preserving generated/child content, and `nth-child(... of ...)` accepts selector lists immediately after the required whitespace-before-`of` separator.
- DONE third metric-driven pass raised the unchanged WPT Static v1 manifest from 151/200 (75.50%) to 172/200 (86.00%).
- DONE Selectors 4 `:has()` with descendant/child/adjacent/general-sibling relative selectors, maximum-argument specificity and nested/pseudo-element rejection; empty-namespace attribute selectors now preserve CSS whitespace rules.
- DONE background-only empty inline boxes with no padding/border keep zero geometry and no longer create fake lines or interrupt collapsible whitespace.
- DONE fourth metric-driven pass raised the unchanged WPT Static v1 manifest from 172/200 (86.00%) to 179/200 (89.50%).
- DONE table formatting now treats structural `display:contents` wrappers as transparent during anonymous row/cell fixup while preserving non-table contents nodes as inherited-style carriers.
- DONE fifth metric-driven pass raised the unchanged WPT Static v1 manifest from 179/200 (89.50%) to 181/200 (90.50%).
- DONE `::first-letter` is a distinct fragment pseudo: selector/cascade support feeds grapheme-aware inline styling, including Regional Indicator flags and inherited-style preservation through `display:contents`.
- DONE sixth metric-driven pass raised the unchanged WPT Static v1 manifest from 181/200 (90.50%) to 182/200 (91.00%).
- DONE self-collapsing zero-height blocks now keep their adjoining margin set pending through inline-only wrappers and empty parents, so block-in-inline margin collapse can propagate across the parent instead of summing top/bottom margins.
- DONE seventh metric-driven pass raised the unchanged WPT Static v1 manifest from 182/200 (91.00%) to 183/200 (91.50%).
- DONE initial BFC/float foundation: `flow-root` and `flow-root list-item` are block-level BFCs, left/right floats have scoped geometry, `clear` advances below matching floats, BFCs contain child floats and avoid active outer floats, and floated tables retain the table formatter.
- DONE CSS2 compatibility support for single-colon `:before`/`:after`/`:first-letter` plus inherited `visibility:hidden` that keeps layout geometry while suppressing text/image/decorative paint.
- DONE eighth metric-driven pass raised the unchanged WPT Static v1 manifest from 183/200 (91.50%) to 185/200 (92.50%).
- DONE initial flex formatting context: computed `display:flex`/`inline-flex`, default single-line row item layout, anonymous text flex items, atomic shrink-to-content inline-flex and recursive `display:contents` item flattening.
- DONE ninth metric-driven pass raised the unchanged WPT Static v1 manifest from 185/200 (92.50%) to 187/200 (93.50%).
- DONE first positioning-driven pass raised the unchanged WPT Positioning v1 manifest from
  18/100 (18.00%) to 21/100 (21.00%) while WPT Static v1 remained 187/200 (93.50%).
- DONE second positioning-driven pass added inline static-position markers, horizontal inline margins,
  atomic `display:inline-block`, positioned shrink-to-fit width and intrinsic SVG replaced sizing,
  raising the unchanged WPT Positioning v1 manifest from 21/100 (21.00%) to 25/100 (25.00%);
  WPT Static v1 remains 187/200 (93.50%).
- DONE third positioning-driven pass implemented CSS2 split-inline continuations, logical LTR/RTL
  fragment edges, required empty intermediate fragments, relative-inline offsets for split block/float
  descendants and safe used-value clamping for very large finite lengths, raising the unchanged
  WPT Positioning v1 manifest from 25/100 (25.00%) to 36/100 (36.00%).
- DONE initial per-run relative-inline containing rectangles for positioned descendants, with
  computed first/last fragment padding edges, LTR/RTL anchoring and relative visual translation;
  five new regression tests guard ordinary, nested, wrapped, RTL and legacy static-position cases.
  The unchanged pinned WPT Positioning v1 remains 36/100 (36.00%) with no regressions.
- DONE cross-run relative-inline containing rectangles: fragment tracking survives block boundaries,
  absolute children are deferred until the full containing rectangle is known, nested deferred
  subtrees resolve correctly, and relative ancestor visual offsets propagate to deferred geometry.
  CSS2 RTL/static horizontal block placement uses hypothetical flow width for absolute and fixed.
  Eight focused layout integration tests were added (36 total in the inline test suite).
  The unchanged WPT Positioning v1 is 36/100 with no score regression.
- DONE CSS2 font-derived inline decoration height is separated from line-height/half-leading.
  Inline-block vertical-align:baseline uses its last in-flow line baseline rather than its bottom.
  Block backgrounds now paint before inline backgrounds, preventing following blocks from erasing
  overlapping inline content. WPT Positioning v1 improved from 36/100 to 38/100; WPT Static v1
  remains 187/200 on unchanged manifests. Layout and paint regressions added.
- DONE initial foreground paint phase for CSS positioned blocks: absolute/fixed (and
  standalone relative blocks) now retain foreground decoration and text/image tags across
  inline and atomic layout boundaries. Normal-flow ink paints first; the foreground follows.
  Relative parents with positioned descendants preserve their ordinary internal text phase
  instead of incorrectly obscuring nested absolute children. Added engine and paint regressions;
  WPT Static v1 remains 187/200 and Positioning v1 remains 38/100 on frozen manifests.
- DONE first flat positioned z-index slice: non-inherited CSS integer/auto/global keywords,
  PaintKey propagation to decorations/text/images, sorted z-level and source-order groups.
  CSS and end-to-end paint regressions added; pinned WPT Static 187/200 and Positioning
  38/100 remain unchanged.
- DONE initial atomic block stacking-context groups: positioned blocks with explicit z-index
  now contain their descendants; absent parent pixels do not remove the context.
  Negative root groups paint beneath in-flow blocks; equal-z siblings follow final DOM
  preorder after parser reparenting. New integration regressions added with stable
  WPT Static 187/200 and Positioning 38/100 on the frozen subsets.
- DONE first positioned inline paint groups: relative inline box fragments (including
  wrapped continuations) now carry paint keys for backgrounds, text, and images;
  explicit z-index isolates nested positioned spans, while auto-z preserves
  descendants' participation in the surrounding context. Atomic inline-block
  children join their ancestor's paint group without losing separately positioned
  children; local inline arena indices no longer leak into independent contexts.
  Regressions cover layering, nested inline groups, auto-z, wrapping, atomic
  backgrounds and retained reflow. Frozen WPT Static 187/200 and Positioning
  38/100 remain unchanged with zero render errors.
- DONE CSS positioned auto-z own-ink grouping: relative blocks and inline-blocks
  now tag their own backgrounds/text at level zero even when separately positioned
  descendants exist. Their own ink follows z=0 source order while explicitly
  stacked children keep independent keys and can escape the auto-z parent.
  Added five regressions for zero-level ties, positive/negative children, explicit
  inline-block stacking isolation and reflow. WPT Static 187/200, Positioning
  38/100 remain unchanged with zero render errors.
- DONE table-part relative positioning and auto table column sizing: table cells
  now receive relative offsets inherited from row, row-group, header/footer-group
  and cell ancestors; relative row/section backgrounds are painted at their own
  stacking level. Empty all-absolute rows do not emit stray one-pixel section ink.
  Auto-sized table columns now honor explicit pixel-sized block descendants and
  shrink toward intrinsic preferred widths rather than always filling the parent.
  Relative table sections become containing blocks for their absolute cell
  descendants. New geometry/empty-row/containing-block tests added. Frozen WPT
  Positioning v1 improved 38/100 -> 53/100, Static v1 held at 187/200, both
  without render errors.
- DONE auto-width table wrapper consistency: table_box now resolves its own
  intrinsic width before painting its background, borders, and captions, using
  the same column min/preferred measurements as the grid. Horizontal
  border-spacing and box padding/border remain in the wrapper width, while
  explicitly sized tables remain authoritative. For table-layout:fixed with
  width:auto the engine keeps the automatic intrinsic algorithm. Six regressions
  cover narrow one/two-column tables, spacing/extras, explicit widths, fixed
  layout with auto width, and min-width. Frozen WPT Static 187/200 and
  Positioning 53/100 remain unchanged with zero render errors.
- DONE first multi-row rowspan height reconciliation: compute normal row track
  heights from non-spanning cells, then satisfy overlapping spanning-cell
  minimums across all covered rows (including internal border-spacing). Grow
  the final covered track and translate later cell output by the resulting
  row-origin deltas. This avoids double-counting a 120px rowspan as 140px
  of table height; nested/overlapping spans and subsequent rows have engine
  regressions, as does intrinsic colspan sizing. Frozen WPT Positioning remains
  53/100 and Static remains 187/200 without render errors.
- DONE focused WPT Static v1 compatibility recovery: fix near-black OKLab/OKLCH
  on the current 8-bit SDR output without changing bright wide-gamut clipping,
  suppress raw text directly under HTML select, and distinguish hidden SVG defs/
  SVG text display:contents from nested SVG containers while expanding simple
  text references from SVG use. Three previously failing static reftests pass,
  improving the unchanged frozen suite 187/200 -> 191/200 (93.50% -> 95.50%),
  with zero render errors. Add color and engine regressions. These are narrow
  slices, not complete CSS gamut mapping, HTML forms, or SVG rendering.
- DONE first bounded group-opacity and invert-filter pipeline: opacity and
  filter:invert() now participate in the computed CSS cascade. Opacity/filter
  owners and their positioned descendants form nested paint layers, rendered
  by the Win32 painter into independent white/black offscreen surfaces and
  recomposited once into a premultiplied-alpha image. Two GDI overlap tests
  and a computed cascade regression cover this. The frozen Static WPT suite
  improves 191/200 -> 192/200 (96.00%), resolving
  composited-filters-under-opacity, with no Positioning regression (53/100).
  Size and recursion budgets prevent unbounded offscreen allocations; oversized
  groups retain content without effects. This is not full CSS filter/opacity.
- DONE limited self-document visited-link handling: an empty href always
  points back to the current document, already visited during navigation.
  Preserve unvisited treatment for other links until history-backed,
  privacy-safe :visited styling exists. This closes the frozen
  color-mix-currentcolor-visited reftest, bringing Static to 193/200.
- DONE first URL-based CSS background-image support for blocks and tables:
  resolve authored background/background-image URLs through the computed
  cascade, respect !important and explicit inherit/reset, resolve stylesheet
  relative URLs through the existing bounded image loader, and tile decoded
  image pixels under a clipped decoration layer before borders. The Windows
  WIC path now explicitly converts embedded ICC profiles to sRGB by creating
  source color-context COM objects, then using WICColorTransform on the frame
  before premultiplication. This closes tagged-images-004 on the unchanged
  exact-pixel WPT Static v1 manifest: 194/200 (97.00%). Positioning stays
  53/100, no render errors. Tests cover cascade, tiling/clipping and ICC.
- DONE bounded CSS Color 5 @color-profile first slice: tokenize named
  @color-profile --name { src:url(...) } from inline/linked author CSS; load
  referenced ICC profiles through the existing filtered/budgeted network
  worker using stylesheet-relative addresses; resolve color(--name R G B)
  across matched declaration tokens without altering CSS strings/comments;
  convert RGB through native WIC into sRGB, caching repeated conversions.
  Invalid/unknown profiles remain invalid. Parser, CSS token, WIC regressions
  and the unchanged pinned at-color-profile-001 reference pass. Static
  194/200 -> 195/200 (97.50%), no WPT render errors.
- DONE first-line fragment styling foundation: recognize ::first-line and the
  legacy :first-line spelling in selector parsing and the author cascade,
  compute a fragment pseudo-style independently from generated content,
  and paint first-line text and background during the actual first line
  flush rather than recoloring the whole block. Preserve colors on later
  lines and on descendants with explicit color. Use GDI glyph metrics for
  first-line background geometry so the same ink box matches an ordinary
  inline span. Dedicated engine regressions verify color, breaks, and
  background heights. Frozen WPT Static advances 195/200 -> 196/200
  (98.00%) with first-line-bidi-002 passing exactly; no WPT fixtures
  or comparison thresholds were modified.
- DONE first-line currentcolor dependency resolution: preserve relative
  background-color and border-color provenance through computed style and
  inline box decoration, re-resolve it only on first-line fragments that
  inherit the host text color. Explicit colors and subsequent lines retain
  their original paint. Frozen currentcolor-003 now passes unchanged;
  Static 196/200 -> 197/200 (98.50%) without render errors or test edits.
- NEXT investigate XYZ byte rounding (source WPT fuzzy allowance=1) and
  two Rec.2020 references versus current gamma 2.4 CSS Color 4 transfer.
  Keep the strict WPT score unmodified; expand gradient/shadow support
  and positioning separately rather than add test-specific constants.
- DONE WPT-authored fuzzy metadata reporting without changing the strict
  metric: --report-wpt-fuzzy parses only original HTML meta[name=fuzzy]
  limits, validates both per-channel max difference and total differing
  pixels (including inclusive ranges and per-reference overrides), and
  reports a second visibly separate percentage. Correct the normal default
  font-size to conventional 16px; adapt geometry assertions to the changed
  CSS em/rem and glyph metrics. The xyz-003 rectangular area now covers
  exactly 18432 pixels and qualifies under its upstream maxDifference=0-1,
  totalPixels=0-18432 metadata. Strict Static remains 197/200 (98.50%)
  while optional WPT-metadata-aware Static is 198/200 (99.00%).
  Positioning remains 53/100; no suite/manifest/reference/tolerance edits.
- NEXT remaining two WPT-authored Static failures are the Rec.2020 colorspace
  references versus current CSS Color 4 gamma 2.4. Investigate reference
  provenance, improve wider color conversion and positioning independently,
  and do not hardcode reference colors into the engine.

## M3 - Original JavaScript engine

Status: IN PROGRESS.

- DONE initial owned ECMAScript lexer for scalar literals, identifiers/keywords, comments and
  arithmetic/comparison/assignment punctuation.
- DONE initial parser/AST for single `let`/`const`/`var` declarations, assignment, unary,
  arithmetic, comparison/equality and scalar literals.
- DONE initial bytecode format/compiler for the implemented syntax.
- DONE initial stack interpreter with persistent globals, mutable/const bindings, scalar
  coercion, arithmetic, string concatenation, truthiness and loose/strict equality.
- DONE initial control-flow bytecode: blocks, if/else, while, break/continue, multiple
  declarators and short-circuit &&/|| compile to patched jumps executed by an instruction-pointer
  VM; a bounded instruction budget aborts runaway loops instead of hanging the renderer.
- DONE second parser-measurement pass raised the unchanged Test262 Parser v1 manifest from
  364/1983 (18.36%) to 391/1983 (19.72%).
- DONE initial reference-object heap with stable ObjectId identity, own string-keyed properties,
  dot/computed member access and assignment, ordinary object/array literals, array holes/length
  growth and prototype-chain lookup/mutation with cycle rejection.
- DONE object/prototype parser pass raised the same Test262 Parser v1 manifest again to
  408/1983 (20.57%).
- LATER garbage collector; the current bounded object/environment heaps intentionally retain
  allocations for the runtime lifetime.
- DONE initial functions/calls/returns, function expressions/declarations, recursion, block lexical
  environments, function-scoped var, closure capture and scope-unwind for break/continue. Named
  function expressions keep a private recursive name binding; callable objects expose initial
  name/length properties and execution is bounded by call depth.
- DONE function/parser pass raised the same Test262 Parser v1 manifest again to
  504/1983 (25.42%).
- DONE initial broader control flow and abrupt completions: C-style for, do/while, switch
  case/default fallthrough, prefix/postfix ++/--, explicit throw, try/catch/finally, optional catch
  bindings and function-declaration hoisting. Return/throw/break/continue propagate through finally;
  switch break and continue-to-outer-loop use patched control targets with lexical-scope unwind.
- DONE broader-control/parser pass raised the unchanged Test262 Parser v1 manifest to
  508/1983 (25.62%).
- DONE initial call/constructor/error-object semantics: method calls preserve receivers, bare calls
  use the runtime global object for non-strict this, user functions receive array-like arguments,
  new allocates from constructor.prototype and follows constructor return rules, function objects
  own prototype objects with constructor links, and Error/TypeError/ReferenceError are built-in
  constructors. Runtime Type/Reference failures inside try regions become catchable JavaScript
  error objects while execution-limit failures remain engine-level guards.
- DONE this/new/error parser pass raised the unchanged Test262 Parser v1 manifest to
  523/1983 (26.37%).
- NEXT labels and for-in/for-of; arrow/default/rest/destructuring forms; per-iteration lexical
  environments for for(let); fuller built-ins/property descriptors; and explicit VM call frames
  instead of native recursive calls.
- PARTIAL self-hosted Promise, bounded microtask reactions and four array-like combinators (M4.12, M4.15).
- LATER modules.
- PARTIAL Error/TypeError/ReferenceError, JSON.parse/stringify and selected Promise methods (M4.15).
- DONE initial Test262 parse-expectation probe and combined compatibility command; this is
  explicitly not runtime conformance yet.
- NEXT build the real Test262 harness progressively as language/runtime semantics land.
- LATER JIT only if profiling justifies it after correctness.

## M4 - DOM scripting and Web APIs

- LATER Web IDL binding layer.
- LATER DOM mutation/events.
- LATER timers and event loop.
- PARTIAL same-origin GET fetch, guarded Request/Headers, Response.json and checked redirects (M4.12-M4.15).
- LATER URL/Encoding/Streams.
- LATER forms/editing.
- LATER storage.
- LATER workers/service workers.
- LATER WebSocket.
- LATER Canvas/SVG/MathML.

## M5 - Browser product architecture

Status: IN PROGRESS (foundations only; current UI still exposes one renderer/tab).

- DECIDED browser/renderer ownership and staged process split in ADR-0002; current navigation
  worker remains a temporary in-process renderer boundary.
- NEXT implement browser/renderer IPC and one renderer process per active tab.
- LATER renderer sandbox and process/site isolation policy after IPC is stable.
- DONE UI-independent `op_browser_core` tab model foundation.
- DONE lifecycle states: active/background/throttled/frozen/discarded/restoring.
- DONE initial discard protection and memory-pressure candidate policy with retained restore state;
  OS memory-pressure triggering and renderer teardown/restoration remain later.
- NEXT connect multiple tab-owned renderers to the native UI and session persistence.
- LATER built-in task manager using browser/renderer memory/CPU counters.
- LATER downloads/history/bookmarks/settings/permissions.
- DONE initial native request-filter layer on document/stylesheet/image loads with host/wildcard
  rules, exceptions, resource types, site allowlisting and counters.
- NEXT scalable filter indexing, list subscriptions/updates, third-party/domain options and UI;
  cosmetic filtering depends on DOM/style integration.

## M6 - Advanced platform

- LATER WebAssembly.
- LATER WebGL.
- LATER WebGPU.
- LATER media pipeline.
- LATER accessibility tree.
- LATER advanced security/isolation.
- LATER performance and power optimization.

## Continuous work

Every milestone continuously tracks security, WPT/Test262 regressions, startup time,
memory use, binary size, background CPU, and dependency growth. `tools/compatibility.ps1`
provides the project-owned baseline plus an optional Test262 parse probe; owned unit-test
counts must not be mislabeled as external conformance percentages.

## M4.5 - Capture and event cancellation primitives (2026-10-08)

- DONE boolean capture registration in addEventListener/removeEventListener.
  Click dispatch follows root-to-target capture, target, then bubbling
  with eventPhase 1/2/3.
- DONE event.stopPropagation() halts traversal after listeners on the
  current element; event.preventDefault() sets defaultPrevented.
- DONE VM regressions verify exact phase ordering, cancellation before
  target and observable defaultPrevented state.
- LIMITATION stopImmediatePropagation, option objects, once/passive,
  keyboard events and native default-action cancellation remain absent.
- NEXT make classic script execution parser-aware, then add defer/async.

## M4.8 - Document lifecycle (2026-10-08)

- DONE document.readyState transitions from loading to interactive after HTML
  tree construction, then complete after initial script work finishes.
- DONE bounded document readystatechange and DOMContentLoaded listeners,
  window load listeners, removal and onreadystatechange/onload properties.
  Lifecycle events run in retained JS VM and flush DOM text mutations.
- DONE deferred scripts can subscribe before DOMContentLoaded, and the
  lifecycle event sequence is covered by engine tests.
- LIMITATION no independent event loop, resource-aware load completion,
  bubbling of lifecycle events, document.write or general Web APIs.
- NEXT implement a real task queue and interactive async resource completion.

## M4.9 - Page-owned timeout task queue and post-presentation repaint (2026-10-08)

- DONE minimal own one-shot timer queue in the retained op_js runtime.
  setTimeout(function, delay, ...args) schedules callable closures and
  returns a numeric id; clearTimeout(id) cancels pending tasks.
- DONE timer deadlines use std::time::Instant and stay inside the page VM.
  No per-timer OS threads are created. A Win32 page worker waits on
  navigation requests with a bounded recv_timeout and executes due
  callbacks on the single page-owning engine thread.
- DONE after callback textContent mutations, the engine recomputes
  styles and layout, then sends a reflow paint result to Win32 even
  without mouse or keyboard activity.
- DONE per-page limits: 64 simultaneously pending timers, 512 scheduled
  timers total, at most 16 callbacks per pump, and delay clamped to 60s.
  Reload/new navigation discards all old page timers.
- DONE engine/VM regressions cover post-load firing, timer cancellation,
  callback arguments, delayed tasks, error isolation, navigation reset
  and click-triggered delayed DOM repaint.
- LIMITATION only function callbacks (no eval/string timers),
  setInterval, Promise/microtask queues, task priority, nested timeout
  throttling, offscreen/background throttling and independent async
  script completion after first paint are not yet implemented.
- NEXT unify browser task scheduling across timers, resource fetches,
  input and future microtasks; add interval policy and background tabs.

## M4.10 - Repeating timers and bounded microtask checkpoints (2026-10-08)

- DONE setInterval(callable, delay, ...args) and clearInterval(id) share the
  existing retained page task queue and numeric handle namespace with
  setTimeout and clearTimeout; either clear method cancels either timer.
- DONE repeated callbacks requeue their own interval before invocation,
  permitting clearInterval from inside the callback. Repeats are scheduled
  from the end of the preceding execution rather than catching up missed
  ticks. The minimum interval is 4ms.
- DONE queueMicrotask(function) uses its own FIFO queue and runs after a
  classic script, DOM event dispatch, or timer callback, before the next
  timer job. Microtasks queued inside a checkpoint join the same FIFO.
- DONE bounded resource policy: at most 256 queued microtasks, 1024 queued
  in total per page, and 256 executed per checkpoint. Outstanding bounded
  work wakes the existing page worker; no extra threads or busy polling.
- DONE tests for interval self-cancellation and navigation reset, timeout/
  interval cross-cancellation, FIFO microtask order, macrotask ordering,
  DOM click checkpoints, and recursive microtask budget exhaustion.
- LIMITATION this is still not full browser event-loop scheduling: Promise
  jobs, MutationObserver, post-presentation async fetches, event-source
  arbitration, and background tab throttling are not implemented.
- NEXT integrate resource completions into the same event loop and add
  more ECMAScript runtime support; do not claim Promise compatibility yet.

## M4.11 - Post-presentation bounded text-network tasks (2026-10-08)

- DONE experimental nonstandard opFetchText(url, callback) host function. It
  returns a request id and invokes callback(text, null) on success or
  callback(null, errorString) on failure on the page-owning JS worker.
  This is explicitly NOT the standards-based fetch() Promise API.
- DONE supported text MIME types: text/plain, text/html, text/javascript,
  application/javascript, application/json, text/css (plus no Content-Type).
  Text responses are decoded within a 64 KiB bound per request.
- DONE same-origin enforcement and existing request_filter policy run on
  bounded background WinHTTP/local-file loading, including rechecking the
  final redirect URL. The page never executes JavaScript in network workers.
- DONE mpsc completion task channel, 20ms polling only while a page has
  in-flight resources; otherwise the page worker parks without busy-loop.
  At most eight outstanding requests per page, 32 requests per page VM,
  and 16 simultaneous network workers across the process.
- DONE navigation generation tags discard late replies from older documents.
  Callback textContent updates run through the retained DOM, CSS and
  display-list reflow pipeline after the initial page render.
- DONE regressions for delayed real WinHTTP text/JSON, local files,
  cross-origin rejection, and late completion after navigation.
- LIMITATION this does not implement standard fetch(), Request/Response,
  Promise, cookies/credentials, CORS, streaming or cancellation. Network
  filter policy is copied as a snapshot to background workers; filter
  statistic counters are not yet aggregated back into the main instance.
  Some script-side local file URL restrictions are inherited from the
  earlier same-origin classic-script subset.
- NEXT integrate a standards-grounded Promise/reaction system and fetch(),
  and unify task arbitration rather than 20ms in-flight polling.

## M4.12 - Self-hosted Promise reactions and initial fetch/Response (2026-10-09)

- DONE OPBrowser's original JS interpreter now bootstraps its own self-hosted
  Promise implementation. The constructor invokes executors synchronously;
  pending/fulfilled/rejected states, single settlement, adoption of returned
  promises/thenables, FIFO reaction dispatch, then/catch/finally and
  Promise.resolve/Promise.reject are exercised by VM tests.
- DONE Promise reactions use the existing page-owned bounded queueMicrotask
  pipeline. Callbacks run after scripts, click/lifecycle events, timers and
  post-paint network completions, not inline inside Promise.then.
- DONE initial fetch(url[, init]) GET facade, fulfilled with a basic Response
  object exposing ok/status/statusText, single-consumption text() Promise,
  and rejected on load/filter errors. The implementation reuses the M4.11
  filtered, same-origin, bounded text request workers and generation gating;
  all JS execution stays on the page worker.
- DONE VM regressions for queue ordering, pending settlement, returned
  Promise adoption, thrown errors, finally and fetch Response consumption;
  integration tests for local GET to real native pixels and cross-origin
  network-policy rejection. Manual demo: examples/js/promise-fetch.html.
- LIMITATION this is a deliberately partial standards-shaped API: text-only,
  GET-only and same-origin. Successful Response metadata currently reports
  synthetic 200/OK; HTTP 4xx/5xx currently reject instead of producing a
  Response with ok=false. No Headers/Request object, JSON parsing, CORS,
  streaming, abort, credentials, body upload, Promise combinators,
  async/await, full thenable/species semantics or unhandled-rejection events.
  Jobs remain subject to existing 256-per-checkpoint and 1024-per-page
  microtask quotas; request completions still use 20ms active polling.
- NEXT M4.13 move HTTP status/headers/redirect metadata into real Response,
  distinguish HTTP errors from network failures, then implement proper
  RequestInit/CORS and Promise conformance as the parser/runtime advances.

## M4.13 - Real HTTP status and response headers (2026-10-09)

- DONE WinHTTP text transport retains HTTP status/statusText, final URL,
  redirected flag, selected bounded response headers and decoded text.
  Fetch HTTP 4xx/5xx returns a fulfilled Response with ok=false; ordinary
  HTML/CSS/script/image resource loaders keep their old strict non-2xx errors.
- DONE NetworkContext::load_text_response_for_page preserves same-origin,
  request filtering and final-redirect origin checks. Page generation tags
  drop stale completions; no background worker executes JavaScript.
- DONE op_js::TextResponse metadata and native ObjectKind::Headers get/has
  methods with case-insensitive ASCII names. Set-Cookie and Set-Cookie2
  never reach JavaScript. Response exposes status, statusText, ok,
  redirected, url and one-use text() Promise.
- DONE original opFetchText(url, callback) remains source compatible and
  still reports HTTP errors via its error callback.
- DONE VM, native WinHTTP and end-to-end engine tests verify status 201,
  404, 500, redirects, header casing/cookie filtering, and an HTTP error
  body updating the retained DOM and visible page.
- LIMITATION still same-origin GET and allowlisted text MIME only,
  64 KiB per resource. Local files use synthetic 200/OK; no public
  Headers constructor, mutation/iteration, Request class, full
  RequestInit, CORS/preflight, credentials, JSON.parse, streams,
  POST, cache or abort. WinHTTP follows redirects before the final-origin
  postcheck; strict per-hop redirect enforcement remains outstanding.
- NEXT M4.14: Request/Headers primitives, controlled RequestInit,
  status/cache policy, JSON support, and redirect-hop origin safety.

## M4.14 - Request, Headers, controlled RequestInit and per-hop redirects (2026-10-09)

- DONE browser-context native Headers constructor with get/has/set/append/delete,
  bounded fields/values, validated ASCII header-name tokens, copies from an
  existing Headers object, JS object-record initializers and arrays of pairs.
- DONE self-hosted Request(input[, init]) and fetch(Request[, init]) on the
  independent OPBrowser JS engine. Request may clone another Request.
  Supported init subset: GET method, same-origin mode, credentials: omit,
  redirect: follow/error, and request headers from Headers/object/pair list.
  Unsupported RequestInit fields and unsafe request headers reject promptly,
  not silently ignored.
- DONE safe outbound headers including Accept, Accept-Language,
  If-None-Match, If-Modified-Since and X-* on WinHTTP GET. Native and network
  layers enforce field/byte budgets and reject cookie/authorization/hop-by-hop
  or control-character injection. Local file GET rejects nonempty headers.
- DONE for text/fetch GET specifically: disabled WinHTTP automatic redirects
  and implemented explicit <=5-hop redirect following, with same-origin
  checks of every Location BEFORE opening the next URL. redirect:error
  refuses the first redirect; normal HTML/CSS/image/script loaders preserve
  their prior behavior.
- DONE VM tests cover Headers validation, copying, case-insensitive get,
  append/set/delete and RequestInit rejections. Real local HTTP-server
  tests verify safe headers across redirects, cross-origin hop blocked before
  connection and redirect:error; engine integration verifies a JS Request
  header reaches the server and updates DOM/native paint after response.
- LIMITATION native Headers is an early subset, not full standards prototype/
  iterator API. Only GET, same-origin, bounded text MIME and 64KiB bodies.
  CORS/preflight, request body, cookies/credentials, POST, AbortSignal,
  streaming, response.json(), cache controls, request credentials and full
  Fetch/ECMAScript semantics remain absent. The supported credentials default
  is currently omit, unlike standard fetch's same-origin default, and that
  difference is deliberate pending proper browser cookie isolation.
- NEXT M4.15: native JSON parsing and Response.json(), Promise combinators,
  standardized error handling, then CORS/cache/cookie architecture.

## M4.15 - Strict JSON, Response.json, Promise combinators (2026-10-09)

- DONE strict native JSON grammar in op_js/src/json.rs; payloads never go
  through JavaScript eval. Values are converted into owned VM arrays/objects.
  Supports Unicode escapes/surrogate pairs, exponent numbers, nested values.
- DONE JSON.parse and JSON.stringify built-ins on the original VM with
  64 KiB size, 64-depth and 4096-element parse bounds, cycle detection,
  catchable SyntaxError/TypeError and safe own __proto__ data keys.
- DONE Response.json returns a Promise and consumes the body exactly once,
  reusing existing same-origin safe GET and microtask dispatch.
- DONE self-hosted Promise.all, race, allSettled and any; bounded array-like
  input up to 256 entries, index ordering, pending-input settlement,
  and AggregateError-like name/errors on Promise.any failure.
- DONE parser, VM and native engine regressions for JSON, Promise, invalid
  input, consumption, and a fetched JSON object updating painted DOM text.
- LIMITATION JSON.stringify uses sorted rather than insertion-order
  object keys, does not call toJSON/replacer/space, JSON.parse reviver
  is absent, and ECMAScript number formatting/unpaired-surrogate strings
  are not exact. Combinators lack the Symbol.iterator protocol; no full
  AggregateError class, species/thenable conformance or unlimited jobs.
- NEXT M4.16: runtime Test262 harness; language/property/array/string
  conformance, callback JSON semantics and broader modern DOM support.

## M4.16 - Pinned executable Test262 subset and primitive globals (2026-10-09)

- DONE independent Test262 runtime probe: fresh original VM per fixture,
  pinned 91-file manifest drawn from upstream arithmetic/equality,
  minimal assertion/Test262Error/$ERROR helpers, metadata-aware
  module/async/strict/unsupported-include/parse-negative SKIPs, per-case
  FAIL reasons and JSON report with a verified upstream git commit.
- DONE tools/compatibility.ps1 now reports a separate runtime score
  alongside the existing parser subset, never combining their totals.
- DONE native Boolean/Number/String conversion functions, isNaN/isFinite,
  Object() allocation/identity and Array() basic construction, global
  Infinity/NaN and standard Number constants. Numeric conversion
  accepts 0x/0b/0o prefixes. Bounded arrays cannot allocate huge lengths.
- DONE fixture measurement 18/91 (19.78%) -> 59/91 (64.84%) on the
  unchanged pinned manifest. This is a narrow arithmetic/equality
  classic-script subset, not broad Test262 conformance.
- LIMITATION Boolean/Number/String functions do not yet construct boxed
  objects under new; full Object.prototype, Date, eval, Symbol and true
  ToPrimitive/valueOf semantics are not implemented. 32 tests remain
  FAIL. No broad ECMAScript runtime score can be inferred.
- NEXT M4.17: object/primitive coercion and boxing, correct property
  descriptors/enumeration, plus modern expression syntax; broaden
  the pinned runtime subset by creating version 2, never altering v1.

## M4.17 - Object coercion, primitive boxing, instanceof and void (2026-10-09)

- DONE real Boolean/Number/String boxed primitive identities for new
  constructors; Object(primitive) boxes without exposing fake primitive
  properties, and Object(existingObject) preserves its reference.
- DONE Object.prototype valueOf/toString defaults and boxed prototype
  methods, with correct receiver checks for boxed primitive methods.
- DONE ordered object-to-primitive conversion in binary operators,
  invokes own or inherited callable valueOf then toString, preserves
  user-thrown errors and throws TypeError when neither returns a
  primitive; no conversion on strict equality of two objects.
- DONE instanceof checks constructor.prototype against prototype chain
  without converting left operands; catches invalid right operands.
  Implemented void with operand side effects and sloppy implicit
  global bindings on assignment to previously unresolvable identifiers.
- DONE fixed-pinned Test262 runtime v1 rerun on verified upstream:
  59/91 (64.84%) -> 82/91 (90.11%), same 91 attempted and no skips.
  Unit tests cover boxing, identity, inherited methods, override order,
  thrown errors, instanceof, void and assignment side effects.
- LIMITATION intentionally narrow classic-script suite focused on
  arithmetic/equality. Remaining nine failures need eval, Date and
  Symbol. Runtime semantics still lack complete strict-mode, String
  ToPrimitive hint, object property descriptors, Object.assign,
  Symbol.hasInstance and modern iterators. Do not extrapolate 90.11%
  to overall ECMAScript or website compatibility.
- NEXT M4.18 broaden representative pinned Test262 runtime suite v2
  across modules/features rather than optimizing the now-narrow v1.

## M4.18 - Broad pinned Test262 Runtime v2, typeof and conditional expressions (2026-10-09)

- DONE pinned Test262 Runtime v2: blind deterministic path-only selection,
  289 cases from 25 expression/statement/builtin feature directories,
  up to 12 per family, pinned at upstream revision c8c798898646638cd0c24879f8e0374e847e7d74.
  Manifest is committed independently from the 91-case v1 suite.
- DONE real native typeof operator: JavaScript type categories, functions,
  null as object, and unbound identifier returning undefined without a
  ReferenceError; property access on null still errors.
- DONE lazily compiled ternary conditional expression, nested right-
  associative branches and assignment-expression arms.
- DONE standard static Array.isArray, Array.of (bounded own Array),
  Number.isNaN and Number.isFinite (no argument coercion), and
  Object.is with NaN, signed zero and identity semantics.
- DONE Test262 runner v2 suite labeling, fail-closed multiline includes/
  flags parsing, separate JSON result/score and integration into
  compatibility.ps1. New engine test checks native DOM repaint.
- MEASURED same 289 fixture selection: 39/179 attempted passing at
  baseline -> 65/179 (36.31%); 110 SKIP and 114 FAIL remain; v1 stays
  82/91. No changes to old manifest or selective test exclusions.
- LIMITATION only classic-script test harness subset; strict variants,
  many test includes, modules/async, Symbol, Date, Function, modern
  property descriptors and iterators missing. Array.of does not yet
  use custom constructors; only standard Array creation is supported.
  No score here is overall JS or website compatibility.
- NEXT M4.19: strengthen standard built-ins and DOM tree manipulation,
  implement more real Test262 helpers and metadata when semantics are
  supported, and keep v2 fixed while improving measured behavior.

## M4.19 - String, Array prototype and SyntaxError (2026-10-09)

- DONE original VM String.prototype.charAt on primitive/boxed strings
  with integer-truncated index and valid UTF-16 scalar coverage.
- DONE original-VM Array.prototype.push/pop for genuine bounded arrays,
  correct length/index/holes and method arities.
- DONE real SyntaxError constructor and prototype for malformed JSON.parse,
  including instanceof behavior; native integration test updates DOM text.
- MEASURED pinned runtime v2 from 65/179 to 78/179 (43.58%) with 110
  explicit SKIP unchanged; narrow v1 stays at 82/91.
- LIMITATION incomplete generic array-like receivers and UTF-16 lone
  surrogates, no live DOM createElement/appendChild mutation pipeline.
- NEXT M4.20: actual DOM tree mutations with persistent node identity,
  authoritative DOM model, layout recalculation and native repaint.

## M4.20 - Actual bounded DOM insertion through native repaint (2026-10-09)

- DONE document.createElement(tag), Element.appendChild(child), live
  element.id and textContent setters. Detached nodes are allocated
  as bounded VM handles and emitted as ordered native DomOperation
  commands (Create, Append, SetId, SetText) to the authoritative
  op_dom Document, not merely placeholder JS objects.
- DONE persistent element object identity across repeated
  document.getElementById calls, parser script boundaries, and
  dynamic attachment of detached nested child nodes; document.body
  is exposed when body exists in the tree-builder, otherwise null.
- DONE DOM host operation replay for parser-blocking, async/defer,
  retained click and timer callbacks. Style/layout/native paint
  refresh when the real tree changes; dynamic element click
  listeners remap synthetic handles to actual NodeId.
- DONE ancestor-cycle prevention at DOM layer, with staging
  safeguards for newly created nested nodes, finite mutation and
  depth/node allocation budgets and conservative tag validation.
- DONE native integration tests: create/append/lookup, detached
  nested subtree, timer-delivered textContent mutation, dynamic
  registered click listener and real repainted pixels, plus DOM
  ancestor-cycle protection tests.
- VERIFIED pinned Test262 runtime v1 82/91 and runtime v2
  78/179 attempted (with 110 explicit SKIP): no unrelated JS
  conformance regression. Neither metric claims overall support.
- LIMITATION no general removeChild/insertBefore, createTextNode,
  class/style/setAttribute/getAttribute, general Node interface,
  DOM mutation observer or style recalculation during a running
  script. Host rejects illegal pre-existing-node cycle moves, but
  asynchronous host replay cannot synchronously throw for every
  invalid move in this first slice. Full DOM event and reflow
  behavior remains partial.
- NEXT M4.21: general DOM attributes, createTextNode/removeChild,
  sibling insertion and live node relationships; strengthen
  operation validation and style/incremental-layout correctness.

## M4.21 - Text nodes, DOM removal/reinsert and live attributes (2026-10-09)

- DONE real createTextNode with mutable textContent/nodeValue/data,
  append/removal/reinsertion, native DOM representation and paint.
- DONE removeChild/insertBefore with stable NodeIds, reference parent
  verification and ancestor cycle protection. VM staging updates nested
  ID visibility through detach/reattach in the same script.
- DONE setAttribute/getAttribute/removeAttribute including id, class,
  style and custom attributes. Parser-owned node metadata synchronizes
  into VM; changes are replayed into the authoritative DOM.
- DONE recascade after timer/click mutations: re-collect author CSS
  and compute new styles while preserving loaded linked CSS and
  downloaded color profile data. Nine new end-to-end native DOM
  tests and independent op_dom validation checks pass.
- VERIFIED Test262 Runtime v1 82/91 and v2 78/179 (110 SKIP),
  unchanged. This is not a whole DOM or JS standards score.
- LIMITATIONS: no general live parentNode/childNodes wrappers,
  DOMTokenList/classList, CSSOM style object, replaceChild, HTML
  insertion or MutationObserver; host replay may reject some complex
  invalid changes asynchronously. Dynamic CSS/image fetches missing.
- NEXT M4.22: live node relationships, richer attributes and style
  object, correctness under reparenting and interactive smoke tests.

## M4.22 - Live Node relationships, classList and bounded style object (2026-10-09)

- DONE live parentNode, firstChild, lastChild and stable childNodes
  collection for DOM Elements and Text nodes, with fresh length,
  numeric index and item(index) lookups after insert/remove/reparent.
  Already-held collection references remain live across timers.
- DONE original native op_dom::replace_child validates direct
  membership and reuses stable node identities, plus JS
  Element.replaceChild and Element.remove() through ordered
  authoritative DomOperation replay and native paint.
- DONE bound DOMTokenList for Element.classList: stable identity,
  length/value/numeric access, contains/add/remove/toggle with an
  optional force argument; reflects the original class attribute
  used in selector matching and supports direct className reads/
  assignments.
- DONE initial live Element.style object with display and simple
  camelCase property reflection, cssText, setProperty, getPropertyValue
  and removeProperty. Bounded property/value validation and
  original DOM style attributes trigger native author CSS recascade.
- DONE physical NodeId binding now retargets previously held element,
  text, NodeList, classList and style objects; cached linked CSS
  and color profiles remain intact across dynamic repaints.
- TESTED nine new M4.22 engine-to-native-paint scenarios: live lists
  through synchronous changes/timers, Text nodes, replace/remove,
  CSS classes/visibility, style objects, and synthetic->physical
  identity transition. Full Rust workspace, Clippy, smoke and
  independent pinned Test262 baseline verified.
- METRICS unchanged: Test262 Runtime v1 82/91 (90.11%) and v2
  78/179 attempted (43.58%, 110 explicitly skipped). These are
  scoped ECMAScript runtime tests, NOT DOM WPT compliance.
- LIMITATIONS: this is a bounded DOM/CSSOM initial implementation,
  not full web standards. No Document childNodes/complete Node
  interface, live element.children HTMLCollection, full CSS grammar
  or CSSOM cascade, style priorities, computedStyle, DOM observers,
  comprehensive HTML events or modern framework readiness.
  classList variadic token inputs and DOMStringMap not supported.
- NEXT M4.23: broaden live Node/Element properties, robust
  CSSStyleDeclaration parsing and classList behavior, WPT-based
  dynamic DOM subset and interactive page compatibility smoke tests.

## M4.23 - Node sibling/connections and first pinned WPT DOM smoke (2026-10-09)

- DONE live Node.previousSibling/nextSibling and isConnected
  computed from authoritative staged parent/child linkage, with
  bounded ancestor traversals. Added Element.contains (identity,
  descendants and null handling) and nodeName/tagName,
  ownerDocument, element nodeValue=null and Text.length
  measured in UTF-16 code units.
- DONE DOMTokenList.add/remove with zero or multiple tokens,
  validation of all arguments before mutation to prevent partial
  writes, and de-duplication during insertion. Existing contains
  and toggle remain live against original class attributes.
- DONE bounded CSS declaration scanner for element.style methods:
  quoted strings, escapes and parentheses protect embedded
  semicolons; declarations still use the project's partial CSS
  engine and are not a complete CSSOM token parser.
- DONE first *actual upstream WPT DOM source* execution path:
  op_engine/src/bin/wpt_dom_probe.rs verifies pinned upstream WPT
  SHA 97fe10c5..., reads original Node-childNodes-cache.html with
  git show, swaps external WPT testharness references for a minimal
  synchronous assert_equals/test adapter (assertions unchanged)
  and checks native text repaint as the verdict.
- FROZEN compat/wpt-dom-smoke-v1.tsv: four manually selected WPT
  fixture paths, 1 attempted/passed; 3 explicitly not attempted
  with reasons (Array.from/arrow, eval/common.js and iframes).
  This is neither a broad/random DOM WPT benchmark nor a full
  WPT harness score; the adapter supports only one fixed fixture.
- DONE three original cross-crate native DOM rendering regressions
  for siblings/connectivity, variadic classList rollback and CSS
  function/quote parsing. Fixed old Test262 runtime v1/v2 scores
  separately from the DOM smoke metric.
- NEXT M4.24: support more real WPT testharness assertions/fixtures,
  broaden DOM collection/Document interfaces and event pipeline,
  then independently track attempted/passed/skipped WPT DOM tests.
  Avoid presenting 1/1 selected-pass as overall browser readiness.

## M4.24 - Pinned WPT DOM smoke v2 and live Element.children (2026-10-09)

- DONE original WPT DOM fixture manifest v2 with ten manually selected
  tests: seven attempted and passing, three explicit unsupported SKIPs.
  Existing v1 smoke manifest frozen. Not a representative WPT score.
- DONE multiple original upstream HTML fixture execution with only
  external testharness imports replaced by the tiny synchronous
  test/assert_equals/assert_true/assert_false adapter. Original
  assertions untouched. Fail closed on commit mismatch, missing
  paths, duplicates, skipped/failed harness scripts and changed
  fixture shape. Visible native PASS marker required.
- DONE live childElementCount, firstElementChild, lastElementChild,
  previousElementSibling, nextElementSibling; text nodes skipped.
- DONE retained Element.children HTMLCollection with filtered length,
  numeric indexes, item, basic namedItem and name property lookup;
  persists through synthetic-to-real binding, callbacks and timers.
- TESTED two native end-to-end scenarios for live collections and
  timers; seven pinned upstream DOM fixtures also passed.
- LIMITATIONS original WPT subset is manually selected, narrow,
  synchronous, and not an official WPT testharness. No overall DOM
  pass rate. Complete HTMLCollection named semantics, iframe,
  iterable Array.from, event Web APIs and CSSOM remain incomplete.
- NEXT M4.25: expand genuine WPT harness and DOM fixture coverage,
  improve DOM Document/Element interfaces and event semantics.

## M4.25 - Multi-test original WPT DOM and live tag collections (2026-10-09)

- DONE WPT DOM smoke v3, same pinned WPT revision:
  13 manually selected original HTML files, 10 attempted/pass, 0 fail,
  3 explicit SKIPs. Ten executable files include 11 original test()
  assertions/callbacks, since Element-hasAttributes.html has two tests.
  The frozen prior v1/v2 manifests remain unchanged.
- DONE limited synchronous testharness shim supports multiple original
  test(callback) calls, assert_equals/true/false, a persistent failure
  flag and strict expected test count per fixture. Later pass cannot
  erase an earlier failure. Two separate negative/positive harness
  unit tests prove error stickiness and fail-closed fixture counting.
  Original WPT assertions themselves remain unchanged.
- DONE original-JS Element.hasAttribute and hasAttributes methods,
  validating receivers and real (parsed or newly added) attributes.
- DONE original-JS Document.getElementsByTagName and
  Element.getElementsByTagName with a live, bounded descendant-only
  element collection (length, indexed access and item) refreshed
  after append/remove/attribute operations and delayed timer mutation.
- DONE two new script-to-authoritative-DOM-to-native-pixel regressions
  testing parsed/new attributes, live global/scoped tag collections,
  removal, late timed creation and persistent collection identity.
- TESTED pinned WPT smoke v3 10/10 attempted, 3 SKIP; Test262
  v1/v2 are independently pinned ECMAScript samples and must not
  be merged. These ten files are manually selected and do NOT
  imply 100% DOM compatibility. No official async WPT harness.
- LIMITATIONS: tag collection covers common HTML names and '*' only,
  not complete namespace/case and named property semantics;
  no live CSS selector querySelector, shadow DOM, iframe,
  official WPT harness, async tests or general Web API completeness.
- NEXT M4.26: additional pinned WPT DOM fixtures, document/element
  lookup semantics, asynchronous test harness and browser events.

## M4.26 - Relational in, simple DOM selectors, programmatic clicks, WPT smoke v4 (2026-10-09)

- DONE original JS lexer/relational parser/VM operator in, bounded
  prototype-chain HasProperty and live DOM accessors. Right-hand
  primitives throw TypeError. Object-key ToPropertyKey coercion
  and full ECMAScript property semantics remain incomplete.
- DONE native original-JS Document/Element.querySelector and
  querySelectorAll, with bounded simple selectors: #id, .class,
  HTML tag names and wildcard *. Queries respect descendant-only
  element scope and authoritative tree order; unsupported compound/
  combinator/pseudo selectors currently raise an explicit error.
  querySelectorAll returns a static NodeList (indexed access,
  item and length), distinct from live tag/children collections.
  Synthetic-to-physical DOM binding preserves saved NodeList refs.
- DONE bounded programmatic element.click() with existing
  capture/target/bubbling click handlers and event object, including
  timer-invoked dispatch; nesting capped at 8. No full default
  activation behavior, keyboard/focus, navigation or generic
  dispatchEvent in this slice.
- DONE manually pinned WPT DOM smoke v4: 14 selected original-source
  fixtures at the same upstream revision, 11 attempted and passed,
  3 explicitly unsupported/skipped, 12 test() callbacks total.
  Added unmodified original Element-childElementCount.html testing
  the JS in operator. Existing WPT v1/v2/v3 manifests stay frozen.
  Shim remains synchronous and is NOT official WPT harness.
- DONE five new script-to-DOM-to-native-paint integration tests
  for in, selector scope, static query snapshot, deferred callback
  node identity and programmatic bubbling/timed clicks.
- CHECKED full Rust workspace, strict Clippy, Win32 smoke, fixed
  WPT v4 and independent pinned Test262 v1/v2 samples.
- LIMITS: partial selector grammar and typed errors, no complete
  CSS Selectors API, custom selector pseudo-classes, ShadowRoot,
  default click actions or async WPT harness. A small manually
  selected WPT fixture set is not an overall DOM conformance rate.
- NEXT M4.27: selector combinators/attribute syntax and standards
  errors, safe general event dispatch, broader real WPT DOM
  coverage and a proper async testharness protocol.

## M4.28b - Element listener once/passive options and isolated errors (2026-10-09)

- DONE original Element.addEventListener(type, fn, options) and
  removeEventListener capture matching accept the boolean or a dictionary
  with capture; registration also supports once and passive.
- DONE a once listener is removed before invocation, including nested
  dispatchEvent and native click. Duplicate registration keeps its first
  option values. Removal during dispatch prevents a queued callback.
- DONE passive prevents cancelation via preventDefault() and legacy
  returnValue=false while allowing a later nonpassive listener to cancel.
- DONE isolated non-budget callback exceptions are recorded in a
  bounded (32-entry) runtime diagnostic and visible via
  Engine.active_event_listener_errors(); following listeners still run.
  VM execution-limit errors remain fatal instead of being hidden.
- TESTED five VM cases plus two native DOM/paint scenarios, including
  dynamic element binding and delayed timer callbacks.
- UNCHANGED pinned WPT DOM/Events v5: manually chosen 12 attempted/pass
  and three explicit skips. This is not an overall WPT/DOM score.
- LIMITS: scoped to Element listeners (not document/window lifecycle
  listener options), no AbortSignal/signal, listener objects,
  full DOMException taxonomy, default browser activation, or
  official async WPT harness. Event diagnostic storage is bounded.
- NEXT expand original WPT event fixtures and official harness behavior,
  add broader event targets, option semantics and DOM APIs.

## M4.28a - Immediate event propagation stop (2026-10-09)

- DONE Event.stopImmediatePropagation on native click events, custom Event() and legacy createEvent('Event').
- DONE stops remaining same-target callbacks and later event-path traversal; stopPropagation continues to allow same-target callbacks.
- DONE custom Event reuse resets the immediate-stop dispatch flag.
- TESTED a native-click VM test and an original JS-to-native-DOM/paint integration test.
- UNCHANGED frozen WPT DOM/Events v5: 12 attempted/pass, 3 SKIP in a manually selected set, not full conformance.
- NEXT complete M4.28 with once/passive listener options, listener-error reporting and wider original WPT event coverage.

## M4.27 - Compound DOM selectors and general Event dispatch with real WPT Events (2026-10-09)

- DONE expanded bounded original JS Document/Element querySelector and
  querySelectorAll: compound selectors like tag.class#id,
  descendant whitespace and direct-child > relationships,
  comma-separated groups, document order, duplicate elimination
  and element-scoped ancestor matching. Existing querySelectorAll
  static snapshot semantics retained. Selector tokens capped at
  256 ASCII bytes / 8 groups / 16 segments and traversal budgets.
  Attribute selectors, pseudo-classes, sibling combinators and
  full CSS syntax still unsupported, with explicit errors.
- DONE original-VM Element.matches and closest on the same
  bounded selector grammar, including matching self/ancestor
  chains and basic null/not-found handling.
- DONE custom Event(type, {bubbles,cancelable}) constructor and
  Element.dispatchEvent for target-specific typed listeners,
  capture/target/bubble propagation, listener removal, correct
  defaultPrevented/return boolean, stopPropagation and currentTarget
  reset, reusable events, trusted=false and guarded reentrancy.
  Typed listener tables persist across synthetic->native NodeId
  mapping, and DOM mutations commit to native paint after events.
- DONE initial legacy Document.createEvent('Event') and
  Event.initEvent / returnValue accessors for older pages.
  Duplicate dispatch while an event is active is rejected;
  nested dispatch depth limited to 16 and path to 64.
- TESTED seven native original JS -> actual op_dom -> native
  display-list integration scenarios: compound selector groups,
  matches/closest, custom event phase/cancelation/stopPropagation,
  event from timers/reuse, legacy returnValue and reentrancy.
  Updated a former M4.26 test which previously required rejection
  of descendant selectors to assert their new successful behavior.
- DONE frozen compat/wpt-dom-smoke-v5.tsv with one additional
  original upstream DOM Events fixture:
  dom/events/EventTarget-dispatchEvent-returnvalue.html.
  Its two original test() callbacks pass unchanged, verifying
  preventDefault() and returnValue. WPT DOM/Events v5:
  15 manually selected files, 12 attempted/pass, 3 explicit SKIP,
  14 executed original synchronous test() callbacks. Pinned
  upstream revision and limited fail-sticky harness retained.
- LIMITATIONS: this is a bounded partial custom event system,
  not standards-complete EventTarget: no stopImmediatePropagation,
  AddEventListener options/once/passive, proper DOMException
  hierarchy, default browser event activation, generic document/window
  dispatchEvent, async tests or exception-isolating listener reports.
  Uncaught event listener errors may stop later listeners.
  Selector grammar is intentionally restricted; Test262 JS
  runtime v1/v2 is separate from WPT DOM/Events metrics.
- NEXT M4.28: WPT-backed selector/error behavior, comprehensive
  listener options and exception handling, broader official
  testharness capabilities and responsive interactive sites.
