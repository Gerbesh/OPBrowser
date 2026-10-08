# OPBrowser engineering progress audit

Reviewed: 9 October 2026. This is a qualitative assessment, not an
externally measured web-compatibility score.

## Fixed, reproducible external subset results

| Pinned test subset | Recorded baseline | Actual coverage |
| --- | --- | --- |
| WPT Static v1 strict | 197/200 (98.50%) | 200 selected static reftests |
| WPT Positioning v1 strict | 53/100 (53.00%) | 100 selected positioning cases |
| Test262 Parser v1 | 523/1983 (26.37%) | Parse-only tests, not JS execution |

These are recorded baselines, not a freshly rerun WPT/Test262 result.
See COMPATIBILITY.md and compat/upstream.env for methodology.

## Qualitative readiness toward everyday independent modern browsing

**Approximately 10-15%**, with wide uncertainty. This describes the
full declared end goal rather than passing percentage of milestones.

- Foundation/native Windows shell: working native browser and navigation,
  build/CI, but not a distributable product.
- Static HTML/CSS/layout: a substantial original pipeline with selected
  static-suite successes; broader layout, typography, responsive modes and
  complex positioning are unfinished.
- JavaScript: original parser, bytecode, VM, closures, events, Promise, JSON;
  parser has only 26.37% on the selected sample, with no comprehensive
  runtime-conformance percentage yet.
- DOM/Web APIs: getElementById/textContent, partial listeners, timers and
  bounded same-origin GET; DOM tree editing, storage, streams, workers and
  numerous standard APIs remain absent.
- Networking and security: WinHTTP, basic filtering and checked fetch
  redirects; full cookies, cache, CORS, abort and permissions are missing.
- Browser product: tab model exists as a core foundation, while native UI
  is single-tab; renderer sandbox, tab processes, task manager, downloads,
  history and finished settings are not implemented.
- Advanced platform: WebAssembly, media, WebGL/WebGPU, accessibility and
  robust isolation are future work.

## Recommended engineering order

1. Establish a pinned Test262 runtime harness and dynamic DOM/WPT subset.
2. Improve core JS parsing/semantics, DOM tree modification and CSS layout.
3. Build multi-tab renderer isolation and real browser-origin/cookie/cache
   policies before claiming readiness for arbitrary public websites.
4. Retain reproducible failing cases and avoid reporting narrow subset
   success as overall browser compatibility.
