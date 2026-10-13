# OPBrowser: active project plan

Updated 2026-10-13. This file contains **decisions, priorities, acceptance gates and current status only**, not a running implementation diary. Completed M0-M4.35 milestone narratives remain recoverable from Git history and are tracked in [DEV_LOG.md](DEV_LOG.md). Measured test history is in [COMPATIBILITY.md](COMPATIBILITY.md).

## Product goal and constraints

Independent lightweight Windows browser with its own Rust HTML/CSS/layout/paint/JavaScript stack. No embedded Chromium, Gecko, WebKit, V8 or other browser/JavaScript engine. Ship readable, scrollable, interactive pages, not a growing list of partially implemented Web APIs.

A green handpicked test fixture is **not** evidence of general website compatibility. Regressions must be reproducible and committed with their source manifest.

## Active priority order (replaces JS EventTarget-first roadmap)

| Priority | Workstream | Current reality | Next deliverable and exit gate |
| --- | --- | --- | --- |
| **P0-1 NOW** | **Original JS VM garbage collection** | **G1/G2/G3 partial:** automatic idle GC, generation-safe recycling, weak AbortSignal.any source/follower ownership and Windows RSS probe (12s, 817 collections, 0 errors). | NEXT: validate 30-minute plateau/pause distribution and complete fetch/DOM host roots and allocation-pressure recovery; renderer stays deferred. See [GC_IMPLEMENTATION.md](GC_IMPLEMENTATION.md) and [GC_STRESS_RESULTS.md](GC_STRESS_RESULTS.md). |
| **P0-2 AFTER GC** | **Usable rendering and scrolling** | **R1.0–R1.2c vertical DONE (bounded); R1.2d–e partial:** ancestor-clipped nested scrollbars, horizontal overflow-x/y, native wheel/track interaction and GDI pixels. | Resume horizontal thumb drag, JS scrollLeft, complex clipping and WPT/real-page comparisons only after GC is safe and reclaiming. |
| **P1-C** | **Flexbox with multiple lines** | **R2a–R2c partial DONE:** row wrapping/gap, proportional grow/shrink, flex basis/shorthand and six `justify-content` values, measured in original GDI pixels. | **R2d NEXT:** cross-axis `align-items`, flex direction/order and full min-content sizing, followed by explicit real-page visual comparison; current subset is not full Flexbox. |
| **P1-D** | **CSS Grid foundation** | Table grid track sizing exists; it is **not** CSS Grid Layout. | R3: explicit/auto tracks, fraction units, item placement and basic gap, then implicit tracks and minmax. Screenshot+geometry tests. |
| **P1-E** | **Backgrounds and typography** | URL image tiling for blocks/tables and GDI font measurement exist, but broad backgrounds, web fonts and modern shaping/fallback do not. | R4: repeat/size/position and clipping for image backgrounds; then DirectWrite shaping/fallback and real @font-face loading with origin/security controls. |
| **P1-F** | **Forms and editing** | Some native/text/checkbox and selector subsets; not a usable complete form implementation. | R5: input/edit focus, keyboard text, caret/selection, submission semantics, accessibility names and persistent state; real click+typing integration. |
| **P2** | **More JS syntax, WebIDL and broad Events conformance** | Original VM has Promise/partial fetch/EventTarget; many modern APIs missing. | Resume **after** R1 and GC safe reachability work. Fix only JS/DOM blockers needed for real sites until then. |

**Priority override 2026-10-13:** finish safe, reclaiming original JavaScript GC before resuming renderer/Flexbox/Grid/forms/API development. Existing rendering improvements remain frozen except for required GC regressions and integration fixes. Do not claim GC completion without live heap plateau and identity-safety proof.

**G3 ownership update:** EventTarget-owned lifecycle/AbortSignal handlers are marked through reachable receivers, not unconditional side-map roots. Orphan signal/closure cycles can be reclaimed; live callbacks survive. The DOM/fetch host-root audit and 30-minute memory gate remain outstanding.

**G3 timeout update:** orphan AbortSignal.timeout deadlines are weak and pruned after a validated sweep. Pending fetch/AbortSignal.any roots and abort observers preserve live timeout signals; the fixed 64-entry timeout queue can be reused after orphan reclamation.

**G3 detached-DOM update:** disconnected node wrappers/listeners/native caches can be reclaimed; five tests cover identity, caller roots and 40 create/remove cycles. Still pending: external host lifetime audit, fetch/Promise races, allocation budgets and long-run process-memory proof.

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
