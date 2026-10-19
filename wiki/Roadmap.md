# Roadmap

OPBrowser aims to be an independent Windows browser with an original Rust web/JavaScript engine. The success criterion is **usable, stable real pages**, not the count of EventTarget methods.

**G3 timeout checkpoint (13 October 2026):** orphan AbortSignal.timeout deadlines no longer pin dead signals. An abort observer, pending fetch or reachable AbortSignal.any composite preserves an active timeout. Five focused regressions pass; the full 30-minute memory proof remains open.

**G3 detached-DOM checkpoint:** GC now releases orphan disconnected node wrappers, listeners and metadata while retaining live DOM view identity; 40 synthetic churn cycles pass. Long-run memory proof remains pending.

**G3 native DOM/fetch checkpoint (14 October 2026):** unused unwrapped child metadata is reclaimed; live querySelectorAll results retain native parent identity. Two new fetch-GC races pass. Full memory proof remains open.

**G3 budget-pressure checkpoint:** automatic idle GC now observes object and lexical-environment arena headroom as well as live allocations. Three regressions pass for low-budget and repeated allocation bursts; long-run RSS remains pending.

## P0 async and visible-page checkpoint (19 October 2026)

Original JS supports ordinary `async function` syntax and Promise settlement **without `await`**. A repeat five-site test records 2/4/11/1/3 failed scripts for Google/YouTube/Pikabu/Reddit/DeviantArt, with Reddit still white and YouTube zero-text. Reddit advances to a genuine unsupported `await` barrier. Next: `await` continuations and microtasks, dynamic scripts and real first viewport pixels. Do not mistake an improved parser diagnostic for web functionality.

## Active P0 priority (16 October 2026)

Real-site usability takes precedence over synthetic-only optimization.
The original VM now advances past YouTube's `delete` syntax blocker,
yet YouTube still paints no text; Pikabu remains visually malformed.
The 30-minute synthetic GC safety/memory gate passed, with long
wall-time pause outliers still to diagnose using a real in-browser
profiler. See the active project plan for current gates.

## Image/real-site checkpoint (16 October 2026)

Browser-owned `new Image()` now uses real DOM nodes and reaches native
paint commands when inserted. YouTube's missing-`Image` error was
removed but it still renders a skeleton without text. Next priority:
the surviving real-site JS syntax/runtime errors and Pikabu's layout,
and interactive CPU/wall profiling rather than synthetic PASS inflation.

## Location and real-page checkpoint (16 October 2026)

The live page URL now reaches shared `window.location` and
`document.location` objects with readable HTTP(S) components and
explicitly rejected navigation writes. The Pikabu reference error
advances to a later runtime failure, but its 12 failed scripts and
broken layout remain; YouTube is still skeleton-only. Prioritize
correct syntax/DOM/script-loading and usable pixels next.

## JavaScript syntax and DOM-first-paint checkpoint (16 October 2026)

The independent VM now handles `??` and bounded synchronous
`for...of`, including real DOM query-result iteration and lexical
closures. New compiler and integration tests pass. The same five
public pages remain visibly broken, so these fixes are **not** a
website-compatibility milestone. Next diagnose YouTube's remaining
for-initializer and backslash syntax failures, Reddit's argument parse
error, and missing layout/paint paths while retaining GC memory guards.

## Real error-context and for-in follow-up (16 October 2026)

Local opt-in byte-context diagnostics now identify specific modern
syntax failures without shipping full site scripts. Bounded native
`for...in` own-key enumeration covers the exact YouTube configuration
loop and verified DOM mutations. YouTube's for initializer syntax
error is gone, but 9 failures and a textless first view remain. The
next verified blockers are regex literals, labeled statements,
unsupported `async function` syntax on Reddit and host API behavior.
Do not increase script budgets simply to hide errors.

## RegExp and comma-operator checkpoint (16 October 2026)

Bounded original RegExp objects and String matching now parse the
previous live YouTube backslash failures. The independent VM also
executes compound assignment and comma sequences with verified call
argument boundaries. A live native YouTube probe advances from 7/9 to
**8 executed / 8 failed** scripts; viewport still has zero text. Next
semantic blockers: object-literal getters/setters, labeled statements,
`async function`, host DOM APIs, dynamic script scheduling and
native paint/layout. The regex subsystem is intentionally partial,
with unsupported ECMAScript patterns rejected rather than faked.

## Native DOM checkpoint (14 October 2026)

Native Document now collects orphan node components and reuses slots with
generation-checked identities. Live detached nodes, views, queries and pending
callbacks preserve their native ancestors/descendants. Sweeps wait for parser
completion and validated JS idle checkpoints; weak image/stylesheet/scroll
cache keys are removed, and native allocation headroom can trigger collection.
Native pauses and arena live/reserved counters are measured separately.
The 30-minute Windows and broad navigation/network stability gates remain open.
See [GC stress results](../docs/GC_STRESS_RESULTS.md).

## Historical priorities (14 October 2026, superseded above)

1. **P0-1 NOW: original JavaScript GC only.** G1/G2/G3 partial now provides browser-owned automatic mark/sweep, generation-safe slot reuse, weak AbortSignal.any follower ownership and GC pause/failure telemetry. A repeatable Windows native-engine timer probe measured 817 successful collections in 12 seconds with zero GC errors and bounded object/environment arenas. **Not a 30-minute memory proof.** Next: sustained RSS/pause measurement and remaining fetch/DOM/abort root audits. See [GC implementation design](https://github.com/Gerbesh/OPBrowser/blob/main/docs/GC_IMPLEMENTATION.md) and [stress procedure](https://github.com/Gerbesh/OPBrowser/blob/main/docs/GC_STRESS_RESULTS.md).
2. **P0-2 AFTER GC: rendering and scrolling.** Existing vertical and bounded horizontal overflow, partial Flexbox and native GDI tests stay intact. Horizontal thumb dragging, JS scrollLeft, positioned clips, Grid and real-page visual comparison resume only after GC passes safety and stability gates.
3. **P1: backgrounds and fonts.** Improve background positioning/sizing/clipping, then DirectWrite fallback/shaping and real origin-checked web font loading.
4. **P1: forms and interaction.** Keyboard editing, focus, selection, form controls and submission with native end-to-end tests.
5. **P2: broader Web APIs and ECMAScript.** Continue when P0 visible/memory gates pass, rather than spending another series of milestones on narrowly chosen Events assertions.

## Evidence and limits

Frozen WPT Static 197/200 exact (only 200 selected reftests), Positioning 54/100, Test262 Runtime v2 78/179 attempted with 110 explicitly skipped, and M4.35 WPT DOM/Events v8 21/21 attempted with 3 skips. These **do not combine into a website readiness percentage**. We still need a real-world visual/site corpus and long-session memory measurements.

See [active plan](https://github.com/Gerbesh/OPBrowser/blob/main/docs/PROJECT_PLAN.md), [Compatibility](https://github.com/Gerbesh/OPBrowser/blob/main/docs/COMPATIBILITY.md), and [Current Status](Current-Status.md) for verified status. Individual implementation updates belong in DEV_LOG and Git history, not the active plan.
