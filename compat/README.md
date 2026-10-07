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
