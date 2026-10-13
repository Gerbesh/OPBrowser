# Roadmap

OPBrowser aims to be an independent Windows browser with an original Rust web/JavaScript engine. The success criterion is **usable, stable real pages**, not the count of EventTarget methods.

## Current priorities (13 October 2026)

1. **P0-1 NOW: original JavaScript garbage collection, exclusively.** G1/G2a provide safe tracing and generation-checked reclamation. **G2b partial now automatically collects at browser-owned idle script/DOM/click/timer/network checkpoints when live allocation pressure is high.** Headers/boxed/DOM query metadata no longer retains orphan owners. Test coverage includes callback closure preservation and repeated script recycling. Next: opaque host/fetch/AbortSignal root audit, failed-GC diagnostics, long-session working-set plateau and pause profiling. See [GC implementation design](https://github.com/Gerbesh/OPBrowser/blob/main/docs/GC_IMPLEMENTATION.md).
2. **P0-2 AFTER GC: rendering and scrolling.** Existing vertical and bounded horizontal overflow, partial Flexbox and native GDI tests stay intact. Horizontal thumb dragging, JS scrollLeft, positioned clips, Grid and real-page visual comparison resume only after GC passes safety and stability gates.
3. **P1: backgrounds and fonts.** Improve background positioning/sizing/clipping, then DirectWrite fallback/shaping and real origin-checked web font loading.
4. **P1: forms and interaction.** Keyboard editing, focus, selection, form controls and submission with native end-to-end tests.
5. **P2: broader Web APIs and ECMAScript.** Continue when P0 visible/memory gates pass, rather than spending another series of milestones on narrowly chosen Events assertions.

## Evidence and limits

Frozen WPT Static 197/200 exact (only 200 selected reftests), Positioning 54/100, Test262 Runtime v2 78/179 attempted with 110 explicitly skipped, and M4.35 WPT DOM/Events v8 21/21 attempted with 3 skips. These **do not combine into a website readiness percentage**. We still need a real-world visual/site corpus and long-session memory measurements.

See [active plan](https://github.com/Gerbesh/OPBrowser/blob/main/docs/PROJECT_PLAN.md), [Compatibility](https://github.com/Gerbesh/OPBrowser/blob/main/docs/COMPATIBILITY.md), and [Current Status](Current-Status.md) for verified status. Individual implementation updates belong in DEV_LOG and Git history, not the active plan.
