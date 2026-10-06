# Compatibility Strategy

OPBrowser does not treat one synthetic score as sufficient proof of compatibility.

Targets:

- HTML5test: product feature-detection target;
- Web Platform Tests: primary HTML/CSS/DOM/Web API conformance suite;
- TC39 Test262: ECMAScript conformance suite;
- project-owned regression tests: fast guardrails for bugs already understood.

Feature-detection tests must not be gamed by exposing non-functional APIs. A feature only
counts when its observable behavior exists.

## Current measurement

Run the owned subsystem baseline with:

```powershell
.\tools\compatibility.ps1
```

The script runs the HTML, CSS, layout, engine and JavaScript project tests. These numbers
measure regression coverage only and are not converted into a WPT percentage.

The restored Windows GitHub Actions workflow also runs this baseline, making regressions visible
on public pushes and pull requests without pretending that project-owned tests are external-suite
conformance percentages.

A local Test262 checkout can be supplied to the same script. The current
`op_js::test262_probe` is intentionally parse-only: it checks positive parse acceptance and
negative parse-error expectations. Runtime Test262 percentage is not reported yet because
objects, functions, built-ins and harness semantics are not implemented.

WPT automation remains the next external-suite integration target. Start with static
HTML/CSS tests that can be driven without testharness.js, then add harness support once DOM
scripting exists.

The detailed command contract and non-goals live in
[docs/COMPATIBILITY.md](../docs/COMPATIBILITY.md).

Compatibility is developed subsystem by subsystem. Regressions receive project-owned tests
in addition to external conformance coverage, and performance/memory metrics are tracked
beside compatibility rather than after it.
