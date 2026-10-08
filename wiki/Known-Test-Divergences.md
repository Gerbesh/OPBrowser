# Known Test Divergences

**We deliberately do not change current CSS Color 4 behavior merely to
match two pinned Rec.2020 green-reference fixtures.** The tests are
still executed and still counted as failures.

- `rec2020-001.html`: reference `greensquare-ref.html`.
- `predefined-012.html`: reference `greensquare-090-ref.html`.

Current CSS Color 4 specifies display-referred Rec.2020 **gamma 2.4**;
OPBrowser implements that rule. These are recorded as **known
legacy-reference divergences**, not as passing or excluded tests.

The remaining `xyz-003.html` failure is different: the upstream test
expressly allows a one-byte pixel difference on up to 18,432 pixels.
It passes only in the separately labeled WPT-metadata report.

Strict Static: **197/200**. Optional source-authored fuzzy: **198/200**.
Both Rec.2020 cases remain failed in both measurements.

Full evidence, revision, review criteria and commands:
[KNOWN_TEST_DIVERGENCES.md](https://github.com/Gerbesh/OPBrowser/blob/main/docs/KNOWN_TEST_DIVERGENCES.md).
