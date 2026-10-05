# OPBrowser Requirements

Status: initial baseline  
Target OS: Windows 11  
Initial architecture target: x86-64  
Future architecture target: ARM64

## 1. Product goal

Build a lightweight, fast and privacy-oriented desktop web browser for Windows 11
using an original browser engine rather than embedding or forking an existing web
engine. The browser must target the real modern public web, not only a small
HTML/CSS subset.

## 2. Hard constraint: original engine

OPBrowser MUST NOT embed, fork, wrap or depend on an existing browser engine for
page execution or rendering.

Forbidden as engine dependencies:

- Chromium / Blink;
- WebKit;
- Gecko;
- Servo as a ready-made rendering engine;
- Edge WebView2 / CEF;
- V8;
- SpiderMonkey;
- JavaScriptCore;
- QuickJS or another ready-made ECMAScript engine.

Allowed dependencies may include Windows APIs, graphics APIs, cryptography/TLS,
codecs, Unicode/locale data, compression, test frameworks and general-purpose
utility libraries.

Allowed dependencies MUST NOT replace the project's own HTML parser, CSS parser,
DOM, style system, layout engine, painter/compositor, JavaScript engine or web API
implementation.

## 3. Performance

OPBrowser SHOULD prioritize:

- fast cold and warm startup;
- low idle and background CPU usage;
- low memory overhead per tab;
- bounded cache growth;
- responsive input and scrolling;
- lazy initialization of expensive subsystems;
- minimal background services;
- measurable performance budgets.

Performance claims MUST eventually be backed by repeatable benchmarks.

## 4. Privacy and telemetry

- No advertising telemetry.
- No hidden analytics.
- No unique installation identifier unless a future feature strictly requires one.
- Optional diagnostics/crash reporting must be explicit and user-controllable.
- Browser features must work without a telemetry backend.
- Browser-initiated network requests must be documented.

## 5. Web compatibility

The long-term target is the current web platform, including at minimum:

- HTML Living Standard;
- DOM;
- modern CSS;
- ECMAScript and ECMA-402;
- Fetch, URL, Encoding, Streams and related WHATWG APIs;
- Web Storage and IndexedDB;
- cookies and modern origin/site security rules;
- History and Navigation APIs;
- Workers and Service Workers;
- WebSocket;
- SVG;
- MathML;
- Canvas 2D;
- WebGL;
- WebGPU;
- WebAssembly;
- media elements and Media Source Extensions where practical;
- forms, accessibility semantics and text editing;
- clipboard, drag-and-drop and pointer/keyboard input;
- modern image formats required by the public web.

Because web standards are living standards, "all modern specifications" is a moving
compatibility target rather than a one-time checkbox.

## 6. Conformance targets

### 6.1 HTML5test

OPBrowser MUST target full support for every feature currently tested by:

https://html5test.co/

No result may be faked by merely exposing a non-functional API.

### 6.2 Web Platform Tests

Primary cross-browser web-platform conformance suite:

https://web-platform-tests.org/

Requirements:

- integrate WPT into automated testing;
- record pass/fail counts by subsystem;
- prevent unexplained regressions;
- progressively approach current interoperable browser behavior;
- treat WPT, not HTML5test alone, as the main web-platform quality signal.

### 6.3 ECMAScript

The original JavaScript engine MUST be tested against TC39 Test262:

https://github.com/tc39/test262

Passing Test262 is a mandatory long-term target for supported ECMAScript features.

### 6.4 Project regression tests

Every parser, layout feature, DOM behavior, security rule and crash fix SHOULD gain
a focused regression test.

## 7. Engine subsystems

The engine is expected to contain independent modules for:

1. URL parsing and navigation.
2. HTTP(S), caching, cookies and content encoding.
3. HTML tokenizer and tree builder.
4. DOM and event system.
5. CSS tokenizer/parser and CSSOM.
6. Selector matching, cascade and computed style.
7. Layout.
8. Text shaping and font handling.
9. Painting and display lists.
10. GPU composition.
11. Images and media.
12. JavaScript lexer/parser.
13. JavaScript bytecode/interpreter.
14. Garbage collector.
15. JavaScript JIT only after correctness is mature.
16. Web IDL bindings.
17. Web APIs.
18. Workers/task queues/event loops.
19. Storage.
20. Accessibility.
21. Sandbox and process isolation.

## 8. Security architecture

Security is a hard requirement from the start.

The browser SHOULD use a multi-process architecture separating at least:

- browser/UI process;
- network service;
- renderer/page processes;
- GPU process where useful;
- utility/media processes where useful.

Renderer processes SHOULD run with reduced privileges and explicit IPC boundaries.

Required security concepts include:

- same-origin policy;
- site/origin isolation strategy;
- sandboxing;
- Content Security Policy;
- CORS;
- CORP / COEP / COOP;
- secure context rules;
- certificate validation;
- mixed-content handling;
- permission model.

## 9. Tab lifecycle and memory management

OPBrowser MUST include intelligent tab lifecycle management.

Tab states:

- Active;
- Background;
- Throttled;
- Frozen;
- Discarded;
- Restoring.

The browser must be able to discard inactive tabs from RAM while preserving enough
state to restore them.

The discard policy SHOULD consider:

- system memory pressure;
- tab recency;
- playing audio/video;
- downloads/uploads;
- active WebRTC;
- unsaved form state;
- pinned tabs;
- foreground/background state;
- page memory footprint;
- user-configured exclusions.

Tabs must not be discarded solely because they are old when doing so would destroy
important active work.

## 10. Built-in task manager

The browser MUST expose at least:

- tab/frame/worker identity;
- process ID;
- CPU usage;
- RAM usage;
- GPU memory where available;
- network activity;
- lifecycle state;
- ability to terminate a misbehaving tab/process;
- ability to manually freeze or discard a tab.

## 11. Content blocking / ad blocking

The browser MUST expose a native request-filtering layer before resource loading.

The design SHOULD support:

- host/domain blocking;
- URL pattern rules;
- resource-type filtering;
- first-party/third-party conditions;
- cosmetic filtering;
- safe scriptlet-like extensions if implemented;
- importing common filter-list formats where practical;
- per-site allowlists;
- per-site blocking statistics.

Network-level blocking must live below page JavaScript so it cannot be trivially
bypassed by the page.

## 12. User interface

Initial Windows 11 UI requirements:

- native top-level window;
- tab strip;
- address/search bar;
- back/forward/reload/stop;
- new tab;
- downloads;
- history;
- bookmarks;
- settings;
- task manager;
- site permissions indicator;
- content-blocking indicator.

The UI layer must be separate from the web engine so the engine can be tested
headlessly.

## 13. Dependency policy

Every third-party dependency must be reviewed for purpose, license, maintenance,
binary/runtime cost, telemetry/network behavior and whether it compromises the
original-engine requirement.

Large framework dependencies SHOULD be avoided when a small focused dependency or
Windows API is sufficient.

## 14. Build and development

- Primary OS: Windows 11.
- Primary compiler/toolchain: stable Rust + MSVC ABI/toolchain.
- Build system: Cargo.
- Source control: Git.
- Automated tests must run locally from the repository.
- Debug builds should expose engine diagnostics without requiring telemetry.
- Release builds must allow diagnostics to be disabled.

## 15. Initial milestones

### M0 - Foundation

- repository structure;
- buildable Windows executable;
- native browser window;
- engine library separated from UI;
- logging/tracing policy;
- test harness;
- basic process and IPC design.

### M1 - Static document engine

- URL loading;
- HTTP(S);
- HTML parsing;
- DOM;
- basic CSS;
- block/inline layout;
- text and image rendering;
- links and navigation.

### M2 - Scripting

- original ECMAScript parser and interpreter;
- garbage collector;
- DOM bindings;
- events;
- timers;
- progressive Test262 coverage.

### M3 - Modern page platform

- Fetch;
- storage;
- workers;
- canvas;
- SVG;
- forms;
- more complete CSS;
- progressive WPT coverage.

### M4 - Browser product features

- task manager;
- tab freezing/discarding;
- ad blocker;
- history/bookmarks/downloads;
- permissions;
- session restore.

### M5 - Advanced compatibility

- WebAssembly;
- WebGL;
- WebGPU;
- advanced media;
- accessibility;
- aggressive conformance work against WPT, Test262 and HTML5test.

## 16. Definition of success

OPBrowser succeeds when it is not a custom shell around another engine, but a
genuinely independent implementation capable of rendering and executing real modern
websites with measurable standards conformance, reasonable security, low overhead
and predictable resource management.
