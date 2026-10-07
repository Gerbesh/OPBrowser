# Compatibility Strategy

OPBrowser does not treat one synthetic score as proof of compatibility. External conformance,
project regressions and product-level feature detection answer different questions.

Targets:

- HTML5test: product feature-detection target;
- Web Platform Tests: primary HTML/CSS/DOM/Web API conformance suite;
- TC39 Test262: ECMAScript conformance suite;
- project-owned regression tests: fast guardrails for bugs already understood.

Feature-detection tests must not be gamed by exposing non-functional APIs. A feature only counts
when its observable behavior exists.

## Current measurement

The project now has two reproducible external metrics rather than milestone guesses:

- **WPT Static v1**: 200 pinned static HTML/CSS reftests rendered at 800 x 600 through the
  ordinary OPBrowser engine/display-list/GDI path. Initial baseline: **86/200, 43.00%**; current
  result after self-collapsing block-in-inline margin propagation: **183/200, 91.50%**.
- **Test262 Parser v1**: a 2,000-path deterministic Test262 language sample. Module entries are
  skipped until module parsing is supported. Initial executable baseline: **364/1983, 18.36%**.

The Test262 number is parse-only. It is not a JavaScript runtime percentage. The WPT number is a
named static subset, not a full-platform WPT percentage.

Pinned upstream commits and manifests live under `compat/`. Run both local external measurements
with:

```powershell
.\tools\compatibility.ps1 `
  -ExternalOnly `
  -Test262Path C:\src\test262\test `
  -WptPath C:\src\wpt
```

The normal `.\tools\compatibility.ps1` command continues to run project-owned subsystem tests.

## CI publication

GitHub Actions runs the pinned WPT/Test262 subsets separately from the normal Windows build.
Results are uploaded as artifacts. Successful pushes to `main` publish only badge endpoint JSON
and the summary to the `metrics` branch, which feeds the README badges. Pull requests do not
receive repository write permission for this publishing step.

A low conformance percentage is not itself a CI failure while the engine is under construction.
Infrastructure failures are. The metric exists so agents and developers can choose failing
families, make an implementation change and show an objective before/after delta.

## Expansion

Static WPT coverage should grow from measured failures. Test262 runtime scoring waits for the
runtime/harness and built-ins needed to execute it honestly. WPT testharness.js coverage waits for
DOM scripting and the required Web APIs. When a metric changes scope or sampling semantics, create
a new version instead of silently redefining the old percentage.

The detailed command contract, exact revisions and non-goals live in
[docs/COMPATIBILITY.md](../docs/COMPATIBILITY.md).
