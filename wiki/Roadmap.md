# Roadmap

## First-party editable selection highlighting (20 October 2026)

Done: blue/white glyph-repaint highlight for active native and script selections in focused input/textarea with Unicode-safe boundaries, password masking and explicit newline row support. Next: mouse drag selection, accurate atomic control box geometry, wrapping/scrolling and RTL/shaped glyph positioning, clipboard and IME.

## Native form click-to-caret (20 October 2026)

Implemented nearest-scalar mouse caret placement for text-like input and explicit textarea rows with live UTF-16 JS selection synchronization and Unicode-safe follow-on edits. P0 next: drag-to-select, exact real control boxes with text overflow/wrapping, clipboard, IME and bidirectional/grapheme cursor movement.

## Native/DOM text selection bridge (20 October 2026)

Implemented JS selectionStart/selectionEnd/selectionDirection and setSelectionRange for text-like controls, including UTF-16 emoji offsets and native editor synchronization. Next: drag-to-select, complete click placement, clipboard, IME composition, setRangeText, and full web selection behavior.

## Next steps following first editable HTML text controls (20 October 2026)

Done: Unicode WM_CHAR including surrogate pairs, first-party input/textarea text editing, scalar-indexed caret/selection, deletion, Home/End/arrows/Ctrl+A, basic multiline Enter, noneditable restrictions, JS .value and cancelable beforeinput/input, native field outlines/caret, manual demo and tests. Highest P0: replace the provisional inline field painter with real atomic/replaced control box layout, selection highlight/drag-to-select, clipboard and text navigation, IME composition, horizontal text scroll/wrapped textarea, InputEvent.data/inputType/change, complete selection semantics and proper form submission. ECMAScript syntax/VM and modern website JS compatibility remain independent major gaps.

## Following programmatic DOM focus and blur (20 October 2026)

Done: first-party Element.focus()/blur() dispatch, activeElement synchronization, original event listener transitions and CSS :focus/native paint handoff with tested native click integration. Next P0 interaction tasks: actual text editing in HTML input/textarea with selection/caret/IME, relatedTarget and activeElement on node detach, CSS-hidden/inert/fieldset focus eligibility, autofocus, script focus scrollIntoView and stronger keyboard modality rules. Continue rigorous conformance via current full Test262/WPT versioned measurements rather than extrapolating from selected historical suites; broader JS syntax, modules, DOM API and modern site startup remain major blockers. YouTube currently lacks rendered text, Reddit blank.

## After activeElement and cancelable native key input (20 October 2026)

Done: DOM document.activeElement initialization and host focus synchronization, focus/blur/focusin/focusout on both Tab and mouse clicks, original KeyboardEvent keydown/keyup dispatch with capture/bubble and preventDefault suppression of Tab/Enter defaults; full Rust/Win32 test matrix. Next P0: JS Element.focus()/blur() and document.activeElement for detached/disabled targets, focus-relatedTarget and autofocus, correct DOM keypress/WM_CHAR/composition and IME editing, backspace/arrow key default actions for real inputs, modifier completeness and long-lived event listener performance. Independent modern-web problems remain JavaScript bootstrap, font/layout fidelity and page painting (YouTube no text, Reddit blank).

## After Tab focus auto-scroll and JS focus event support (20 October 2026)

Done: un-clipped keyboard focus geometry separate from native click targets, nested overflow and root viewport scroll-into-view, and trusted original JS focus/blur/focusin/focusout from Tab. Next P0: mouse-induced focus/blur transitions, correct document.activeElement and relatedTarget, cancelable keyboard keydown/keyup with preventDefault before Tab/Enter actions, scroll anchoring while layouts change, tabindex/inert/fieldset edge cases, real form controls, CSS outlines and accessibility keyboard testing. Independent bootstrap, JS VM DOM conformance, fonts/layout and general modern-site support remain major gaps. An existing interval timing test was flaky on an initial parallel run and merits focused test hardening.

## Next keyboard/accessibility tasks after first Tab navigation (20 October 2026)

Done: bounded page Tab/Shift+Tab order including positive tabindex, original keyboard focus selectors, native visual focus fallback, Enter trusted cancelable anchor activation, input-routing isolation from URL bar and navigation-vs-reflow presentation. Next P0: scroll focused nodes into viewport and nested scroll containers; focus/blur/focusin/focusout and keydown/keyup DOM listeners with preventDefault; correct input/checkbox/select/textarea keyboard behavior; disabled fieldset and inert descendants; browser toolbar-to-page Tab cycling; outline clipping to stacking contexts; HTML/SVG accessibility targets. Continue JS startup, CSS/font/render conformance, and isolating intermittent Pikabu long load. YouTube and Reddit remain far from complete.

## After native preventDefault integration (20 October 2026)

Cancelable trusted native anchor clicks now call the original JavaScript event system before navigation. Remaining P0 interaction tasks: re-read source anchor/href after click mutations, implement programmatic `Element.click()` and keyboard Enter/Tab default actions, synchronize `:focus-visible` and focus events, prevent native pointer-release default actions when press/release targets differ, and reconcile native link hover cursors. Independent compatibility bottlenecks include modern JS bootstrap, CSS/layout/font fidelity, accurate accessibility semantics and a repeated live Pikabu timeout (23s/45s) needing isolated profiling. Browser is not considered ready for YouTube, Reddit or general modern-web use.

## Input and default link actions after pointer-events foundation (20 October 2026)

HTML `pointer-events: auto|none` is now computed and inherited, including pointer-transparent ancestors with explicit-auto descendants; native loaded inline images participate in target picking, and hyperlink activation is validated against the topmost source-DOM anchor instead of being blindly trusted from Windows raster link regions. Next P0: reconcile native painted-link hover cursor with DOM selection, support all atomic/replaced hit regions, CSS paint-specific pointer regions and SVG values, dispatch cancelable JS click/default-action `preventDefault` before navigation, and implement keyboard Tab/focus-visible and synchronized DOM focus events. Broader JS/runtime and layout/browser compatibility are still prerequisites for YouTube/Reddit.

## Input layering stage completed, subsequent P0 tasks (20 October 2026)

Native target picking now respects ordinary positioned z-index precedence, nested stacking contexts, negative layers and DOM-order ties; tests exercise real JavaScript DOM click dispatch on overlapping elements. Next: painter-exact foreground/background hit testing, pointer-events:none and disabled-form behavior, image/inline atomic targets, native hyperlink overlay occlusion, bounded incremental hit-map updates, keyboard Tab/focus-visible and DOM JS event consistency. Independent major blockers remain browser-grade JS startup, DOM APIs, script loading and layout/font fidelity. No claim that YouTube or Reddit render fully.

## Next input/rendering milestones after expanded native hit testing (19 October 2026)

Ordinary ID-less blocks and inline text/link source spans now support real native hover, and pointer press/release drives CSS `:active` without changing the line-break algorithm. Next P0: hit-test according to actual painted stacking/clip hierarchy, cover replaced and atomic inline elements, bound/coalesce worker input under load, implement keyboard focus-visible, and expose synchronized DOM/JS focus and selector state. Independent modern-web readiness blockers: JS VM/event loop, missing bootstrap DOM, font loading and layout correctness. Current Release sites are not fully supported.

## CSS interaction state after native hover/focus support (19 October 2026)

Hover and mouse-focused CSS state now work through the existing original ID-block hit regions, nested selector matching, and native Win32 event dispatch, with transition-only repaint and coalesced queued motion. Next P0 is hit-testing for ordinary inline, ID-less and nested elements, correct pointer press/release `:active`, focus navigation with keyboard `:focus-visible` and API consistency with DOM scripting; improve low-latency style invalidation. Page bootstrap/CSS font/layout and full JS compatibility are independent open obstacles; no modern site is certified usable.


## Responsive CSS and first-paint state matching (19 October 2026)

Original viewport-aware screen `@media` handling and bounded common state pseudo-classes now pass tests and work through native reflow. Next: link actual hit-test/focus/disabled behavior into dynamic selector recalculation; support more media features and `@font-face` without unsafe assumptions; pursue native first-paint layout/scripting failures on YouTube and absent Reddit feed. CSS parser error count reductions do not certify visual compatibility. Compare screenshots to a trustworthy browser oracle before declaring any site supported.


## Real-site blank-viewport investigation (19 October 2026)

Native DOM census now shows two distinct failures: Reddit lacks connected body text, YouTube has CSS-suppressed body text with significant CSS and JS failures. The measured 32-script experiment was reversed due to YouTube errors and Pikabu timeout. Next P0: inspect permitted bootstrap/resource timing and missing first-paint DOM output on Reddit, then narrow YouTube's CSS and JS blockers with focused regression tests. Full JS VM resumability remains a separate important milestone, not a substitute for actual page pixels.


## Async control-flow and real-page diagnostic checkpoint (19 October 2026)

The original engine supports awaited if tests, branches that always return/throw, awaited template and throw values, and multi-declaration await initializers. Its improved bounded parser diagnostics identified an actual live Reddit blocker; Reddit now executes its initial script (1/0) but remains white. Next P0 is real suspend/resume through nonterminal if bodies, loops and try/finally with correct lexical scopes, followed by script/DOM/layout first-viewport repair and interactive CPU/wall profiling. Do not equate script execution with visual or interaction correctness.


## Return-branch await checkpoint (19 October 2026)

One-sided returning ternaries and returning short-circuit RHS expressions now suspend only on the taken branch. Remaining P0: awaits nested in conditions and arbitrary expression/statement positions, `if`/loops/`try/finally` with real resumable interpreter frames, and real-site script loading/layout blockers. Same five sites still show 2/4/11/1/3 JS failures; synthetic success does not count as a working browser page.


## Await continuation update (19 October 2026)

Object data properties and ternaries with direct awaits in both branches are now supported by the original Promise-microtask lowering. Next: real VM-frame resumability for skipped/one-sided logical and conditional branches, try/finally and loops; prioritize real-site content and profiling gates over synthetic-only success.


## Next native JavaScript continuation milestone (19 October 2026)

Expression-level `await` now supports nested eager operands, calls and arrays while maintaining evaluation order, receiver identity and GC safety. For real-world compatibility this still needs a general async frame/suspend mechanism covering `try/catch/finally`, loops, conditional and short-circuit evaluation, plus first-viewport JS/DOM/layout repair. The same five-site live diagnostic still reports 2/4/11/1/3 initial script failures and no verified functional first viewport. Narrow Test262 arithmetic subsets must not be presented as overall engine-readiness percentages.

OPBrowser aims to be an independent Windows browser with an original Rust web/JavaScript engine. The success criterion is **usable, stable real pages**, not the count of EventTarget methods.

**G3 timeout checkpoint (13 October 2026):** orphan AbortSignal.timeout deadlines no longer pin dead signals. An abort observer, pending fetch or reachable AbortSignal.any composite preserves an active timeout. Five focused regressions pass; the full 30-minute memory proof remains open.

**G3 detached-DOM checkpoint:** GC now releases orphan disconnected node wrappers, listeners and metadata while retaining live DOM view identity; 40 synthetic churn cycles pass. Long-run memory proof remains pending.

**G3 native DOM/fetch checkpoint (14 October 2026):** unused unwrapped child metadata is reclaimed; live querySelectorAll results retain native parent identity. Two new fetch-GC races pass. Full memory proof remains open.

**G3 budget-pressure checkpoint:** automatic idle GC now observes object and lexical-environment arena headroom as well as live allocations. Three regressions pass for low-budget and repeated allocation bursts; long-run RSS remains pending.

## P0 await microtasks and native pixels (19 October 2026)

Original async arrows and direct sequential awaits now execute via browser-owned Promise jobs. A pending fetch survives GC, and multiple awaits produce real native DOM text pixels. General expression and try/loop await suspension remain incomplete. Five real URLs show 2/4/11/1/3 JS failures, Reddit blank and YouTube textless. Next: full async frame continuations and real layout/script-loading integration.

## P0 async and visible-page checkpoint (19 October 2026)

Original JS supports ordinary `async function` syntax and Promise settlement **without `await`**. A repeat five-site test records 2/4/11/1/3 failed scripts for Google/YouTube/Pikabu/Reddit/DeviantArt, with Reddit still white and YouTube zero-text. Reddit advances to a genuine unsupported `await` barrier. Next: `await` continuations and microtasks, dynamic scripts and real first viewport pixels. Do not mistake an improved parser diagnostic for web functionality.

## Active P0 priority (16 October 2026)

Real-site usability takes precedence over synthetic-only optimization.
The original VM now advances past YouTube's `delete` syntax blocker,
yet YouTube still paints no text; Pikabu remains visually malformed.
The 30-minute synthetic GC safety/memory gate passed, with long
wall-time pause outliers still to diagnose using a real in-browser
profiler. See the active project plan for current gates.

## Image/real-site checkpoint (16 October 2026)

Browser-owned `new Image()` now uses real DOM nodes and reaches native
paint commands when inserted. YouTube's missing-`Image` error was
removed but it still renders a skeleton without text. Next priority:
the surviving real-site JS syntax/runtime errors and Pikabu's layout,
and interactive CPU/wall profiling rather than synthetic PASS inflation.

## Location and real-page checkpoint (16 October 2026)

The live page URL now reaches shared `window.location` and
`document.location` objects with readable HTTP(S) components and
explicitly rejected navigation writes. The Pikabu reference error
advances to a later runtime failure, but its 12 failed scripts and
broken layout remain; YouTube is still skeleton-only. Prioritize
correct syntax/DOM/script-loading and usable pixels next.

## JavaScript syntax and DOM-first-paint checkpoint (16 October 2026)

The independent VM now handles `??` and bounded synchronous
`for...of`, including real DOM query-result iteration and lexical
closures. New compiler and integration tests pass. The same five
public pages remain visibly broken, so these fixes are **not** a
website-compatibility milestone. Next diagnose YouTube's remaining
for-initializer and backslash syntax failures, Reddit's argument parse
error, and missing layout/paint paths while retaining GC memory guards.

## Real error-context and for-in follow-up (16 October 2026)

Local opt-in byte-context diagnostics now identify specific modern
syntax failures without shipping full site scripts. Bounded native
`for...in` own-key enumeration covers the exact YouTube configuration
loop and verified DOM mutations. YouTube's for initializer syntax
error is gone, but 9 failures and a textless first view remain. The
next verified blockers are regex literals, labeled statements,
unsupported `async function` syntax on Reddit and host API behavior.
Do not increase script budgets simply to hide errors.

## RegExp and comma-operator checkpoint (16 October 2026)

Bounded original RegExp objects and String matching now parse the
previous live YouTube backslash failures. The independent VM also
executes compound assignment and comma sequences with verified call
argument boundaries. A live native YouTube probe advances from 7/9 to
**8 executed / 8 failed** scripts; viewport still has zero text. Next
semantic blockers: object-literal getters/setters, labeled statements,
`async function`, host DOM APIs, dynamic script scheduling and
native paint/layout. The regex subsystem is intentionally partial,
with unsupported ECMAScript patterns rejected rather than faked.

## Native DOM checkpoint (14 October 2026)

Native Document now collects orphan node components and reuses slots with
generation-checked identities. Live detached nodes, views, queries and pending
callbacks preserve their native ancestors/descendants. Sweeps wait for parser
completion and validated JS idle checkpoints; weak image/stylesheet/scroll
cache keys are removed, and native allocation headroom can trigger collection.
Native pauses and arena live/reserved counters are measured separately.
The 30-minute Windows and broad navigation/network stability gates remain open.
See [GC stress results](../docs/GC_STRESS_RESULTS.md).

## Historical priorities (14 October 2026, superseded above)

1. **P0-1 NOW: original JavaScript GC only.** G1/G2/G3 partial now provides browser-owned automatic mark/sweep, generation-safe slot reuse, weak AbortSignal.any follower ownership and GC pause/failure telemetry. A repeatable Windows native-engine timer probe measured 817 successful collections in 12 seconds with zero GC errors and bounded object/environment arenas. **Not a 30-minute memory proof.** Next: sustained RSS/pause measurement and remaining fetch/DOM/abort root audits. See [GC implementation design](https://github.com/Gerbesh/OPBrowser/blob/main/docs/GC_IMPLEMENTATION.md) and [stress procedure](https://github.com/Gerbesh/OPBrowser/blob/main/docs/GC_STRESS_RESULTS.md).
2. **P0-2 AFTER GC: rendering and scrolling.** Existing vertical and bounded horizontal overflow, partial Flexbox and native GDI tests stay intact. Horizontal thumb dragging, JS scrollLeft, positioned clips, Grid and real-page visual comparison resume only after GC passes safety and stability gates.
3. **P1: backgrounds and fonts.** Improve background positioning/sizing/clipping, then DirectWrite fallback/shaping and real origin-checked web font loading.
4. **P1: forms and interaction.** Keyboard editing, focus, selection, form controls and submission with native end-to-end tests.
5. **P2: broader Web APIs and ECMAScript.** Continue when P0 visible/memory gates pass, rather than spending another series of milestones on narrowly chosen Events assertions.

## Evidence and limits

Frozen WPT Static 197/200 exact (only 200 selected reftests), Positioning 54/100, Test262 Runtime v2 78/179 attempted with 110 explicitly skipped, and M4.35 WPT DOM/Events v8 21/21 attempted with 3 skips. These **do not combine into a website readiness percentage**. We still need a real-world visual/site corpus and long-session memory measurements.

See [active plan](https://github.com/Gerbesh/OPBrowser/blob/main/docs/PROJECT_PLAN.md), [Compatibility](https://github.com/Gerbesh/OPBrowser/blob/main/docs/COMPATIBILITY.md), and [Current Status](Current-Status.md) for verified status. Individual implementation updates belong in DEV_LOG and Git history, not the active plan.
