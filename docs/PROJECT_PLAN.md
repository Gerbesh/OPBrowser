# OPBrowser: active project plan

## P0 atomic textarea explicit-line viewport and CSS box geometry (24 October 2026)

Replaced the provisional textarea glyph stream and synthetic per-line nonbreaking-space padding with original op_layout::flow::Context::textarea_atomic. It creates one bounded first-party inline atomic box with HTML cols (default 20, maximum 160) and rows (default 2, maximum 100), CSS width/height/min/max and content-/border-box semantics. Each explicitly separated line paints only the Unicode-safe prefix that fits the content width and only rows that fit the fixed height; blank textareas still have one full hit area, and following inline text stays in the surrounding flow. Native TextEditor::scroll_start represents first visible explicit newline row for textarea (it remains a Unicode scalar start for input). Engine::render adjusts the row viewport after computing actual CSS hit dimensions, while mouse caret positioning, selected fragment Y placement and focus caret subtract/add the same row offset. Full DOM value, JS selection, clipboard and undo/redo remain untruncated. Original layout and JS click/insertion regression tests verify multi-line scrolling and Unicode. **Soft wrapping is not yet implemented**: it needs a shared visual-line map for native layout, caret, selection and drag. Wheel/touch scrollbars, scrollbar DOM APIs, bidi/grapheme shaping, IME and exact CSS padding alignment remain open.

## P0 horizontal caret-follow scrolling for native single-line input (24 October 2026)

The first-party TextEditor now stores a Unicode scalar-indexed visible start for its focused input. A bounded full-value/visible-suffix measuring helper moves the start only when the current caret leaves the available content width, or resets it when the full string fits or the caret moves left. op_layout exposes FormLayoutState and layout_document_with_form_scrolling while keeping the old axis-scrolling entrypoint source compatible: native input scroll starts are passed separately from CSS scrollLeft and the input atom measures/paints the selected Unicode suffix. PreparedDocument::render first resolves the field's CSS-sized hit geometry, then reflows once only if the editor's visible start changes. Caret/selection overlay and mouse click-to-caret use the same hidden-prefix text extent, preserving the untruncated DOM value, UTF-16 selection, clipboard and Undo/Redo. Test cases cover CJK/emoji, password bullets, Home/End scrolling, scrolled mouse placement, selection and full native editing. Not yet browser-grade bidirectional/grapheme shaping, CSS padding offsets, partial-glyph clipping, per-field scroll persistence across blur, IME or textarea scrolling.

## P0 atomic single-line HTML input geometry (24 October 2026)

The original op_layout inline compositor now carries an optional generation-safe input hit identity through its InlineAtomic path. Supported single-line text/search/email/url/tel/password/number inputs receive one bounded box with default/size-derived intrinsic width, author CSS width/height plus min/max sizing and content-box/border-box handling. Visible glyphs are measured and restricted to a Unicode-safe prefix fitting the provisional content width, while the full actual DOM value, native Unicode selection and clipboard remain untouched. Existing inline CSS fragment background/border/padding/margins and relative positioning stay attached to the box; CSS fragment height now follows the atomic input's height rather than the surrounding font metrics. Empty inputs have real clickable geometry, including the unoccupied part of the field. Regression tests check sizing, masked Unicode, long content, inline siblings, CSS border sizing and a real DOM click+input callback. Scope: **single-line inputs only**. Still missing native horizontal caret-follow scrolling, accurate shaped glyph clipping, full layout ink/selection coherence with author padding, and atomic textarea wrapping/scrolling; script-visible text and editing remain fully retained instead of being truncated.

## P0 bounded native form edit history and undo/redo (24 October 2026)

First-party input/textarea editing now retains up to 32 reversible Unicode-safe value/caret/anchor snapshots for the currently active native TextEditor. Ctrl+Z undoes, Ctrl+Y or Ctrl+Shift+Z redoes, and any new edit clears redo. A script-assigned DOM value invalidates stale history; a new document clears any pending canceled/control-key WM_CHAR suppression. Undo/redo reuses the existing cancelable DOM keydown and beforeinput path and triggers normal input events plus UTF-16 selection synchronization on success. Native clipboard cut/paste, character input, Backspace/Delete all use the same edit pipeline. Verified editor and real HTML/JS regressions, including emoji selections, canceled keydown and script mutations. Limitations: history is per current focused editor (switching focus resets it), keystrokes are not grouped, no complete InputEvent.inputType/historyUndo/historyRedo and no cross-field/IME undo transactions.

## P0 first-party form size/cols/rows hints in inline layout (23 October 2026)

The original op_layout form path now honors positive, bounded HTML input size (default 20), textarea cols (default 20) and textarea rows (default 2). Width padding is generated independently on each explicit text row, rather than appended once after the final row; a textarea only receives extra line boxes until the requested minimum row count is met. Existing original Unicode DOM values, native click regions, text selection and clipboard paths remain intact. Regression tests compare per-control hit widths for different size/cols, row-dependent layout height, swapped unequal text rows and bounded hostile numeric hints. Remaining P0: true fixed-size replaced-element boxes, CSS width/height priority, viewport-clipped overflowing text, soft wrapping, horizontal/vertical editing scroll and accurate bidirectional shaping. This is an incremental sizing improvement, not a claim of full HTML form geometry.

## P0 Windows Unicode clipboard for native HTML form fields (23 October 2026)

The original OPBrowser form editor now supports Ctrl+C, Ctrl+X and Ctrl+V against Windows CF_UNICODETEXT without embedding a platform EDIT control or external web engine. Copy reads UTF-16 web selections using safe Unicode scalar indices; cut removes only selected text after a successful OS clipboard write; paste normalizes Windows CRLF into LF for textarea and removes newlines for single-line input. The existing original beforeinput/input and JS selection synchronization paths run for native edits. A page's canceled keydown prevents these defaults. Passwords are excluded from copying, readonly fields are copy-only, and disabled fields are excluded. Win32 clipboard allocation is bounded and encoded/decoded with tested UTF-16/CRLF helpers. Remaining work: native copy/cut/paste DOM ClipboardEvents, edit-history/undo, precise replaced-element geometry, soft wrapping, bidirectional/grapheme handling and IME. OS clipboard end-to-end was not automated to avoid modifying the user's system clipboard.

## P0 captured mouse drag selection in first-party form controls (22 October 2026)

Win32 mouse capture now forwards pointer moves even outside the page and coalesces busy-frame movement without losing press/release order. An original input/textarea pointer-down captures a Unicode-scalar anchor; dragging at least 3 pixels extends the selection using the existing glyph measurer, including explicit textarea newlines and password masks. The original JS runtime receives UTF-16 start/end/direction updates while the selected glyphs repaint through the existing clipped blue/white display commands. A completed text drag suppresses click/link activation; a stationary press keeps existing click, caret and hyperlink behavior. Real engine/native regressions cover forward/backward emoji selection, multiline editing, click suppression, capture beyond viewport and queued move ordering. Remaining gaps: pixel-accurate replaced-element sizing, soft wrapping and horizontal scroll, grapheme/bidi shaping, clipboard and IME.

## P0 visible solitary input/textarea forms without unrelated DOM content (21 October 2026)

Fixed a CSS margin-collapsing fast path in the original op_layout::flow engine: block ancestors containing a paintable input or textarea were mistakenly treated as empty when they had no regular text siblings, preventing both glyph painting and native click hit regions. First-party text/search/email/url/tel/password/number inputs and textareas now count as real layout content for this check, while input[type=hidden], boolean hidden and unsupported non-rendered input types are still excluded. The fix restores standalone text controls, nested inline/block controls and script-only neighbor cases without disabling ordinary empty-block margin collapse. Real DOM tests verify solitary mouse editing, JS selection highlight and textarea multiple-line drawing; full precise form box geometry remains future work.

## P0 first-party HTML text selection highlight (20 October 2026)

Native Unicode editor and JavaScript-selected text ranges in focused input/textarea fields are now visually highlighted using the original op_paint command stream. A measured row-by-row scalar range yields clipped blue selection backgrounds followed by white selected glyphs, including emoji, masked password bullets and explicit textarea newline rows. A noncollapsed selection hides the insertion caret; after native replacement it collapses and the caret returns. Tests cover selection geometry, JS-selected emoji repaint and multiline textarea rows. This is provisional overlay painting over an inline control model, not pixel-perfect shaping/wrapping or a ready-made Windows text widget. Mouse drag selection, IME, clipboard, bidirectional text, soft wrapping remain separate work; standalone minimal markup hit-region coverage was fixed on 21 October 2026.

## P0 native click-to-caret for original HTML text controls (20 October 2026)

OPBrowser now maps a mouse click on supported text-like input and textarea controls to a Unicode-safe native insertion caret using its own text measurer. Mouse coordinates and CSS font size feed a bounded nearest-glyph-boundary search; password controls measure mask bullets, and textarea maps explicit newline rows using the current provisional line-height. The resulting editor selection is synchronized to JS selectionStart/End in UTF-16 *before* focus/click listeners. Clicking an already-focused input repaints when the cursor moves. Real DOM regression tests verify click-to-start, emoji/Unicode insertion and textarea second-line clicks. Limits: provisional inline field layout, no wrapping or horizontal-scroll offset, no exact shaped/bidi caret positioning, drag highlight, clipboard or IME.

## P0 live DOM text selection and UTF-16 bridging (20 October 2026)

Original input/textarea JS proxies now expose selectionStart, selectionEnd, selectionDirection and setSelectionRange() for applicable text-like types, with DOM-facing UTF-16 code-unit offsets. Engine::edit_focused and PreparedDocument::render synchronize those selections with the original scalar-indexed TextEditor. Native replacement edits update the live JS selection before input listeners run, including surrogate-pair emoji handling. Unsupported non-text input types report null selection getters. A real HTML + VM + native edit regression replaces a selected emoji and observes the changed JS caret. This remains bounded: no actual selection highlight, complete click/drag selection, setRangeText, IME, clipboard, grapheme-aware movement or full text selection specification coverage.

## P0 native HTML text editing foundation (20 October 2026)

Original first-party OPBrowser now supports a bounded initial `input`/`textarea` editing slice, without Windows EDIT/WebView/Chromium controls. The original layout paints DOM-backed text, placeholder, basic password masking and stable-width hit regions; original paint adds an outline and visible caret. Windows `WM_CHAR` commits Unicode, including UTF-16 surrogate pairs; browser worker queues `TextInput` with key events. `op_engine::forms::TextEditor` tracks Unicode scalar offsets for caret/anchor, selection, Backspace/Delete, arrows, Home/End and Ctrl+A. Enter inserts a newline in textarea; readonly/disabled inputs reject writes. Author JS `.value` reads/writes reflect a bounded DOM `value` attribute, native edits synchronize VM and tree; native cancelable `beforeinput` and bubbling `input` run through original EventTarget, keydown preventDefault suppresses following WM_CHAR. Added `examples/forms/index.html` for manual evaluation. Native Win32 Cyrillic + emoji surrogate tests, real HTML/JS input/textarea/selection/cancelation/mouse tests, cargo fmt, strict Clippy, full workspace and native link/image/startup smoke pass. Previous 128-deep inline stress test now runs with a dedicated 16MiB test stack on Windows; production layout unchanged. This is **not full HTML form conformance**: inputs currently use simplified inline layout, no accurate visual selection highlight, click-to-caret, full script selection conformance, clipboard, IME composition, form submission/validation or browser-grade multiline wrapping/scrolling. Preserve those gaps as P0.

## P0 script-owned Element.focus()/blur() (20 October 2026)

Original `op_js` DOM elements now expose `focus()` and `blur()`, implemented directly in the first-party JS runtime with native target eligibility (connected nodes; explicit tabindex; standard focusable HTML controls/links; disabled/hidden input exclusion). Programmatic focus changes preserve `document.activeElement`, dispatch non-cancelable `blur`/`focusout` before `focus`/`focusin`, avoid duplicate transitions, and bound nested focus callbacks to depth eight. Host paint consumes an explicit pending programmatic focus change to update original CSS `:focus` state, even when no text mutation was made; a focused element may also change inside a native click handler. New real-HTML tests prove initial script focus, disabled/no-op and repeat-focus rules, focus event ordering and CSS color, blur-to-body behavior and focus from a live click callback. Full Rust workspace, strict Clippy, native Win32 startup/link/image smoke and 5-site Release sample pass. Still incomplete: accurate CSS display:none/inert/fieldset eligibility, relatedTarget, focused-node detach, focus() scrollIntoView, keyboard modality heuristics, form caret and IME. No new full Test262 run or basis to raise the previous ~30–35% engineering estimate of the JS engine.

## P0 DOM activeElement, mouse focus events, cancelable keyboard input (20 October 2026)

Native focus now synchronizes read-only `document.activeElement` with the original DOM/JS runtime: `body` is the initial fallback, and Tab/mouse focus transitions update the live element before `focus`, `blur`, `focusin` and `focusout` callbacks. The existing pointer-click engine now dispatches focus changes **before** the trusted click and preserves handler DOM mutations. Original `JsRuntime::dispatch_dom_native_event_path` accepts trusted, cancelable and bubbling `keydown/keyup` with modern `key`, `code`, `shiftKey`, `ctrlKey`, `repeat` (alt/meta currently false), using the existing registered listener capture/target/bubble, once/passive and `preventDefault` semantics. Win32 sends ordered page keydown/up events through the browser worker, keeping address Enter, Ctrl+L and F5 independent. `Engine::keyboard_event` delivers JS listeners first and only advances Tab focus or activates Enter if keydown was not canceled; other keys still deliver DOM events. Native tests prove canceled Tab/Enter, valid uncanceled Enter navigation, keyup metadata, keydown through body/document without focused element, mouse focus/activeElement callback ordering, worker queue ordering, and Win32 toolbar/shortcut isolation. All Rust tests, strict Clippy, native startup/link/image smoke and five public Release load probes passed. Remaining P0: correct programmatic focus()/blur() and auto-focus, relatedTarget, activeElement when a focused node is removed, keyboard input and editing/IME in real HTML form controls, keypress/composition, alt/meta key state and full default key actions.

## P0 Tab scroll-into-view and DOM focus event delivery (20 October 2026)

The native keyboard focus system now reveals offscreen nodes: original `op_layout::LayoutTree.focus_regions` records un-clipped DOM rectangles before overflow trimming, leaving `click_regions` correctly clipped for hit testing. `PreparedDocument::reveal_nested_focus` walks up to 128 ancestor DOM nodes, recomputing layout as needed while scrolling nested CSS overflow containers from inner to outer, including horizontal offsets and JS-owned `scrollTop`. It sends final document coordinates once through `Engine::take_focus_scroll_bounds` to the Win32 worker; `NativeBrowserWindow::reveal_document_bounds` adjusts the root document's existing scrollbar and skips unnecessary movement. The original JavaScript event dispatcher now supports trusted non-cancelable `focus`/`blur` and bubbling `focusin`/`focusout` using existing capture/target/bubble, once/passive listener machinery. Tab changes dispatch previous blur/focusout then new focus/focusin; handler DOM mutations survive to the next painted frame. Original click behavior and CSS :focus-visible are retained. Native Windows viewport, nested scrolling, root bounds, listener propagation/DOM mutation, workspace, strict Clippy and browser smoke tests pass. P0 limitations: mouse-induced focus change does not yet emit all four events, no relatedTarget or document.activeElement synchronization, no keydown/keyup/keypress listener delivery, full form controls and accessibility remain incomplete.

## P0 native keyboard Tab focus, focus-visible and Enter link activation (20 October 2026)

The original Win32 shell now forwards Tab, Shift+Tab and Enter **only when the browser's page HWND receives WM_KEYDOWN**; the toolbar address control retains its own Tab/Enter handling, and clicking in the page gives focus back to the parent HWND. `NavigationEvent::{FocusNext,ActivateFocused}` are kept in order by the busy worker and passed to `Engine::{focus_next,activate_focused}`. The retained DOM builds a bounded sequential focus order: positive `tabindex` ascending, then zero and natural anchors/form controls in document order, skipping disabled/hidden/`display:none`/`visibility:hidden` and negative tabindex. The focused generation-safe NodeId updates author CSS `:focus/:focus-visible/:focus-within`, and a 2px native blue fallback focus rectangle is painted when the focused node has layout hit geometry. Enter dispatches the original cancelable trusted JS click and honors `preventDefault` before activating current `a[href]`/area URLs (read from DOM after the event). Browser worker detects Enter navigation through history changes and presents page and URL as navigation rather than as reflow. Tests cover forward/reverse order, skip rules, CSS focus-visible and ring, mouse modality reset, DOM cancellation, genuine local-file navigation, native Win32 routing and queue ordering. Full Rust workspace, strict Clippy, native startup/link smoke pass. P0 open: automatic scroll-into-view, focus/blur/keyboard DOM event dispatch, complete form/control focus ink (current fallback ring requires hit geometry), focus scopes, toolbar Tab integration and complete browser accessibility semantics.

## P0 cancelable native link default action and trusted click propagation (20 October 2026)

Original browser link activation now runs a trusted, bubbling, cancelable DOM click through the existing JS capture/target/bubble path **before** its default navigation. A typed `op_js::DomClickDispatch {handled, default_prevented}` preserves the cancellation flag when the original event dispatch finishes, including early stopPropagation exits. `op_engine::scripts::dispatch_click` commits DOM mutations, exposes default prevention and fails closed when VM dispatch fails; `Engine::activate_link_at` reuses the normal native `click_at` event/focus path and navigates only if the verified anchor's event was not canceled. HTML `onclick` handler **property** returning exactly `false` now cancels the event, unlike a normal `addEventListener` callback whose return is ignored. Native tests check `preventDefault` from target, ancestor capture and onclick; legacy `returnValue=false`; property return false; passive listeners and stopPropagation; and normal navigation. All workspace tests, strict Clippy and native startup/link/image smoke pass. Remaining: node/link URL mutation after event, programmatic `Element.click()` default action, more precise keyboard activation, and full UI fidelity.

## P0 HTML pointer-events and painted link occlusion (20 October 2026)

The original CSS computed style now includes inherited `PointerEvents::{Auto,None}`, parsing `pointer-events` with normal cascade precedence and the CSS-wide `inherit/initial/unset` keywords. Hit testing for native hover, press, and JS click skips computed `pointer-events:none` / `visibility:hidden` targets while retaining descendant overrides (a child explicitly declaring `auto` can receive input). Inline raster images now provide generation-safe image-element click regions at their real painted bounds and inherited paint-key. The Win32 painted-link path reports document coordinates with each href (`FollowLinkAt`); `Engine::activate_link_at` verifies the topmost CSS/DOM target is inside the matching `a[href]` before navigating, otherwise dispatches an ordinary DOM click to the overlay. Native link rectangles are examined back-to-front. Includes tests for a high-z pointer-transparent overlay, click-through JS handler, covered-link rejection, valid clickable link under a `none` overlay, and image-link navigation. All Rust workspace tests, Win32 startup/link/image smoke, Clippy and native five-public-site probes pass without changing the site's old partial/blank outcomes. Remaining: precise painted hit/clip/opacity coverage, image atomics without loaded raster, link hover cursors may be conservative/inaccurate, JS click `preventDefault` on navigation, keyboard focus and broader CSS pointer-events semantics.

## P0 paint-group-aware native hit target precedence (20 October 2026)

Implemented original CSS stacking-context-aware ordering for cached hit rectangles in `op_engine::order_native_hit_regions`. `op_layout::ClickRegion` retains the `PaintKey` from positioned block and inline text/fragment output; `mark_positioned_outputs_since` transfers the actual paint group into descendant hit output without re-laying out text. The engine orders candidate targets using `LayoutTree.paint_groups` parent contexts and sorted negative/normal/nonnegative children, matching the original paint compositor's broad traversal; DOM preorder breaks ties for ordinary in-flow overlap. Native hover and press consume the resulting cached front-to-back order, while click dispatch derives matching order from its fresh layout. New Rust regressions confirm z=8 over z=2 and z=-2, nested child z=999 trapped beneath sibling z=2, normal flow over a negative context, later same-level positioned siblings, and actual JS event delivery to the frontmost target. Full Rust workspace, Clippy, Win32 startup and link smoke pass. Five anonymous real-site classifications are unchanged. Limits: this is block and inline-fragment coverage; precisely interleaved text/decoration ink, replaced/atomic inline targets, pointer-events and accurate painted clipping/opacity still require follow-up.

## P0 ordinary DOM hit-testing and pointer-active lifecycle (19 October 2026)

Original layout now produces hit rectangles for all rendered block elements, not just those with an `id`. Inline text and links use source-DOM UTF-8 ranges mapped into glyph-measured text rectangles (bounded to 256 per text run), **without creating synthetic inline CSS boxes or changing wrapping**. Rectangles follow native layout translation, nested scrolling and overflow clipping. Original `Engine::hover_at` can use ordinary elements and text links; `Engine::press_at/release_press` adds pointer `:active` on mouse-down and clears it on mouse-up, cancellation, and navigation. Windows captures left-button input, reports out-of-page release and capture-loss cancellation, and the browser worker retains press/click sequence while coalescing hover moves. Browser startup, native link, Rust, layout and UI regressions pass. Next P0: stacking-order/pseudo/replaced-element precision, style invalidation costs, DOM event-model consistency, keyboard focus-visible and real-site JS bootstrap. No fully functional modern site yet.

## P0 native pointer hover and mouse-driven focus (19 October 2026)

Original Rust CSS author matching now accepts a generation-safe `StyleInteractionState` snapshot, threading it through selector recursion (including `:is/:not/:has`, sibling combinators and nested `:nth-child` filtering). `:hover` matches the live native target and its ancestors; `:focus-within` matches the focused node's ancestor chain; `:focus` matches the focused element. Windows `WM_MOUSEMOVE/WM_MOUSELEAVE` provides pointer coordinates, the original engine uses layout-owned ID-block hit regions cached from the last render, and a target transition alone triggers CSS recascade and native repaint. Repeated mouse moves are coalesced while the worker is busy. Clicking a focusable ID-bearing native block (including `tabindex`) updates `:focus` even without JS handlers. Pending: ordinary inline/non-ID hit testing, keyboard focus-visible behavior, press-time `:active`, JS event/selector state synchronization and screenshot correctness. The same five Release public sites show unchanged JS and CSS errors, no YouTube text and blank Reddit; this is native interaction support, not modern-site certification.


## P0 responsive screen media and static state selectors (19 October 2026)

Original `op_css` now evaluates a bounded CSS `@media` subset against actual viewport dimensions (screen/all/print, conjunctions, OR lists, min/max/exact width/height in px and orientation), preserving source order and nested `@supports`. Unsupported feature conditions fail closed with a specific diagnostic rather than flattening conditional CSS. `op_engine` threads the viewport through inline/linked author styles and recomputes CSS on window resize. Supported initial-state pseudo-classes include `:hover`, `:active`, `:focus`, `:focus-visible`, `:focus-within` (false until input state is wired), plus bounded attribute-based `:enabled`, `:disabled`, `:checked`, `:read-only`. The native diagnostic groups CSS errors without leaking author-defined selectors. Real YouTube CSS errors reduce **35→22**, Pikabu **532→114**, DeviantArt **676→170**, but YouTube paints no text and the visible content counts of the other two pages change significantly. **No verified visual compatibility gain**; future P0: correct dynamic pseudo states, broader media queries, content bootstrap and native layout.


## P0 blank-viewport root-cause census (19 October 2026)

Added a privacy-preserving, read-only connected-DOM census in `op_engine` and per-site values in the native Release probe. Counts DOM elements, form/iframe/input/script structure, body text, CSS-suppressed text, visible candidates and source-only text, without copying page strings or URLs. Real Reddit is now proven to have **zero nonwhitespace text nodes in its body**, despite executing its one script successfully; its markup contains one form, four input elements and no external script tags. The response may be a service/interstitial shell, but this measurement alone does not prove why. YouTube has 13 body text nodes, all CSS-suppressed, 9 external scripts, 35 CSS parser errors and 4 JS failures. Raising the script quota from 16 to 32 made YouTube errors increase from 4 to 7 without painting text and caused Pikabu to time out; reverted. Next: targeted script/bootstrap loading and CSS/DOM reconstruction, never mask absent content with fake pixels.


## P0 live Reddit parser-unblock and guarded async control flow (19 October 2026)

Original Rust async lowering now supports awaited if-tests, terminal return/throw branches (including nested if), template substitutions with pre-await stringification, awaited throw expressions, and **ordered multi-declarator let/const/var initializers**. Parser diagnostics retain the exact source byte offset of the await and label the unsupported statement category. JS VM/GC and HTML -> native paint regressions pass. A repeat real-HTTPS native probe changes Reddit from **0 executed / 1 failed script** to **1 executed / 0 failed**, with no DOM mutations, **0 text** and a white viewport. This is a parser/JS execution milestone, NOT working Reddit. The other four site script failure counts remain Google 2 / YouTube 4 / Pikabu 11 / DeviantArt 3. P0 still requires resumable loop/try/finally and nonterminal branch frames, real content/layout correctness and on-demand profiling.


## P0 short-circuit and one-sided return awaits (19 October 2026)

The original JS lowerer now handles `return flag ? await a() : b`, `return flag ? a : await b()`, and `return value &&/||/?? await expression` where the awaited operand is direct and sole. The compiler emits actual conditional return paths, avoiding Promise jobs on skipped branches, retaining left-hand side identity/truthiness/nullish semantics. Focused JS/GC and native text paint regressions pass. Not covered: variable-initializer and expression-statement one-sided branches, awaited conditions, more deeply nested branch expressions, loops, and try/finally. On five unchanged public URLs the initial JS failures remain **2/4/11/1/3** (Google/YouTube/Pikabu/Reddit/DeviantArt); YouTube paints no text and Reddit stays blank, so no real-site improvement is certified.


## P0 bounded conditional and object awaits (19 October 2026)

Ordinary object literal fields now preserve prior evaluations across suspension and GC. A ternary supports a direct await in **each** arm, selecting the branch before suspension. JS, GC and original HTML-to-native-paint tests pass. The unchanged five-site native Release probe still shows 2/4/11/1/3 JS failures (Google/YouTube/Pikabu/Reddit/DeviantArt), with Reddit blank and YouTube textless. Accessors, __proto__, one-sided/nested branch awaits, loop/try/finally and short-circuit-RHS awaits still reject; a full suspended VM frame and verified first paint remain P0.


## P0 original JS expression-await checkpoint (19 October 2026)

The independent JS parser now lowers awaited values within bounded *eager* expressions, including binary/unary operators, positional function and method-call arguments, arrays, sequences, computed property keys, simple assignments, conditional tests and constructor arguments. Pre-await operands and getters are captured in evaluation order. A compiler-only `CallWithReceiver` emits the native receiver-preserving VM call opcode, not writable `function.call`. Explicit GC and HTML -> native paint tests pass. This is NOT a full paused-stack VM: awaited conditional branches, logical RHS, loops, try/finally, and certain object/assignment patterns still reject. Five-site repeat remains Google **2**, YouTube **4**, Pikabu **11**, Reddit **1**, DeviantArt **3** initial JS failures, with Reddit blank and YouTube textless. Next: semantics-correct suspend/resume through arbitrary try/loop control flow plus visible-site script/layout fixes.

Updated 2026-10-16. This file contains **decisions, priorities, acceptance gates and current status only**, not a running implementation diary. Completed M0-M4.35 milestone narratives remain recoverable from Git history and are tracked in [DEV_LOG.md](DEV_LOG.md). Measured test history is in [COMPATIBILITY.md](COMPATIBILITY.md).

## Product goal and constraints

Independent lightweight Windows browser with its own Rust HTML/CSS/layout/paint/JavaScript stack. No embedded Chromium, Gecko, WebKit, V8 or other browser/JavaScript engine. Ship readable, scrollable, interactive pages, not a growing list of partially implemented Web APIs.

A green handpicked test fixture is **not** evidence of general website compatibility. Regressions must be reproducible and committed with their source manifest.

## Active priority order (replaces JS EventTarget-first roadmap)

| Priority | Workstream | Current reality | Next deliverable and exit gate |
| --- | --- | --- | --- |
| **P0-1 NOW** | **Modern web rendering and JS integration** | Live-network baseline: 20 services, zero verified fully functional pages; blank SPAs, JS errors, malformed rendering and HTTP blocks separated. | Repair first paint, prioritize actual JS/DOM blockers and visual layout differences, rerun the same public URLs and screenshots. See [REAL_WEB_BASELINE.md](REAL_WEB_BASELINE.md). |
| **P0-2 NOW** | **Layout, scrolling and on-demand profiler** | Bounded nested scrolling and original Win32 paint exist, but modern Flex/Grid/forms are incomplete; GC has synthetic metrics but no interactive CPU/wall phase profiler. | Restore real-site layout/scrolling; add opt-in phase tracing for load, JS, DOM, style, layout, paint and GC during interactive browsing. |
| **P0-GUARD** | **Original JS VM GC correctness/performance** | Two 30-minute synthetic stress runs passed safety; native DOM arena bounded, Private Commit near a plateau, but rare 448-ms wall GC pause remains unexplained. | Maintain GC correctness gates; investigate pause outliers on real browsing traces rather than blocking all rendering work. |
| **P1-C** | **Flexbox with multiple lines** | **R2a–R2c partial DONE:** row wrapping/gap, proportional grow/shrink, flex basis/shorthand and six `justify-content` values, measured in original GDI pixels. | **R2d NEXT:** cross-axis `align-items`, flex direction/order and full min-content sizing, followed by explicit real-page visual comparison; current subset is not full Flexbox. |
| **P1-D** | **CSS Grid foundation** | Table grid track sizing exists; it is **not** CSS Grid Layout. | R3: explicit/auto tracks, fraction units, item placement and basic gap, then implicit tracks and minmax. Screenshot+geometry tests. |
| **P1-E** | **Backgrounds and typography** | URL image tiling for blocks/tables and GDI font measurement exist, but broad backgrounds, web fonts and modern shaping/fallback do not. | R4: repeat/size/position and clipping for image backgrounds; then DirectWrite shaping/fallback and real @font-face loading with origin/security controls. |
| **P1-F** | **Forms and editing** | Some native/text/checkbox and selector subsets; not a usable complete form implementation. | R5: input/edit focus, keyboard text, caret/selection, submission semantics, accessibility names and persistent state; real click+typing integration. |
| **P2** | **More JS syntax, WebIDL and broad Events conformance** | Original VM has Promise/partial fetch/EventTarget; many modern APIs missing. | Resume **after** R1 and GC safe reachability work. Fix only JS/DOM blockers needed for real sites until then. |

**Priority override 2026-10-13:** finish safe, reclaiming original JavaScript GC before resuming renderer/Flexbox/Grid/forms/API development. Existing rendering improvements remain frozen except for required GC regressions and integration fixes. Do not claim GC completion without live heap plateau and identity-safety proof.

**G3 ownership update:** EventTarget-owned lifecycle/AbortSignal handlers are marked through reachable receivers, not unconditional side-map roots. Orphan signal/closure cycles can be reclaimed; live callbacks survive. The DOM/fetch host-root audit and 30-minute memory gate remain outstanding.

**G3 timeout update:** orphan AbortSignal.timeout deadlines are weak and pruned after a validated sweep. Pending fetch/AbortSignal.any roots and abort observers preserve live timeout signals; the fixed 64-entry timeout queue can be reused after orphan reclamation.

**G3 detached-DOM update:** disconnected node wrappers/listeners/native caches can be reclaimed; five tests cover identity, caller roots and 40 create/remove cycles. Still pending: external host lifetime audit, fetch/Promise races, allocation budgets and long-run process-memory proof.

**G3 follow-up (2026-10-14):** GC purges native descendants without JS wrappers; live static query results preserve backing ancestors. Five new tests include two fetch completion/cancellation races across GC. Full host-root audit and sustained native memory proof are still outstanding.

**G3 allocation-pressure update:** browser idle GC now observes remaining object/environment budget headroom (including free slots), so low configured limits and allocation bursts are reclaimed before the next task. Three tests pass; oversized single tasks and extended navigation/RSS profiling remain outstanding.

**G3 host boundary fix:** binding preserves actual connectivity and canonical subtree links; the 256-node staging quota now counts outstanding unbound nodes rather than all creations since page load. Native Document storage now sweeps orphan tree components and recycles generation-checked slots while preserving retained detached identity and pruning weak host caches.

**G3 measurement update:** each successful automatic collection now contributes to a fixed-size pause histogram (p50/p95/p99 upper bounds), including passes between CSV samples. The probe separates timer allocation churn from detached DOM/listener churn and reports authoritative native DOM arena slots. Native sweeping follows validated JS roots, including static queries/views/callbacks and native ancestors/descendants; parser-owned handles defer sweeping until parsing completes. The 30-minute Windows gate, pause outliers and broader navigation/fetch audit remain open.

**G3 synthetic 30-minute Windows gate (2026-10-14): PASS for Release DOM workload only.** 1,800 per-second CSV samples, 80,939 JS/native sweeps with zero errors, 323,854 nodes reclaimed, 21 reserved native slots at finish; Working Set shows no sustained increase but was trimmed by the OS. The p99 JS pause bucket is <=525 us; one 273,375 us wall-time outlier needs diagnosis. Full browsing/host-memory acceptance remains open.

**G3 pause attribution and fetch audit (2026-10-15):** The 273 ms historical outlier cannot be retrospectively assigned to a GC phase. The new probe captures worst-pass mark/metadata/sweep wall times and counts >=16/50 ms. A composite AbortSignal owning a pending fetch remains alive over 16 explicit sweeps, cancels correctly and discards stale completion. Reproduce a long-session outlier and profile thread scheduling, then broaden real-network/navigation and committed-memory gates.

**G3 full host checkpoint and navigation isolation (2026-10-15):** Added per-page timing for the entire callback-bound GC checkpoint through weak-resource cleanup, plus >=16/50 ms outlier counts. A regression verifies that a delayed old-generation response cannot fulfill a same-ID fetch in the new document; a matching current-generation result can. Longer phase-aware profiling, repeated real HTTP navigation, committed/private bytes and multi-tab memory remain open.

**G3 private memory and loopback HTTP update (2026-10-15):** Added extended Windows per-process private-commit samples and an offline CSV validator that distinguishes PrivateUsage from OS-trimmable working set. A true localhost HTTP test navigates away while a fetch is in flight, then verifies the delayed previous response cannot corrupt the next page even under a reused request ID. Broad real-site/multi-tab workloads, GC CPU-time attribution and leak gates remain open.

**P0 real-web priority update (2026-10-15; supersedes the 2026-10-13 GC-first freeze):**
The original GC passes extended synthetic correctness and private-memory
plateau checks, although rare 448ms wall-time stalls remain under watch.
Modern website functionality is now the first product gate: the 20 real
public URLs in [REAL_WEB_BASELINE.md](REAL_WEB_BASELINE.md) gave 10
JS-error partial pages, 2 skeletons, 3 blank viewports, 3 403/429 blocks,
1 malformed/unverified render and 1 timeout. Prioritize JS/DOM blockers
and practical CSS/layout/paint improvements that measurably change those
screenshots. Integrate an on-demand interactive CPU/wall-time profiler
for GC/performance diagnosis during real browsing; do not resume
synthetic-only micro-optimization without an observed failure.

## Real-site development gates (15 October 2026)

The next compatibility iteration is measured on **Google search, YouTube,
Pikabu, Reddit and DeviantArt**, with the other fifteen public URLs retained
as a regression/control cohort. Each run records real native pixels,
first-page navigation outcome, bounded initial JS failure categories,
skipped-script causes and runtime/GC failures. A changed screenshot is
not automatically an improvement: compare the screenshot and actual visible
content, then separately verify navigation, input and scrolling.

1. **Immediate (JS blocker audit):** name the top syntax/runtime failure causes
   from actual public pages rather than relying on the failure count alone.
   Implement conforming language primitives with focused VM tests.
2. **First visible milestone:** make at least two of the five priority pages
   display a coherent, substantially correct first viewport (navigation
   and primary content), not just isolated text or skeletons; record
   explicit evidence and remaining unsupported features, with no baseline
   regressions hidden by a changed URL or missing network response.
3. **Functional milestone:** interact with a real page (click, scroll, text
   entry where applicable), not simply finish a headless screenshot.
4. **Profiling gate:** gather on-demand CPU-time vs elapsed-time traces
   of navigation, script, DOM, layout, paint and GC inside the running browser;
   pursue the old 448-ms GC wall-time spike only if reproduced or independently
   explained. Synthetic-only microbenchmark improvement is not the product goal.
5. **Integrity:** do not add Chromium/other JS engines, counterfeit web
   responses, auth/captcha workarounds or unconditional caps increases just
   to manipulate compatibility results. The public-site suite is opt-in;
   offline CI and GC safety tests must remain green.

**P0 performance timing follow-up (2026-10-15):** The original page VM
now provides a browser-owned `window.performance` object with monotonic
`now()` and Unix-epoch `timeOrigin`. It rejects illegal method receivers
and construction. The updated real DeviantArt run still reports four
failed scripts, including unsupported backtick syntax and null/undefined
access; the former missing-`performance` error did not recur. No visual
improvement is certified. Full Performance APIs and interactive profiling
remain open.

**P0 modern-JS syntax follow-up (2026-10-15):** Implemented original
untagged JavaScript template literals: multiline cooked strings,
`${expression}` substitutions, nested templates, escapes and ordered
concatenation. Four focused regressions and full workspace verification
pass. The five public priority URLs were rechecked with the same
manifest: DeviantArt's original unsupported backtick changed into a
later `expected expression` blocker, but the page still has four JS
failures; YouTube and Reddit remain skeleton/blank, and Google/Pikabu
still have JS errors. No visually correct first viewport is certified.
Tagged templates, full Unicode surrogate handling, arrow functions,
and the opt-in interactive profiler remain pending.

**P0 arrow-function follow-up (2026-10-15):** The original JS
lexer/parser/bytecode/VM now support ordinary `x => expr`,
`(x, y) => expr` and block-bodied arrow functions. Arrow calls
capture lexical `this` and `arguments`, preserve closures across
GC and cannot be constructed with `new`; there is no own
`prototype`. Async arrows, default/rest/destructured parameters,
and full ECMAScript validation remain open. On a repeat five-site
live pass, YouTube script failures dropped 12 -> 11 and Pikabu
14 -> 12 compared with the immediately preceding template-only
run; Google/Reddit/DeviantArt remain at 2/1/4. No modern site is
now visibly functional, and live content can change between runs.

**P0 JS delete operator (16 October 2026):** The original classic-script
parser/compiler/VM now recognizes `delete`, preserves computed-member
evaluation and deletes ordinary object/array fields without changing array
length. Bound identifiers remain protected; host and descriptor edge cases
are partial. On live YouTube the missing-`delete` error disappeared,
6 scripts executed (previously 5) and 10 failed (previously 11), but
**zero text** is still painted. Pikabu has 12 failures. The new settle
window was 1300ms, versus 1000ms before, so this is diagnostic only.

**P0 Image constructor (16 October 2026):** `new Image()` and
`Image()` now produce browser-owned detached `img` nodes, reflecting
`src`/`alt`/`width`/`height`. Script attachment paints actual PNG
pixels in an engine integration regression. Real YouTube advances to
**7 executed / 9 failed** scripts (previously 6/10) and its prior
missing-`Image` diagnostic disappears, but it still paints **zero
text**. Pikabu remains **12 failed**; neither page passes visual
acceptance. Detached image network load/error events, natural dimensions,
absolute URL reflection and full HTMLImageElement remain open.

**P0 read-only page Location (16 October 2026):** Loaded HTTP(S) URL now reaches a shared `window.location` / `document.location` object and `document.URL`, `documentURI`, `baseURI`. URL component reads work; navigation setters explicitly reject. VM and parser-DOM integration tests pass. Pikabu's `location is not defined` diagnostic becomes a later `value is not callable` failure; Pikabu still has **12 failed** scripts, malformed layout. YouTube remains **9 failed / zero text**. Next: actual JS syntax blockers, dynamic script loading and visual layout, not a cosmetic error-count pass.

**P0 original modern JavaScript (16 October 2026):** Bounded lazy `??` coalescing and a synchronous `for...of` subset now execute through the original lexer/parser/bytecode/VM. Arrays, Unicode strings and selected DOM collections are supported with per-iteration `let`/`const` scopes, `break`/`continue` and a 4,096-item cap. Custom iterator protocols, destructuring, `for...in`, mutation-sensitive array iteration and general ECMAScript parity remain open. Both real 5-site checks still report 2/9/12/1/4 JS failures (Google/YouTube/Pikabu/Reddit/DeviantArt); YouTube paints no text and Reddit is blank. Further improvements must target observed failures, not widen budgets blindly.

**P0 real-site JS triage and for-in (16 October 2026):** Added explicit opt-in syntax-context diagnostics (`OPBROWSER_JS_ERROR_CONTEXT=1`), bounded to a small printable UTF-8-safe excerpt and disabled by default. Native YouTube diagnostics identified actual `for(k in a[0])`, five regex-literal parse failures, a labeled statement and later Web API errors; Reddit's first inline script uses `async function`. Original JS now runs a bounded own-key `for...in` subset for identifiers and single `var`/`let`/`const` bindings. On native YouTube the former `expected ';' after for initializer` is replaced by a later `Type: cannot read properties of null or undefined` failure; still 7 scripts executed / 9 failed, zero painted text. Next: RegExp literal lexing and JS regex behavior, callback/async semantics, host APIs, and actual pixels. Own-key order, inherited keys, descriptors and full ECMAScript for-in remain incomplete.

**P0 RegExp and expression-grammar follow-up (16 October 2026):** Added native original JS lexer distinction between regex literals and division, `RegExp` objects with bounded `test/exec`, and String `replace/match/split`. A small finite-automata `regex` dependency performs matching, not HTML/DOM or JavaScript execution. Added `+=`/`-=` assignments with receiver-once evaluation and the sequence/comma operator without merging function arguments. UTF-16 RegExp offsets and literal string splitting have tests. Measured live YouTube at **8 executed / 8 failed / zero text**, compared to 7/9 earlier. `get`/`set` object accessors, labeled statements, async functions, dynamic scripts and many Web APIs remain. Regex support is a documented subset: unsupported backreferences/lookaround, missing constructor/protocol features and a conservative slash lexical heuristic. Do not treat zero syntax failures as guaranteed visual progress.

**P0 accessor follow-up (16 October 2026):** Original JS now executes
getter/setter object fields and concise methods through GC-traced
function descriptors, with receiver-binding and closure tests.
Five-site 1300 ms run: Google 2, YouTube 8, Pikabu 11, Reddit 1,
DeviantArt 3 JS failures. YouTube still has **zero painted text**,
Reddit remains blank. Next blockers: remaining syntax (including
labels/async functions), dynamic scripts, DOM and visible layout,
plus on-demand interactive profiling. Failure counts alone are not
visual compatibility.

**P0 labeled-control flow (16 October 2026):** Independent JS parser,
bytecode and VM now support contextual statement labels, named
break/continue, nested loops and finally-safe named exits through
nested try/catch/finally. Invalid labels are rejected. A real
HTML-to-DOM-to-paint regression and VM tests pass. Five 1300ms
real-site probes still report failed JS Google **2**, YouTube
**8**, Pikabu **11**, Reddit **1**, DeviantArt **3**. YouTube remains
**zero-text** and Reddit blank. YouTube's formerly rejected labeled
jump now advances to another `expected expression` error. NEXT:
investigate its `expected ')' after for-of iterable` and remaining
actual runtime failures, async functions, script loading and visible
layout. This change is a language-semantic fix, not a full-site pass.

**P0 modern JS syntax unblocking (16 October 2026):** The original
parser now handles a comma-expression RHS in `for...in`, observed
directly in live YouTube code, without broadening `for...of`.
Native `*=`/`/=`/`%=`, shifts `<<`/`>>`/`>>>`, their compound
assignments, 32-bit signed/unsigned semantics and correct operator
precedence are implemented. Parser/VM plus HTML-to-native-paint
regressions pass. In a 1300 ms live five-site run, failed JS:
Google **2**, YouTube **8**, Pikabu **11**, Reddit **1**,
DeviantArt **3**. YouTube's three syntax errors now proceed to
`RegExp` and `Math` missing-global ReferenceErrors; it
still paints **zero text**, Reddit is blank. NEXT: original
`Math` and `RegExp` globals with standards-based tests,
async function parsing/runtime, script scheduling, and real layout.
Do not treat error transitions as successful website rendering.

**P0 original Math/RegExp globals (16 October 2026):**
`Math` includes 17 intrinsic methods, standard constants and
non-cryptographic random; `RegExp()` and `new RegExp()` reuse
the bounded literal regex implementation, now supporting
`instanceof RegExp`. The VM and HTML-to-native-paint tests pass.
Five real site measurements remain **2/8/11/1/3** failed JS
(Google/YouTube/Pikabu/Reddit/DeviantArt). YouTube still has
zero text; Reddit is blank. The two missing globals progress to
`navigator`/`Symbol` and TypeErrors. Next: real feature detection,
symbols, async functions, dynamic scripts and visible rendering.

**P0 navigator and UTF-16 String search (17 October 2026):**
Original document-bound navigator now honestly exposes
OPBrowser identity, provisional language fallback,
`cookieEnabled=false` and bounded hardware concurrency.
Undefined host capabilities remain absent. Native VM supports
`String.indexOf/includes` with UTF-16 positions and strict
builtin receivers via `Function.prototype.call`, retaining
sloppy global-this behavior for user functions. VM and native
DOM-to-paint regressions pass. Five public-site checks still
show failed scripts **2/8/11/1/3** (Google/YouTube/Pikabu/
Reddit/DeviantArt). The earlier YouTube `navigator` missing
global advances to a TypeError. YouTube remains zero-text
and Reddit blank. NEXT: real Symbol primitives and well-known
properties, async function syntax, dynamic scripts and
actual viewport rendering. These cannot be replaced by
optimistic fake globals.

**P0 original Symbol semantics (17 October 2026):**
`JsValue::Symbol` adds identity-bearing symbols;
`Symbol()`, `Symbol.for`, `keyFor`, selected well-known names,
symbol-keyed object maps and GC tracing now work in the
original VM. `in`, property read/write/delete/update,
prototype lookup and JSON/for-in omission have regressions,
as do invalid numeric coercions and HTML-to-native-paint.
Five public-site JS failures remain **2/8/11/1/3**
(Google/YouTube/Pikabu/Reddit/DeviantArt); the native YouTube
Symbol ReferenceError advances to a later TypeError. YouTube
still has zero text and Reddit is blank. NEXT: diagnose
non-callable global/WebIDL methods, real async functions,
dynamic scripts, DOM APIs and layout. Full symbol reflection,
computed object-literal symbol keys and custom iterators
remain unsupported and must not be silently faked.

**P0 measured YouTube JS progress (17 October 2026):**
Opt-in capped callsite diagnostics identify attempted
non-callable method names. Original `Array.shift/unshift`
handles sparse arrays and bounds sizes; `Object.create`
supports object/null prototypes; basic
`Object.getOwnPropertyDescriptor` handles own
data/accessor values, with missing descriptor attributes
explicitly documented. VM and native HTML-to-paint tests
pass. New real-site JS failures **2/5/11/1/3**
(Google/YouTube/Pikabu/Reddit/DeviantArt),
compared with previous **2/8/11/1/3**.
YouTube still renders no text, Reddit remains blank.
NEXT: original Object.defineProperty with true
descriptor semantics, verified namespace-aware
`document.createElementNS`, async JS, script scheduling,
and real visible layout. Do not call this a site pass.

**P0 descriptor semantics (17 October 2026):**
Original `Object.defineProperty` and sparse per-object
attribute flags now control `writable`, `enumerable`,
`configurable`, delete, assignment, `for...in`, JSON and
`Object.getOwnPropertyDescriptor`. Basic data/accessor
descriptors, invalid redefinition and GC are covered in
VM and HTML-to-native-paint tests. Live failed script
counts **2/4/11/1/3** (Google/YouTube/Pikabu/Reddit/
DeviantArt), versus YouTube **5** before this change and
**8** at start of turn. Still no text on YouTube and
blank Reddit; 19 YouTube scripts are skipped by count limit.
NEXT: original namespace-aware `createElementNS`,
null/undefined errors, missing async functions, dynamic JS,
Web APIs and actual viewport rendering. Full descriptors
and WebIDL semantics remain incomplete.

**P0 namespace-aware DOM creation (17 October 2026):**
Original JS `document.createElementNS` validates XML-qualified names
and selected reserved namespace/prefix combinations. `op_dom` now
stores each element namespace and supports native namespace-aware
allocation. The host replays dedicated CreateElementNS operations and
preserves metadata across synthetic/native identity binding,
document reset and GC. Cross-script HTML-to-DOM-to-native-paint and
VM/DOM tests pass, along with full workspace tests. Real YouTube
no longer fails at createElementNS, but still reports **4**
JS errors and zero painted text. Opt-in null-property diagnostics
now reveal `set`, `responseStart`, `prototype` x2.
NEXT: identify the missing receiver origins, improve native
Performance Timing/JS object support without fabricated browser
state, address async scripts/DOM, and actual modern layout.
SVG drawing and foreign-content parser namespaces are NOT done.

**P0 bounded async-functions checkpoint (19 October 2026):** The original parser, compiler and VM now support declared/anonymous `async function` forms **without `await`**. Calls return original self-hosted Promises with result adoption and rejection for throws/runtime errors; async functions cannot be constructed. Real `await` suspension, async arrows and complete JS scheduling remain unsupported (explicit syntax rejection). New parser/VM tests pass. The 1300ms five-site native rerun retains JS failures **2/4/11/1/3** (Google/YouTube/Pikabu/Reddit/DeviantArt). Reddit advances to unsupported `await` but remains blank; YouTube paints zero text. NEXT: compliant `await` and script scheduling plus visibly usable first viewport.

**P0 sequential await and async arrow checkpoint (19 October 2026):** Original JavaScript now supports contextual async arrows and **sequential direct** `await` variable initializers, standalone statements and returns. Awaited Promises resume through the original microtask queue, including pending network replies after GC, with lexical this/arguments and rejections preserved. Multiple-await VM regressions and a native HTML-to-painted-text integration pass. This is NOT arbitrary-expression/loop/try await or a general paused-stack VM. Five live sites still have JS failures **2/4/11/1/3** (Google/YouTube/Pikabu/Reddit/DeviantArt), Reddit blank and YouTube zero text. Reddit now reaches an explicitly unsupported complex-await placement after parsing its async arrow. NEXT: general continuation semantics and visible-page blockers.

## Engineering gates

1. **Visible behavior:** every rendering change must prove pixels and/or actual native input behavior, not only CSS parser acceptance. Scroll tests include nested containers, clipping of backgrounds/text, scroll offsets, click targets after scrolling, and restoration after resize.
2. **Long-session stability:** use M4.35 `JsRuntime::heap_usage()` counters, process working set, and repeated navigation/DOM churn; count unreachable-but-retained allocations honestly. After G2/G3 require plateau under bounded workloads and no stale ObjectId or closure references.
3. **Security/correctness:** preserve bounded allocations, original same-origin rules, event error isolation, and default-deny unsupported operations.
4. **Independent comparisons:** never silently mutate a frozen WPT/Test262 manifest. Add new, explicitly named test versions and record PASS/FAIL/SKIP. A new category needs an honest initial baseline, even if it is poor.
5. **Release checks:** Rustfmt, strict Clippy, full workspace tests, Win32 smoke, code intelligence, compatibility regressions, clean public main and Wiki.

## Measured baseline (separate, non-comparable selections)

| Suite | Latest recorded measurement | What it does **not** prove |
| --- | --- | --- |
| Pinned WPT Static v1 | **194/200 exact after R2a** (197/200 before); three extra FAIL in modern `margin-trim` flex reftests now that `gap` is painted | Only 200 selected reftests; current flex lacks logical margin-trim and cannot claim broad CSS readiness |
| Pinned WPT Positioning v1 | 54/100 exact (R1.1) | Substantial positioned-layout gaps remain |
| Pinned Test262 Parser v1 | 523/1,983 attempted | Parser subset, not whole ECMAScript |
| Pinned Test262 Runtime v2 | 78/179 attempted; 110 explicitly SKIP | Handpicked language/runtime families, not overall JS compatibility |
| Pinned WPT DOM/Events v8 (M4.35) | 21/21 attempted files PASS, 0 FAIL, 3 SKIP; 24 selected | Manually selected synchronous files, not the official full WPT harness |
| Real-world visual/site smoke | **No representative published baseline** | Cannot yet claim production-site readiness |
| Long-session GC/memory plateau | **G2b automatic idle GC, no 30-minute process-memory measurement** | Callback/host roots and working-set plateau not fully verified |

## Immediate implementation sequence

- G2b partial DONE: browser-owned automatic mark/sweep after scripts, DOM clicks and network/timer tasks, triggered by live allocation pressure. Weak metadata no longer retains dead Headers/boxed/DOM collection objects. VM and engine tests cover stale IDs, callback closures and reuse between scripts. NEXT GC ONLY: host/fetch/abort roots, error diagnostics, working-set plateau and pause distribution.
- R1.0–R1.1 DONE: block clipping and vertical JS/native scrolling with original GDI pixel and hit-target checks; retain pinned WPT baselines.
- R1.2a–R1.2c DONE (bounded vertical); R1.2d partial ancestor clips; R1.2e partial: one-value overflow-x/y longhands and cascade, bounded horizontal child extents and offsets, Shift/horizontal wheel, native track clicks and GDI. Next: JS scrollLeft, thumb dragging, positioned clips and real-page comparisons.
- R2a–R2c partial DONE alongside P0: multiline row/gap, proportional growth/shrink, basis/shorthand and justification, each with native pixel or CSS regression. R2d next: cross-axis alignment/direction/order, independent visual pages. Horizontal overflow and GC still pending.

## Documentation ownership

- `PROJECT_PLAN.md`: this active roadmap, kept below ~200 lines. No per-commit DONE narrative.
- `DEV_LOG.md`: chronological engineering diary and milestone history.
- `COMPATIBILITY.md`: sourced metric definitions, exact pinned revisions, failures and SKIPs.
- `GC_IMPLEMENTATION.md`: tracing/collection design, hazards and acceptance criteria.
- `CODE_GRAPH.md`, `CODE_SLICES.md` and generated reports: actual source structure.
- `wiki/Current-Status.md` and `wiki/Roadmap.md`: public, updated when milestones ship.

Historical details removed from this plan are **not deleted from project history**: they remain in Git and the detailed DEV_LOG. Do not paste future release notes into this active plan.
