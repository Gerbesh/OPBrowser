# OPBrowser Project Plan

Last updated: 2026-10-05

## North star

Build an independent, lightweight, privacy-oriented Windows 11 browser with an
original web engine and original ECMAScript engine, while progressively reaching
modern web-platform conformance.

Primary conformance targets:

- HTML5test feature coverage: full target, never faked.
- Web Platform Tests: primary browser-platform conformance signal.
- TC39 Test262: primary ECMAScript conformance signal.

## Status legend

- DONE: implemented and verified at the current milestone level.
- IN PROGRESS: active engineering work.
- NEXT: queued immediately after active work.
- BLOCKED: configured or implemented, but prevented by an external dependency/state.
- LATER: planned but intentionally deferred.

## M0 - Foundation

Status: DONE at initial level.

- DONE Rust workspace under C:\OPBrowser.
- DONE Native Win32 top-level browser window.
- DONE Separate browser, engine, DOM, HTML, CSS, layout, paint, JS, network, and Windows crates.
- DONE Git repository.
- DONE rustfmt / Clippy / test workflow.
- DONE debug and release builds.
- DONE non-interactive Win32 startup/paint smoke test.
- DONE documented dependency policy.
- DONE project documentation workflow, code graph, code slices, and local wiki.
- DONE public GitHub repository: https://github.com/Gerbesh/OPBrowser
- DONE continuous Git commit/push workflow to public main.
- BLOCKED GitHub Actions Windows CI execution: the workflow is configured, but
  GitHub currently refuses to start jobs because the account is locked due to a billing issue.

## M1 - First static document pipeline

Goal: own bytes -> own HTML parser -> own DOM -> own layout -> own paint -> pixels.

- DONE initial HTML tokenizer state machine.
- DONE initial DOM arena with stable NodeId values, attributes, and parent/child relationships.
- DONE initial HTML tree builder from tokenizer output into op_dom::Document.
- DONE initial document-to-layout pipeline with basic text flow and wrapping.
- DONE initial paint/display-list primitives.
- DONE Win32 GDI backend consuming OPBrowser display-list commands.
- DONE first in-memory HTML page rendered by the complete OPBrowser pipeline.
- DONE smoke test verifies WM_PAINT actually ran.
- DONE local filesystem document loading.
- DONE file: URL loading with percent decoding.
- DONE data:text/html URL loading with percent and base64 decoding.
- DONE startup source argument wired through op_net -> engine -> renderer.
- DONE navigation history core with navigate/back/forward/reload.
- DONE failed navigations leave history unchanged.
- DONE new navigation after Back discards the old forward branch.
- DONE post-startup native address input, Go/Back/Forward/Reload, display replacement.
- DONE initial owned HTTP(S) URL parser (ASCII hosts, ports, IPv6, UTF-8 paths/query).
- DONE bounded HTTP(S) HTML fetching using WinHTTP transport and system TLS/proxy.
- DONE worker-thread loading with visible loading/errors and history rollback.
- DONE mouse-wheel scrolling and block defaults inside structural HTML containers.
- DONE external https://example.com navigation + native repaint verified.
- DONE clickable text hyperlinks with measured hit regions and scroll-aware input.
- DONE relative HTTP(S)/local-file link resolution using the loaded document base.
- DONE link input -> worker -> load -> history -> pixels smoke coverage.
- NEXT broader HTML decoding/charset and URL conformance.
- NEXT basic image loading.
- NEXT richer block/inline layout behavior.

Exit condition: OPBrowser renders a non-trivial local HTML document using only its
own HTML/DOM/layout/paint pipeline. Achieved at the initial M1 level. The current
iteration also opens external UTF-8 HTML sites through the address bar; full CSS,
images, scripting, and modern-site compatibility remain later milestones.

## M2 - CSS foundation

- LATER CSS tokenizer/parser.
- LATER selectors and matching.
- LATER cascade and inheritance.
- LATER computed values.
- LATER box model.
- LATER normal flow block layout.
- LATER inline formatting and line breaking.
- LATER fonts/text shaping integration.
- LATER progressively expand CSS WPT coverage.

## M3 - Original JavaScript engine

- LATER ECMAScript lexer.
- LATER parser/AST.
- LATER bytecode format/compiler.
- LATER bytecode interpreter.
- LATER values/objects/prototypes.
- LATER garbage collector.
- LATER functions/closures.
- LATER exceptions.
- LATER promises/microtasks.
- LATER modules.
- LATER standard built-ins.
- LATER Test262 harness and progressive conformance.
- LATER JIT only if profiling justifies it after correctness.

## M4 - DOM scripting and Web APIs

- LATER Web IDL binding layer.
- LATER DOM mutation/events.
- LATER timers and event loop.
- LATER Fetch.
- LATER URL/Encoding/Streams.
- LATER forms/editing.
- LATER storage.
- LATER workers/service workers.
- LATER WebSocket.
- LATER Canvas/SVG/MathML.

## M5 - Browser product architecture

- LATER browser/renderer/network process separation.
- LATER renderer sandbox.
- LATER process/site isolation policy.
- LATER tab model and session restore.
- LATER lifecycle states: active/background/throttled/frozen/discarded/restoring.
- LATER intelligent memory-pressure tab discarding.
- LATER built-in task manager.
- LATER downloads/history/bookmarks/settings/permissions.
- LATER native request-filter layer and ad-block list support.

## M6 - Advanced platform

- LATER WebAssembly.
- LATER WebGL.
- LATER WebGPU.
- LATER media pipeline.
- LATER accessibility tree.
- LATER advanced security/isolation.
- LATER performance and power optimization.

## Continuous work

Every milestone continuously tracks security, WPT/Test262 regressions, startup time,
memory use, binary size, background CPU, and dependency growth.
