# Original JS VM garbage collector: implementation plan

Status: **G1.0 MARK-ONLY + G1.1 EXECUTION BARRIERS AND EXTERNAL TRACE ROOT INPUT IMPLEMENTED; RECLAMATION / GC NOT IMPLEMENTED** (2026-10-13). Both object and captured-environment arenas remain append-only. No memory is freed.

## Observed implementation

- `JsRuntime.heap: Vec<JsObject>`; `ObjectId(usize)` directly indexes an object. `JsObject` owns dynamic `properties: HashMap<String, JsValue>`, `prototype: Option<ObjectId>`, and optional `FunctionObject` holding an `EnvironmentId` closure.
- `JsRuntime.environments: Vec<Environment>`; each has a parent and variable bindings, including JS object references. Both vectors append until the page VM is discarded.
- Default budgets are 100,000 objects and 100,000 environments, separately. Exhausting either returns a JS execution-limit error. These are resource caps, not collection.
- `JsRuntime::heap_usage()` from M4.35 exposes allocated slot counts and budgets. Counters include unreachable allocations and are **not** live-memory measurements.
- DOM event listeners, `onclick`, global/document listeners, AbortSignal follower graph, timers, microtasks, pending fetch callbacks, active composedPath, DOM node/object caches, Headers and boxed primitive side maps all hold handles which the collector must account for.
- The stack/activation frames are currently recursive native interpreter calls with `EnvironmentId`, `JsValue` temporaries and synchronous nested event dispatch. Collecting in the middle of such a call before roots are explicit risks a use-after-free or wrong-object alias.

## G1.0 implemented: conservative idle-checkpoint reachability

`JsRuntime::trace_heap_reachability()` now traverses global
objects/prototypes, lexical bindings, captured closures and their
parent environments, DOM node caches, event listener options,
AbortSignal follower graphs, timers/microtasks, pending text
callbacks and retained host side maps. It returns
`JsHeapReachability` counts of reachable/unreachable object
and environment slots without mutating either arena.

Four VM regressions cover live closures, orphan cyclic objects,
DOM listener/timer roots, microtasks/pending text callbacks,
dangling callback handles, and refusal during active event dispatch.
`Engine::active_script_heap_reachability()` exposes the snapshot
for an idle real page worker, tested with a scripted page and
a pure HTML page with no VM. Side maps are deliberately treated
conservatively as roots, so the result is a tracing diagnostic,
not the minimal possible live set.

## G1.1 partial: safe checkpoint barrier and caller-provided roots

The original run_code and call_value functions now use a RAII
ActiveExecutionGuard backed by Arc<AtomicUsize>; it preserves the browser's
Send capability and tracks nested native interpreter/built-in execution.
The guard unwinds on all error and early-return paths. Mark-only tracing
refuses to run during a live JS call, DOM event dispatch or programmatic
DOM click. Host code retaining a returned JsValue may pass its temporary
handles to trace_heap_reachability_with_roots(&[JsValue]) for one snapshot.
Three regressions test nested/error barriers, host closure and environment
reachability, and programmatic DOM dispatch.

Remaining G1 audit: all native callback entry paths, persistent escaping host
handles and root ownership beyond single diagnostic snapshots. G2 needs
generation-safe recyclable slots before any memory can be reclaimed.
No slot reuse, sweeping, GC pauses or memory plateau have been implemented.

## GC phase G1: safe non-destructive root tracing

Implement an ownership-aware scanner that marks `ObjectId` and `EnvironmentId` reachable from the following roots:

1. Global object, built-in prototypes, global lexical environment and document host.
2. Live interpreter operand/evaluation values and lexical/call frames. Until explicit frames/stack maps exist, restrict tracing to **safe checkpoints after `execute`/event task completion**, or preserve current active frames in a verified root stack.
3. Function closures -> captured environments -> parent environments -> variable bindings -> object values; cycles are expected.
4. DOM node identities and collection caches; live event listeners and their callback objects; property handlers, listener option signals, active event paths and method handles.
5. Timer callbacks, microtasks/reactions, pending fetch callbacks, AbortController/Signal chains, response/header data, boxed values, and all opaque retained host handles.
6. Reachable object prototypes, own property values and special internal maps which use ObjectIds **as keys** or values.

Mark-only telemetry must report retained vs unreachable slots and avoid modifying the arenas. Verify with cyclic references, closure captures, detached DOM, nested dispatch, promise then/catch, settled/pending fetch, timers and abort graphs.

## GC phase G2: reclamation without broken identities

- Keep the engine original, use nonmoving mark-and-sweep initially. `ObjectId` indexes must never silently resolve a different object after recycling. Introduce generation-checked slots or a strictly safe no-reuse strategy before sweeping; update every direct indexing/access path and host identity map.
- Sweep unreachable `JsObject`s and `Environment`s only when tracing confirms no live reference, and remove stale side-map entries coherently. Account for object and env cycles in the same traversal.
- Preserve JS equality/identity, closure capture, prototype chain, `once`/AbortSignal listener behavior, document/window references and stable native DOM node alias resolution across GC cycles.
- Keep object/environments budgets meaningful: count **live or reserved slots**, not lifetime allocation. Do not bypass existing timers/microtask/network limits.

## GC phase G3: bounded triggering and long-running proof

- Trigger at safe task/microtask boundaries and on allocation pressure; cap pause work and never call a collector while Rust holds borrows or unregistered JS activation values.
- Add `cargo test` scenarios with repeated transient object + closure loops, DOM append/remove, listeners + abort, timer rescheduling, pending/settled Promises, fetch abort, page navigation and cycles.
- Publish allocation slots, reachable/unreachable after mark, estimated heap bytes, process working set (Windows) and GC pause distribution. Compare against a fixed G0 no-GC baseline.
- **Exit gate:** under an agreed bounded 30-minute simulated browsing workload, memory must reach a plateau after warm-up and remain within documented threshold, no premature execution-limit exhaustion from garbage, no wrong-object aliases, and all existing Rust/WPT/browser smoke tests remain green. Record actual numbers rather than calling tests green in advance.

## Current measurable baseline / next action

M4.35 demonstrates transient allocation growth. G1.0 detects unreachable allocations, and G1.1 partial now fences native recursive calls and accepts explicit ephemeral host roots for idle trace. Both are **diagnostic only**, not a memory fix. Finish permanent escaped-host-root ownership and all callback audits; then implement generation-safe G2 mark/sweep with slot reuse and follow with G3 long-session plateau verification. All renderer and unrelated work is deferred until GC is safe and effective.
