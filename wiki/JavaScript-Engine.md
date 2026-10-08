# JavaScript Engine

`op_js` is OPBrowser's original ECMAScript implementation. It does not embed V8,
SpiderMonkey, JavaScriptCore, QuickJS or another JavaScript engine.

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
