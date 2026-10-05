# Compatibility Strategy

OPBrowser does not treat one synthetic score as sufficient proof of compatibility.

Targets:

- HTML5test: full feature coverage target;
- Web Platform Tests: primary web-platform conformance suite;
- TC39 Test262: ECMAScript conformance suite.

Feature-detection tests must not be gamed by exposing non-functional APIs.

Compatibility is developed subsystem by subsystem and regressions should receive
project-owned tests in addition to external conformance suites.
