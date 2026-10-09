# Compatibility measurement

Compatibility percentages must come from repeatable external suites, not milestone intuition or
project-owned regression counts. OPBrowser reports each metric with a named manifest version and
a pinned upstream revision.

## Known legacy-reference divergences (not excluded)

Two pinned `rec2020` reftests (`predefined-012.html` and
`rec2020-001.html`) expect green references that diverge from the
current W3C CSS Color 4 display-referred transfer rule (Rec.2020
BT.1886 gamma **2.4**). OPBrowser intentionally retains the newer
specified transfer instead of hardcoding those legacy reference colors.
These cases are classified as **known, non-blocking reference
divergences**. They are **not skipped, removed, or converted to PASS**:
they continue to lower the reported WPT score.

- Static v1 exact: **197/200 (98.50%)**, including both failures.
- Static v1 with source-authored `meta[name=fuzzy]`: **198/200 (99.00%)**;
  the separate XYZ byte-level allowance accounts for the difference.
- Positioning v1 exact: **53/100 (53.00%)**.

See [KNOWN_TEST_DIVERGENCES.md](KNOWN_TEST_DIVERGENCES.md) for the
pinned fixture names, current W3C specification URL, scope, review triggers
and commands. No global exclusion list or relaxed CI threshold is used.

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
remain unsupported. The next text-fragment pass recognizes CSS
::first-line/:first-line in selector parsing and author cascade, computes
its fragment style, and paints the first actual line with its own inherited
color and inline-sized background without altering following lines.
selectors/first-line-bidi-002.html now matches the reference image
pixel-for-pixel. Frozen Static v1 advances 195/200 -> 196/200 (98.00%)
on the unchanged exact comparator. Positioning remains 53/100 (53.00%)
without new render errors. The four outstanding Static cases are:
currentcolor-003.html, predefined-012.html, rec2020-001.html and
xyz-003.html. currentcolor-003 still needs first-line dynamic inheritance
and CSS gradient/shadow/filter painting. XYZ differs by one blue byte in
a region where the WPT explicitly declares maxDifference=0-1, but
the project's strict score intentionally does not apply that allowance.
Both Rec.2020 pinned reference colors differ from the gamma 2.4 current
CSS Color 4 transfer; the reference manifests have not been changed.
Computed CSS now preserves currentcolor-dependent background and per-side
border color provenance through inline styles. The first-line paint
resolves only those dependent decorations to the fragment color, leaving
explicit red colors, independently blue children, and later lines alone.
The original frozen currentcolor-003 reference now passes pixel-exactly:
Static 197/200 (98.50%), Positioning still 53/100 (53.00%). Exactly
three Static failures remain: predefined-012, rec2020-001, xyz-003.
XYZ differs by one blue channel byte within its WPT-declared fuzzy
allowance, but the strict score still reports it as a failure. Neither
thresholds, manifest entries nor reference images have been altered.

The next pass corrects the default font-size used by OPBrowser's UA
computed styles from 18px to the conventional 16px, including em/rem
box measurements; tests that encoded the former default were updated
to assert the new layout, not to conceal broken rendering. The frozen
Static WPT exact score remains 197/200 (98.50%) and Positioning
remains 53/100 (53.00%). WPT reftests are permitted to specify both
a maximum RGB channel difference and a total differing-pixel range
via <meta name=fuzzy>. For xyz-003 the upstream test explicitly permits
0-1 difference and 0-18432 pixels. With normal 16px em geometry,
the render differs by only one blue byte over exactly 18432 pixels,
so a new opt-in --report-wpt-fuzzy measurement gives 198/200 (99.00%).
The default probe, JSON badge, frozen manifest and strict comparison
remain at 197/200. The two Rec.2020 reftests fail in either mode;
their expected colors conflict with gamma 2.4 in the current W3C
CSS Color 4 display-referred transfer definition. Upstream fuzzy
allowances are never invented by the probe.

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

## Test262 Runtime v1 (M4.16, independent of parser v1)

The new op_js/test262_runtime_probe executes 91 selected upstream
addition/equality classic-script tests against a fresh OPBrowser JS
runtime per case. The suite uses a pinned Test262 revision
c8c798898646638cd0c24879f8e0374e847e7d74, checks local Git
revision when available, and writes a per-case machine-readable JSON
pass/fail/skip report. Parse-only negatives, unsupported test includes,
modules, async and strict-only tests are explicitly skipped, not
credited as passes. Remaining script parse failures count as FAIL.

- Before basic global primitive constructors: 18/91 (19.78%).
- After M4.16 Boolean/Number/String/Object/Array/isNaN/isFinite,
  Number constants and radix conversion: **59/91 (64.84%)**.
- These 91 files are concentrated in ES legacy arithmetic/equality:
  their result is NOT a score for all JS runtime semantics, modern ES
  or modern websites. The 2000-case Test262 parser metric remains
  separate and unchanged pending its next external run.

Use tools/compatibility.ps1 -ExternalOnly -RuntimeOnly -Test262Path <pinned-Test262-test-path>
to run only the pinned runtime subset, or omit -RuntimeOnly
to rerun both Test262 v1 subsets. The runtime JSON reports failed
file paths and reasons for new conformance work.

## Test262 Runtime v1 progress after M4.17

The same pinned upstream revision and 91 classic-script fixture paths
were rerun with no exclusions or altered skip rules. With boxed
primitives, ordered object coercion, instanceof, unary void and sloppy
global assignment, the result is **82/91 (90.11%)**, up from
**59/91 (64.84%)** after M4.16. Remaining cases are all
missing eval/Date/Symbol support; these are *failures*, not skips.

Because v1 deliberately samples older arithmetic and equality tests,
90.11% is only valid for this small selected fixture set. The overall
JS runtime conformance percentage is unknown. Next build a separate,
fixed and broader Test262 Runtime v2 rather than replacing v1.

## Test262 Runtime v2 (M4.18): broader 25-family selection

In addition to the narrow 91-case v1 suite, a second manifest now
locks 289 independently chosen files from 25 Test262 feature directories
in the exact same pinned upstream revision. Each group contributes up to
12 evenly spaced, sorted file paths, not selected by expected success.
The committed manifest, not the generator's evolving output, is the
long-term comparable target. Re-generate only to audit identical output.

Runtime v2: 39/179 (21.79%) at M4.17 baseline, rising to 65/179
(36.31%) after M4.18. 289 listed, 179 attempted, 110 explicitly
skipped, 114 attempted failures. v1 remains at 82/91 (90.11%).
The wide score is much lower because features in JSON, Promise, Object,
Array, String, modern syntax and builtin families remain incomplete.
Both scores are scoped, not whole-ECMAScript conformance.

M4.18 changes: native typeof including missing-identifier behavior,
lazy and nested conditional expressions, Array.isArray/Array.of,
Number.isNaN/Number.isFinite without implicit numeric coercion,
and Object.is with NaN and signed-zero SameValue semantics.
The runner parses Test262 list metadata including multiline includes/
flags and labels the machine-readable suite name by manifest version.

Run both with tools/compatibility.ps1 -ExternalOnly -RuntimeOnly
-Test262Path <pinned-Test262/test>. Parse-only v1 and WPT retain their
separate metrics; they were not rerun by the narrow runtime-only command.

## Test262 Runtime v2 after M4.19

The unchanged pinned 289-file v2 sample passes 78/179 attempted
(43.58%) instead of 65/179 (36.31%). Exactly 110 tests remain
explicit SKIP and 101 attempted tests FAIL. Narrow v1 stays 82/91.
Changes are native String.prototype.charAt, bounded Array.push/pop and
SyntaxError inheritance for JSON.parse. This is not overall JavaScript
or website compatibility. UTF-16 lone-surrogate string construction and
generic array-like methods are still incomplete.

## M4.20 DOM vertical slice; Test262 score unaffected

M4.20 adds native author-script node creation and attachment through
the authoritative op_dom tree, not a new ECMAScript builtin score.
Cross-layer tests cover real paint after createElement/appendChild,
document.body timing, nested detached node discovery, timer updates,
click listeners, and ancestor-cycle rejection. Existing Test262
Runtime v1 remains 82/91 (90.11%), and Runtime v2 remains 78/179
(43.58%) attempted plus 110 explicit SKIPs. DOM Web Platform
conformance beyond these vertical cases is unmeasured; broad
interactivity and full DOM interfaces remain incomplete.

## M4.21 dynamic DOM and style updates

Nine new native end-to-end tests check createTextNode, removeChild,
insertBefore, id/class/style/data attributes, real text paint,
descendant lookup after detachment, reparent and timer-driven
CSS changes. Existing external CSS remains cached during recascade.
The unchanged Test262 Runtime v1 and v2 scores remain 82/91 and
78/179 (110 explicit skips). These DOM tests are narrow, not a
full DOM WPT score. Many live Node, CSSOM and MutationObserver
surfaces are absent.

## M4.22 live DOM accessors, classList, style and replacement

The browser now exposes a limited real parentNode / firstChild /
lastChild and live Element.childNodes NodeList, including indexed
access and item(), plus replaceChild/remove, Element.classList,
className and a small CSSStyleDeclaration-style Element.style
surface. Native DOM mutation and CSS recascade are tested across
timers and original JS handle rebinding.

Nine new end-to-end tests confirm DOM identity and actual pixels,
not WPT DOM conformance. This is a bounded partial subset:
incomplete Document/Node/Element prototype surfaces, selector and
CSS syntax, CSSStyleDeclaration parsing, style priorities,
variadic DOMTokenList mutation and browser event model.

Pinned ECMAScript subsets remain independent and unchanged:
Test262 Runtime v1 82/91 (90.11%), Runtime v2 78/179
(43.58%) attempted with 110 explicit skips. No overall
website readiness score can be inferred.

## WPT DOM smoke v1 (M4.23), not an official WPT run

For the first time OPBrowser executes the **unmodified assertions
of an original upstream WPT DOM file**. The fixed local WPT revision
is 97fe10c5d0e12e4a9d90f77b8db0602c64f3ad2d, checked at
runtime by op_engine/src/bin/wpt_dom_probe.rs. It takes the exact
upstream text of dom/nodes/Node-childNodes-cache.html using git
show and swaps only external testharness references for the
limited synchronous test()/assert_equals() adapter. The native
engine then runs the fixture and reports its visible outcome.

The manual, frozen compat/wpt-dom-smoke-v1.tsv contains four
selected fixture paths: 1 attempted/pass, 0 failed, 3 explicitly
skipped due to unsupported syntax/harness/iframe requirements.
**This is a manually scoped smoke test, not a representative WPT
sample. No DOM conformance percentage is inferred.** It does
not use the full official WPT runner or cover other assertions.

To reproduce:
  cargo run -p op_engine --bin wpt_dom_probe -- target/compat-wpt
Use a checkout at the pinned WPT revision. A wrong revision fails
rather than silently comparing different fixtures. Test262 Runtime
v1 (82/91) and v2 (78/179 attempted, 110 SKIP) are independent
JS samples and must not be merged with the DOM smoke count.

## M4.24: pinned original WPT DOM smoke v2

The fixed manual manifest compat/wpt-dom-smoke-v2.tsv lists ten
upstream WPT DOM source files at revision
97fe10c5d0e12e4a9d90f77b8db0602c64f3ad2d.
Seven are attempted and pass; three are explicitly SKIP. Cases
cover Node.childNodes caching and Element.childElementCount and
first/lastElementChild, including dynamic insertion/removal.
The runner checks all source paths, rejects duplicates, checks
exact revision and requires visible native PASS. It replaces
only the external WPT testharness imports with a tiny synchronous
assertion shim, not the original test assertions.

Reproduce: cargo run -p op_engine --bin wpt_dom_probe -- target/compat-wpt
This is not an official or representative WPT run. No WPT DOM
conformance percentage can be inferred. Test262 remains separate.

## M4.25: pinned WPT DOM smoke v3, multi-case harness

Frozen manually selected manifest compat/wpt-dom-smoke-v3.tsv
lists 13 original DOM WPT HTML files at the unchanged pinned
revision 97fe10c5d0e12e4a9d90f77b8db0602c64f3ad2d.
10 are attempted and pass; 3 remain explicit unsupported SKIP.
The ten attempted files contain 11 test(callback) calls.
The runner now requires the manifest's expected number of test()
calls, and uses a sticky failure state so a failing earlier test
cannot be overwritten by a passing later test. Dedicated
pass/failure-control tests exercise the shim. The original WPT
test callback bodies and assertions are not edited. The only
substitution is a deliberately limited synchronous testharness
adapter, NOT the official WPT harness.

New actual original fixtures include Element-nextElementSibling,
Element-previousElementSibling and Element-hasAttributes (two
test callbacks). The original VM now supports hasAttribute(s)
and live getElementsByTagName on Document/Element, with
corresponding native paint and timer integration regressions.

Reproduce:
  cargo run -p op_engine --bin wpt_dom_probe -- target/compat-wpt

The 10/10 result is a manually chosen smoke sample, NOT
a representative full DOM WPT pass percentage. It cannot
be compared directly with separate pinned Test262 runtime v1/v2.
