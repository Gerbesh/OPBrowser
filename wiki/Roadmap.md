# Roadmap

OPBrowser aims to be an independent Windows browser with an original Rust web/JavaScript engine. The success criterion is **usable, stable real pages**, not the count of EventTarget methods.

## Current priorities (11 October 2026)

1. **P0-A: rendering and scroll UX.** R1.0–R1.1 clip overflow and scroll vertically via `scrollTop` and Win32 wheel with native pixels/hit testing. R1.2a now reconciles JS `scrollTop` with the actual layout scroll range after render and reflow, including CSS height changes. Next: visible scrollbars, horizontal axes, upstream overflow WPT, positioned/inline fragments, then flex-wrap and CSS Grid.
2. **P0-B: original JS garbage collection.** G1.0 has non-destructive mark-only object/environment reachability at idle checkpoints. Both arenas still retain allocations until VM destruction: **no GC yet**. Next: complete native activation roots, generation-safe sweeping and long-session memory-plateau benchmarks. See [GC implementation design](https://github.com/Gerbesh/OPBrowser/blob/main/docs/GC_IMPLEMENTATION.md).
3. **P1: backgrounds and fonts.** Improve background positioning/sizing/clipping, then DirectWrite fallback/shaping and real origin-checked web font loading.
4. **P1: forms and interaction.** Keyboard editing, focus, selection, form controls and submission with native end-to-end tests.
5. **P2: broader Web APIs and ECMAScript.** Continue when P0 visible/memory gates pass, rather than spending another series of milestones on narrowly chosen Events assertions.

## Evidence and limits

Frozen WPT Static 197/200 exact (only 200 selected reftests), Positioning 54/100, Test262 Runtime v2 78/179 attempted with 110 explicitly skipped, and M4.35 WPT DOM/Events v8 21/21 attempted with 3 skips. These **do not combine into a website readiness percentage**. We still need a real-world visual/site corpus and long-session memory measurements.

See [active plan](https://github.com/Gerbesh/OPBrowser/blob/main/docs/PROJECT_PLAN.md), [Compatibility](https://github.com/Gerbesh/OPBrowser/blob/main/docs/COMPATIBILITY.md), and [Current Status](Current-Status.md) for verified status. Individual implementation updates belong in DEV_LOG and Git history, not the active plan.
