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

The project now has three reproducible external metrics rather than milestone guesses:

- **WPT Static v1**: 200 pinned static HTML/CSS reftests rendered at 800 x 600 through the
  ordinary OPBrowser engine/display-list/GDI path. Initial baseline: **86/200, 43.00%**; current
  result after the initial flex formatting/display-contents pass: **187/200, 93.50%**.
- **WPT Positioning v1**: 100 pinned static reftests from CSS2 positioning/visual formatting and
  CSS Positioned Layout. Initial baseline after the first absolute/fixed/relative pass:
  **18/100, 18.00%**; current result after viewport/definite-height geometry, inline static-position,
  inline-block/replaced sizing and the CSS2 split-inline continuation pass: **36/100, 36.00%**.
  Unsupported sticky, multicol, full inline-containing-block rectangles, stacking and vertical-writing
  cases remain in the sample.
- **Test262 Parser v1**: a 2,000-path deterministic Test262 language sample. Module entries are
  skipped until module parsing is supported. Initial executable baseline: **364/1983, 18.36%**;
  current parser result: **523/1983, 26.37%**.

The Test262 number is parse-only. It is not a JavaScript runtime percentage. Each WPT number is a
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

WPT Static v1 stays frozen for historical comparability while Positioning v1 exposes the broader
layout gap. Additional static families should get separately versioned manifests rather than
inflating the old score. Test262 runtime scoring waits for the
runtime/harness and built-ins needed to execute it honestly. WPT testharness.js coverage waits for
DOM scripting and the required Web APIs. When a metric changes scope or sampling semantics, create
a new version instead of silently redefining the old percentage.

The detailed command contract, exact revisions and non-goals live in
[docs/COMPATIBILITY.md](../docs/COMPATIBILITY.md).
