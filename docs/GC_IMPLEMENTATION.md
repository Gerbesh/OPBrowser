# Original JS VM garbage collector: implementation plan

Status: **G2a PARTIAL: EXPLICIT IDLE MARK-AND-SWEEP WITH GENERATION-VALIDATED SLOT REUSE IMPLEMENTED** (2026-10-13). Explicit invocation can reclaim unreachable objects and environments. Automatic GC and full host-root audit are still incomplete.

## Observed implementation

- `JsRuntime.heap: Vec<JsObject>` stores reserved slots; `ObjectId(usize)` encodes an index plus generation. Recycled slots never validate an older handle.
- `JsRuntime.environments: Vec<Environment>` uses the same generation and free-list protocol for captured lexical bindings and parent references. Explicit sweep releases owned maps, cycles and function captures.
- Budgets remain 100,000 reserved object and environment slots each. Allocation reuses freed slots, but does NOT automatically trigger a collection at the limit.
- `JsRuntime::heap_usage()` is the reserved high-water count, not the live heap size. Explicit `JsGcReport` returns freed/live and reserved counts; process memory and pauses have not been measured.
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

## G2a partial: explicit generation-safe mark-and-sweep

The nonmoving arenas now encode slot index and generation in ObjectId and
EnvironmentId. Both types track alive/generation per slot; validation rejects
stale recycled handles. The idle-only collect_garbage_with_roots API traces
all conservative VM/DOM/task roots plus caller-supplied host handles. If root
validation fails, no sweep occurs. Otherwise it releases unreachable object
properties, function captures and environment bindings, breaks cycles, and
increments generations before placing slots on free lists. Allocation reuses
these slots without growing the reserved arena high-water size; exhausted
generation counters permanently retire their slot to avoid ABA aliasing.
Five tests check cycles, stale identities, preserved closure environments,
failed-pass atomicity, and 80 manual collection cycles with bounded slots.

Remaining: audit EVERY opaque host root/side map and async callback; prove
DOM/timer/AbortSignal/Promise/fetch identity across sweeps; schedule GC at
safe task checkpoints and on allocation pressure. No automatic collection,
real process working-set plateau or GC pause distribution is yet measured.

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

G1 mark-only diagnostics and G2a explicit generation-safe reclamation now coexist. Five targeted G2a tests pass, including 80 manual sweep/reuse cycles with stable reserved slots, but this is NOT an automatic browser GC or a measured 30-minute working-set plateau. Next GC-only work: complete host/async root audit, add persistent host ownership and task-safe triggering, then verify real memory growth, pauses, and pinned compat suites before returning to UI work.
