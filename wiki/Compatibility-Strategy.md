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

- **WPT Static v1 exact**: 200 pinned reftests, **197/200 (98.50%)**.
  A separate opt-in report applying only original WPT-authored
  `meta[name=fuzzy]` tolerances gives **198/200 (99%)**.
- **WPT Positioning v1**: 100 pinned position/layout reftests,
  **53/100 (53.00%)**. Vertical writing, sticky, multicol and
  complete positioning remain substantially incomplete.
- **Test262 Parser v1**: 2,000 pinned entries, 17 module cases skipped;
  **523/1983 (26.37%)** of executable parse expectations pass.
  This is **not** JavaScript runtime conformance.

The two remaining Rec.2020 cases use legacy color reference targets:
they remain **counted as failed** but are classified as known non-blocking
reference divergences against CSS Color 4 gamma 2.4. See
[Known Test Divergences](Known-Test-Divergences.md). The XYZ case is
accepted only in the opt-in source-authored fuzzy report.

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

For a visual comparison of a failing WPT test/reference pair, use the opt-in diagnostic
BMP dump rather than guessing at the pixel difference:

```powershell
cargo run -p op_browser --bin wpt_probe -- `
  target\compat-wpt compat\wpt-positioning-v1.tsv `
  --dump-failures target\wpt-debug
```

It saves a pair of top-down BGRA BMPs per failure, named with their manifest row number,
with a bounded maximum of twelve pairs per invocation. The normal CI path does not
save these bitmaps, and enabling dumps does not change any metric.

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
[docs/COMPATIBILITY.md](https://github.com/Gerbesh/OPBrowser/blob/main/docs/COMPATIBILITY.md).
