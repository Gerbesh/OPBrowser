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
- LATER: planned but intentionally deferred.

## M0 - Foundation

Status: DONE at initial level.

- DONE Rust workspace under C:\OPBrowser.
- DONE Native Win32 top-level browser window.
- DONE Separate browser, engine, DOM, HTML, CSS, JS, network, and Windows crates.
- DONE Git repository.
- DONE rustfmt / Clippy / test workflow.
- DONE debug and release builds.
- DONE non-interactive Win32 startup smoke test.
- DONE documented dependency policy.
- DONE project documentation workflow, code graph, code slices, and local wiki.
- DONE public GitHub repository: https://github.com/Gerbesh/OPBrowser
- DONE GitHub Actions Windows CI and continuous commit/push workflow.

## M1 - First static document pipeline

Goal: own bytes -> own HTML parser -> own DOM -> own layout -> own paint -> pixels.

- DONE initial HTML tokenizer state machine.
- DONE initial DOM arena with stable NodeId values, attributes, and parent/child relationships.
- DONE initial HTML tree builder from tokenizer output into op_dom::Document.
- IN PROGRESS document-to-layout pipeline.
- NEXT minimal block/inline layout.
- NEXT paint/display-list primitives.
- NEXT Win32 surface painting of engine-generated content.
- NEXT local file/data document loading.
- NEXT URL parser and navigation state.
- NEXT HTTP(S) fetching.
- NEXT basic image loading.
- NEXT first useful static HTML page.

Exit condition: OPBrowser renders a non-trivial local HTML document using only its
own HTML/DOM/layout/paint pipeline.

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
