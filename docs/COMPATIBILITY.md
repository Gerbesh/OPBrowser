# Compatibility measurement

Compatibility percentages must come from repeatable external suites, not milestone intuition or
project-owned regression counts. OPBrowser reports each metric with a named manifest version and
a pinned upstream revision.

## Local project baseline

Run the fast project-owned regression baseline with:

```powershell
.\tools\compatibility.ps1
```

This runs the HTML, CSS, layout, engine-integration and JavaScript project tests. Those counts
are regression coverage, not WPT or Test262 percentages.

## Versioned external subsets

Pinned revisions live in `compat/upstream.env`. The actual test lists are committed in
`compat/test262-parser-v1.txt`, `compat/wpt-static-v1.tsv` and
`compat/wpt-positioning-v1.tsv`, so two runs of the same commit measure the same tests even if
upstream repositories later change.

Run the external subsets with local checkouts:

```powershell
.\tools\compatibility.ps1 `
  -ExternalOnly `
  -Test262Path C:\src\test262\test `
  -WptPath C:\src\wpt
```

The script writes machine-readable JSON, Shields endpoint JSON and a Markdown summary under
`artifacts/compatibility/`.

### Test262 parser v1

The v1 manifest contains 2,000 deterministic paths sampled across `test/language/**/*.js` at
Test262 revision `c8c798898646638cd0c24879f8e0374e847e7d74`. Module tests encountered in
that sample are skipped because the current probe parses classic scripts only.

A test passes when a positive script is accepted or a Test262 negative parse test is rejected
during the expected parse phase. Runtime behavior, harness semantics, built-ins and module
execution are not measured by this score.

Initial baseline:

- 1,983 executable script tests;
- 364 parse expectations passed;
- 17 module tests skipped;
- **18.36% Test262 Parser v1**.

Measured progress on the unchanged v1 manifest:

- initial control-flow/parser pass: 391/1983, **19.72%**;
- object/array/member parser pass: 408/1983, **20.57%**;
- function/call/return parser pass: 504/1983, **25.42%**;
- broader control/update/exception parser pass: 508/1983, **25.62%**;
- this/new constructor parser pass: 523/1983, **26.37%**;
- current result: 523 passed, 1,460 failed expectations, 17 module tests skipped.

This must never be described as "26.37% JavaScript support" or full ECMAScript conformance.

### WPT positioning v1

The positioning v1 manifest adds 100 deterministic static reftests from CSS2 positioning/visual
formatting and CSS Positioned Layout at the same pinned WPT revision. It intentionally includes
families OPBrowser does not support yet, including sticky positioning, vertical writing modes and
multicol interactions, so the score is a useful readiness baseline rather than a flattering subset.

Initial baseline after the first absolute/fixed/relative layout pass was 18/100 (18.00%).
The first positioning-geometry follow-up reached 21/100 (21.00%), and the inline-static-position /
inline-block / replaced-SVG follow-up reached 25/100 (25.00%). After the split-inline continuation pass:

- 100 reftests checked;
- 36 passed;
- 64 failed;
- 0 render/infrastructure errors;
- **36.00% WPT Positioning v1**.

The implementation recognizes `position:absolute|fixed`, removes those boxes from normal flow,
uses the nearest positioned ancestor padding box or viewport as the containing block, preserves
relative-position visual offsets without moving following flow, and supports px/percentage insets on
all four sides when the corresponding axis is definite. It also supports bottom-only placement,
opposing-inset auto width/height stretching, shrink-to-fit positioned auto widths, direct
percentage-height resolution from definite block heights and zero-width inline markers for initial
static positions. Horizontal inline margins, including negative margins, now affect advance;
`display:inline-block` is an atomic initial BFC. Replaced image sizing can use intrinsic width,
height and ratio metadata from the initial bounded SVG raster slice as well as raster images. CSS2
block-inside-inline handling now creates continuation fragments, suppresses physical edges according
to logical LTR/RTL start/end, preserves required empty intermediate line boxes, and carries relative
inline visual offsets onto split block/float descendants. Large finite CSS lengths survive computed
style and are bounded at used layout geometry instead of being discarded. The 2026-10-08
non-replaced absolute/fixed margin pass now distributes single/both auto margins against definite
opposing insets on each axis, obeys RTL/LTR precedence for overconstrained horizontal values and
places negative auto-margin remainders on the appropriate side. Four project regression tests pass;
the unchanged pinned Positioning v1 suite remains at 36/100 (36.00%) with no regression. Sticky
positioning, full inline containing-block rectangles, complete bidi/vertical writing, multicol,
stacking contexts and complex/replaced abspos constraints remain deliberately visible gaps.

This new metric does not replace WPT Static v1. The older 200-test manifest remains frozen so its
43.00% -> 93.50% history stays directly comparable.

### WPT static v1

The v1 manifest contains 200 static HTML reftests at WPT revision
`97fe10c5d0e12e4a9d90f77b8db0602c64f3ad2d`. It samples CSS2 box display/model,
CSS Box, CSS Color, CSS Display and Selectors. Script/testharness.js tests, reftest-wait,
mismatch references and non-local references are excluded from this first slice.

`op_browser::wpt_probe` renders both the test and its reference at 800 x 600 through
`Engine::render_source`, the normal display-list pipeline and
`op_platform_win::render_display_list_to_bgra`. The offscreen path calls the same Win32 GDI
paint-command implementation as the visible browser window. The v1 score uses exact BGR pixel
comparison; alpha is ignored because the DIB is an opaque final surface.

Initial baseline:

- 200 reftests checked;
- 86 passed;
- 114 failed;
- 0 render/infrastructure errors;
- **43.00% WPT Static v1**.

This is a score for the named static subset, not full WPT conformance.

Measured progress on the unchanged v1 manifest:

- first CSS Color 4 pass: 126/200, **63.00%**;
- system-color/@supports and selector-semantics pass: 151/200, **75.50%**;
- deferred-color/display-contents/nth-grammar pass: 172/200, **86.00%**;
- relational-selector/empty-inline pass: 179/200, **89.50%**;
- table-structural `display:contents` pass: 181/200, **90.50%**;
- grapheme-aware `::first-letter` pass: 182/200, **91.00%**;
- self-collapsing block-in-inline margin pass: 183/200, **91.50%**;
- flow-root/BFC/float/visibility pass: 185/200, **92.50%**;
- initial flex formatting/display-contents pass: 187/200, **93.50%**;
- current result: 187 passed, 13 failed, 0 render/infrastructure errors.

The manifest and upstream revision are unchanged, so 43.00% -> 63.00% -> 75.50% -> 86.00% -> 89.50% -> 90.50% -> 91.00% -> 91.50% -> 92.50% -> 93.50% is directly comparable. Two pinned Rec.2020 tests still encode the older piecewise Rec.2020 transfer expectation; OPBrowser follows the current 2026 CSS Color 4 BT.1886 gamma-2.40 definition instead of special-casing those tests. Near-zero OKLab/OKLCH failures are left for real gamut mapping rather than threshold hacks. The v1 harness also intentionally retains exact BGR comparison, so WPT fuzzy metadata does not silently change historical scoring semantics.

## GitHub Actions and public metrics

The CI workflow has separate responsibilities:

1. `Windows / Rust` gates formatting, generated data, Clippy, workspace tests, native smoke
   tests and the release build.
2. `Compatibility / WPT + Test262` fetches only the pinned upstream revisions, runs Test262 Parser
   v1 plus WPT Static v1 and WPT Positioning v1, and uploads the JSON/Markdown results as a workflow
   artifact.
3. After a successful push to `main`, `Publish compatibility badges` copies only the badge
   endpoint JSON and summary to the `metrics` branch. Pull-request jobs remain read-only.

README badges load their values from the `metrics` branch. Conformance failures inside the
subset do not make CI red; runner, checkout, manifest or rendering infrastructure failures do.
The percentage is a development target rather than an artificial requirement that unfinished
features already pass.

## Refresh policy

Do not silently change an existing metric. If only the pinned upstream revision is refreshed
while selection semantics stay the same, regenerate the manifests with
`tools/build_compat_manifests.py` and record the revision change. If scope or sampling semantics
change, create a new manifest version such as v2 so historical percentages remain comparable.

See `compat/README.md` for the exact generator command.

## Future expansion

The next compatibility stages are:

- expand static WPT categories from measured failures, with positioning now tracked separately;
- add Test262 runtime execution once the ECMAScript runtime/harness surface exists;
- add WPT testharness.js support after DOM scripting and required Web APIs exist;
- eventually add a full browser automation adapter instead of presenting a selected static
  subset as the whole platform.

HTML5test remains a product-level feature-detection target. APIs must only be exposed after their
observable behavior exists; stubs added only to increase a score are forbidden.

Compatibility work is tracked beside startup time, private memory, retained-tab memory and
page-load/reflow timing. A higher score obtained by making the browser pathologically heavy is
still an engineering regression.
