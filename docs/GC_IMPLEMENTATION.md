# Original JS VM garbage collector: implementation plan

## G3 browser host checkpoint latency and navigation boundaries (2026-10-15)

`scripts::collect_at_idle` reports phase timing only when both JS and
native collections complete successfully. A `PreparedDocument` records
complete post-callback GC stop-the-world wall time (JS + native +
VM reclaimed-node metadata cleanup + per-page native-resource cache pruning)
for timer/network and DOM click checkpoints. `Engine` exposes per-page
samples, last/max pause, worst-pass phase breakdown, and >=16/50 ms counts.
These counters reset whenever a new page is prepared and are distinct from
parser-internal JS/native sweeps, script execution and layout/repaint costs.
`gc_stress_probe` records the browser counters alongside separate VM
and native timing; wall-clock still includes potential OS preemption.

A deterministic engine regression sends an obsolete network completion
after a new page has registered a `fetch` with the same request ID; the
navigation generation must reject that stale result, and the current
generation completes only its own Promise. This verifies ownership and
routing without claiming live HTTP transport testing.

## G3 Windows pause attribution and composite-fetch regression (2026-10-15)

The 30-minute Windows Release DOM test passed on 2026-10-14: 80,939
successful JS/native collections, zero callback or collection failures and a
bounded 21-slot native DOM arena. Its worst JS wall-clock pause was 273,375 us,
but the historical probe had no timing breakdown. New `JsGcPhaseTimings`
measures root marking, weak metadata cleanup and object/environment sweep.
For the **slowest successful automatic** VM collection, `JsGcTelemetry`
exposes each phase alongside counts of pauses >=16 ms and >=50 ms. Phase
timings are nested inside the existing stop-the-world wall-clock pause.
They may still include OS thread preemption; host/native page-resource
cleanup is not part of this timing. The probe now exports five extra fields.
A new `fetch` + `AbortSignal.any` regression verifies 16 repeated idle GC
passes, cancellation and rejection of a stale network reply. These are not
complete real-navigation/multi-tab or private-memory acceptance gates.

## G3 native arena collection and ownership boundary

Native Document now stores optional NodeSlots and generation-checked NodeIds with a reusable free list. On x86_64 the handle uses 32 slot bits and 31 generation bits; the upper bit stays clear to avoid the VM synthetic-ID range. Exhausted generations retire rather than wrap. Document::len reports live native nodes, reserved_slots reports the arena high-water mark, and allocation_headroom includes freed slots. All native node reads/mutations validate generation.

The engine requests native sweeping only after JS roots validate at a browser-owned idle boundary. Native host roots include surviving wrappers, static query backing nodes and DOM view/collection backing nodes; pending JS callbacks retain wrappers through normal closure tracing. Document keeps each retained node's ancestors, siblings and descendants. The VM also traces wrapper ancestors/descendants across unwrapped intermediates. Invalid roots abort native sweeping before any node is destroyed and increment failure telemetry.

No native sweep occurs while the parser holds open-element or deferred-script NodeIds. A forced validated checkpoint after parsing releases orphan parser-created subtrees. Timer/network/native-click boundaries collect on JS pressure or native arena headroom, after host writes commit. textContent replacements clear old child connectivity in VM metadata at commit, and respect native allocation headroom. Weak node-keyed image, generated/background image, linked stylesheet and scroll caches drop dead identities; live detached resources survive. Page-level bounded color profiles retain their existing lifetime.

Native mark/sweep last/max pauses are separate from the JS histogram; their measurements exclude subsequent VM metadata/page-cache pruning. Neither histogram nor native pause maximum proves a bounded total host checkpoint. Native component safety and bounded short-run native arena reuse are verified; the 30-minute Windows and real navigation/fetch/Promise stress gates remain open.

## G3 host fix: binding and staging-budget recovery

A real DOM timer workload exposed two host boundary bugs: native binding unconditionally added detached nodes to `dom_attached`, and `next_virtual_dom_node > 256` exhausted a page-lifetime quota despite collection. Binding now transfers the actual connectivity state only to the canonical physical ID and remaps native parent/child edges, preserving later detached subtree edits. The shared element/text allocator bounds 256 outstanding unbound wrappers and queued operations while retaining monotonic, non-recycled synthetic IDs. Three VM regressions check these ownership/budget rules. Native `Document` storage is still not collected.

## G3 measurement: every automatic pause and authoritative DOM slots

Each successful automatic idle collection records a pause in a bounded, 64-bucket power-of-two nanosecond histogram. Percentiles are nearest-rank bucket upper bounds, not exact durations; explicit sweeps and failed attempts are excluded. The probe reports the histogram sample count so it can be compared with successful collections.

`--workload dom` creates/removes native element/text subtrees and orphan listener cycles while retaining a separate detached subtree and listener to verify identity after GC. `Engine::active_document_node_slots()` now measures reserved_slots, including reusable arena capacity; native live_nodes is reported separately. The 17439ec baseline exposed append-only native storage despite VM wrapper cleanup; generation-safe native sweeping and weak host-cache cleanup now address that retention. The full 30-minute/browsing gate remains open.

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


Status: **G3 PARTIAL: SAFE AUTOMATIC JS/NATIVE DOM GC AND MEASURABLE HOST OWNERSHIP** (2026-10-14). G2a generation-safe sweeping is retained. GC runs after script/DOM/click/timer/network tasks when live slot pressure crosses a bounded trigger, but complete host-root audits and a long-session memory plateau are still outstanding.

## Observed implementation

- `JsRuntime.heap: Vec<JsObject>` stores reserved slots; `ObjectId(usize)` encodes an index plus generation. Recycled slots never validate an older handle.
- `JsRuntime.environments: Vec<Environment>` uses the same generation and free-list protocol for captured lexical bindings and parent references. Explicit sweep releases owned maps, cycles and function captures.
- Budgets remain 100,000 reserved object and environment slots each. Allocation reuses freed slots, but does NOT automatically trigger a collection at the limit.
- `JsRuntime::heap_usage()` is the reserved high-water count, not the live heap size. Explicit `JsGcReport` returns freed/live and reserved counts; short process-memory and pause measurements are recorded in GC_STRESS_RESULTS.md; long-session gates remain open.
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

Automatic JS and native DOM sweeping now recycles slots while preserving live
node/view/query/callback identities, including native components without wrappers.
Short synthetic DOM churn crosses the former 20,000 lifetime native-node limit
while using a bounded reserved arena. Next: the 30-minute Windows memory/pause
gate, wider native host ownership and fetch/Promise/navigation race matrix.
Renderer work remains deferred until those gates pass.
