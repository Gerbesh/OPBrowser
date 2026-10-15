# OPBrowser: real public-web compatibility baseline

**Baseline date:** 2026-10-15. [Detailed 20-site results](live_web/2026-10-15/REPORT.md) |
[Raw per-site diagnostics](live_web/2026-10-15/results.json).

This suite opens **live pages from actual public services** directly in OPBrowser's
original Rust HTML/DOM/CSS/layout/JS stack, then rasterizes the actual
`DisplayList` with the Windows native renderer. No Chromium, WebView,
Playwright, pre-fetched mock HTML or compatibility polyfill is involved.
All 20 tests used separate worker processes to bound crashes and hangs.

- Browser viewport: **1280 × 720**; allow **1 second** for network/timer callbacks
  after navigation; child-process limit **23 seconds**, three workers in parallel.
- Anonymous HTTPS only. No login, cookies, scraping API substitution,
  captcha evasion, raw HTML capture or account-specific pages.
- Each successful render saves a real BMP then optionally converts it to PNG.
  Screenshots and diagnostic JSON are in `docs/live_web/2026-10-15/`.
- The set deliberately includes modern SPAs and asset-heavy feeds, with
  Wikipedia/Hacker News as useful relatively static contrasts.
- Live pages may change by date, network region, security challenges and ads.
  A `navigation_error` (HTTP 403/429) cannot automatically be attributed to
  parser or layout defects.

## Baseline outcomes: 20 named services

| Outcome | Sites | Count |
| --- | --- | ---: |
| Content painted but JS errors | Google, DeviantArt, Pikabu, Figma, Notion, Habr, Wikipedia, MDN, Hacker News, Amazon | 10 |
| Skeleton/decorative output, **no text drawn** | YouTube, Twitch | 2 |
| Essentially white viewport | Reddit, TikTok, X | 3 |
| HTTP access refused/rate-limited | Stack Overflow (403), Instagram (429), Canva (403) | 3 |
| Some output, no recorded JS errors, **not visually correct** | GitHub Explore | 1 |
| Child exceeded timeout | Discord | 1 |

**Working modern sites: 0 confirmed.** The outcome
`rendered_unverified` only means a screenshot and paint commands exist.
It is not a pass. An explicit reference-browser image/interaction
oracle is still required before calling a page supported.

Examples of real screenshots:
[GitHub Explore](live_web/2026-10-15/github_explore.png) shows a badly
expanded dark navigation menu;
[YouTube](live_web/2026-10-15/youtube.png) mostly shows blank gray
skeleton cards;
[DeviantArt](live_web/2026-10-15/deviantart.png) has a partial feed with
empty image placeholders;
[Pikabu](live_web/2026-10-15/pikabu.png) places some navigation and
post fragments incorrectly;
[Reddit](live_web/2026-10-15/reddit.png) is entirely white.

## Priority implications

1. **Recover useful, visibly complete first paint:** why Reddit/TikTok/X
   have empty rendering; why YouTube/Twitch leave only skeletons.
   Add missing high-impact JS runtime/DOM functionality as identified
   by actual page failures, not arbitrary API breadth.
2. **Make CSS/layout usable on real HTML:** GitHub's overexpanded menu,
   Pikabu's misplaced blocks and DeviantArt's empty image area prove
   that passing isolated static/WPT tests is insufficient. Investigate
   Flexbox/Grid, modern fonts, clipping, image formats and lazy loading.
3. **Record unsupported JS syntax and Web APIs precisely**, beyond
   aggregate `script_failed` counts. Then prioritize most frequent
   real-site blockers.
4. **Instrument the interactive browser** with on-demand load/script/
   DOM/layout/paint/GC CPU and elapsed-time tracing. The headless probe
   is a regression baseline, not a substitute for an in-process profiler.
5. **Add visual comparison and interaction checks** only once pages
   render enough content; block/403 cases require independent verification.

## 15 October 2026: diagnostic follow-up on the five priority sites

The new browser-owned `ScriptRunner` now retains at most **24 concise
reasons** per page, distinguishing actual JS errors from skipped scripts.
The live-site runner reports each reason and aggregates counts by category
without recording script source or page HTML.

The focused *pre-fix* run identified these primary obstacles:

| Site | Measured blockers | Render |
| --- | --- | --- |
| Google | unsupported bitwise `&` and `^` | incomplete text |
| YouTube | syntax failures, absent `Date`/`Image`, script-count cap | skeleton only |
| Reddit | expression/argument parsing failure | white viewport |
| Pikabu | invalid syntax, missing `loadAsyncScript`/`location`, `~`, unsupported script types | broken fragments |
| DeviantArt | missing `performance`, backtick syntax, null/undefined access, module skips | partial feed |

As the first **actual language fix**, the Rust lexer, parser and VM now
support JavaScript bitwise `&`, `|`, `^` and unary `~`, including
ToInt32 conversion, signed 32-bit values and correct operator precedence.
VM regression cases verify the semantics, including overflow/NaN cases.
On new live requests, the original Google failures for `&`/`^`
no longer appeared, but other syntax blockers did; **Google still had
2 failed scripts**, and its page is not visually corrected. After unary
`~` support, Pikabu's direct `~` parser failure disappeared, but a
subsequent parse error appeared; its **14 failed scripts** remained.
YouTube still had 12 failed scripts and a skeleton-only viewport.
This confirms progress past specific syntax barriers, **not** proof
that compatibility improved in the rendered experience.

Next: use the captured bounded diagnostics to prioritize frequently
missing Web APIs/syntax, then verify real first-view screenshots. The
interactive browser CPU/wall profiler remains a separate milestone.

## Reproduce explicitly (never run against sites in CI by default)

```powershell
cargo build -p op_browser --bin live_site_probe --release
python tools/live_web_suite.py --output artifacts/live_web/manual --workers 2 --timeout-seconds 23 --settle-ms 1000

# Focus on the five requested sites:
python tools/live_web_suite.py --only google,youtube,pikabu,reddit,deviantart --output artifacts/live_web/focus

# Shuffle order reproducibly and select eight:
python tools/live_web_suite.py --shuffle-seed 42 --max-sites 8 --output artifacts/live_web/sample
```

Manifest: [20 real public URLs](../tests/live_web_sites.json).
Runner: [tools/live_web_suite.py](../tools/live_web_suite.py).
Engine/native raster probe:
[crates/op_browser/src/bin/live_site_probe.rs](../crates/op_browser/src/bin/live_site_probe.rs).
The tool writes `results.json`, `REPORT.md`, and PNG (or BMP if Pillow
is unavailable) into the output directory. The `artifacts/` directory is
gitignored; the dated, reviewed baseline is committed under `docs/live_web/`.
