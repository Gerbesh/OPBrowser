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
