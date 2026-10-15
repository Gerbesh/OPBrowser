# OPBrowser: active project plan

Updated 2026-10-16. This file contains **decisions, priorities, acceptance gates and current status only**, not a running implementation diary. Completed M0-M4.35 milestone narratives remain recoverable from Git history and are tracked in [DEV_LOG.md](DEV_LOG.md). Measured test history is in [COMPATIBILITY.md](COMPATIBILITY.md).

## Product goal and constraints

Independent lightweight Windows browser with its own Rust HTML/CSS/layout/paint/JavaScript stack. No embedded Chromium, Gecko, WebKit, V8 or other browser/JavaScript engine. Ship readable, scrollable, interactive pages, not a growing list of partially implemented Web APIs.

A green handpicked test fixture is **not** evidence of general website compatibility. Regressions must be reproducible and committed with their source manifest.

## Active priority order (replaces JS EventTarget-first roadmap)

| Priority | Workstream | Current reality | Next deliverable and exit gate |
| --- | --- | --- | --- |
| **P0-1 NOW** | **Modern web rendering and JS integration** | Live-network baseline: 20 services, zero verified fully functional pages; blank SPAs, JS errors, malformed rendering and HTTP blocks separated. | Repair first paint, prioritize actual JS/DOM blockers and visual layout differences, rerun the same public URLs and screenshots. See [REAL_WEB_BASELINE.md](REAL_WEB_BASELINE.md). |
| **P0-2 NOW** | **Layout, scrolling and on-demand profiler** | Bounded nested scrolling and original Win32 paint exist, but modern Flex/Grid/forms are incomplete; GC has synthetic metrics but no interactive CPU/wall phase profiler. | Restore real-site layout/scrolling; add opt-in phase tracing for load, JS, DOM, style, layout, paint and GC during interactive browsing. |
| **P0-GUARD** | **Original JS VM GC correctness/performance** | Two 30-minute synthetic stress runs passed safety; native DOM arena bounded, Private Commit near a plateau, but rare 448-ms wall GC pause remains unexplained. | Maintain GC correctness gates; investigate pause outliers on real browsing traces rather than blocking all rendering work. |
| **P1-C** | **Flexbox with multiple lines** | **R2a–R2c partial DONE:** row wrapping/gap, proportional grow/shrink, flex basis/shorthand and six `justify-content` values, measured in original GDI pixels. | **R2d NEXT:** cross-axis `align-items`, flex direction/order and full min-content sizing, followed by explicit real-page visual comparison; current subset is not full Flexbox. |
| **P1-D** | **CSS Grid foundation** | Table grid track sizing exists; it is **not** CSS Grid Layout. | R3: explicit/auto tracks, fraction units, item placement and basic gap, then implicit tracks and minmax. Screenshot+geometry tests. |
| **P1-E** | **Backgrounds and typography** | URL image tiling for blocks/tables and GDI font measurement exist, but broad backgrounds, web fonts and modern shaping/fallback do not. | R4: repeat/size/position and clipping for image backgrounds; then DirectWrite shaping/fallback and real @font-face loading with origin/security controls. |
| **P1-F** | **Forms and editing** | Some native/text/checkbox and selector subsets; not a usable complete form implementation. | R5: input/edit focus, keyboard text, caret/selection, submission semantics, accessibility names and persistent state; real click+typing integration. |
| **P2** | **More JS syntax, WebIDL and broad Events conformance** | Original VM has Promise/partial fetch/EventTarget; many modern APIs missing. | Resume **after** R1 and GC safe reachability work. Fix only JS/DOM blockers needed for real sites until then. |

**Priority override 2026-10-13:** finish safe, reclaiming original JavaScript GC before resuming renderer/Flexbox/Grid/forms/API development. Existing rendering improvements remain frozen except for required GC regressions and integration fixes. Do not claim GC completion without live heap plateau and identity-safety proof.

**G3 ownership update:** EventTarget-owned lifecycle/AbortSignal handlers are marked through reachable receivers, not unconditional side-map roots. Orphan signal/closure cycles can be reclaimed; live callbacks survive. The DOM/fetch host-root audit and 30-minute memory gate remain outstanding.

**G3 timeout update:** orphan AbortSignal.timeout deadlines are weak and pruned after a validated sweep. Pending fetch/AbortSignal.any roots and abort observers preserve live timeout signals; the fixed 64-entry timeout queue can be reused after orphan reclamation.

**G3 detached-DOM update:** disconnected node wrappers/listeners/native caches can be reclaimed; five tests cover identity, caller roots and 40 create/remove cycles. Still pending: external host lifetime audit, fetch/Promise races, allocation budgets and long-run process-memory proof.

**G3 follow-up (2026-10-14):** GC purges native descendants without JS wrappers; live static query results preserve backing ancestors. Five new tests include two fetch completion/cancellation races across GC. Full host-root audit and sustained native memory proof are still outstanding.

**G3 allocation-pressure update:** browser idle GC now observes remaining object/environment budget headroom (including free slots), so low configured limits and allocation bursts are reclaimed before the next task. Three tests pass; oversized single tasks and extended navigation/RSS profiling remain outstanding.

**G3 host boundary fix:** binding preserves actual connectivity and canonical subtree links; the 256-node staging quota now counts outstanding unbound nodes rather than all creations since page load. Native Document storage now sweeps orphan tree components and recycles generation-checked slots while preserving retained detached identity and pruning weak host caches.

**G3 measurement update:** each successful automatic collection now contributes to a fixed-size pause histogram (p50/p95/p99 upper bounds), including passes between CSV samples. The probe separates timer allocation churn from detached DOM/listener churn and reports authoritative native DOM arena slots. Native sweeping follows validated JS roots, including static queries/views/callbacks and native ancestors/descendants; parser-owned handles defer sweeping until parsing completes. The 30-minute Windows gate, pause outliers and broader navigation/fetch audit remain open.

**G3 synthetic 30-minute Windows gate (2026-10-14): PASS for Release DOM workload only.** 1,800 per-second CSV samples, 80,939 JS/native sweeps with zero errors, 323,854 nodes reclaimed, 21 reserved native slots at finish; Working Set shows no sustained increase but was trimmed by the OS. The p99 JS pause bucket is <=525 us; one 273,375 us wall-time outlier needs diagnosis. Full browsing/host-memory acceptance remains open.

**G3 pause attribution and fetch audit (2026-10-15):** The 273 ms historical outlier cannot be retrospectively assigned to a GC phase. The new probe captures worst-pass mark/metadata/sweep wall times and counts >=16/50 ms. A composite AbortSignal owning a pending fetch remains alive over 16 explicit sweeps, cancels correctly and discards stale completion. Reproduce a long-session outlier and profile thread scheduling, then broaden real-network/navigation and committed-memory gates.

**G3 full host checkpoint and navigation isolation (2026-10-15):** Added per-page timing for the entire callback-bound GC checkpoint through weak-resource cleanup, plus >=16/50 ms outlier counts. A regression verifies that a delayed old-generation response cannot fulfill a same-ID fetch in the new document; a matching current-generation result can. Longer phase-aware profiling, repeated real HTTP navigation, committed/private bytes and multi-tab memory remain open.

**G3 private memory and loopback HTTP update (2026-10-15):** Added extended Windows per-process private-commit samples and an offline CSV validator that distinguishes PrivateUsage from OS-trimmable working set. A true localhost HTTP test navigates away while a fetch is in flight, then verifies the delayed previous response cannot corrupt the next page even under a reused request ID. Broad real-site/multi-tab workloads, GC CPU-time attribution and leak gates remain open.

**P0 real-web priority update (2026-10-15; supersedes the 2026-10-13 GC-first freeze):**
The original GC passes extended synthetic correctness and private-memory
plateau checks, although rare 448ms wall-time stalls remain under watch.
Modern website functionality is now the first product gate: the 20 real
public URLs in [REAL_WEB_BASELINE.md](REAL_WEB_BASELINE.md) gave 10
JS-error partial pages, 2 skeletons, 3 blank viewports, 3 403/429 blocks,
1 malformed/unverified render and 1 timeout. Prioritize JS/DOM blockers
and practical CSS/layout/paint improvements that measurably change those
screenshots. Integrate an on-demand interactive CPU/wall-time profiler
for GC/performance diagnosis during real browsing; do not resume
synthetic-only micro-optimization without an observed failure.

## Real-site development gates (15 October 2026)

The next compatibility iteration is measured on **Google search, YouTube,
Pikabu, Reddit and DeviantArt**, with the other fifteen public URLs retained
as a regression/control cohort. Each run records real native pixels,
first-page navigation outcome, bounded initial JS failure categories,
skipped-script causes and runtime/GC failures. A changed screenshot is
not automatically an improvement: compare the screenshot and actual visible
content, then separately verify navigation, input and scrolling.

1. **Immediate (JS blocker audit):** name the top syntax/runtime failure causes
   from actual public pages rather than relying on the failure count alone.
   Implement conforming language primitives with focused VM tests.
2. **First visible milestone:** make at least two of the five priority pages
   display a coherent, substantially correct first viewport (navigation
   and primary content), not just isolated text or skeletons; record
   explicit evidence and remaining unsupported features, with no baseline
   regressions hidden by a changed URL or missing network response.
3. **Functional milestone:** interact with a real page (click, scroll, text
   entry where applicable), not simply finish a headless screenshot.
4. **Profiling gate:** gather on-demand CPU-time vs elapsed-time traces
   of navigation, script, DOM, layout, paint and GC inside the running browser;
   pursue the old 448-ms GC wall-time spike only if reproduced or independently
   explained. Synthetic-only microbenchmark improvement is not the product goal.
5. **Integrity:** do not add Chromium/other JS engines, counterfeit web
   responses, auth/captcha workarounds or unconditional caps increases just
   to manipulate compatibility results. The public-site suite is opt-in;
   offline CI and GC safety tests must remain green.

**P0 performance timing follow-up (2026-10-15):** The original page VM
now provides a browser-owned `window.performance` object with monotonic
`now()` and Unix-epoch `timeOrigin`. It rejects illegal method receivers
and construction. The updated real DeviantArt run still reports four
failed scripts, including unsupported backtick syntax and null/undefined
access; the former missing-`performance` error did not recur. No visual
improvement is certified. Full Performance APIs and interactive profiling
remain open.

**P0 modern-JS syntax follow-up (2026-10-15):** Implemented original
untagged JavaScript template literals: multiline cooked strings,
`${expression}` substitutions, nested templates, escapes and ordered
concatenation. Four focused regressions and full workspace verification
pass. The five public priority URLs were rechecked with the same
manifest: DeviantArt's original unsupported backtick changed into a
later `expected expression` blocker, but the page still has four JS
failures; YouTube and Reddit remain skeleton/blank, and Google/Pikabu
still have JS errors. No visually correct first viewport is certified.
Tagged templates, full Unicode surrogate handling, arrow functions,
and the opt-in interactive profiler remain pending.

**P0 arrow-function follow-up (2026-10-15):** The original JS
lexer/parser/bytecode/VM now support ordinary `x => expr`,
`(x, y) => expr` and block-bodied arrow functions. Arrow calls
capture lexical `this` and `arguments`, preserve closures across
GC and cannot be constructed with `new`; there is no own
`prototype`. Async arrows, default/rest/destructured parameters,
and full ECMAScript validation remain open. On a repeat five-site
live pass, YouTube script failures dropped 12 -> 11 and Pikabu
14 -> 12 compared with the immediately preceding template-only
run; Google/Reddit/DeviantArt remain at 2/1/4. No modern site is
now visibly functional, and live content can change between runs.

**P0 JS delete operator (16 October 2026):** The original classic-script
parser/compiler/VM now recognizes `delete`, preserves computed-member
evaluation and deletes ordinary object/array fields without changing array
length. Bound identifiers remain protected; host and descriptor edge cases
are partial. On live YouTube the missing-`delete` error disappeared,
6 scripts executed (previously 5) and 10 failed (previously 11), but
**zero text** is still painted. Pikabu has 12 failures. The new settle
window was 1300ms, versus 1000ms before, so this is diagnostic only.

**P0 Image constructor (16 October 2026):** `new Image()` and
`Image()` now produce browser-owned detached `img` nodes, reflecting
`src`/`alt`/`width`/`height`. Script attachment paints actual PNG
pixels in an engine integration regression. Real YouTube advances to
**7 executed / 9 failed** scripts (previously 6/10) and its prior
missing-`Image` diagnostic disappears, but it still paints **zero
text**. Pikabu remains **12 failed**; neither page passes visual
acceptance. Detached image network load/error events, natural dimensions,
absolute URL reflection and full HTMLImageElement remain open.

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
