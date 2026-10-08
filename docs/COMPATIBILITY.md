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

### Inspecting failed WPT reftests

The `wpt_probe` binary can optionally save original test/reference raster pixels for failing
reftests without adding an image encoder dependency:

```powershell
cargo run -p op_browser --bin wpt_probe -- `
  target\compat-wpt compat\wpt-positioning-v1.tsv `
  --dump-failures target\wpt-debug
```

The `--dump-failures` switch writes Windows-compatible top-down 32-bit BGRA BMP files,
paired as `case-0001-actual.bmp` / `case-0001-reference.bmp`, into the named directory.
The numbers refer to manifest row positions, so the pair can be mapped back to source.
Dumping is explicitly opt-in, limited to the first **12** rendered failures per run,
and excludes files with render errors. It is disabled in normal CI to avoid storing
hundreds of megabytes of raw bitmaps. The score and failure threshold are unchanged.
To isolate a later test, create a temporary one-row manifest and run the same command.

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
- 38 passed;
- 62 failed;
- 0 render/infrastructure errors;
- **38.00% WPT Positioning v1**.

The next foreground-paint slice adds positioned block/inline decoration and text/image
phases. Out-of-flow absolute/fixed content now overlays normal-flow text, and standalone
relative table captions can cover earlier absolute siblings. Position-relative parents
with nested absolute children preserve their in-flow text underneath those descendants.
The frozen suite scores remain unchanged (WPT Static 187/200, Positioning 38/100,
Test262 Parser 523/1983). The next pass adds atomic positioned block contexts
using parent paint groups, corrects negative root stacking relative to normal
block backgrounds, and uses final DOM preorder for same-level ties after HTML
foster parenting. Pinned WPT Static and Positioning remain 187/200 and 38/100,
both with zero render errors. The subsequent inline pass tags relative inline
fragments and text/images with paint groups, handles nested inline z-index and
auto-z descendants, and fixes cross-context inline-block arena indices and
parent/atomic background paint order. Its five end-to-end regressions pass,
and the same frozen WPT Static/Positioning scores remain 187/200 and 38/100
with zero render errors. Full CSS stacking still requires remaining auto-z
nuances, positioned atomic-inline owner cases, additional context triggers
and complete paint phases. The next relative-auto pass moves the parent's own
paint into the zero-level source-ordered group even with positioned children;
explicit children remain independent and negative children paint underneath
the parent's background. Five regression tests cover block/inline-block same-z
ties, positive and negative children, explicit inline-block atomic isolation
and reflow. Frozen WPT Static is still 187/200 and Positioning still 38/100
with zero render errors; remaining auto-z paint-phase interactions are not
fully conformant. The subsequent table-positioning slice supports relative
offsets on tbody/thead/tfoot/tr/td and the containing-block origin for
absolute descendants of positioned table parts. It also paints positioned
table-row/section backgrounds, suppresses backgrounds on effectively empty
absolute-only row tracks, and uses intrinsic pixel widths from block descendants
for auto table columns. On the unchanged manifests WPT Positioning improves
from 38/100 to 53/100 (53.00%) while WPT Static remains 187/200 (93.50%);
both suites have zero render errors. The next auto-table wrapper correction
aligns painted table backgrounds/borders and caption widths with intrinsic
column tracks for width:auto, including horizontal border-spacing, table
padding and border. Explicit widths, minimum widths and fixed table-layout
with automatic width have engine regressions. Frozen WPT Static remains
187/200 (93.50%), Positioning remains 53/100 (53.00%), zero render errors:
the new work fixes additional cases outside the frozen subsets. Full table
sizing, percent insets, row spans, and multi-column group painting remain
incomplete. The next rowspan reconciliation slice treats spanning cell
minimum heights as constraints across all covered row tracks, instead of
charging the height entirely to the originating row. Later cells move with
the accumulated extra track heights; vertical border-spacing and overlapping
spans are covered by dedicated regressions. Frozen WPT Static stays
187/200 and Positioning stays 53/100, both with zero render errors. CSS
row-height distribution, row-group painting and baseline corner cases remain
partial. Subsequent WPT Static improvements now include limited near-black
OKLab/OKLCH gamut correction, direct select text suppression, and basic SVG
definitions/use and display:contents paint filtering. With the exact same
frozen Static v1 manifest, 191/200 (95.50%) now pass, up from 187/200 (93.50%);
Positioning remains 53/100 (53.00%). Neither suite has render errors.
The nine remaining static failures concern ICC color profiles or tagged
images, color-mix(:visited) with history state, composited filters and
opacity, first-line currentcolor/shadows, Rec.2020 color references,
XYZ fractional precision/reftest fuzziness and bidi first-line painting.
W3C CSS Color 4 now specifies gamma 2.4 for Rec.2020, while frozen
reference values in two cases appear to target an older transfer function:
they were not modified or made to pass through special-case constants.
The WPT probe remains pixel-exact and neither the manifest nor its thresholds
have changed. The first composited effects pipeline supports CSS opacity
and one invert() filter expression. It isolates nested groups into bounded
Win32 offscreen buffers and merges each group's internal overlapping
children once into its parent. A prior failure involving two invert-filtered
absolute siblings beneath a half-opacity container now passes:
composited-filters-under-opacity.html. On the unchanged manifests Static
is 192/200 (96.00%), Positioning 53/100 (53.00%), both with zero render
errors. Remaining eight static failures involve ICC profiles and image
backgrounds, :visited history colors, first-line shadows/currentcolor and
bidirectional text, color conversion rounding/precision and two Rec.2020
references with pre-2026 transfer expectations. The effect implementation
does not yet cover all filters/opacity contexts, color-managed compositing,
gradient/shadow effects or exact antialiasing. The subsequent image
background/ICC pass supports one URL background layer for ordinary blocks,
tables and cells, with URL resolution based on CSS stylesheet origin,
bounded loading and intrinsic-size repeat tiling clipped to the decoration.
The Windows WIC decoder explicitly transforms embedded PNG ICC color
contexts into output sRGB before premultiplication. A separate known
self-document href="" special case permits :visited color-mix tests to use
the existing currentcolor interpolation, without consulting other history.
On the original unchanged exact-pixel Static v1 manifest this improves
192/200 -> 194/200 (97.00%): both color-mix-currentcolor-visited.html
and tagged-images-004.html now pass. Positioning remains 53/100 (53%).
The six remaining Static failures are at-color-profile-001, currentcolor-003,
predefined-012, rec2020-001, xyz-003 and first-line-bidi-002. The xyz-003
author-declared fuzzy allowance is still NOT applied by our exact-pixel
probe; tolerance settings and manifest remain unchanged. Full CSS
background layers, general ICC CSS5 @color-profile, link history and bidi
are pending. A subsequent bounded custom-color profile slice now parses
@color-profile --name { src:url(...) } in inline and linked CSS, loads ICC
bytes with origin-relative resource resolution, and converts three numeric
or percentage color(--name R G B) components through Windows WIC's native
ICC engine before normal computed-style evaluation. Stylesheet strings
and unknown profiles are preserved, and repeated conversions cached.
The original frozen at-color-profile-001.html now passes, increasing
Static to 195/200 (97.50%) from 194/200. Positioning remains 53/100
without render errors. The five remaining Static failures are
currentcolor-003, predefined-012, rec2020-001, xyz-003 and
first-line-bidi-002. The XYZ WPT fuzzy metadata is still not used
in the exact-pixel score, and no references or tolerances were changed.
General Color 5 color-profile alpha/CMYK and history-dependent styling
remain unsupported.

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
the unchanged pinned Positioning v1 suite remains at 36/100 (36.00%) with no regression.
The next inline-containing-block pass preserves unstyled relative inline ancestors in the line
box arena, records first/last fragment padding-edge rectangles within a formatting run, handles
LTR/RTL edge choice, and applies relative visual offsets to inline paint and absolute descendants.
Five initial inline containing-block regressions pass. A further pass accumulated
fragment geometry across separate block/line formatting runs, deferred absolute layout
until the containing inline fragments were complete, resolved nested pending subtrees,
and corrected horizontal RTL static positioning against hypothetical flow width.
Eight more deterministic layout regressions pass; WPT Positioning v1 remains
**36/100 (36.00%)** without losing old passes, WPT Static v1 remains **187/200 (93.50%)**,
and the Test262 Parser v1 sample remains **523/1983 (26.37%)**. Sticky, vertical
writing, multicol, stacking contexts and complex/replaced abspos constraints remain gaps.

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
