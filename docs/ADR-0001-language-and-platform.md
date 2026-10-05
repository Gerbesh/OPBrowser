# ADR-0001: Language and platform

Status: Accepted for project start

## Decision

Use **Rust** as the primary implementation language for OPBrowser.

Use Windows 11 as the first supported desktop platform, targeting x86-64 first and
ARM64 later.

Use the MSVC Windows toolchain and direct Windows APIs where appropriate.

## Why Rust

### Memory safety

A browser engine parses hostile, attacker-controlled HTML, CSS, JavaScript, images,
fonts and network traffic. Memory corruption in this class of software is a major
security risk. Rust removes broad categories of use-after-free, dangling-pointer and
data-race bugs from normal safe code.

### Native performance

Rust produces native code without a managed runtime or mandatory garbage collector
for the browser itself. That fits the low-overhead goal.

### Concurrency

The engine will need parallel networking, workers, raster work, process IPC and
background scheduling. Rust makes ownership and thread-safety much more explicit.

### Windows integration

Rust can call Win32, Direct3D, DirectWrite, DirectComposition and other Windows APIs
without requiring a heavyweight application framework.

### One core language

Keeping the browser, web engine, JavaScript VM, scheduler and most platform
integration in one language reduces FFI boundaries and makes refactoring easier.

## Why not C++ as the primary language

C++ provides excellent performance and ecosystem access, but accidental memory
unsafety is substantially easier in exactly the hostile-input code a browser runs.

C++ remains acceptable only at unavoidable platform/library interop boundaries.

## Why not C#

C# is productive for Windows application code, but a fully independent browser
engine would still need substantial native interop for low-level rendering,
sandboxing, JIT work and some performance-sensitive paths. The managed runtime also
reduces low-level control over memory behavior across the whole engine.

## Why not Zig

Zig offers excellent low-level control and C interop, but currently provides less
memory-safety enforcement and a smaller ecosystem for a project of this scope.

## Windows rendering/platform direction

Initial stack:

- Win32 for top-level window/process integration;
- DirectWrite for font integration where suitable;
- Direct2D and/or a project-owned display-list/raster path for early rendering;
- Direct3D 12 / DirectComposition for GPU composition as the engine matures.

Using operating-system graphics APIs does not violate the original-engine rule.
HTML/CSS parsing, style, layout, painting decisions, DOM, JavaScript and Web APIs
remain project-owned.

## JavaScript

The JavaScript engine will also be original and implemented in Rust.

Initial execution strategy:

1. lexer;
2. parser / AST;
3. bytecode compiler;
4. bytecode interpreter;
5. garbage collector;
6. ECMAScript built-ins;
7. modules/promises/async;
8. Web IDL bindings;
9. optional JIT only after correctness and profiling justify it.

Correctness and Test262 conformance come before JIT complexity.

## Consequences

Advantages:

- memory-safe default;
- native performance;
- strong concurrency model;
- no mandatory heavyweight runtime;
- good fit for security-sensitive parsing and execution.

Costs:

- some Windows and graphics APIs require unsafe FFI boundaries;
- implementing the modern web platform from scratch is a very large undertaking;
- the JavaScript engine and CSS/layout compatibility will dominate early engineering;
- full modern-web compatibility is continuous work rather than a finite checklist.

## Rule for exceptions

Any proposal to embed an existing browser engine or JavaScript engine requires
changing this ADR and explicitly abandoning the original-engine constraint.
