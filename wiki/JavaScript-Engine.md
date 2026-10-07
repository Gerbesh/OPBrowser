# JavaScript Engine

`op_js` is OPBrowser's original ECMAScript implementation. It does not embed V8,
SpiderMonkey, JavaScriptCore, QuickJS or another JavaScript engine.

The executable core contains an owned lexer, AST parser, bytecode compiler and stack
interpreter. The current language subset covers scalar literals, `let`/`const`/`var`
declarations including comma-separated declarators, identifier load/assignment, unary
`+`/`-`/`!`, arithmetic, comparisons, loose/strict equality, string concatenation,
short-circuit `&&`/`||`, blocks, `if/else`, `while`, `break` and `continue`.

Control flow compiles to patched bytecode jumps and the VM now runs with an explicit instruction
pointer. Every execution has an instruction budget, so a runaway loop is terminated with an
execution-limit error instead of monopolizing the renderer thread. Globals persist in one
`JsRuntime`; `const` bindings reject assignment.

This is still not page scripting. Block lexical environments are not implemented yet, so the
current declaration storage is global rather than full ECMAScript scope semantics. Objects and
prototypes, functions/closures, exceptions, garbage collection, built-ins, promises/modules and
DOM bindings are also still absent.

## Test262 measurement

`op_js` includes `test262_probe`. It measures only whether the current parser accepts
positive parse tests and rejects Test262 tests whose frontmatter explicitly expects a parse
error. It intentionally does not report those results as runtime conformance.

Example:

```powershell
cargo run -p op_js --bin test262_probe -- C:\src\test262\test --limit 2000
```

For the combined local subsystem check use:

```powershell
.\tools\compatibility.ps1 -Test262Path C:\src\test262\test -Test262Limit 2000
```

The unchanged Test262 Parser v1 subset moved from **364/1983 (18.36%)** to
**391/1983 (19.72%)** after the initial control-flow pass. This remains a parse-expectation metric,
not runtime conformance.

The next JS work is objects/properties/prototypes followed by functions, lexical environments and
calls. Those are the main blockers both for useful page scripting and for substantially broader
Test262 parsing/execution.
