# Known compatibility reference divergences

Status reviewed: 2026-10-08. Applies only to the pinned WPT Static v1 manifest
at revision `97fe10c5d0e12e4a9d90f77b8db0602c64f3ad2d`.

## Policy: expected failure is not a skipped test

The following tests are **known legacy-reference divergences** and are
**non-blocking for a release** while the implementation remains aligned with
the current CSS Color 4 transfer function. They still run and are counted as
**FAIL** in both the default WPT score and the metadata-aware report. No
exclusion, reference replacement, per-case bypass, manifest edit, or altered
tolerance is authorized by this classification.

| Pinned WPT test | Reference | Classification | Review trigger |
| --- | --- | --- | --- |
| `css/css-color/rec2020-001.html` | `greensquare-ref.html` | Legacy reference color vs CSS Color 4 display-referred Rec.2020 gamma 2.4 | Upstream test/reference update, CSS WG clarification, or independently reproducible math error |
| `css/css-color/predefined-012.html` | `greensquare-090-ref.html` | Legacy percent-form Rec.2020 reference vs CSS Color 4 gamma 2.4 | Same |

Current specification: [W3C CSS Color Module Level 4, 7 October 2026](https://www.w3.org/TR/2026/CRD-css-color-4-20261007/#predefined), the `rec2020` section. The
transfer function states **gamma 2.40 (BT.1886)**; its sample code decodes
each component with `sign(c) * abs(c) ** 2.4`. OPBrowser follows that
equation in `crates/op_css/src/color.rs`. The pinned fixture's asserted
green RGB values differ from our rendered values. We have verified the
standard's stated transfer function; a legacy reference mismatch alone
does **not** prove all of OPBrowser's gamut mapping is correct. Future
upstream/spec evidence can reopen this classification.

## Other strict failures

`css/css-color/xyz-003.html` is **not** classified as obsolete. It has
explicit source-authored fuzzy metadata
`maxDifference=0-1;totalPixels=0-18432`. After the 16px UA-font fix, the
remaining one-byte blue-channel delta over 18,432 pixels is allowed by
that metadata. The default exact comparator still lists this test as
**FAIL**. The optional metadata report counts it as a pass.

## Measured, separate numbers

| Scope | Passed | Failed | Reporting |
| --- | ---: | ---: | --- |
| Static v1, exact RGB comparison | **197/200 (98.50%)** | 3 | Default canonical CI/badge result |
| Static v1, WPT-authored fuzzy metadata | **198/200 (99.00%)** | 2 | Opt-in `--report-wpt-fuzzy`, separately labeled |
| Positioning v1, exact | **53/100 (53.00%)** | 47 | Independent pinned subset |
| Test262 Parser v1 | **523/1983 (26.37%)** | 1460 | Parse-only expectations; 17 module entries skipped |

These figures are **not full web-platform or JavaScript conformance**.

To reproduce:

```powershell
cargo run -p op_browser --bin wpt_probe -- target\compat-wpt compat\wpt-static-v1.tsv
cargo run -p op_browser --bin wpt_probe -- target\compat-wpt compat\wpt-static-v1.tsv --report-wpt-fuzzy
```

Inspect differences without silently hiding them:

```powershell
cargo run -p op_browser --bin wpt_probe -- target\compat-wpt compat\wpt-static-v1.tsv --dump-failures target\wpt-debug
```

Review these classifications whenever pinned upstream versions are refreshed,
the normative transfer function changes, or a new conversion regression is
identified. Never silently remove a known failure from the frozen manifest.
