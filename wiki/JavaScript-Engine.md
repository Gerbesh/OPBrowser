# JavaScript Engine

`op_js` is OPBrowser's original ECMAScript implementation. It does not embed V8,
SpiderMonkey, JavaScriptCore, QuickJS or another JavaScript engine.

The first executable slice contains an owned lexer, AST parser, bytecode compiler and stack
interpreter. The deliberately small current language subset covers scalar literals,
`let`/`const`/`var` single declarations, identifier load/assignment, unary
`+`/`-`/`!`, arithmetic, comparisons, loose/strict equality and string concatenation.
Globals persist in one `JsRuntime`; `const` bindings reject assignment.

This is not yet page scripting. Objects/prototypes, functions/closures, exceptions, garbage
collection, built-ins, promises/modules and DOM bindings are still absent.

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

The next JS work expands syntax and values while keeping Test262 measurement attached from
the start, instead of building a large interpreter and discovering semantic errors late.
