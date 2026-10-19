# OPBrowser: active project plan

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
