# Roadmap

OPBrowser aims to be an independent Windows browser with an original Rust web/JavaScript engine. The success criterion is **usable, stable real pages**, not the count of EventTarget methods.

**G3 timeout checkpoint (13 October 2026):** orphan AbortSignal.timeout deadlines no longer pin dead signals. An abort observer, pending fetch or reachable AbortSignal.any composite preserves an active timeout. Five focused regressions pass; the full 30-minute memory proof remains open.

## Current priorities (13 October 2026)

1. **P0-1 NOW: original JavaScript GC only.** G1/G2/G3 partial now provides browser-owned automatic mark/sweep, generation-safe slot reuse, weak AbortSignal.any follower ownership and GC pause/failure telemetry. A repeatable Windows native-engine timer probe measured 817 successful collections in 12 seconds with zero GC errors and bounded object/environment arenas. **Not a 30-minute memory proof.** Next: sustained RSS/pause measurement and remaining fetch/DOM/abort root audits. See [GC implementation design](https://github.com/Gerbesh/OPBrowser/blob/main/docs/GC_IMPLEMENTATION.md) and [stress procedure](https://github.com/Gerbesh/OPBrowser/blob/main/docs/GC_STRESS_RESULTS.md).
2. **P0-2 AFTER GC: rendering and scrolling.** Existing vertical and bounded horizontal overflow, partial Flexbox and native GDI tests stay intact. Horizontal thumb dragging, JS scrollLeft, positioned clips, Grid and real-page visual comparison resume only after GC passes safety and stability gates.
3. **P1: backgrounds and fonts.** Improve background positioning/sizing/clipping, then DirectWrite fallback/shaping and real origin-checked web font loading.
4. **P1: forms and interaction.** Keyboard editing, focus, selection, form controls and submission with native end-to-end tests.
5. **P2: broader Web APIs and ECMAScript.** Continue when P0 visible/memory gates pass, rather than spending another series of milestones on narrowly chosen Events assertions.

## Evidence and limits

Frozen WPT Static 197/200 exact (only 200 selected reftests), Positioning 54/100, Test262 Runtime v2 78/179 attempted with 110 explicitly skipped, and M4.35 WPT DOM/Events v8 21/21 attempted with 3 skips. These **do not combine into a website readiness percentage**. We still need a real-world visual/site corpus and long-session memory measurements.

See [active plan](https://github.com/Gerbesh/OPBrowser/blob/main/docs/PROJECT_PLAN.md), [Compatibility](https://github.com/Gerbesh/OPBrowser/blob/main/docs/COMPATIBILITY.md), and [Current Status](Current-Status.md) for verified status. Individual implementation updates belong in DEV_LOG and Git history, not the active plan.
