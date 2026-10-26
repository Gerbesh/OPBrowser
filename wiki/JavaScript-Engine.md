# JavaScript Engine

## globalThis and native console logging (26 October 2026)

globalThis now points to the real first-party Window object, with classic identifier lookup and local parameter shadowing. The original JavaScript VM now implements console.log/info/warn/error/debug as non-constructible methods whose output is retained as up to 64 bounded, drainable page-local records, 2048 UTF-8 bytes each. Web script messages are not printed to stderr or sent to a telemetry service. There is no existing DevTools console UI or complete object-inspection/formatting support.

The original Release 20-site before/after samples improved Notion initial script failures 6 to 5 and removed MDN's missing-console exception. Pikabu remains 16 executed/0 failed/18 skipped. Modern pages are not visually/functionally certified, and YouTube/Reddit still have major rendering gaps.

## Native XMLHttpRequest GET, Window global properties and window messages (26 October 2026)

First-party JavaScript XMLHttpRequest provides an asynchronous same-origin GET subset backed by the existing original op_net request queue, not a mocked HTTP response. Supports open/send/setRequestHeader/getResponseHeader, real HTTP status/statusText/responseURL/responseText, limited text/JSON response types, readystatechange/load/error/abort/loadend callbacks, GC-safe pending closures, and document-relative URLs through native networking. Sync requests, POST, bodies, timeouts, credentials, full header enumeration and real streaming remain unsupported. A native HTTP 404 integration test verifies real headers and response body rendered into the HTML document.

Classic script identifiers resolve new properties on window (and explicit top-level var bindings mirror to Window), fixing `window.s = element; s.type = ...`. DOM Node.parentElement now returns the actual parsed or staged parent element. Single top-level window aliases `parent`/`top` and provides queued postMessage to itself with default same-origin target, specific-origin comparison and bounded JSON-safe event data. There is no iframe communication or complete structured-clone algorithm; do not describe this as a full postMessage implementation.

Native Pikabu now runs 16 initial scripts without JS errors (18 skipped, rendered_unverified), but 5-site rerun has other incomplete sites and no visual conformance acceptance. Larger script budget exposed seven disallowed external script URLs; conservative existing policy retained pending explicit third-party script security/privacy design.

## First-party Date and Array/String.concat compatibility (26 October 2026)

OPBrowser now implements Date.now, new Date with current/system milliseconds, epoch inputs, strict ISO UTC strings and bounded field arguments, Date.parse/UTC/getTime/valueOf/toISOString/toUTCString and UTC getters, and native Date.setMilliseconds/setUTCMilliseconds. It uses its own Rust Gregorian calendar math and GC-traced Date object state, without third-party JS. ISO/timezone-free inputs and local setters are UTC approximations until actual local-timezone semantics are developed; incomplete Date methods/locale formatting must not be presented as standards-complete. Arrays and strings now have native concat with bounded memory and array-hole preservation, but not all JavaScript exotic objects/species. Focused real Pikabu progressed from missing Date to missing setMilliseconds, then concat, then XMLHttpRequest; script result stays 10 executed/6 failed. Next architecture goal is genuine XHR based on original network and host event loop.

## Browser DOM head, Element.append and in-memory document.cookie (26 October 2026)

Real public Pikabu execution exposed missing `document.head`, an absent `Element.append()` method, and then missing `document.cookie`. The first-party native JS runtime now binds actual parser-built `<head>` to document.head, implements append with native DOM Node/text creation and staged DOM append operations, and gives document.cookie an initially empty value with bounded script-local name/value assignments. This is deliberately not a persistent/network cookie jar; the network loader does not send Cookie or retain Set-Cookie. New opt-in diagnostics report only bounded sanitized names of recent nullish member reads and method returns and warn that correlation is not proof of causation. Real Pikabu execution reached 10 successful and 6 failed initial scripts, then exposed the missing Date global. Full cookie RFC semantics and complete DOM compatibility remain pending.

## Live-site-driven spread, destructuring, bind and forEach (26 October 2026)

From a new original-engine Pikabu live diagnostic, OPBrowser's own lexer/parser/VM now handle object `...source` spread of owned enumerable fields and simple identifier array destructuring (`var [a,b] = array`, including classic for initializers), with bounded source sizes. Browser `self` is the same window object. First-party Function.prototype.bind captures this/partial arguments, supports normal/constructor calls, and keeps those references alive through original GC. Array.prototype.forEach invokes native JS callbacks on present array entries with index/array arguments and optional thisArg. Pikabu initial script stats improved 7 executed / 9 failed to 8 / 8, but the page remains incomplete.

Limits: spread copying is delayed until object construction and is not fully correct when later object-literal expressions mutate spread sources; no complete iterator/nested/default/rest destructuring, generic Array.forEach, global Function constructor, or complete modern-site support. The JS call depth limit is reduced from 64 to 32 after a reproducible Windows stack overflow test, and the full workspace regression passes with that bound.

## Real-site driven function defaults and Array.includes (26 October 2026)

An anonymous full 20-site native browser sweep exposed specific Pikabu JavaScript failures. The original parser now accepts basic named function parameters with per-call default expressions, including `function loadAsyncScript(key, url, attrs = {})`, lowering them into guarded assignments only when the argument is undefined. The original VM now exposes `Array.prototype.includes` on actual arrays with SameValueZero comparisons, handling NaN, holes, and positive/negative fromIndex. Targeted unit regressions pass, and in a focused Release Pikabu sample initial scripts executed rose 5 to 7 while failures fell 11 to 9. The page still does not pass visually or functionally. Destructuring/default-arrow syntax, precise default-parameter Function.length, generic array-like includes and object spread remain incomplete.

## Native Windows Unicode copy, cut and paste (23 October 2026)

The first-party HTML input/textarea editor now supports Ctrl+C, Ctrl+X and Ctrl+V without platform text-widget delegation. Keydown listeners run through the original JS runtime first, including preventDefault cancellation; selected ranges are mapped from JS UTF-16 offsets to Unicode scalars. Windows CF_UNICODETEXT transfers only text; native cut/paste route through existing beforeinput/input DOM mutation and selection synchronization. Passwords cannot be copied and readonly fields remain copy-only. HTML ClipboardEvents, custom clipboard MIME types, IME and undo/redo remain unsupported.

## Live mouse-drag selections in HTML form fields (22 October 2026)

The original Windows browser now tracks text-like input and textarea selections from captured mouse press/move/release without Win32 EDIT or external JavaScript engines. A scalar-safe anchor and caret are converted into JavaScript-visible UTF-16 selectionStart, selectionEnd and backward/forward selectionDirection; existing native selected-glyph paint updates during movement. Moving outside the field clamps to the first/last line. Releasing an actual drag does not dispatch a spurious DOM click. Explicit textarea lines and password glyph masking share the existing approximate text geometry; accurate wrapping, bidi/grapheme shaping, clipboard and IME are not implemented.

## Standalone editable form visibility (21 October 2026)

A body containing only an input or textarea (or such a control with a neighboring script) now renders its text, mouse hit area, JavaScript-selected highlights and caret. The original layout's margin-collapse detection no longer discards ancestors of real text controls. Hidden/nonpainted inputs remain invisible. This is not yet full replaced-element/textarea geometry conformance.

## Visual selection in editable HTML fields (20 October 2026)

JavaScript setSelectionRange() and first-party keyboard selections in focused input/textarea controls now produce a clipped blue selection highlight with selected glyphs repainted in white. The renderer uses Unicode scalar boundaries measured with its own font measurer, including masked password characters and explicit textarea newlines. Editing the selection collapses the highlight back to a caret. This is not drag selection, a full contenteditable implementation, or pixel-perfect shaping/soft wrapping.

## HTML form click-to-caret (20 October 2026)

Native mouse clicks on text-like input and textarea controls now position the caret using the original font measurer, without Win32 EDIT/WebView. The original JS runtime sees the resulting UTF-16 selectionStart/End before focus and click listeners run, and subsequent input inserts at the chosen position. Explicit textarea newline rows and password masking are supported; drag selection, complete wrapping/RTL/glyph shaping and drag selection are not.

## HTML text selection interoperation (20 October 2026)

Applicable first-party text input and textarea proxies support selectionStart, selectionEnd, selectionDirection and setSelectionRange(). Script coordinates are UTF-16 offsets and map into safe Unicode scalar positions in OPBrowser's native text editor; subsequent typing, replacement and input events synchronize back to the script. Non-text input types expose null selection getters. Drag selection, complete click/drag semantics, IME and complete selection standard conformance are not yet implemented.

## Form value reflection and beforeinput/input DOM events (20 October 2026)

Original element proxies now expose bounded input/textarea `.value` getters/setters; native edit commits synchronize source DOM attributes and VM state. Original EventTarget dispatch delivers cancelable bubbling `beforeinput` and noncancelable bubbling `input`. A keydown listener's preventDefault prevents subsequent Windows WM_CHAR insertion. This is native DOM event functionality, not a claimed change in overall ECMAScript conformance. Missing complete InputEvent.data/inputType, visual and full-range selection behavior, composition, change/submit, constraint validation and form-specific default actions.

## JavaScript Element.focus()/blur() lifecycle (20 October 2026)

Native DOM element proxies now expose real `focus()`/`blur()` methods, handled by `DomFocus/DomBlur` builtins in the original VM rather than a WebView. The methods validate basic connected HTML focusability, synchronize `document.activeElement`, dispatch noncancelable blur/focusout and focus/focusin events, and hand the new focused DOM node to the Rust engine at the next paint boundary for matching CSS `:focus`. Repeated focus and non-active blur are no-ops; nested listener focus transitions have a recursion budget. The original engine tests verify CSS color and document.activeElement from scripts and inside click listeners. Modern ECMAScript/Test262 conformance remains only partially measured: frozen parser subset 523/1,983 (~26.4%) and runtime subset 78/179 attempted (~43.6%, 110 skipped), neither is an overall JS percentage. Focus eligibility and relatedTarget/scroll-to-focus semantics remain partial.

## Original KeyboardEvent delivery and read-only activeElement (20 October 2026)

Native page events now use original EventTarget dispatch for trusted cancelable/bubbling keydown/keyup with key, code, ctrlKey, shiftKey and repeat; listener once/passive, preventDefault and capture/bubble rules share the existing runtime. Host-driven activeElement changes update the original document property before focus/blur handlers, and the initial/no-focus reference is body. Native mouse click focus transitions use the same blur/focusout/focus/focusin JS path as Tab, before the click. Keyboard changes made by JavaScript update the retained DOM before default handling. An un-canceled Tab performs traversal and scroll-into-view; Enter activates through the existing cancelable click/default-link pipeline. Programmatic element.focus/blur, relatedTarget, focused-node detach, alt/meta modifiers and advanced editing events remain incomplete.

## Native focus DOM events (20 October 2026)

The original JS EventTarget dispatcher now handles trusted noncancelable `focus`/`blur` (capture and target only), and `focusin`/`focusout` (capture, target and bubble) from native Tab transitions. Each old/new focus sequence is delivered through bounded original DOM ancestry; original once/passive listener registrations are reused, and mutations from handlers apply before the browser restyles/repaints. This extends the existing click/default-action dispatcher rather than adding another VM or event system. Mouse click focus changes, relatedTarget, document.activeElement synchronization, keyboard keydown/keyup and form default actions are still open.

## Cancelable native browser click default actions (20 October 2026)

Native link clicks now execute the existing original JS capture/target/bubble dispatch before navigation. `JsRuntime::dispatch_dom_click_path_with_result` returns handler-delivery and `defaultPrevented` flags; the old bool API remains compatible. Target, capturing-ancestor and `onclick` listeners can cancel the default action via `preventDefault()` or legacy `event.returnValue=false`; an `onclick` property callback returning exactly false also cancels it. Passive handlers cannot cancel, and `addEventListener` return values are ignored. Engine mutations are committed before deciding whether to navigate; dispatch VM failure cancels default activation conservatively. Programmatic `Element.click()` still needs its own default-action pipeline, and script-mutated href/removed nodes and keyboard semantics remain follow-ups.

## Cross-layer JS/bootstrap diagnosis (19 October 2026)

The new `Engine::active_dom_content_diagnostics` is read-only and emits aggregate numbers, separating parsed DOM content, suppressed text, script sources and disconnected nodes. It confirmed that Reddit's now-running inline script creates no visible body text, while YouTube contains CSS-hidden text and failed scripts. Increasing the maximum accepted scripts from 16 to 32 merely increased YouTube failures (4→7) and timed out Pikabu; the change was rolled back. Prioritize genuine bootstrap, Web APIs, DOM and CSS handling over unbounded script execution.


## Guarded awaited branches, template/throw and multi-variable declarations (19 October 2026)

The original JS parser and Promise CPS pipeline now handle `if (await test)`, nested if arms that guarantee return/throw, `throw await p`, awaited template substitutions and ordered `const a=before(), b=await pending, c=after()`. The parser preserves the byte position of the unsupported await and categorizes its statement kind. GC/VM/native-pixel regressions are green. A live Reddit script previously rejected on an awaited variable initializer now executes (1 executed / 0 failed), but the actual page stays blank and produces no DOM mutations. Nonterminal if branches, loops, try/finally and full ECMAScript scheduling remain unsupported.


## Branch-aware direct return awaits (19 October 2026)

The browser-owned async lowerer now recognizes `return flag ? await a() : b` and `return value &&/||/?? await expression` with one direct await. It compiles conditional return paths so the unselected path never awaits or runs its operand. `??` distinguishes only null/undefined, while ordinary `&&` and `||` preserve operand values. GC, VM scheduling and native HTML-to-paint regressions pass; non-returning branches and general `if`/loop/`try/finally` awaits still require a full continuation mechanism.


## Async object fields and two-arm ternary (19 October 2026)

`{x:before(), y:await pending, z:after()}` now resumes ordinary data fields in their original evaluation order. `flag ? await a() : await b()` evaluates only one branch. Regression tests cover pending GC and native HTML/DOM/paint. Object accessors/prototype setters and arbitrary await in loops, one-sided branches, logical RHS and try/finally are unsupported.


## Original expression-level await lowering (19 October 2026)

New `await_lowering.rs` decomposes the first awaited eager subexpression, captures operands evaluated before suspension and schedules its remainder through browser-owned Promise microtasks. Nested await arguments, arithmetic, arrays, sequences, constructor arguments and simple assignments are covered. The compiler-only `CallWithReceiver` expression emits a native VM call retaining the original receiver and function, rather than relying on mutable `Function.prototype.call` or function-owned `.call`. GC-safe pending continuation and real DOM-paint regression tests pass. Await in conditionally executed branches, short-circuit right operands, loops and `try/finally` remains deliberately rejected. This is bounded CPS lowering, not fully general async VM-frame suspension.

`op_js` is OPBrowser's original ECMAScript implementation. It does not embed V8,
SpiderMonkey, JavaScriptCore, QuickJS or another JavaScript engine.

## Sequential await and async arrow subset (19 October 2026)

The parser accepts `async x => expression`, `async (x,y) => expression`, async arrow blocks and direct awaits. Simple `const/let/var x = await value`, `await value;`, and `return await value` become Promise.resolve/.then continuations, suspended on pending Promises. Original GC retains closures until network/Promise settlement. Lexical arrow this/arguments and rejection are regression-tested. Nested expressions, loops/try and complex control-flow awaits still reject rather than pretending to run synchronously; this is not a full suspended-stack VM. Reddit's new blocker is an unsupported complex await after the async-arrow parser barrier.

## Bounded async functions and Promise settlement (19 October 2026)

The original parser interprets contextual `async` preceding `function`. Declarations and expressions compile to original closure bytecode flagged as async. Without `await`, calls return real self-hosted Promises, adopt returned Promises and reject thrown JS values or VM runtime errors. Async functions have no constructible prototype. Real `await` suspension, async arrows and complete ECMAScript async scheduling remain unsupported: the parser rejects `await` in async bodies rather than synchronously faking it. Two focused parser/VM tests pass. Reddit's diagnostic advances to this `await` blocker; live rendering remains unchanged.

## Namespace-aware DOM creation and YouTube diagnostics (17 October 2026)

The original DOM and JS host now support
`document.createElementNS(namespaceURI, qualifiedName)` with
separate namespace metadata, preserved synthetic/native node identity,
XML prefix validation and `namespaceURI`, `localName`, `prefix`,
`tagName` and `nodeName` getters. HTML nodes retain the XHTML
namespace. This is a bounded ASCII qualified-name subset, not an
SVG rasterizer or a full foreign-content HTML parser.

Opt-in `OPBROWSER_JS_ERROR_CONTEXT=1` now also adds sanitized
property names to null/undefined dereference diagnostics. YouTube's
`createElementNS` failure disappears, but four script failures
remain: `set`, `responseStart`, `prototype` twice. YouTube
still has no painted text, and Reddit remains blank.

## Property descriptors and Object.defineProperty (17 October 2026)

The own JavaScript VM now supports `Object.defineProperty`
for original ordinary objects, arrays (excluding redefining
`length`) and functions, recording `writable`,
`enumerable` and `configurable` only where needed.
The VM honors these flags during assignment, deletion,
`for...in` and JSON serialization and reflects them through
`Object.getOwnPropertyDescriptor`. String and symbol data
properties plus basic string-key accessors are tested.
Unsupported host descriptors, symbol accessors, full prototype
write rules and array-length descriptor changes remain open.

YouTube's missing defineProperty call now progresses to
later errors: the release probe falls from **5 to 4** failed
scripts but YouTube still paints no text.

## Native callable diagnostics and YouTube JS methods (17 October 2026)

With `OPBROWSER_JS_ERROR_CONTEXT=1`, the original VM
records at most 16 recent member reads to identify a failed
method call without storing entire public site scripts.
Real YouTube identified missing `Array.shift`,
`Object.create`, `Object.getOwnPropertyDescriptor`
and `document.createElementNS`.

The original VM now supports `Array.shift/unshift` with
sparse-hole handling; `Object.create(object|null)` and
partial `Object.getOwnPropertyDescriptor` on own
data/accessor cells. Full descriptor attributes,
Object.defineProperty and DOM namespace creation remain
future work; unsupported Object.create property descriptors
reject instead of silently pretending to work. The same
real-site probe improves YouTube initial script errors
from **8 to 5**, but still draws no text.

## Original Symbol primitive (17 October 2026)

OPBrowser's own JS VM now supports genuine unique `Symbol()`
primitives, `typeof === 'symbol'`, `Symbol.for()` and
`Symbol.keyFor()`, and selected well-known symbols. Symbols
are not strings: `obj[Symbol('x')]` uses a separate property
map, can be inherited/deleted, and is ignored by `for...in`
and JSON object serialization. These property values are
GC-traced. Invalid implicit numeric conversions fail rather
than silently producing NaN. VM and native HTML-to-paint
regressions pass.

This is a subset: computed symbol-key fields in object literals,
own-symbol reflection APIs, protocol-driven iteration,
`Symbol.toPrimitive` and full ECMAScript descriptors remain
incomplete. YouTube's missing Symbol global has been removed,
but eight scripts still fail and no text is painted.

## Navigator feature detection and String search (17 October 2026)

Each document gets a minimal `navigator` also available at
`window.navigator`. It exposes OPBrowser's own honest name
and User-Agent, a temporary `en-US` language fallback,
`languages`, `platform`, bounded `hardwareConcurrency`,
and `cookieEnabled=false`. Unsupported geolocation,
serviceWorker, beacon and UA-Client-Hints features remain
absent, not stubbed as working. This does **not** yet read
the Windows UI locale.

The original JS VM adds `String.prototype.indexOf` and
`includes`, with UTF-16 indexes and bounded search.
`Function.prototype.call(null)` now preserves null for
native builtins but substitutes the global object for
non-strict user-defined functions. Native DOM-to-paint tests
pass. YouTube still lacks `Symbol` and renders zero text.

## Math and RegExp globals (16 October 2026)

Original `Math` supplies 17 numeric functions (including
`floor`, `pow`, `max`, `round`, `random`) and standard
constants. `Math.random` is non-cryptographic and in [0,1).
`RegExp()`, `new RegExp()` and regular-expression literals use
the same bounded Rust regex subset; methods, flags, prototypes
and `instanceof` have tests. Unsupported regex syntax remains
explicitly unsupported. Live YouTube has moved beyond missing
Math/RegExp but remains zero-text and incomplete.

## YouTube parser and 32-bit operators (16 October 2026)

The original interpreter now supports full-expression right-hand
operands in `for...in`, including a comma operator after an
assignment, while `for...of` retains its narrower RHS grammar.
Supported numeric operators now include `*=`, `/=`, `%=`,
`<<`, `>>`, `>>>` and their compound assignment forms. Shift
counts use the low five bits and unsigned right shifts produce a
nonnegative uint32 result. Member assignments evaluate the receiver
and property once. VM and HTML-to-native-text regressions pass.

Native YouTube's three previously observed syntax rejections
progressed to later missing-global errors (`RegExp`, `Math`).
It still has eight failed scripts and no painted text. Native
rendering, async JS, and broad Web API coverage remain incomplete.

## Labeled statements and named break/continue (16 October 2026)

The original parser/bytecode/VM support statement labels and
`break label`/`continue label`, including nested labeled loops,
`for...of`, `for...in`, `switch`, and labeled blocks. Named exits
can pass through nested try/finally with ordered finalizer execution
and correct scope unwinding. Invalid labels and cross-function jumps
are rejected. A deterministic HTML-to-pixels regression passes.
Live YouTube still has eight script failures and zero text primitives:
the previous labeled-try error is now followed by a later syntax error.
This does not establish real-site compatibility.

## Original getters, setters and concise methods (16 October 2026)

Object literals now execute `get x(){}`, `set x(v){}`, and
`method(){}` through original JS bytecode and GC-traced closures.
Prototype receiver binding and ordinary `get` fields are tested.
Exact thrown-value propagation, computed keys and descriptor APIs
remain incomplete. Five real-site diagnostics improved Pikabu and
DeviantArt error counts, but YouTube still paints no text.

## Basic arrow functions (15 October 2026)

Original Rust lexer/parser/VM support `x => x + 1`,
`(a, b) => a + b`, nested closures and block-bodied arrows.
Arrows use lexical `this` and `arguments`, have no own
`prototype`, and throw when used with `new`. A regression verifies
that a captured value survives VM garbage collection. Async arrows
and default/rest/destructured parameters are not implemented.
A repeat live probe showed script error counts down from 12 to
11 for YouTube and from 14 to 12 for Pikabu, but the pages
remain skeleton-only and visibly malformed. Not a compatibility pass.

## Untagged template literal support (15 October 2026)

The independent Rust lexer, parser, and bytecode interpreter now
support multiline backtick strings, nested `${expression}` interpolation,
common cooked escapes, and left-to-right value conversion. There are
explicit nesting/substitution budgets and syntax tests for malformed
literals. Tagged templates and arbitrary UTF-16 surrogate code units
are not yet implemented. In the live DeviantArt probe, the former
unsupported backtick error disappeared but another syntax failure
followed, with four failed scripts still recorded. Google, YouTube,
Pikabu and Reddit also remain incomplete.

## Page performance clock (15 October 2026)

The original per-document VM exposes `window.performance.timeOrigin`
(Unix epoch milliseconds) and `window.performance.now()` (elapsed
monotonic milliseconds). The method rejects incorrect receivers and
construction. A live DeviantArt recheck no longer reports the previously
observed missing `performance` global, but still contains four script
failures and incomplete rendering. Full Performance APIs and the
interactive CPU/wall-time browser profiler remain open.

## Current real-site blocker audit (15 October 2026)

`Engine::active_script_diagnostics` exposes bounded, per-document
initial JS failure types and script-skip reasons. Live compatibility
runs categorize these findings rather than only adding up script
failure totals. Google and Pikabu exposed missing bitwise operators;
the original lexer/parser/VM now implement JS `&`, `|`, `^` and
`~` with correct ToInt32/signed conversion and precedence. Those
specific parse blockers no longer appear in the follow-up, but
Google/Pikabu still have other JS errors and broken layouts.
This is an implementation step, not a full-web compatibility claim.
See [real-site priorities](../docs/REAL_WEB_BASELINE.md).

## Real-world JS compatibility baseline (15 October 2026)

The 20-site anonymous HTTPS test exercises original OPBrowser navigation,
JS execution, DOM updates and Windows rendering, not synthetic browser
workloads. None is verified fully functional: ten render partial content
with script errors, two show only uninitialized placeholders, three are
blank, three are HTTP-blocked, one renders an unverified broken layout,
and one times out. These measured failures should drive next VM/DOM
priorities ahead of nonblocking EventTarget conformance work.
See [real-web report](../docs/REAL_WEB_BASELINE.md) and
[dated screenshots](../docs/live_web/2026-10-15/REPORT.md).
The probe does not replace an interactive CPU/wall-time profiler.

## Current GC status (15 October 2026, G3 partial)

Native Document now collects orphan node components and reuses slots with
generation-checked identities. Live detached nodes, views, queries and pending
callbacks preserve their native ancestors/descendants. Sweeps wait for parser
completion and validated JS idle checkpoints; weak image/stylesheet/scroll
cache keys are removed, and native allocation headroom can trigger collection.
Native pauses and arena live/reserved counters are measured separately.
Two 30-minute Windows Release synthetic DOM runs passed their correctness
gate. The latest completed 83,777 JS/native sweeps, reclaimed 335,126
native nodes and ended with 19 reserved DOM slots and zero failures.
Private Commit stayed near 2.4 MB after warmup, with a +69,632-byte
change between the early and late medians, not a zero-leak proof.
The interactive pause-tail gate is not passed: six checkpoints exceeded
16 ms, two exceeded 50 ms, and the worst took 448.18 ms. OS scheduling
versus collector CPU time remains undiagnosed.
This does not validate real pages, network/navigation or multi-tab GC.
Broad real-navigation, network and multi-tab stability gates remain open.
See [GC stress results](../docs/GC_STRESS_RESULTS.md).

Long-session diagnostics now include Windows private committed bytes
(`PROCESS_MEMORY_COUNTERS_EX.PrivateUsage`) alongside the working set.
A standalone CSV analyzer checks per-second counter consistency and
reports warmup-filtered memory medians and slopes. This distinction matters:
Windows can trim the working set without the collector freeing anything.
A loopback HTTP regression also confirms a delayed response from an old
page cannot settle the next page's `fetch`, even when request IDs match.

The page host also measures the combined stop-the-world checkpoint:
JS collection, native DOM reclaim, VM native metadata pruning and page
image/stylesheet/scroll-cache cleanup. A separate Engine API and stress
probe export its last/worst wall pause and four-phase breakdown; parser
sweeps are not counted as event-loop checkpoints. Navigation generations
reject stale network responses even if a later page reuses the request ID.
These results do not yet establish full real-network/multi-tab stability.

The JS GC now attributes the worst automatic wall pause to root marking,
weak side-table cleanup and arena sweeping, with >=16 ms and >=50 ms
outlier counts in `JsGcTelemetry` and the stress probe. Such measurements
cannot alone establish CPU cost versus Windows thread preemption.
A composite `AbortSignal.any` owning a pending fetch survives 16
intervening sweeps; abort frees its callback and rejects a late response.

Native DOM binding now preserves detachment instead of re-rooting orphan
wrappers. Element/text creation recovers its bounded staging quota after
commit or collection; synthetic IDs are never reused. Native subtree links
are canonical after binding, so retained detached children remain editable.

Automatic GC now records every successful pause in a fixed-size histogram,
exporting p50/p95/p99 bucket upper bounds. The timer and detached-DOM probe
profiles also report authoritative native DOM slots. Native slots now recycle safely; short profiles verify bounded arenas but
do not establish the complete long-session gate. See
[GC stress results](../docs/GC_STRESS_RESULTS.md) for reproduction and limits.

The VM uses generation-safe mark/sweep at idle browser checkpoints. Live
EventTargets preserve their callbacks and listener AbortSignals, while an
orphan AbortSignal's listener metadata no longer roots its entire cycle.
AbortSignal.any follower metadata also has weak lifetime ownership.
Timeout deadlines are weak unless an abort handler is registered; a reachable
global, pending fetch or composite signal also keeps its deadline valid.
Orphan deadlines are pruned after successful GC and their fixed 64-slot
pending budget becomes reusable. Live listeners still receive TimeoutError.
Connected DOM nodes remain roots; detached node wrappers and their event
listener cycles are reclaimable when unreachable. Live retained nodes and
DOM collection/style/classList handles keep identities across sweeps.
Successful collection also prunes orphan native snapshot metadata.
GC also purges never-wrapped native descendants of dead detached subtrees.
Retained static query results preserve their backing native parent identity.
Fetch completion and cancellation have GC-root regressions: native callbacks
survive pending requests, while late replies after abort are ignored.
Automatic idle GC now also reacts to object/environment arena headroom;
near-budget allocation bursts can recycle slots even when the adaptive
live-object trigger has not been reached. Three VM regressions cover this.
Other DOM/fetch host roots and a 30-minute memory plateau remain unverified.
Older milestones below describe their historical state.

The executable core contains an owned lexer, AST parser, bytecode compiler and stack
interpreter. The current language subset covers scalar literals, `let`/`const`/`var`
declarations including comma-separated declarators, identifier load/assignment, unary
`+`/`-`/`!`, prefix/postfix `++`/`--`, arithmetic, comparisons, loose/strict equality,
string concatenation, short-circuit `&&`/`||`, blocks, `if/else`, `while`, C-style `for`,
`do/while`, `switch` fallthrough, `break` and `continue`.
It also supports object and array literals, shorthand data properties, dot/computed member access,
member assignment, sparse array slots, dynamic index-driven array length growth and reference
identity through runtime-owned `ObjectId` handles.

Control flow compiles to patched bytecode jumps and the VM runs with an explicit instruction
pointer. Every execution has an instruction budget, plus bounded object/environment allocation and
call depth, so runaway loops/recursion terminate instead of monopolizing the renderer thread.
Bindings live in an environment arena: scripts use the global environment, calls create function
environments, blocks create lexical environments, let/const stay block-scoped and var targets the
nearest function/global environment.

Objects live in a bounded runtime heap instead of being copied inside `JsValue`. Ordinary object
lookups walk an explicit prototype chain; object-literal `__proto__` setters and later
`__proto__` assignments can change that chain while cycle creation is rejected. Arrays currently
reuse the same property store with indexed keys and an own `length` property. String `.length`
uses UTF-16 code units.

Functions are heap objects backed by owned bytecode templates. Function declarations and
expressions accept positional parameters, return values, recurse and expose initial `name` and
`length` properties. Each function captures its creation environment, so closures can read and
mutate bindings after their defining function or block has exited; named function expressions get
a private recursive self-binding. Direct function declarations are instantiated before the other
statements in their compiled statement list, providing initial declaration hoisting.

Explicit thrown values use a distinct abrupt-completion path and can cross function calls into
`catch`. `try/catch/finally` preserves or overrides return/throw/break/continue according to the
finalizer completion, including break/continue that leave the try region and resume an outer
loop/switch. Runtime-generated Reference/Type failures are still engine errors rather than
catchable JavaScript Error objects.

Method calls now preserve their receiver, bare non-strict calls receive the runtime global object
as `this`, and every user function receives an array-like `arguments` object. User functions own
a prototype object with a constructor backlink; `new` allocates from `constructor.prototype`,
binds the new receiver as `this`, and honors object-return versus primitive-return constructor
rules. `Error`, `TypeError` and `ReferenceError` are initial built-in constructors with
prototype chains, name/message properties and catchable objects. Runtime Type/Reference failures
inside try regions are converted to those JavaScript objects; execution-limit failures deliberately
remain engine-level guards.

## G1.0: traced reachability without memory reclamation

JsRuntime::trace_heap_reachability now conservatively marks
reachable object and lexical-environment slots from globals,
prototypes, JS function closures, retained DOM/Events references,
timers, microtasks and request callbacks. It detects unreachable
cyclic allocations without freeing anything, handles cycles
iteratively and refuses active event-dispatch checkpoints.
Engine::active_script_heap_reachability exposes the report for
an idle scripted document. Unit/integration tests pass.

Still **no real GC**: no object slot release, environment sweep,
generation-safe handle reuse, automatic collection, or long
session memory plateau. Native recursive interpreter frames
need explicit root registration before automatic collection
can be safe.

## M4.35: pre-dispatch events fixed, GC is outstanding

Original new/legacy Events preserve stopPropagation and
cancelBubble state already set when dispatch begins. The
flag clears on dispatch completion. Event phase constants and
document.documentElement allow further original upstream WPT
validation: 21/21 attempted v8 fixtures PASS, 3 SKIP.

The VM still stores every JsObject and captured Environment
in append-only vectors for the page lifetime. heap_usage()
now exposes allocation slot counts and budgets, including
unreachable slots. It is diagnostic only, **not GC**.
G1 trace roots, G2 reclaim and G3 bounded collection are
explicit required tasks ahead of more minor Web APIs.

## M4.34: WPT-guided Function.call and Event cleanup

Original function objects now inherit a shared Function prototype with Function.prototype.call. call_value binds the supplied this and forwards arguments, enabling genuine upstream WPT test contexts. Event.srcElement aliases target. initEvent clears propagation-stop flags, and dispatch completion clears them while preserving defaultPrevented and target. The v7 original WPT selection passed 16/16 attempted files, with 3 SKIP (39 original callbacks) at verified pinned SHA. It is not broad WPT compliance.

## M4.33a-b: original EventTarget and object callbacks

OPBrowser now allocates new EventTarget() instances through the
original Rust VM, with a shared EventTarget.prototype and inherited
dispatch/add/remove methods. Existing Window, Document, Element
and AbortSignal host objects also inherit from EventTarget.
Dispatch uses existing Event path, listener ID, passive/once/
AbortSignal and exception-isolation primitives. A standalone
event emitter's active composedPath has one member, its target.

EventListener callback objects with handleEvent are supported:
the method is resolved at callback time and executed with the
listener object as this. Dedup, removal and AbortSignal cleanup use
listener object identity. Function callbacks still receive
currentTarget as this. Null/undefined callbacks are accepted as
no-ops, and bad handleEvent methods yield bounded diagnostics.
Original JS and native repaint regressions cover these cases.
This is a bounded EventTarget subset, not full DOM WebIDL or WPT.

## M4.32b: native isTrusted and EventInit.composed

The original page engine dispatches actual hit-tested clicks via
dispatch_dom_click_path_with_trust(...,true), while the script
Element.click() builtin passes false. JavaScript listeners observe
the distinction through event.isTrusted, without changing the
capture-target-bubble route or original op_dom/op_paint replay.
Native click events carry composed=true. new Event() parses the
EventInit.composed dictionary member alongside bubbles/cancelable;
legacy Events default to composed=false. Browser-origin lifecycle
and abort events are marked trusted and non-composed, while
script-created events remain untrusted.

The current original VM treats common Event fields as read-only
under ordinary non-strict property assignments, so scripts cannot
forge target, phase, isTrusted or defaultPrevented via assignment.
This is not full WebIDL property descriptor behavior, trusted
default activation or Shadow DOM composition.

## M4.32a: event path lifetime and legacy cancelBubble

The owned Event objects (constructed, legacy and host-driven) support
composedPath(). During a dispatch the runtime retains an ObjectId
path for Element capture/target/bubble, including real connected
ancestors and document/window, or a global-target path for document,
window and AbortSignal. The method builds an original JS Array; after
dispatch the path is deleted and composedPath() returns [].
Stop propagation and immediate-stop semantics remain unchanged.

The legacy cancelBubble getter/setter reads the actual stop flag;
setting true stops subsequent ancestor delivery but false does not
undo a stop. The native click event also uses the existing
returnValue=false cancellation handling. VM and native text-paint
tests exercise these paths, including real HTML/body ancestry.
This is not an implementation of Shadow DOM composed paths,
retargeting or all WHATWG Event WebIDL descriptors.

## M4.31: registration identity and ordered EventTarget delivery

Every original VM Element, Document, Window or AbortSignal callback
registration obtains a monotonic token. Dispatch snapshots retain
callback identity *and* registration token; a remove/re-add of the
same function within a running dispatch creates a different record,
which cannot be called from the old snapshot. EventListenerOptions
once/passive/signal and dynamically bound node references preserve
their existing semantics.

Element.onclick now joins addEventListener target callbacks in
registration order. The native document.onreadystatechange,
window.onload and AbortSignal.onabort property handlers also take
their corresponding position in the event target's registration
stream. Stale property records are skipped after removal; direct
lifecycle property reassignment preserves its slot. Native
DOM-to-pixels regressions include newly created elements, timers,
load events and abort events.

The dispatch algorithm remains a bounded partial original
implementation. It does not support listener object callbacks,
complete WHATWG event path/Shadow DOM retargeting, native default
activation or broad official upstream WPT Events compatibility.

## M4.30b-d: DOMException, composed/timeout signals, fetch cancellation

The original JS VM constructs branded DOMException instances with
name/message, supported legacy numeric code mappings, toString and
ordinary assignment protection. The default reason for
AbortController.abort() and AbortSignal.abort() is an AbortError
DOMException; AbortSignal.timeout() emits a TimeoutError DOMException.

AbortSignal.timeout schedules a bounded deadline via
JsRuntime.next_timer_wait/run_due_timers and never launches its
own OS thread. AbortSignal.any accepts bounded array-like values,
tracks dependent signals using abort_followers and preserves the
abort reason for nested composition and listener cancellation.
The VM enforces 64 input signals and 60s timeout limits.

The original self-hosted Request class retains the signal when
created/copied. fetch passes it to the native opFetchText command,
which validates AbortSignal identity and binds each pending text
callback to the signal. When aborted, the signal listener rejects
the Promise with signal.reason; pending/queued native completions
are discarded, including late HTTP replies. The underlying WinHTTP
request may still complete on an already-started worker thread,
so the implementation is best described as **observable fetch
cancellation**, not physical cancellation of blocking network I/O.

This remains an intentionally limited same-origin GET-only,
buffered-response fetch; no streams, non-GET bodies, CORS or
complete WHATWG Fetch processing. Original-source WPT scores
have not been recomputed.

## M4.30a: original AbortController and AbortSignal

The page VM allocates native AbortController and AbortSignal object
kinds, with a stable controller.signal reference. The signal exposes
read-only aborted/reason, an abort event target with onabort, and
throwIfAborted() that throws the stored JS reason. AbortController
abort(reason) is idempotent and dispatches the abort event once;
AbortSignal.abort(reason) creates an already-aborted signal without
dispatching a new event.

EventListenerOptions.signal is validated on Element/Document/Window
addEventListener. A previously aborted signal skips registration;
active signals are stored with first-registration listener options.
Abort synchronously removes matching listeners from both native
Element and global listener maps. Delivery checks active lists so
abort inside a handler skips later removed callbacks. Existing
once/passive, exception isolation and native click event routing
remain in effect. Test coverage includes six further handler cases,
signal factory/throw checks, and native repaint after timer abort.

Limitations: default AbortError reason is currently a string rather
than DOMException, signal.timeout/any and fetch/timer cancellation
are not implemented, and this is not full WPT compliance.

## M4.29b: unified connected Element event propagation

The page VM receives the authoritative parser document root from
op_engine. Connected Element.dispatchEvent(Event), Element.click()
and hit-tested native clicks now dispatch across the full bounded
Window -> Document -> Element ancestry route. Capture executes even
for events with bubbles=false; bubbling to ancestors and globals
requires bubbles=true. Once/passive listeners, handler exceptions,
stopPropagation and stopImmediatePropagation follow the prior owned
listener implementation. Detached or removed nodes remain local.
Native document/window-only click listeners participate even when
no Element registers a callback.

The final event path is not yet standards-complete: no Shadow DOM
event retargeting, composed paths, default activation or AbortSignal;
the frozen original WPT sample does not establish global compatibility.

## M4.29a: document/window custom events and lifecycle options

Document and window now implement bounded addEventListener and
removeEventListener options dictionaries (capture/once/passive),
plus dispatchEvent for user-created Event objects. A
document.dispatchEvent event uses window capture, document target
and window bubbling (if bubbles=true), while dispatchEvent on window
delivers to the window target alone. Handler ordering and eventPhase,
currentTarget cleanup, cancelation and per-event dispatch guards
are covered by original VM tests.

Host-driven document readystatechange, DOMContentLoaded and
window load callbacks also apply once removal before invocation,
stopImmediatePropagation and error isolation. Ordinary handler errors
are recorded in the bounded Engine.active_event_listener_errors
diagnostic without aborting later listeners. Previous initial-load
event.target behavior for window load is preserved for compatibility.

Limitations: Element->document/window event paths, AbortSignal,
listener-object callbacks, generic default actions and the full WPT
async harness remain outside this original EventTarget slice.

## M4.28b: Element listener options and handler errors

Element.addEventListener and removeEventListener now recognize
options.capture (or boolean capture). addEventListener supports
once and passive and preserves the first options on duplicate
registration. once registrations are removed before callback
entry, preventing nested dispatch from invoking them twice.
During a passive listener, Event.preventDefault() and legacy
event.returnValue=false do not cancel the event. Other JS
handler exceptions are isolated and recorded (max 32 entries),
while execution-limit errors remain fatal. Native click, custom
Event, timers and synthetic-to-physical DOM binding use these
same options. Engine.active_event_listener_errors() exposes
diagnostics without treating listener exceptions as whole-script
failure. This is a bounded Element-only EventTarget slice,
not complete browser event compatibility.

## M4.28a: stopImmediatePropagation

Original JS native click, Event() and createEvent('Event') objects now expose stopImmediatePropagation(). Unlike stopPropagation(), it prevents later listeners on the current target as well as propagation to ancestors. Reused custom Events reset dispatch-specific stop state. VM and original DOM-to-pixel tests verify the behavior. once/passive listener options, exceptions across listeners and full DOM EventTarget conformance remain unfinished.

## M4.1: first actual page scripting slice

Classic inline `<script>` contents now execute once during document
preparation, in source DOM order with shared JS global bindings. A first
built-in DOM host exposes `document.getElementById(id)` and
`element.textContent` reads/writes, returning detached mutation records
to the browser engine. Real DOM child text is replaced before normal
CSS/layout/Win32 paint. Reflow retains the change. VM instruction,
object, string and mutation limits apply; exceptions are tracked via
`Engine::active_script_report` rather than crashing page navigation.

Try `cargo run -p op_browser -- examples/js/dom-text.html` to see the
JS-modified heading. This is not browser-grade script processing:
no `script src`, parser-blocking evaluation, modules, event handlers,
`addEventListener`, DOM creation, timers or HTML5test score.
Unsupported scripts can fail while the rest of the page remains visible.

The ECMAScript language/runtime is still incomplete. Arrow/default/rest/destructuring forms, labels, for-in/of,
property descriptors/accessors, full array-length mutation rules, primitive boxing/ToPrimitive,
garbage collection, broad standard built-ins, promises/modules and DOM bindings are still absent.
C-style `for(let ...)` has one loop lexical environment rather than fresh per-iteration bindings,
and ASI/line-terminator restrictions around throw/postfix updates are still incomplete. Function
calls currently recurse through the native stack; the temporary depth limit is 64 until calls move
to explicit VM frames.

## M4.2: external JavaScript via native network loader

External `<script src="…">` now works for relative local JS files and
same-origin HTTP(S) classic scripts. Scripts run in DOM order with
adjacent inline scripts and a shared VM. The resource loader checks
the URL's origin, content-filter rules using `ResourceType::Script`,
response MIME and script byte budgets; the final origin after a
redirect is checked before execution. A failed external load is counted,
but does not prevent a following script from running.

Try `target\\release\\op_browser.exe examples\\js\\external.html`
and inspect the green result block. The page sources its JS from the
sibling `external.js` file. Loopback WinHTTP, blocked requests,
cross-origin rejects, file loading and retained reflow have dedicated
tests. `async`/`defer`/`integrity` scripts are deliberately skipped,
and scripts still run after whole-document parse rather than at their
HTML parser insertion points. HTTP redirects are validated only after
a response is fetched. Browser events, security policies, modules and
sufficient HTML5test Web APIs are not yet implemented.

## M4.3: retained click handlers

OPBrowser now keeps the original JS VM alive for the active page, so
`document.getElementById("control").addEventListener("click", fn)`
and `element.onclick = fn` retain their function closures. Native
Win32 mouse clicks are converted to page coordinates and passed to the
engine, which hit-tests id-bearing block rectangles from the current
layout and invokes matching callbacks. A basic `click` event has
`type`, `target`, and `currentTarget`; `this` is the clicked
element. Text mutations update the actual DOM and trigger reflow.
Try `examples/js/click.html` and click the block multiple times.

Limits: no event bubbling, capture, keyboard activation,
`removeEventListener`, `preventDefault`, `stopPropagation`,
full default actions or event-loop scheduling; complex inline/flex
hit-testing is not complete. This is **not** general DOM Events
compatibility, and html5test.co still cannot compute its score.

## M4.4: bubbling click dispatch and removal

When a native click lands on an id-bearing block box, the engine
selects the smallest hit element even if it has no own listener.
The retained JS VM invokes callbacks on the target and then its
ancestors, in DOM parent order. `event.target` remains the hit element;
`event.currentTarget` and callback `this` identify the current
listener. `eventPhase` is 2 at target and 3 during bubbling;
`event.bubbles` is true. `element.removeEventListener("click", fn)`
removes the previously registered callback by its function identity.

Try `examples/js/bubble.html` in OPBrowser. This is not complete
DOM Events: capture, stopPropagation, preventDefault, default actions,
keyboard events, listener options and non-block hit targets remain
unsupported. Script execution is still post-parse.

## Test262 measurement

`op_js` includes `test262_probe`. It measures only whether the current parser accepts
positive parse tests and rejects Test262 tests whose frontmatter explicitly expects a parse
error. It intentionally does not report those results as runtime conformance.

Example:

```powershell
cargo run -p op_js --bin test262_probe -- C:\src\test262\test --limit 2000
```

For the combined local subsystem check use:

```powershell
.\tools\compatibility.ps1 -Test262Path C:\src\test262\test -Test262Limit 2000
```

The unchanged Test262 Parser v1 subset moved from **364/1983 (18.36%)** to
**391/1983 (19.72%)**, then **408/1983 (20.57%)**, **504/1983 (25.42%)** after functions/closures,
**508/1983 (25.62%)** after broader control flow, updates and exceptions, and
**523/1983 (26.37%)** after this/new constructor parsing. This remains a parse-expectation metric,
not runtime conformance.

The next JS work is modern function/parameter syntax, labels/for-in/of, per-iteration lexical
bindings, broader built-ins/property semantics and explicit VM call frames. In parallel, M4 can now
start consuming the VM through a narrow DOM binding boundary because ordinary receiver calls,
constructors, arguments and catchable runtime errors exist; page scripting still must not be
advertised as compatible until that binding/event-loop work lands.

## M4.5: capture and cancellation primitives

addEventListener("click", fn, true) now registers a capture listener.
Native click dispatch walks root-to-target capture (eventPhase 1), target
(phase 2), then target-to-root bubbling (phase 3). Capture-aware
removeEventListener uses the same boolean flag.

event.stopPropagation() stops traversal after the current element and
event.preventDefault() marks the cancelable event as defaultPrevented.
Tests cover exact phase order and cancellation before target. Option
objects, once/passive, stopImmediatePropagation, keyboard events and
native default-action cancellation remain outside this bounded subset.

## M4.6: parser-blocking classic scripts

The HTML tree builder now pauses after closing each script tag and calls
the engine's bounded classic-script runner before inserting later HTML
nodes. Inline scripts and same-origin external classic scripts execute in
tree-construction order with the same retained page VM, globals, closures,
and event listeners. The engine refreshes the JS-visible DOM at each
script boundary and again after parsing completes, so late DOM nodes are
visible to callbacks during user interaction. Detached textContent mutations
are applied to the live DOM before tree construction resumes.

This is intentionally a **tree-builder pause**, not a complete streaming
HTML parser: tokenization happens ahead of tree construction, document.write
cannot reenter the tokenizer, external fetches are synchronous, and async,
defer, integrity, modules, DOMContentLoaded, and load scheduling remain
unsupported. M4.6 regression tests prove that earlier scripts cannot query
future DOM IDs and that late nodes become available after parser completion.

## M4.7: external classic async and defer scheduling

OPBrowser now differentiates blocking, deferred and async external classic
scripts. The filtered same-origin loader fetches deferred and async
resources concurrently on bounded scoped workers without granting those
threads access to JS VM or live DOM. Deferred scripts execute after HTML
tree construction in document order. Async scripts execute in completion
order when polled at script boundaries or during initial-load finalization.
When both external flags occur, async wins. Inline classic scripts ignore
both timing attributes. Failed downloads remain non-fatal.

Initial navigation still waits for scheduled workers: this is not an
independent event loop. Async cannot execute at arbitrary tokenizer points,
during interactive frames or after page presentation. No document.readyState,
DOMContentLoaded, load event scheduling, modules, SRI or document.write yet.

## M4.8: document lifecycle events

The engine now transitions document.readyState from loading to interactive
after HTML parsing, then to complete after the initial script queue drains.
It sends readystatechange and DOMContentLoaded to document, and load to
window, with retained listeners, removal and matching on* properties.
Callbacks run in the single JS runtime; mutations are applied before
painting. This is limited initial navigation sequencing, not a general
HTML event loop or resource-complete browser load state.

## M4.8: readyState, DOMContentLoaded and window load

The page VM exposes host-owned document.readyState. Parser-blocking scripts
observe "loading"; after HTML tree construction the engine sets "interactive"
and dispatches readystatechange to document. Deferred classic scripts
then execute in order, followed by DOMContentLoaded on document. Pending
async classic scripts complete before readyState changes to "complete",
triggers readystatechange again and dispatches window load.

Lifecycle listeners use document.addEventListener/removeEventListener
("readystatechange", "DOMContentLoaded") or window.addEventListener /
removeEventListener("load"). Both document.onreadystatechange and window.onload
properties work. Non-bubbling, non-cancelable event objects expose type,
target, currentTarget and eventPhase 2; this identifies the receiver.
Mutations cross the same DOM text boundary before layout/paint. JavaScript
cannot overwrite document.readyState.

This is initial-load-only scheduling. There is no interactive event loop,
post-presentation async delivery, microtasks, timers, document.write,
general Web APIs or module lifecycle.

## M4.9: retained one-shot timers and post-paint updates

The window/global scope exposes setTimeout(callable, delay, ...args)
and clearTimeout(handle). Callbacks run on the original page worker
after the specified deadline, not during HTML parsing. Timer callbacks
may update textContent and trigger an unsolicited reflow/presentation
without requiring clicks, navigation or resize. Delay is capped at
60 seconds, at most 64 timer jobs may be pending, and a page cannot
schedule more than 512 timer IDs. Only the first 16 due jobs run per
worker tick. New navigation drops the previous page's timers.

Timers do not create their own threads. Unlike a complete browser
event loop, there are no Promise microtasks, setInterval, nested
throttling, string-based timer evaluation or post-presentation async
network fetch scheduling.

## M4.10: interval and microtask support

OPBrowser now exposes setInterval, clearInterval and queueMicrotask
alongside setTimeout/clearTimeout. Both timer removal methods cancel
either kind of timer. Intervals use a 4ms minimum and a shared bounded
numeric ID pool, and can cancel their own next firing. Microtasks are
FIFO and run after each classic-script, DOM event or timer task before
the next macrotask, with per-page and per-checkpoint limits.

This is a limited VM job queue. Promise and MutationObserver jobs,
general async network completions after initial painting and hidden
tab throttling are outside the supported feature set.

## M4.11: small async text requests after rendering

A page can call opFetchText("data.json", function(text, error) {...})
to load limited same-origin content on a background worker and receive
a callback through the retained JS runtime. Text/JSON responses and
errors are delivered after the initial page render and can change DOM
and native pixels without input. Per-resource cap is 64 KiB; the
page budget is eight concurrent requests and 32 total. A navigation
change prevents any old callback from changing the new document.

M4.11's callback function is retained as a compatibility bridge.

## M4.12: Promise reaction jobs and the first fetch/Response subset

`Promise` is self-hosted by the original OPBrowser interpreter rather than
imported from any ready-made JavaScript engine. Its `then`, `catch`,
`finally`, `Promise.resolve`, and `Promise.reject` schedule FIFO
reactions through the bounded existing `queueMicrotask` host queue.
Promise executors run immediately; registered reactions run at
microtask checkpoints. Pending promise resolution and chained
promise/thenable adoption are covered by VM regression tests.

Page scripts can use:

```javascript
fetch("message.txt")
  .then(function(response) { return response.text(); })
  .then(function(text) {
    document.getElementById("output").textContent = text;
  })
  .catch(function(error) {
    document.getElementById("output").textContent = error.message;
  });
```

This fetch is restricted to same-origin GET text resources of 64 KiB or
less, through the existing M4.11 background request pipeline and
page-generation checks. A successful Response has ok=true, status=200,
statusText="OK" and single-use text() that returns a Promise.

Important conformance gaps: the 200/OK metadata is synthetic; HTTP error
responses currently *reject*, unlike standard fetch, and Response has no
real status/headers or stream. There are no Request/Headers interfaces,
CORS, credentials, POST bodies, AbortController, Promise.all/race/any,
async/await or unhandled-rejection events yet. The VM's 256 queued /
1024 per-page microtask limits remain in force. Use the manual sample
`examples/js/promise-fetch.html` to observe an asynchronous DOM repaint.

Worker request filtering uses an immutable snapshot; statistics from
its cloned counters do not aggregate with the main filter yet.

## M4.13: real HTTP status, response URL and Headers lookup

Same-origin GET fetch now fulfills for HTTP 404/500. Response.ok is
false; status/statusText are the server's values, url is the final
resource URL, and redirected reflects a followed redirect. The native
Response.headers.get(name) and .has(name) support ASCII-insensitive
header names, but never disclose Set-Cookie or Set-Cookie2.

~~~javascript
fetch("missing.txt").then(function(response) {
  if (!response.ok) {
    document.getElementById("output").textContent =
      "HTTP " + response.status + ": " + response.statusText;
  }
  return response.text();
});
~~~

Network, request filter, encoding or byte-budget failures reject the
Promise; an HTTP 404 response itself does not. The original opFetchText
callback still reports HTTP errors as errors.

This remains limited to same-origin GET text resources (64 KiB).
Local files have synthetic 200/OK. The public Headers constructor and
iteration/mutation, CORS, POST, credentials, abort, streaming bodies,
response.json(), and full RequestInit do not exist yet. WinHTTP follows
redirects before the final-origin check, rather than enforcing origin
at each intermediate hop.

## M4.14: Request, mutable Headers and origin-safe redirect handling

The browser-global Headers constructor supports initializers from a
record, pairs of arrays, or an existing Headers. It exposes get, has,
set, append and delete with case-insensitive validated header names
and tight memory/field limits. The self-hosted Request class accepts
a URL or Request and a guarded subset of RequestInit:

- method: GET (case-insensitive lowercase get accepted)
- mode: same-origin only
- credentials: omit only
- redirect: follow or error
- headers: a Headers instance, record, or pair array. Outbound headers
  are limited to Accept, Accept-Language, If-None-Match, If-Modified-Since
  and X-* under strict size/ASCII checks. Cookies/Authorization are
  not supported.

~~~javascript
var request = new Request("message.txt", {
  method: "GET",
  headers: new Headers({ "X-Client": "OPBrowser" }),
  redirect: "error"
});
fetch(request).then(function(response) {
  return response.text();
}).then(function(text) {
  document.getElementById("output").textContent = text;
});
~~~

Unlike the earlier transport, text/fetch GET requests no longer allow
WinHTTP to follow redirects automatically. Every Location is resolved
and checked against the original origin *before* the next request
(maximum five hops). With redirect:error any HTTP redirect rejects.
Other document/image/script/stylesheet loaders keep their existing
WinHTTP redirect handling. Legacy opFetchText callbacks remain
compatible and also benefit from the safer text path.

This is not full Fetch or Headers conformance. Only bounded same-origin
GET text responses are supported. credentials defaults to omit
instead of the web standard's same-origin pending cookie isolation.
There is no general Header iteration, Headers guards, CORS, POST,
request body, streams, AbortSignal or HTTP cache. Response.json is
implemented in M4.15 but full Fetch remains absent.

## M4.15: JSON and four Promise combinators

The original VM now supports bounded JSON.parse and JSON.stringify.
Its standalone strict parser creates owned JavaScript objects and arrays
without evaluating JSON as executable JavaScript. SyntaxError and
circular-structure TypeError are catchable by page scripts.

Response.json returns a Promise and consumes the body once, using
the existing same-origin text-only GET network path. Promise.all,
Promise.race, Promise.allSettled and Promise.any are self-hosted through
the bounded FIFO microtask queue, with array-like input up to 256 values.

The JSON parser limits inputs to 64 KiB, 64 nesting levels and
4096 parsed nodes. JSON reviver, replacer, spacing and toJSON callbacks,
true insertion-order enumeration and ECMAScript canonical number
formatting are missing. The VM cannot represent unpaired UTF-16
surrogates as values. Promise combinators do not yet support arbitrary
iterables or a fully native AggregateError class.

## M4.16: real Test262 runtime execution and basic global constructors

The original JS interpreter now implements basic callable Object,
Array, Boolean, Number and String builtins, isNaN/isFinite and Number
static numeric constants. String-to-number coercion accepts the
standard hexadecimal, binary and octal prefixes.

An independent Test262 runtime probe now executes 91 pinned upstream
classic-script arithmetic/equality fixtures, each in a fresh VM, with
a small self-hosted assertion bootstrap and explicit pass/fail/skip
cases. Its first 18/91 score rose to 59/91 (64.84%) on unchanged
fixture paths after implementing the above builtins. This narrow
metric must not be confused with the 523/1983 parser-only metric.

Limitations: boxed primitive constructors, full Object/Array prototype
methods, Date, eval, Symbol and modern ES features remain incomplete.
Only the supported classic-script assertion helpers are supplied.

## M4.17: real boxed values and ordered object-to-primitive conversion

The VM now distinguishes new Boolean/Number/String objects from
their primitive counterparts. Constructor prototypes expose
valueOf/toString methods, and Object(primitive) boxes values.
Binary operators use ordered valueOf then toString calls with
catchable user exceptions. instanceof tests prototype-chain
membership, while void evaluates its argument and returns undefined.
Sloppy assignments to undeclared variables create global bindings.

Re-running the unchanged 91-case Test262 Runtime v1 suite against
its pinned revision produced **82/91 (90.11%)**, versus 59/91
(64.84%) at M4.16. Remaining nine failures require eval, Date
or Symbol. This result must not be confused with broad ECMAScript
runtime or parser compliance, nor with static WPT.

## M4.18: broader Test262 measurements and syntax/builtin work

A new deterministic Test262 runtime v2 manifest spans 289 upstream
files across 25 independently named ES expression, statement and
built-in families, selected without consulting pass/fail results.
Of these, 179 are attempted and 110 are explicitly skipped as beyond
the supported Test262 harness subset. M4.18 raised passes from 39/179
(21.79%) to 65/179 (36.31%) on the same immutable manifest.
The narrow Runtime v1 remains 82/91 (90.11%), not an overall score.

The original VM gained correct typeof handling (including missing
identifiers), nested lazy conditional expressions, Array.isArray,
Array.of, Number.isNaN/Number.isFinite and Object.is SameValue.
An integration test verifies DOM text updates become native pixels.

Major omissions persist: modern ECMAScript syntax, Symbol, Date,
iterators, Array.of custom constructor semantics, strict modes,
many Test262 includes and complete JS/DOM/Web API compatibility.

## M4.19: String/Array methods and SyntaxError

Original native JS now supports String.prototype.charAt for primitive
and boxed string values, bounded Array.prototype.push/pop, and
SyntaxError inheritance for malformed JSON.parse. A native page test
verifies visible DOM text changes through these methods.
Pinned Runtime v2: 78/179 (43.58%) PASS, 101 FAIL, 110 SKIP;
narrow Runtime v1 unchanged at 82/91. Dynamic DOM insertion, generic
array-like methods and lone UTF-16 surrogates remain incomplete.

## M4.20: real DOM creation and insertion

The original VM now supports document.createElement(tag),
element.appendChild(element), id and textContent on detached and
attached elements. DOM objects preserve identity across lookup,
parser boundaries and click dispatch. document.body is available
after the HTML tree-builder instantiates it (otherwise null).
Mutations are queued in execution order to the page-owning engine,
which creates real op_dom nodes, recalculates layout and paints them
natively. Dynamic event handlers and timer changes work.
This is bounded and partial, not a complete DOM specification.

## M4.21: mutable Text nodes and real DOM tree reordering

Native JavaScript supports createTextNode with editable data,
nodeValue/textContent, removeChild/insertBefore and element
attribute get/set/remove including class/id/style. Host replay
modifies the authoritative DOM tree with reference and cycle
checks. CSS rematching after timers/clicks preserves loaded
external CSS and color profiles. Tests verify actual paint.
Missing: full live Node relations, classList, CSSOM and observers.

## M4.22: live childNodes, classList and style

The original JS VM now supports Element/Text parentNode,
firstChild/lastChild and stable live childNodes, with current
length, item(index) and bracket access. Original native
replaceChild/remove mutate the real tree and preserve handles.

A bounded Element.classList exposes contains/add/remove/toggle
and reflects className and the original CSS cascade. Element.style
supports a first subset of property reads/writes and
setProperty/getPropertyValue/removeProperty plus cssText.
Retained JS handles remain valid when newly created nodes
receive physical native NodeIds. Timer-driven changes reach
original DOM, author style recascade and native pixels.

Nine new end-to-end checks pass, but many DOM/CSSOM/Web API
semantics are incomplete. Pinned Test262 v1/v2 JS runtime
scores remain unchanged at 82/91 and 78/179 attempted.

## M4.23: Node metadata, multi-token classList and real WPT fixture

Added live previousSibling/nextSibling/isConnected, ownerDocument,
nodeName/tagName, Text.length and Element.contains, plus atomic
variadic classList.add/remove and safer CSS declaration splitting
inside quoted/function values. New integration tests verify
native repaint.

The browser can now execute original WPT Node-childNodes-cache.html
from the exact pinned upstream commit using a narrow synchronous
test/assert_equals adapter. The WPT DOM smoke v1 manifest contains
four manually selected files: one attempted/pass and three
unsupported skips. This is not an official or representative WPT
DOM score. Broader harness, iframe, JS and DOM APIs remain missing.

## M4.24: Element.children and expanded pinned WPT DOM smoke

Original JS now supports live Element.children with filtered
indices/item/basic namedItem, childElementCount, first/last
element child and element siblings. Two native page tests check
identity and timer-driven mutations. Seven original upstream
WPT DOM fixtures now pass using the limited synchronous
testharness adapter, with three explicit skips. This is a
manually selected smoke test, not broad WPT conformance.

## M4.25: live getElementsByTagName and multi-test WPT DOM smoke

The original VM supports Element.hasAttribute/hasAttributes,
Document/Element.getElementsByTagName and live descendant-only
collections with current length, item and indexed lookup across
parser scripts, DOM edits and timer callbacks.

Manually scoped WPT DOM smoke v3 executes ten original pinned
upstream files successfully (11 test() callbacks); three files
are explicitly unsupported/skipped. The synchronous adapter
now keeps a sticky failure bit and verifies expected test calls,
so later successes cannot mask earlier failures. Not the
official WPT harness or representative DOM conformance.

## M4.26: JS in, simple query selectors and programmatic click

Original JS lexer/parser/VM implements bounded relational in,
including ordinary and live DOM object property checks.
Document/Element.querySelector and querySelectorAll currently
support only #id/.class/tag/*; the latter returns a static
indexed NodeList, unlike existing live collections.
element.click dispatches existing native capture/target/
bubbling listeners and updates real native page pixels,
including from timers, with finite recursion. No full CSS
selector grammar, generic dispatchEvent or default activation.

The manually pinned original-source WPT DOM smoke v4 has
14 selected files: 11 attempted and passed, 3 explicit SKIP,
12 original synchronous test() callbacks. This is not an
official WPT conformance result.

## M4.27: compound Selectors and EventTarget.dispatchEvent

Original VM DOM selectors support bounded tag.class#id,
descendant, direct-child and comma-group matching with
Document/Element.querySelector/querySelectorAll and
Element.matches/closest. Complex pseudo, attribute and
sibling syntax is not yet supported.

The new original Event(type,{bubbles,cancelable}) and
Element.dispatchEvent use typed listener capture/target/
bubble propagation, preventDefault, stopPropagation,
defaultPrevented, and return cancelation status.
Legacy document.createEvent/initEvent/returnValue is also
available in a restricted form. Recursion and event path
budgets apply, but full EventTarget/DOMException semantics
and listener exception handling remain incomplete.

Pinned manually scoped original WPT DOM/Events smoke v5:
15 files selected, 12 original files attempted/passed,
3 explicit SKIP, 14 synchronous test() callbacks.
Not a representative WPT conformance rate.
