# Compatibility measurement

Compatibility percentages must come from repeatable suites, not milestone intuition.

## Local baseline command

```powershell
.\tools\compatibility.ps1
```

This runs project-owned HTML, CSS, layout, engine-integration and JavaScript tests. Those
counts are regression coverage, not a Web Platform Tests score.

The Windows GitHub Actions workflow runs the same compatibility regression baseline on every
main push and pull request. External Test262/WPT percentages remain separate work and must not be
inferred from this green baseline.

## Test262

With a local TC39 Test262 checkout:

```powershell
.\tools\compatibility.ps1 -Test262Path C:\src\test262\test -Test262Limit 2000
```

The current external probe is parse-only. It records positive scripts accepted by the
parser and negative parse tests rejected as expected. Runtime conformance remains
unmeasured until Test262 harness semantics and built-ins exist.

## Web Platform Tests

WPT remains the primary HTML/CSS/DOM/Web API target. Automated WPT browser driving is not
wired yet. Until it is, project-owned tests must never be described as a WPT percentage.

The first WPT automation slice should cover static HTML/CSS tests that do not require
JavaScript harness APIs, then add testharness.js support after DOM scripting exists.

## HTML5test

HTML5test is a product-level feature-detection target. APIs must only be exposed after their
observable behavior exists; implementing stubs just to improve the score is forbidden.

## Performance metrics

Compatibility work should be recorded beside startup time, private memory, retained-tab
memory and page-load/reflow timing. A compatibility gain that makes the browser
pathologically heavy still needs engineering work.
