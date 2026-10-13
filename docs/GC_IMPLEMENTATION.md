# Original JS VM garbage collector: implementation plan

## G3 partial: idle GC on arena allocation headroom

The automatic idle checkpoint now also checks object/environment arena
capacity, not only the adaptive live-object trigger. When a reserved
arena has reached seven-eighths of its configured allocation budget and
the remaining free slots are scarce, a safe mark/sweep runs before the
next JavaScript task's allocation burst. The 1/8 margin gives typical
scripts time to reach an idle checkpoint without prematurely hitting
the hard limit. Recycling retains generational ObjectId/EnvironmentId
identity and never permits collection within interpreter call frames.
Three VM tests cover exhausted object and environment budgets and two
consecutive bursts that reuse freed slots under the cap. This is not
in-interpreter GC: an individual oversized task may still hit the hard
allocation limit before reaching its next checkpoint.

## G3 partial: unwrapped detached DOM descendants and fetch completion races

Validated DOM sweeps traverse unwrapped native descendants of discarded
detached wrappers using pending-parent and child relationships. They purge
orphan text/tag/attribute/tree records even when no child JavaScript wrapper
was allocated. Live wrappers and query/tag collections protect retained
native children. A live static query result traces its closest materialized
ancestor wrapper, preserving parentNode and object identity.

Three DOM regressions verify orphan descendant reclaim, a live detached
parent and querySelectorAll retention. Two fetch GC regressions verify
callback/signal roots across completion and cancellation while native I/O
is in flight; late replies do not revive aborted Promise callbacks.
No behavioral network changes were needed. Comprehensive host audits,
real browsing and a 30-minute process memory plateau remain open.

## G3 partial: detached DOM nodes and event listener lifetime

Only connected native DOM nodes are unconditional roots. Detached node
wrappers survive through JS globals, explicit host roots, parent/child
relationships or live DOM views, rather than through unconditional caches.
Listener callbacks, onclick, AbortSignal options and cached classList/style/
childNodes/children are traced from the reachable owning DOM node.
Validated sweeps prune dead wrapper identities, callbacks and node metadata
(tags, text, attributes, virtual aliases and native tree links) before
generation-safe slot reuse. Five VM regressions include 40 DOM churn cycles.
Full native host-root audit, fetch races and 30-minute RSS proof remain open.

## G3 partial: weak AbortSignal.timeout deadlines with observer/fetch ownership

Pending AbortSignal.timeout deadlines are no longer unconditional JS heap
roots. During tracing, a pending timeout roots its signal only while an
abort listener or onabort property handler is registered. Other live owners
(global bindings, pending fetch signal maps, AbortSignal.any composites)
mark the signal via their existing strong edges. After a validated mark,
unreachable timeout metadata is pruned *before* generational object-slot
recycling, restoring the 64-pending-timeout budget for new tasks. A live
observer still receives TimeoutError on its scheduled deadline. Five new
regressions cover 64 orphan deadlines, both handler types, removed listeners,
pending host network roots and timeout -> composite abort propagation.
No speculative mid-interpreter GC is introduced. Remaining: DOM host roots,
complete fetch cancellation races, and a 30-minute process-memory plateau.

## G3 partial: EventTarget-owned lifecycle and AbortSignal listeners

Lifecycle listener/options maps trace callback objects, captured environments
and option AbortSignals only when their EventTarget receiver is reachable.
Global window/document targets are already roots. An orphan AbortSignal with
a callback closing over itself can now be reclaimed instead of being pinned
by its host listener metadata. Validated sweeps prune dead listener lists,
options and onabort registration-order metadata before slot reuse. Three VM
regressions cover orphan cycles, live callbacks and option-signal retention. Other DOM/fetch
ownership and the 30-minute process-memory plateau remain open.

## G3 partial: AbortSignal.any lifetime and measurable browser memory

A live composite AbortSignal now traces the reverse follower graph back to its sources, including nested composites. The map is no longer an unconditional root: orphan composites/sources are collected and metadata pruned before slot reuse. When a composite aborts, connections to *other* sources are unlinked because its reason cannot change. Two G3 tests cover orphan graph reclamation and nested abort propagation through a sweep.

The browser now exposes `active_script_gc_telemetry()` (live/reserved slots, success/failure counts, latest sweep report and pauses). The standalone `gc_stress_probe` executes a scripted page timer on the browser-owned worker and samples Windows process working set each second. A 12-second DEBUG trial had 817 successful passes, zero GC failures, stable reserved arenas of 414 object / 247 environment slots, and a worst pause of 1,164 microseconds. Detailed reproduction and limitations: [GC_STRESS_RESULTS.md](GC_STRESS_RESULTS.md). A 30-minute steady-state proof is still pending.


Status: **G3 PARTIAL: SAFE AUTOMATIC GC, WEAK ABORT COMPOSITES AND NATIVE PROCESS RSS PROBE** (2026-10-13). G2a generation-safe sweeping is retained. GC runs after script/DOM/click/timer/network tasks when live slot pressure crosses a bounded trigger, but complete host-root audits and a long-session memory plateau are still outstanding.

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

## G2b partial: task-boundary automatic GC and weak owner metadata

In the browser-owned VM, collect_if_needed_at_idle is called after HTML script
execution and DOM mutation commit, after dispatched clicks, and after the
timer/network task batch. It uses a live object+environment count threshold:
512 slots initially, then at least twice the previous surviving live set.
No collection occurs while native JS call frames or event dispatch are active.
Generic embedders calling JsRuntime::execute still receive an uncollected
return value and must use explicit host root ownership when collecting.
There is deliberately NO collection from an allocator inside a recursive
native evaluation frame.

Four owner-indexed metadata tables now have weak keys:
dom_tag_collections, dom_query_lists, header_values, boxed_values.
They no longer root their owner objects by themselves. When a boxed wrapper
is genuinely reachable, its boxed JsValue is traversed as a strong edge.
After a validated mark, metadata for dead owners is removed before slots
can be recycled. Tests cover metadata pruning, boxed child retention,
idle threshold/reuse, prohibited in-flight GC, pending DOM/timer callbacks,
and a real browser parser running consecutive scripts with bounded heap.

Remaining: audit other conservative DOM/AbortSignal/fetch host ownership,
track and diagnose failed automatic passes, add allocation-pressure recovery
without collecting inside the interpreter, and measure sustained process
working-set plateau and GC pauses. These are not claimed by the above tests.

## G2c partial: automatic GC diagnostics and pending network callback ownership

JsRuntime now exposes the number of failed automatic idle sweeps, the last successful JsGcReport, and last/maximum successful stop-the-world pause durations. Failed sweeps preserve the previous success report while incrementing the error count.
A new isolated network callback regression shows that a pending host callback and captured environment survive mark/sweep until that callback is removed, after which the closure can be reclaimed.
These measurements are per-VM instrumentation, not a complete fetch/AbortSignal audit or a 30-minute process working-set plateau.


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

G2b enables browser-owned automatic idle collection at live allocation pressure, in addition to explicit G2a sweeping. Tests prove two real page scripts reuse object slots, and pending timer/listener callbacks survive a sweep. Next: complete opaque host/fetch/AbortSignal roots, report failed GC passes, profile pauses and process working set under a fixed 30-minute workload. Long-session memory stability is NOT established, so unrelated renderer work remains deferred.
