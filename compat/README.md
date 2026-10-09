# Versioned compatibility subsets

These manifests make OPBrowser conformance measurements repeatable. A percentage is meaningful
only together with the manifest version and the pinned upstream revision.

## Test262 parser v1

- manifest: `test262-parser-v1.txt`;
- upstream revision: `c8c798898646638cd0c24879f8e0374e847e7d74`;
- source scope: `test/language/**/*.js`;
- sample: 2,000 deterministic evenly spaced paths;
- module tests found in the sample are skipped because the current parser probe is script-only;
- score: positive scripts accepted plus negative parse-error tests rejected as expected.

This is a parser metric, not a JavaScript runtime or full ECMAScript conformance percentage.

## WPT static v1

- manifest: `wpt-static-v1.tsv`;
- upstream revision: `97fe10c5d0e12e4a9d90f77b8db0602c64f3ad2d`;
- sample: 200 deterministic static HTML reftests;
- source areas: CSS2 box display/model, CSS Box, CSS Color, CSS Display and Selectors;
- excluded: script/testharness.js tests, reftest-wait tests, mismatch references and non-local references;
- viewport: 800 x 600 by default;
- score: the test and reference are rendered through the normal OPBrowser engine and the same
  Win32 GDI paint-command path used by the browser, then their BGRA pixels are compared.

The initial baseline for the commit that introduced this infrastructure was 86/200 (43.00%).

## WPT positioning v1

- manifest: `wpt-positioning-v1.tsv`;
- upstream revision: `97fe10c5d0e12e4a9d90f77b8db0602c64f3ad2d`;
- sample: 100 deterministic static HTML reftests;
- source areas: CSS2 positioning/visual formatting/dimensions plus CSS Positioned Layout;
- excluded: script/testharness.js tests, reftest-wait tests, mismatch references and non-local references;
- deliberately includes unsupported sticky, multicol and vertical-writing cases to expose real gaps;
- viewport and pixel-comparison rules are identical to WPT Static v1.

Initial baseline after the first absolute/fixed/relative pass: 18/100 (18.00%).

## Refreshing a manifest

Refreshes are deliberate because changing the upstream revision or selection changes the meaning
of the percentage. Check out the revisions in `upstream.env`, then run:

```powershell
py -3 tools/build_compat_manifests.py `
  --test262-root C:\src\test262\test `
  --wpt-root C:\src\wpt `
  --output compat `
  --test262-revision <TEST262_REVISION> `
  --wpt-revision <WPT_REVISION>
```

If selection semantics change rather than merely refreshing upstream, create a new manifest
version instead of silently redefining v1.

## Test262 runtime v1 (M4.16)

- Manifest: test262-runtime-v1.txt, 91 pinned paths from the same Test262
  revision as the parser sample. It selects S-prefix matches in the
  addition/equals/strict-equals directories, including the three
  lowercase symbol cases present in the Windows glob selection.
- This is a small deliberately narrow arithmetic/equality executable
  sample, NOT full Test262 runtime coverage. It is separate from the
  independent 2000-case parse-expectation manifest and static WPT.
- Uses a fresh original JsRuntime per test, a minimal self-hosted
  Test262Error/assert/$ERROR/assorted assertions bootstrap and metadata
  checks. Unsupported includes, modules, async, onlyStrict and
  parse-negative cases are SKIP, never PASS. All other failed parses,
  runtime assertions or wrong expected errors are FAIL. Negative
  runtime tests must throw the expected error type.
- Manifest and observed git checkout revision must match; a copied
  tree without git metadata is reported as unverified in JSON.
- tools/compatibility.ps1 -RuntimeOnly runs this narrow runtime sample
  without requiring the separate 2000-file parser checkout. By default,
  supplying -Test262Path runs both independently.
- JSON output includes case-by-case status/reasons and exact
  skipped/attempted/passed/failed counts. Run with:

    cargo run -p op_js --bin test262_runtime_probe --       target/test262-upstream/test --manifest compat/test262-runtime-v1.txt       --json-out artifacts/compatibility/test262-runtime-v1.json

- Checkout Test262 revision c8c798898646638cd0c24879f8e0374e847e7d74
  before running. The 9 October 2026 M4.16 baseline after the new
  standard primitive builtins is 59/91 = 64.84%, with 32 FAIL and
  0 SKIP. Earlier baseline before the builtins was 18/91 = 19.78%.
- Scope caveats: classic non-strict scripts only, subset of Test262
  harness helpers, no includes such as propertyHelper or agent,
  no automatic strict-mode variant, no host APIs.

## Test262 runtime v2 (M4.18): broad deterministic 25-family sample

- Manifest: test262-runtime-v2.txt (289 pinned upstream files).
  This independent set spans 7 language expression groups, 6 statement
  groups and 12 standard built-in groups, including JSON, Array, Number,
  String, Object, Promise and Boolean.
- Selection uses ONLY sorted upstream file paths, never fixture contents
  or the engine's passing status. For each named directory, up to 12
  midpoints of evenly partitioned lexicographic paths are chosen.
  Any missing directory or wrong upstream commit fails generation.
- Reproduce selection from the pinned Git checkout:
    py -3 tools/build_test262_runtime_v2.py --root <Test262/test> --output compat/test262-runtime-v2.txt --check
  The published manifest is frozen; do not regenerate it to change score.
- Runner retains its classic sloppy-script scope and unsupported-test
  classification. Pass percentages use attempted cases; skip counts are
  always displayed separately, never counted as passes.
- M4.18 initial 39/179 (21.79%) after M4.17, final 65/179 (36.31%):
  all 289 listed cases retained, 110 SKIP and 114 FAIL. Typical gaps:
  Date, Symbol, eval, Function, modern syntax, property descriptors,
  iterator methods, and unsupported Test262 harness inclusions.
- Native end-to-end engine test verifies typeof/conditional/static
  functions paint the expected DOM text. Full Test262 runtime coverage
  and most browser Web APIs remain distant targets.

M4.19 on unchanged Test262 Runtime v2: 78/179 (43.58%) attempted
PASS, 101 FAIL and 110 explicit SKIP, up from 65/179; v1 82/91
unchanged. No fixture manifest was modified.

## DOM WPT smoke v1 (M4.23)

compat/wpt-dom-smoke-v1.tsv explicitly lists four manually chosen
DOM WPT cases: one executable original upstream source and three
unsupported skips. This is NOT full WPT DOM conformance.
Run cargo run -p op_engine --bin wpt_dom_probe -- target/compat-wpt
after checking out the revision in compat/upstream.env. The runner
pins the commit, reads original source, uses a narrow sync
testharness adapter and validates a native render verdict. Do
not interpret 1 attempted/1 pass as overall DOM or browser score.

M4.24: WPT DOM smoke v2 uses ten manually scoped source fixture
paths at the pinned revision; seven executed/pass, three explicit
SKIP. Run cargo run -p op_engine --bin wpt_dom_probe -- target/compat-wpt.
The original v1 manifest is retained; v2 is not an official
or representative WPT DOM conformance suite.

M4.25 adds compat/wpt-dom-smoke-v3.tsv, 13 manually scoped
pinned original-source WPT DOM fixtures: 10 attempted/pass,
3 explicit SKIP, 11 actual synchronous WPT test() callbacks.
The multi-test adapter retains failures, checks expected counts
and does not rewrite original assertions. This is not the
official WPT harness or DOM conformance.

M4.26 adds compat/wpt-dom-smoke-v4.tsv: 14 manually chosen
pinned original-source WPT DOM files, 11 attempted/PASS with
12 synchronous test() callbacks, 3 explicit SKIP. The new
Element-childElementCount fixture exercises relational JS in
and live DOM properties without changing upstream assertions.
This is not the official WPT testharness or a general DOM score.
