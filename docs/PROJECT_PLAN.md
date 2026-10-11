# OPBrowser: active project plan

Updated 2026-10-11. This file contains **decisions, priorities, acceptance gates and current status only**, not a running implementation diary. Completed M0-M4.35 milestone narratives remain recoverable from Git history and are tracked in [DEV_LOG.md](DEV_LOG.md). Measured test history is in [COMPATIBILITY.md](COMPATIBILITY.md).

## Product goal and constraints

Independent lightweight Windows browser with its own Rust HTML/CSS/layout/paint/JavaScript stack. No embedded Chromium, Gecko, WebKit, V8 or other browser/JavaScript engine. Ship readable, scrollable, interactive pages, not a growing list of partially implemented Web APIs.

A green handpicked test fixture is **not** evidence of general website compatibility. Regressions must be reproducible and committed with their source manifest.

## Active priority order (replaces JS EventTarget-first roadmap)

| Priority | Workstream | Current reality | Next deliverable and exit gate |
| --- | --- | --- | --- |
| **P0-A** | **Usable rendering and scrolling** | **R1.0 DONE:** CSS `overflow:hidden/clip` clips normal block descendants in original layout/paint/GDI and hidden hit regions, including nested bounds. `scroll`/`auto` and axis longhands have no functional scrolling yet. | **R1.1 NEXT:** per-element scroll offsets and `scrollTop`, nested wheel input, paint/hit-test translations, scrollbar ranges, and original WPT overflow fixtures. |
| **P0-B** | **Memory safety / original VM GC** | JS heap and environment arena retain all allocations until runtime destruction, both with 100,000-slot budgets. M4.35 exposes allocation counters; **no GC yet**. | G1: explicit traced roots and non-moving reachability verification. G2: reclaimed slots with safe handles, cycles/closures/DOM/timer/Promise references. G3: bounded automatic GC safepoints and long-session stress. See [GC_IMPLEMENTATION.md](GC_IMPLEMENTATION.md). |
| **P1-C** | **Flexbox with multiple lines** | A bounded single-line row formatter exists. | R2: flex-wrap, lines, gap, order, cross-axis layout, min-content constraints. Test resize and 2D pixel positions against frozen fixtures. |
| **P1-D** | **CSS Grid foundation** | Table grid track sizing exists; it is **not** CSS Grid Layout. | R3: explicit/auto tracks, fraction units, item placement and basic gap, then implicit tracks and minmax. Screenshot+geometry tests. |
| **P1-E** | **Backgrounds and typography** | URL image tiling for blocks/tables and GDI font measurement exist, but broad backgrounds, web fonts and modern shaping/fallback do not. | R4: repeat/size/position and clipping for image backgrounds; then DirectWrite shaping/fallback and real @font-face loading with origin/security controls. |
| **P1-F** | **Forms and editing** | Some native/text/checkbox and selector subsets; not a usable complete form implementation. | R5: input/edit focus, keyboard text, caret/selection, submission semantics, accessibility names and persistent state; real click+typing integration. |
| **P2** | **More JS syntax, WebIDL and broad Events conformance** | Original VM has Promise/partial fetch/EventTarget; many modern APIs missing. | Resume **after** R1 and GC safe reachability work. Fix only JS/DOM blockers needed for real sites until then. |

P0-A and P0-B are **both mandatory**; a renderer that cannot scroll or that exhausts its heap during ordinary use is not ready. P1 features may be delivered in narrow, end-to-end, user-visible slices, but cannot displace either P0. Avoid another series of Events-only milestones.

## Engineering gates

1. **Visible behavior:** every rendering change must prove pixels and/or actual native input behavior, not only CSS parser acceptance. Scroll tests include nested containers, clipping of backgrounds/text, scroll offsets, click targets after scrolling, and restoration after resize.
2. **Long-session stability:** use M4.35 `JsRuntime::heap_usage()` counters, process working set, and repeated navigation/DOM churn; count unreachable-but-retained allocations honestly. After G2/G3 require plateau under bounded workloads and no stale ObjectId or closure references.
3. **Security/correctness:** preserve bounded allocations, original same-origin rules, event error isolation, and default-deny unsupported operations.
4. **Independent comparisons:** never silently mutate a frozen WPT/Test262 manifest. Add new, explicitly named test versions and record PASS/FAIL/SKIP. A new category needs an honest initial baseline, even if it is poor.
5. **Release checks:** Rustfmt, strict Clippy, full workspace tests, Win32 smoke, code intelligence, compatibility regressions, clean public main and Wiki.

## Measured baseline (separate, non-comparable selections)

| Suite | Latest recorded measurement | What it does **not** prove |
| --- | --- | --- |
| Pinned WPT Static v1 | 197/200 exact; 198/200 with source-authored fuzzy rules | Only 200 selected reftests; not broad CSS or visual web readiness |
| Pinned WPT Positioning v1 | 53/100 exact | Substantial positioned-layout gaps remain |
| Pinned Test262 Parser v1 | 523/1,983 attempted | Parser subset, not whole ECMAScript |
| Pinned Test262 Runtime v2 | 78/179 attempted; 110 explicitly SKIP | Handpicked language/runtime families, not overall JS compatibility |
| Pinned WPT DOM/Events v8 (M4.35) | 21/21 attempted files PASS, 0 FAIL, 3 SKIP; 24 selected | Manually selected synchronous files, not the official full WPT harness |
| Real-world visual/site smoke | **No representative published baseline** | Cannot yet claim production-site readiness |
| Long-session GC/memory plateau | **Not measured / no GC** | Current object/environment growth is unbounded until per-runtime budgets |

## Immediate implementation sequence

- Close M4.35 regression fixes and freeze the old WPT sample as a test guardrail; **stop growing Events API for its own sake**.
- R1.0 measure scroll/clipping capabilities with reproducing HTML pages and current WPT overflow cases; R1.1 build scrollable box boundaries/clip stacks; R1.2 native wheel/hit testing; R1.3 programmatic scroll APIs.
- G1.0 enumerate all heap roots and record mark-only reachability statistics without freeing; G1.1 add generation-safe stable handles / free list; G2 collect both objects and captured environments; G3 automatic bounded collections with performance baselines.
- Only then expand flex-wrap, CSS Grid and form fidelity in visible, testable increments.

## Documentation ownership

- `PROJECT_PLAN.md`: this active roadmap, kept below ~200 lines. No per-commit DONE narrative.
- `DEV_LOG.md`: chronological engineering diary and milestone history.
- `COMPATIBILITY.md`: sourced metric definitions, exact pinned revisions, failures and SKIPs.
- `GC_IMPLEMENTATION.md`: tracing/collection design, hazards and acceptance criteria.
- `CODE_GRAPH.md`, `CODE_SLICES.md` and generated reports: actual source structure.
- `wiki/Current-Status.md` and `wiki/Roadmap.md`: public, updated when milestones ship.

Historical details removed from this plan are **not deleted from project history**: they remain in Git and the detailed DEV_LOG. Do not paste future release notes into this active plan.
