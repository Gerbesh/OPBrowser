# Original JS VM garbage collector: implementation plan

Status: **G1.0 MARK-ONLY TRACE IMPLEMENTED; RECLAMATION / GC NOT IMPLEMENTED** (2026-10-11). Original Rust op_js. Both JS objects and captured environments still allocate into append-only arenas; this stage never frees memory.

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

**Remaining G1 work:** register native interpreter call frames,
operand temporaries, currently executing async callbacks and
host-held object references explicitly before enabling tracing
as an automatic safepoint. No sweeping, slot reuse, GC pauses or
memory plateau have been implemented. G2/G3 remain blockers.

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

M4.35 demonstrates transient allocation growth, and G1.0 can now distinguish reachable from unreachable allocated slots without freeing either category. This is a diagnostic, **not** a memory fix. G1.1 must cover all native activation frames, operands and host-held references at safe checkpoints, then prove that tracing cannot miss live objects. Do not reclaim any slots until generation-safe handles and full root coverage are implemented and verified.
