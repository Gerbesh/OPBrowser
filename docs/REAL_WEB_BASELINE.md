# OPBrowser: real public-web compatibility baseline

## 26 October 2026: native XHR, global Window lookup, parentElement and message tasks

Reproduced Pikabu `XMLHttpRequest is not defined` (10 executed/6 failed initial scripts). Added a first-party JavaScript XMLHttpRequest façade on the original op_js VM; GET requests use the existing native `opFetchText` -> `Engine::dispatch_text_requests` -> `NetworkContext::load_text_response_for_page_with_options` -> `JsRuntime::complete_text_response_request` pipeline. It exposes readyState, status/statusText, responseText/responseURL, responseType text/JSON, validated headers, load/error/abort/readystatechange/loadend events. A real native WinHTTP test served an HTTP 404 with body and custom header and verified that XHR updated the page's painted text. XHR cleared the next blocker and Pikabu advanced to 11 executed / 5 failed.

New opt-in Reference diagnostics bounded to at most three sanitized identifier candidate contexts exposed exact site pattern `window.s = document.createElement('script'); s.type = ...`; no actual bytecode call-site map is claimed. Fixed classic `window` property identifier fallback, mirrored global `var` writes in both directions without leaking global `let/const` to Window, then implemented native DOM `parentElement` for real and staged element ancestors. Pikabu advanced to 12 executed / 4 failed with `postMessage` missing. Original same-window queued `postMessage`, sender origin checks and bounded JSON-safe copy then removed all four errors; support for omitted targetOrigin (default `/`) was required by actual Pikabu code.

Final original Release Pikabu `artifacts/live_web/pikabu-message-default-after-2026-10-26/` and final five-site rerun `artifacts/live_web/five-after-xhr-globals-message-2026-10-26/`: **16 executed / 0 failed / 18 skipped** initial JS scripts, classified `rendered_unverified`, not visually accepted. The five-site rerun retained Google/DeviantArt with JS errors, YouTube without visible body text, Reddit blank. Real page and pixel acceptance is not established.

Script-budget canary `artifacts/live_web/pikabu-cap24-canary-2026-10-26/` increased the classic script slot count from 16 to 24: **17 executed / 7 failed / 10 skipped**; all seven new failures were `external_script_url_resolution_failed` under the deliberately same-origin external-script resolver. Reverted the cap to 16 instead of weakening the policy or misrepresenting those failures as successes. An expanded third-party script policy must be designed separately.

Known limits: XHR GET-only and same-origin; no synchronous/POST requests, request body, streaming/real progress, credentialed requests, timeout, or complete response header enumeration. `postMessage` supports same-window events and bounded JSON-safe cloning; no cross-window/frame messaging or full structured clone. DOM/text/pixel fidelity, unsupported script types, network script origins and adblock policy remain distinct blockers. No ready-made rendering or JS engine was embedded.

## 26 October 2026: Date and concat measured next blockers

After the previous native Pikabu baseline with 10 initial scripts executed / 6 failed, the opt-in diagnostic reported `Date is not defined`. Implemented first-party original Rust JS `Date` global using SystemTime and bounded UTC Gregorian calendar helpers in op_js::date (epoch milliseconds, TimeClip, ISO date parsing/formatting, Date.now, new Date, Date.UTC, Date.parse, UTC component getters, getTime/valueOf). Native Release `artifacts/live_web/pikabu-date-after-2026-10-26/` advanced to missing `setMilliseconds`; real bounded Date.setMilliseconds/setUTCMilliseconds then advanced to missing `concat` (`artifacts/live_web/pikabu-date-setter-after-2026-10-26/`). Own Array.prototype.concat and String.prototype.concat removed that error and exposed missing XMLHttpRequest (`artifacts/live_web/pikabu-date-concat-after-2026-10-26/`). Five downstream undefined s errors remain.

Pikabu classification remains `rendered_with_js_errors` with 10 executed / 6 failed initial JS scripts, despite real downstream execution progress. A five-site original Release rerun `artifacts/live_web/five-after-date-concat-2026-10-26/` retained Google and DeviantArt with JS failures, YouTube without visible text, Reddit blank, Pikabu with JS errors. Changes to script counts or pixels across mutable remote responses are not acceptance criteria.

Explicit subset limitations: Date textual representation, date-only/timezone-free parsing and multi-argument constructors currently use UTC rather than browser-local timezone rules; only strict bounded ISO strings parse, locale formatting/most setters/ECMAScript Date protocol are missing. Array.concat currently handles first-party arrays and their own indexed slots, preserving sparse holes within its 4096-element limit; generic array-like/species/inherited holes are unsupported. String.concat output bounded to 64 KiB. XMLHttpRequest is not implemented, and fabricating a no-op constructor would not be acceptable.

## 26 October 2026: DOM head/append and document.cookie error chain

The original Release Pikabu baseline (`artifacts/live_web/pikabu-dom-baseline-2026-10-26/`) was 8 executed / 8 failed initial scripts, including nullish appendChild, append and indexOf property reads and five downstream undefined s references. A concrete DOM omission was confirmed: document.head had no binding, while document.body and document.documentElement did. Exposing the parser-built head through original sync_dom_tree removed the appendChild null-receiver error and improved Pikabu to 9 executed / 7 failed (`artifacts/live_web/pikabu-head-after-2026-10-26/`).

Added actual Element.append(...nodesOrStrings) through native DOM createTextNode plus appendChild mutation path, retaining node order and returning undefined. Pikabu improved from 9/7 to 10/6 (`artifacts/live_web/pikabu-append-after-2026-10-26/`); next error null/undefined indexOf. New opt-in bounded diagnostic tracks the last four nullish property/call observations by sanitized method name and explicitly labels them **not necessarily causal**. `artifacts/live_web/pikabu-nullish-trace-2026-10-26/` pointed to recent document.cookie reads. The first-party network loader explicitly strips Set-Cookie and does not send Cookie headers. Implemented document.cookie as an empty string initially with a bounded, page-local JavaScript setter/getter of cookie name/value pairs; this is not a network cookie jar and does not implement domain/path/expiry/security attributes. `artifacts/live_web/pikabu-cookie-after-2026-10-26/` no longer errors at indexOf, but reports Date is not defined, followed by five errors s is not defined; counters remain 10 executed / 6 failed. Do not confuse diagnostic advancement with full page compatibility.

Focused engine integration tests assert real document.head identity, Element.append mixed node/text order, and empty/updated JS-only document.cookie. Full Rust conformance is a later check. Zero site-specific DOM nodes or JavaScript placeholders were fabricated.

## 26 October 2026: follow-up Pikabu syntax, callable and native GC pass

Fresh original-Release Pikabu before new changes (`artifacts/live_web/pikabu-before-spread-2026-10-26/`) reported 7 executed / 9 failed initial scripts. Adding bounded object spread and simple identifier-array variable destructuring removed both targeted Syntax errors; subsequent Release checks reached 8 executed / 8 failed (`pikabu-spread-array-after-2026-10-26/`). Exposing the real window `self` alias moved the next call-site diagnostic from undefined `self` to unavailable `bind`. Adding original Function.prototype.bind (including GC-traced captured target/this/arguments) moved it to unavailable array `forEach`, and implementing array forEach moved it to a null/undefined `indexOf` receiver; the final focused run `artifacts/live_web/pikabu-foreach-after-2026-10-26/` is still 8 executed / 8 failed and `rendered_with_js_errors`. This is a diagnostic progression, not a visual pass.

The rebuilt Release after the call-depth safety change is archived at `artifacts/live_web/pikabu-final-depth32-2026-10-26/`: 8 initial scripts executed / 8 failed, with null appendChild/append/indexOf and missing s references remaining.

A five-site Release regression (`artifacts/live_web/five-after-spread-bind-2026-10-26/`) retained existing classifications: Google and DeviantArt rendered with JS errors, YouTube no text, Reddit blank, and Pikabu rendered with JS errors. Current Pikabu issues include null `appendChild`, null `append`, null `indexOf`, and multiple undefined `s` references. Remote page variability prevents exact visual/performance equivalence assertions.

Implemented language/runtime subsets are intentionally not complete ECMAScript: object spread copies only owned enumerable properties and currently defers property copying until object construction (source mutations later in the literal may affect copies); simple array destructuring accepts only arrays and identifier slots/holes, no general iterators/defaults/rest/nested patterns; Array.forEach accepts only actual arrays, while bind does not install the full global Function constructor. JavaScript call-depth safety cap is 32 rather than 64 on this Windows test configuration due to deterministic stack overflow in the previous full-workspace run. No browser engine substitution or bot bypass.

## 26 October 2026: 20-site test-first JavaScript correction

Original Release 20-site manifest, 1280x720, 1300ms settle, 23-second per-site timeout: 10 rendered with JS errors (Google, DeviantArt, Pikabu, Figma, Notion, Habr, Wikipedia, MDN, Hacker News, Amazon), 2 skeleton/no text (YouTube, Twitch), 3 blank (Reddit, TikTok, X), 2 navigation failures (Stack Overflow, Instagram), 2 rendered but visually unverified (GitHub Explore, Canva), and 1 timeout (Discord). Ignored evidence: `artifacts/live_web/all-before-2026-10-26/`. These are changing third-party pages, NOT browser compatibility passes.

Opt-in JS failure context identifies Pikabu `function loadAsyncScript(key, url, attrs = {})` as a parser blocker and a missing `includes` member method. Original plain-identifier function default initializers and native Array.prototype.includes now have regression tests. Focused native Release Pikabu changed from 5 executed / 11 failed initial scripts to 7 executed / 9 failed; intermediate runs confirm the individual contributions. Post-fix output: `artifacts/live_web/pikabu-both-after-2026-10-26/`. Status remains `rendered_with_js_errors`; text primitives, screenshot, and elapsed time are not comparable visual/performance oracles across changing public page responses.

Open real-site blockers: object spread and array destructuring syntax, failed DOM node access (`appendChild`/`append` on null), downstream globals such as `s`, YouTube's four null/undefined property accesses and 27 skipped scripts, Google's missing eval and unsupported regex lookahead, missing module semantics, incomplete CSS/layout, and site-specific blocking. No bot bypass or raw site-source archival.

## 20 October 2026: native editable controls Release sample

Original Release suite at ignored `artifacts/live_web/native-form-editing-2026-10-20/`: Google rendered_with_js_errors (2 failures, 378 ms, 6 paint commands), YouTube no_text_rendered (4, 1,595 ms, 130 commands), Reddit blank_viewport (0, 242 ms, 1 command), DeviantArt rendered_with_js_errors (3, 8,901 ms, 2,609 commands), Pikabu rendered_with_js_errors (11, 12,740 ms, 1,522 commands). Statuses unchanged; changing remote pages prevent rigorous paint/performance comparison. Native WM_CHAR Unicode and local HTML editing are tested offline; public load probes do not type into forms. Full-browser compatibility still limited.

## 20 October 2026: programmatic focus Release site smoke

Release build and five anonymous noninteractive site samples in ignored `artifacts/live_web/programmatic-focus-2026-10-20/` retained previously observed statuses: Google rendered_with_js_errors (~1.83 s), YouTube no_text_rendered (~3.02 s), Reddit blank_viewport (~1.59 s), Pikabu rendered_with_js_errors (~12.84 s), DeviantArt rendered_with_js_errors (~11.83 s). These probes do not execute focused user interactions and must not be used to claim full site, JavaScript or DOM interoperability. Pinned Test262 Parser v1 (523/1983) and Runtime v2 (78/179 attempted with 110 SKIP) remain historical selected-subset measurements; external full Test262 conformance was not rerun here.

## 20 October 2026: synchronized focus and KeyboardEvent baseline

Native Release run in ignored `artifacts/live_web/native-key-events-2026-10-20/`: Google `rendered_with_js_errors` (2 failed JS, ~1.97 s), YouTube `no_text_rendered` (4 failures, ~3.16 s), Reddit `blank_viewport` (0 failed JS, ~1.64 s), DeviantArt `rendered_with_js_errors` (3 failures, ~16.23 s), Pikabu `rendered_with_js_errors` (11 failures, ~19.42 s). High-level outcomes remain as before. These noninteractive probes cannot verify keyboard/mouse events; dedicated engine/Win32 DOM tests demonstrate cancellation and activeElement. No claim of complete YouTube/Reddit compatibility or a solved Pikabu latency problem.

## 20 October 2026: focus reveal and DOM focus event Release sample

Anonymous native Release suite in ignored `artifacts/live_web/focus-reveal-events-2026-10-20/`: Google rendered_with_js_errors (2 failed, 394 ms), YouTube no_text_rendered (4 failed, 1,561 ms), Reddit blank_viewport (0 failed, 252 ms), Pikabu rendered_with_js_errors (11 failed, 14,697 ms), DeviantArt rendered_with_js_errors (3 failed, 12,482 ms). These noninteractive load probes cannot exercise Tab/Shift+Tab or focus/blur listeners and do not certify modern website functionality. Nested CSS scroll, real native Win32 document scroll and DOM event propagation have dedicated offline tests. One existing engine interval self-cancel timing test failed during an initial concurrent full-workspace pass but passed in isolation and on full-suite rerun. Timing flakiness remains a monitoring item; no claim of a root-cause fix.

## 20 October 2026: native keyboard focus compatibility checkpoint

The native optimized Release run `artifacts/live_web/native-keyboard-focus-2026-10-20/` (ignored local artifacts) checked the same five anonymous URLs: Google rendered_with_js_errors (~1.8s), YouTube no_text_rendered (~2.9s), Reddit blank_viewport (~1.7s), DeviantArt rendered_with_js_errors (~14.6s), and Pikabu rendered_with_js_errors (~18.4s). Pikabu previously timed out repeatedly at 23s and even 45s, so its later successful load is evidence of variability, **not** a diagnosed fix. These probes do not issue keyboard input and do not certify page usability; the keyboard feature is verified by original Rust engine, real Win32 message and UI queue integration tests. YouTube textless and Reddit blank remain major unresolved JS/DOM/layout blockers.

## 20 October 2026: cancelable native link compatibility sample, Pikabu timeout

Native Release 5-site run `artifacts/live_web/cancelable-native-links-2026-10-20/` (ignored local files): Google rendered_with_js_errors (2 failed JS, 6 paint commands), YouTube no_text_rendered (4 failed JS, 130 paint commands), Reddit blank_viewport (0 failed JS, 1 command), DeviantArt rendered_with_js_errors (3 failed JS, 2,821 commands), **Pikabu timeout at 23s**. Standalone Pikabu rerun with the same 23s limit also timed out, and an extended 45s standalone run still timed out. Previous sample loaded Pikabu in ~17s, so treat this as an unresolved real-site regression/performance diagnostic until controlled before/after comparisons establish whether it is network variability, content, or a local engine stall. This interaction update does not claim to fix modern JS/bootstrap or existing page rendering failures. Native click cancellation itself is covered by local Rust and Win32 regression tests, not by the five noninteractive web probes.

## 20 October 2026: pointer-events and validated native links Release sample

Live anonymous native Release test output at gitignored `artifacts/live_web/pointer-events-native-link-2026-10-20/`: Google rendered_with_js_errors (2 JS failed, 6 commands), YouTube no_text_rendered (4 JS failed, 130 commands), Reddit blank_viewport (0 JS failed, 1 command), DeviantArt rendered_with_js_errors (3 JS failed, 1,229 commands), Pikabu rendered_with_js_errors (11 JS failed, 1,370 commands). Compared with earlier stage, high-level statuses and failure counts are unchanged; paint counts and load times fluctuate on remote sites, so these do not indicate a measured graphics improvement or regression. Headless-style probes do not click interactive link targets; native link/input regressions were checked in offline browser tests and Win32 smoke. Continued P0 blockers: modern JS bootstrap, fonts/layout fidelity, complete DOM event model.

## 20 October 2026: z-index hit precedence Release baseline

At gitignored `artifacts/live_web/stacked-hits-2026-10-20/`, five native Release HTTPS probes retained the existing statuses: Google rendered_with_js_errors (2 JS failed), YouTube no_text_rendered (4 JS failed), Reddit blank_viewport (0 JS failed), DeviantArt rendered_with_js_errors (3 JS failed), Pikabu rendered_with_js_errors (11 JS failed). Load times were ~382 ms, 1495 ms, 242 ms, 9199 ms, and 12139 ms respectively (one changing-site snapshot, not a stable performance benchmark). Layer-aware interaction is verified by local hit-region and JS event tests, not by the headless-style real-site probes. Other layout/JS incompatibilities continue to block modern sites.

## 19 October 2026: ID-less/inline hit mapping and native press lifecycle

Five public Release probes at ignored `artifacts/live_web/full-hit-and-active-2026-10-19/` retain the same classifications: Google partial with 2 JS failures, YouTube **zero painted text** with 4 JS failures, Reddit **blank viewport** with 0 failed scripts, Pikabu partial with 11 failures, DeviantArt partial with 3 failures. Prior CSS parser errors remain 0/22/1/114/170 respectively. These noninteractive probes cannot certify hover, active pixels or full website compatibility; native Rust and Windows UI regressions exercise those paths separately. The new hit regions preserve old text-wrap tests, unlike the discarded approach that generated CSS boxes for every inline element. Stacking-aware precise hit-testing and comprehensive replaced-element DOM targets still need work.

## 19 October 2026: native mouse hover / focus compatibility check

Native Release baseline rerun on five public anonymous sites (gitignored `artifacts/live_web/native-pointer-states-2026-10-19/`) after pointer-to-style integration: Google 0 CSS / 2 JS failures, YouTube 22 CSS / 4 JS failures and no painted text, Reddit 1 CSS / 0 JS failures and white viewport, Pikabu 114 CSS / 11 JS failures, DeviantArt 170 CSS / 3 JS failures. Categories unchanged from preceding CSS stage. Hover and mouse click style changes have targeted Rust/Win32 tests but are not exercised by noninteractive HTTPS probes. Native block ID hit regions only, not general element picking; do not imply full `:hover` site conformance. Interactive reftest and keyboard/focus-visible follow-up required.


## 19 October 2026: live screen media and CSS state selectors

Measured original native Release at 1280×720 with 1300ms settling. After bounded viewport-aware `@media` parsing, YouTube CSS parse errors fell **35→22** (13 previously ignored @media). After recognizing static pseudo-class states, Pikabu CSS errors fell **532→114**, DeviantArt **676→170** (ignored artifacts `artifacts/live_web/media-and-pseudo-2026-10-19/`). YouTube still has **4 JS failures and 0 text primitives**; Reddit has 0 JS failures and remains blank. Pikabu's text primitives changed from **799→691**, DeviantArt **2493→806** compared with the media-only probe: these are changed native renders, **not** reference-verified visual improvements. No hidden CSS source was retained; error diagnostics now emit only fixed categories. Unsupported dynamic hover/focus and complex/media preference features remain to be implemented before compatibility claims.


## 19 October 2026: blank first-viewport origin, structural census

Original native Release on `reddit,youtube,google` (gitignored `artifacts/live_web/dom-census-extended-2026-10-19/`) adds bounded numeric DOM diagnostics, no saved raw HTML/script/CSS. Reddit: **156 connected nodes, 69 elements, 0 body text nodes, 1 form, 4 inputs, 1 main, 0 external scripts, 0 DOM mutations, 0 painted text**; its one inline script executes, but no feed content exists to paint. This might be an intermediate service response; not enough evidence to classify anti-bot behavior definitively. YouTube: **470 nodes, 13 body text nodes, all suppressed by computed styling, 9 external scripts, 35 stylesheet errors, 4 JS errors, 0 painted text**. Google: 38 nodes and 3 text paint commands, still partial. Negative experiment: doubling the script quota 16→32 yielded YouTube 24 executed / 7 failed / 0 painted text and a 23-second Pikabu timeout; the production cap stays 16. Next fix must target the actual load/bootstrap and CSS/layout failures, not an arbitrary larger script budget.


## 19 October 2026: live Reddit initial-script execution unblocked

A Rust-native Release rerun with 1300 ms settle on the same five anonymous HTTPS services (gitignored `artifacts/live_web/branch-multidecl-five-2026-10-19/`) records Google **2**, YouTube **4**, Pikabu **11**, Reddit **0**, DeviantArt **3** failed initial scripts. Reddit advances from zero executed and one syntax-failed script to **one executed, zero failed**, after supporting multiple variable declarations containing await. Its DOM mutation count remains **0**, paint **1 command**, text **0**, nonwhite pixels **0**: it is still a white viewport, not a usable Reddit page. YouTube is still textless and the other pages remain unverified/malformed. This is a genuine targeted JavaScript execution improvement with no claimed visual/site acceptance. Opt-in diagnostic context remains bounded, and no authentication or anti-bot workaround was attempted.


## 19 October 2026: returning short-circuit and one-sided await

Native Release with 1300ms settling on the same five public URLs, archived only under ignored `artifacts/live_web/one-sided-await-2026-10-19/`: initial JS failures Google **2**, YouTube **4**, Pikabu **11**, Reddit **1**, DeviantArt **3**. YouTube remains a textless skeleton and Reddit white. Object/one-arm logical async regressions reach genuine native text pixels, but these public results do **not** establish a repaired first viewport. No site authentication, scripts archiving, or challenge bypass was performed.


## 19 October 2026: nested-expression await follow-up

Original native release probe (1300 ms settle, same five anonymously accessible real URLs) stored ignored artifacts under `artifacts/live_web/expression-await-five-2026-10-19/`. Google: **2** initial JS failures; YouTube: **4**, still zero text; Pikabu: **11**; Reddit: **1**, still white; DeviantArt: **3**. A bounded opt-in Reddit snippet still reports unsupported await placement within its complex script; this subset does not imply bypassing any site protection or usable content. DOM native pixel tests pass for a synthetic method-call argument await and normal chained awaits. The external Test262 test tree is unavailable in this workstation session; documented historical parser/runtime subset percentages must not be represented as fresh measurements.

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

## 19 October 2026: sequential-await and async-arrow follow-up

The original engine now runs direct sequential awaits on the Promise microtask queue, including pending fetch replies across GC, and parses async arrow syntax. Five anonymous public HTTPS URLs with unchanged 1280x720 native pixel and 1300ms settle settings were checked under ignored `artifacts/live_web/await-arrows-five-2026-10-19/`: Google **2**, YouTube **4**, Pikabu **11**, Reddit **1**, DeviantArt **3** failed scripts. Reddit remains white and YouTube textless. Opt-in bounded diagnostic under `artifacts/live_web/await-arrows-reddit-2026-10-19/` identifies the new Reddit obstacle as unsupported complex-await placement after successful async-arrow parsing. No modern site is certified functional. An earlier interim Pikabu timeout cleared on this final five-site run.

## 19 October 2026: async declaration unblocks parsing only

Original no-`await` `async function` syntax now uses the native VM and existing Promise jobs. Five anonymous live HTTPS URLs were retested at 1280×720 and 1300ms settling, under ignored `artifacts/live_web/async-five-2026-10-19/`. JS failures: Google **2**, YouTube **4**, Pikabu **11**, Reddit **1**, DeviantArt **3**. YouTube remains a textless skeleton; Reddit remains white. Reddit's parse error moved from the async declaration to `await suspension is not supported yet`. This is parser progress, **not** a visual compatibility pass. Native await scheduling, dynamic scripts and layout remain outstanding.

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

## 15 October 2026: page performance clock follow-up

The original Rust page VM now exposes `window.performance.now()`
(monotonic elapsed milliseconds) and `timeOrigin` (Unix epoch milliseconds).
This is a small Web API subset, not the complete Performance API or the
interactive CPU/wall-time profiler.

An opt-in DeviantArt recheck at the same real HTTPS URL completed in
15.05 seconds with **4 failed scripts**, 7 skipped scripts and 2,506
native paint commands. Remaining diagnostics include unsupported
backtick syntax, a non-callable value and null/undefined access.
The earlier missing `performance` diagnostic no longer appears.
Unchanged failure count and dynamic site content mean **no verified
visual or functional improvement**. The diagnostic JSON and native
screenshot are saved locally under
`artifacts/live_web/performance-2026-10-15/`.

## 15 October 2026: real-site template-string rerun

The original lexer/AST/bytecode VM now supports **untagged**
backtick strings, including multiline content, nested `${...}`
substitutions, common escapes, and left-to-right evaluation. Tagged
templates, lone UTF-16 surrogates and several modern language features
remain unsupported.

The same five priority HTTPS URLs were rechecked with the native
Release probe at `artifacts/live_web/templates-2026-10-15/` (real
network and Windows pixel output). Recorded outcomes: Google 2 script
failures/6 paint commands; YouTube 12 failures/130 commands and no
visible text; Reddit 1 failure/1 command and a blank viewport; Pikabu
14 failures/1,159 commands; DeviantArt 4 failures/913 commands.
DeviantArt no longer reports the prior unsupported backtick character,
but reports `expected expression` instead. The other four failures
and the weak first-viewport results remain. Sites change over time;
paint-count changes alone are not compatibility progress.

## 15 October 2026: arrow-function VM and live rerun

The independent parser/bytecode VM now implements plain-parameter
arrow functions with expression or block bodies, closure retention,
lexical `this`/`arguments` and non-constructibility. Async arrows,
destructuring, default and rest parameters remain unsupported.

Recheck using the same five anonymous HTTPS URLs, same window and
settle settings, and real native pixels is in
`artifacts/live_web/arrows-2026-10-15/`. Measured script failures:
Google **2**, YouTube **11**, Reddit **1**, Pikabu **12**,
DeviantArt **4**. In the immediately preceding run with templates
only, the counts were 2, 12, 1, 14 and 4 respectively.
YouTube remains skeleton-only, Reddit blank; Pikabu and
DeviantArt remain broken/partial. The site content and network
conditions are not frozen, so these changes are **diagnostic** and
not proof of a correctly rendered or interactive website.

## 16 October 2026: delete operator, real HTTPS follow-up

With reference-aware `delete` in the Rust VM, the focused native
two-site run under `artifacts/live_web/delete-2026-10-16/` records
YouTube **6 executed / 10 failed** versus **5 / 11** beforehand.
The original `delete is not defined` failure no longer occurs.
However YouTube still paints **zero text primitives** and is
skeleton-only. Pikabu still records 12 failed scripts and has
no verified visual improvement. This run settled for 1300ms versus
1000ms previously; do not attribute all count differences to the
syntax patch or claim production compatibility.

## 16 October 2026: native Image constructor, actual-pixel validation

The original JS VM now creates detached DOM `img` nodes using
`new Image()` and `Image()`, with bounded width/height and reflected
`src` and `alt` attributes. A fully offline integration regression
inserts a JS-created image and verifies **one actual PNG paint command**
in the original Engine. Loading/error events for detached images and
full HTMLImageElement compatibility are not provided.

Focused live HTTPS screenshots at
`artifacts/live_web/image-2026-10-16/` show:
YouTube **7 executed / 9 failed**, versus 6/10 in the prior
delete-only run. `Image is not defined` no longer appears,
but YouTube still has **zero text** and renders skeleton cards.
Pikabu remains **4 executed / 12 failed**, with broken layout.
Both runs used the same 1300-ms settle budget; page/network changes
still prevent turning an error-count change into a pass claim.

## 16 October 2026: page URL exposed through original Location

The page parser now passes the actual loaded HTTP(S) URL to its retained
JS runtime. `location`, `window.location`, `document.location`,
`document.URL`, `documentURI` and `baseURI` share page URL information;
common location components are readable. This initial facade does **not**
simulate location assignments, real navigation, non-HTTP origins, or the
complete URL standard. VM and parser-to-DOM integration tests pass.

The matched 1300-ms-settle HTTPS rerun under
`artifacts/live_web/location-2026-10-16/` reports Pikabu **4 executed /
12 failed** scripts, with the former `location is not defined` replaced
by `value is not callable`. It remains visually malformed, and its
paint-command and text counts are not a valid visual-improvement oracle.
YouTube remains **7 executed / 9 failed**, **0 text primitives** and
skeleton-only. Network responses varied in load time. No real-site
compatibility acceptance gate was passed.

## 16 October 2026: `??` and partial `for...of` syntax follow-up

The original JS pipeline now tokenizes and executes lazy nullish coalescing
and an explicitly bounded synchronous `for...of` subset. New VM regressions
exercise short-circuiting, grouped `&&`/`||`, arrays, string code points,
empty slots, nested loops, closures, `break`/`continue` and rejection of
noniterables/large inputs. Parser-to-DOM tests prove that `??` updates a
real text node and a `querySelectorAll` collection can be traversed.
The array/DOM iteration implementation uses a snapshot and is **not** the
full ECMAScript iterator protocol; live array append behavior differs.

Live real-native runs under `artifacts/live_web/nullish-2026-10-16/` and
`artifacts/live_web/for-of-2026-10-16/` used the same five public URLs
and 1,300 ms settle. Both still report JS failures Google **2**,
YouTube **9**, Pikabu **12**, Reddit **1**, DeviantArt **4**. YouTube
still paints **zero text primitives**, and Reddit is still white.
In `artifacts/live_web/for-diagnosis-2026-10-16/`, even a more precise
`for` parser diagnostic retains `expected ';' after for initializer`:
this particular YouTube case is **not** a recognized single-binding
`for...of` or straightforward `for...in` syntax at the error token.
No visually correct modern first viewport is established. Next:
identify the exact remaining modern expression/initializer pattern and
attack the repeated backslash lexer failures with bounded evidence.

## 16 October 2026: exact source-context diagnostics and original for-in

A bounded, explicit opt-in diagnostic mode is available via
`OPBROWSER_JS_ERROR_CONTEXT=1`; by default it is off. It emits
an 88-character maximum printable script context around the syntax
error offset, never saves a full script or transmits telemetry.
`artifacts/live_web/syntax-context-2026-10-16/` established that:
YouTube's first parser blocker is literally `for(k in a[0])`;
five separate lexer failures are JavaScript regex literals such
as `/\\s+/`, `/https:\\/\\//g` and character classes. Another error
includes a labeled `a: { ... }` statement. Reddit's one failed
inline script uses an `async function` callback, not a mistaken
comma in an ordinary function parameter list.

The browser-owned parser/compiler/VM now implements a deliberately
limited own-key `for...in` subset, with bounded snapshots, lexical
iteration scopes and genuine parser-to-DOM integration verification.
Native YouTube recheck under `artifacts/live_web/for-in-2026-10-16/`
confirms the `for` **syntax error disappeared**; that execution
advances to `cannot read properties of null or undefined`. Overall
YouTube stays **7 executed / 9 failed / 0 text primitives**, so no
visible compatibility milestone was reached. Own-property insertion
order, property descriptors, inherited enumerable properties, proxies
and mutation-observing enumeration are not implemented. Next fix the
actual regex-literal blockers before claiming modern JS readiness.

## 16 October 2026: literal RegExp, compound assignment and comma expressions

The independent Rust lexer now reads `/pattern/flags` with escapes,
character classes and conservative regex-vs-division context. Own
bytecode instantiates RegExp objects. `test`, `exec`, and bounded
String `replace`, `match`, `split` use a lightweight finite-state
regex crate, **not** a browser/JS engine. Unsupported ECMAScript
regex constructs such as lookaround and backreferences reject. Flags
`g/i/m/s/u` are parsed; implementation is a partial subset, not full
ECMAScript RegExp semantics or the `RegExp` constructor. Match indices
and `lastIndex` use UTF-16 indexing, preserving non-BMP characters.
Regular-string `split` treats the delimiter literally.

Original assignment grammar and VM also support `+=`/`-=` with
receiver/key evaluated once, plus the comma sequence expression in
parentheses and for-loop parts while preserving function-argument
boundaries. Original VM and parser-to-authoritative-DOM integration
regressions pass. Regex input length and capture/result counts are
bounded to resist abusive scripts.

`artifacts/live_web/regexp-2026-10-16/` records that YouTube's five
prior unsupported-backslash regex errors disappear, but **7/9**
executed/failed still yields no painted text. In
`artifacts/live_web/regex-compound-2026-10-16/`, the five sites still
report Google **2**, YouTube **9**, Pikabu **12**, Reddit **1** and
DeviantArt **4** failed scripts; several failures advance to runtime.
After adding comma expressions, the separate release YouTube probe at
`artifacts/live_web/regexp-comma-2026-10-16/` advances to **8 executed /
8 failed**, zero text and skeleton-only. Remaining syntax diagnostics
include object-literal getters (`get playbackRate()`, `get capture()`)
and labeled statements; runtime functions and DOM APIs also block.
The public sites change over time; no visual compatibility pass is claimed.

## 16 October 2026: original JS accessors and method shorthand

Native getter/setter and concise-method implementation passed VM/GC,
full workspace tests and strict lint. With 1300 ms settling, the
same five live anonymous URLs in
`artifacts/live_web/accessors-2026-10-16/` report failed scripts:
Google 2, YouTube 8, Pikabu 11, Reddit 1, DeviantArt 3.
Prior Pikabu/DeviantArt counts were 12/4; live responses vary.
YouTube still renders zero text and Reddit is blank; there is no
certified first viewport. Exact thrown-value semantics and full
descriptors remain incomplete.

## 17 October 2026: namespace-aware createElementNS

OPBrowser's original VM now supports `document.createElementNS`
with a bounded XML-qualified-name/prefix validation subset, storing the
namespace on authoritative `op_dom` elements and moving metadata
when synthetic IDs become native IDs. DOM getters expose the preserved
namespace, local name, prefix and correct HTML/SVG name casing.
The engine end-to-end fixture uses two scripts to create an SVG element,
attach it to the real body, read it back by ID, and render native text.
VM/GC/DOM/paint regressions and full Rust workspace tests pass.
SVG rendering, general Unicode XML names, and HTML foreign-content
namespace switching remain unsupported.

Real YouTube release probe (1300 ms settle, ignored
`artifacts/live_web/namespace-2026-10-17/`) no longer reports
`createElementNS` as non-callable. There are still **4**
JavaScript failures and **0 text**. New opt-in diagnostics identify
properties `set`, `responseStart`, `prototype`, and
`prototype` being read on null/undefined values. Other site
results remain Google **2**, Pikabu **11**, Reddit **1**,
DeviantArt **3** failed JS; Reddit's viewport stays blank.
The script-count limit still skips most YouTube scripts,
so these results do not establish general web compatibility.

## 17 October 2026: original Object.defineProperty and attributes

`Object.defineProperty` now consumes original own data/accessor
descriptors, stores sparse `writable`/`enumerable`/
`configurable` flags, enforces non-writable assignment and
non-configurable delete/redefinition, and works with string
and symbol data keys. `for...in`, `JSON.stringify`, and
`Object.getOwnPropertyDescriptor` observe the flags.
VM/GC/native-paint regressions pass. This remains a subset,
not full ECMAScript descriptors or WebIDL DOM reflection.

Native Windows Release run with 1300 ms settle in ignored
`artifacts/live_web/define-property-2026-10-17/`
measured failed JS: Google **2**, YouTube **4**, Pikabu
**11**, Reddit **1**, DeviantArt **3**. YouTube previously
had **5** failures after queue/reflection and **8** at turn
start. YouTube has **0 text primitives**, Reddit is blank.
Nineteen initial YouTube scripts were skipped by the
test-only script count limit, so error count alone is not
an end-to-end compatibility measure. Remaining diagnosed
errors are null/undefined property access and
`document.createElementNS`.

## 17 October 2026: bounded non-callable diagnostics and queue/reflection

With the opt-in `OPBROWSER_JS_ERROR_CONTEXT=1`,
native YouTube `value is not callable` errors were
localized to `Object.getOwnPropertyDescriptor`,
`Object.create`, `Array.shift`, and
`document.createElementNS`. The original JS VM now
implements array shift/unshift (including holes),
`Object.create` for null/object prototypes, and basic
own data/accessor descriptors (not full property attributes).
Synthetic HTML -> JS -> DOM -> native paint regressions pass.

Release probe with 1300 ms settle under ignored
`artifacts/live_web/queue-object-methods-2026-10-17/`
reports failed scripts: Google **2**, YouTube **5**,
Pikabu **11**, Reddit **1**, DeviantArt **3**.
YouTube drops from prior **8** to **5** failures.
The next native diagnostic identifies
`Object.defineProperty` and `document.createElementNS`.
YouTube still paints **zero text** and Reddit is blank.
Script-count skips remain, so this is not complete JS
or viewport compatibility.

## 17 October 2026: native JS Symbol primitive and property keys

The original VM implements real unique Symbol values, registry
and selected well-known symbols. Object symbol properties have
their own GC-traced storage, prototype/in/update/delete support,
and remain absent from own string-key for-in enumeration
and JSON object serialization. Deterministic VM and actual
HTML-to-native-paint regressions pass, including a reachable
object solely held behind a symbol surviving explicit GC.
No ready-made JS engine is embedded.

Live five-site 1300 ms run under ignored
`artifacts/live_web/symbols-2026-10-17/`: failed JS Google **2**,
YouTube **8**, Pikabu **11**, Reddit **1**, DeviantArt **3**.
YouTube's former `Symbol is not defined` ReferenceError now
advances to `Type: value is not callable`; YouTube still
paints **0 text**, Reddit remains **blank**. This is semantic
progress, not visual compatibility. Symbol descriptor reflection,
full protocols and async functions remain open.

## 17 October 2026: navigator and UTF-16 String search

Native browser JS exposes a conservative document-owned
`navigator` (real OPBrowser identity, `en-US` fallback language,
`cookieEnabled=false`, 1..64 available threads, unsupported
features absent). The original VM adds UTF-16-based `indexOf`
and `includes` and separates strict builtin receiver binding
from non-strict user-function `this` substitution.
Deterministic VM and HTML-to-original-paint regressions pass.

Release public-site sampling (1300 ms settle, ignored
`artifacts/live_web/navigator-string-2026-10-17/`):
Google **2**, YouTube **8**, Pikabu **11**, Reddit **1**,
DeviantArt **3** JavaScript failures. Native YouTube's former
`navigator is not defined` becomes a later TypeError;
`Symbol is not defined` remains. YouTube paints **0 text**
and Reddit remains blank; this is not a live-site pass.

## 16 October 2026: Math and RegExp missing-global follow-up

Implemented GC-traced original JS `Math` and `RegExp` globals,
using existing VM builtin dispatch and bounded regex code.
VM/unit and HTML-to-native-paint tests pass. The 1300ms five-
site run under ignored `artifacts/live_web/math-regexp-2026-10-16/`
still reports failed JS Google **2**, YouTube **8**, Pikabu **11**,
Reddit **1**, DeviantArt **3**. YouTube still paints **zero text**,
Reddit remains **blank**. Real YouTube trace shows prior
Math/RegExp ReferenceErrors removed and later navigator/Symbol
missing globals and TypeErrors. No visual acceptance achieved.

## 16 October 2026: operator and for-in parser unblocking

The real YouTube opt-in syntax excerpt exposed a valid comma-expression
RHS in `for(x in h.style[u]=q,C)` that the original parser incorrectly
treated as for-of-style `AssignmentExpression`. Corrected for-in
to parse `Expression`, retaining for-of's restriction. A second
real syntax sample revealed `this.playbackRate*=-1`, and another
showed `1<<e%32` / `Math.random()*1E9>>>0`. Added original
`*=`/`/=`/`%=`, signed/unsigned bitshifts and shift assignments,
with JS ToInt32 and 5-bit shift masking, plus original VM and native
HTML-to-text-paint tests.

Rebuilt Windows Release probe and reran the same five real HTTPS
sites with 1300 ms settling in ignored
`artifacts/live_web/shiftops-2026-10-16/`. JS failed counts:
Google **2**, YouTube **8**, Pikabu **11**, Reddit **1**, DeviantArt
**3**. YouTube still renders **zero text**, Reddit remains
**blank**. In a live context trace, the formerly observed three
YouTube syntax errors now reach `Reference: RegExp is not defined`
and `Reference: Math is not defined`. Those globals and async
script loading are subsequent blockers. This is a parser/VM
regression improvement, not an end-to-end site compatibility pass.

## 16 October 2026: labeled statements and protected finally execution

Original JS supports statement labels plus named `break` and
`continue` for labeled blocks/loops, including nested
`try/catch/finally`. The interpreter propagates the name across
try frames and runs finalizers before branching; deterministic VM
and native DOM-to-paint regressions pass. Unsupported source forms
and incomplete async/script scheduling remain explicit risks.

Same five live HTTPS sites rechecked with 1300 ms settle in
`artifacts/live_web/labels-try-2026-10-16/`: failed scripts
Google **2**, YouTube **8**, Pikabu **11**, Reddit **1**, DeviantArt
**3**. Native YouTube's former "labeled jump across try" rejection
became a later "expected expression" error. YouTube still paints
**0 text** and Reddit is blank; no visual acceptance gate passed.
Public pages and responses vary, so counts do not prove controlled
causality. Earlier intermediate diagnostics in
`artifacts/live_web/labels-2026-10-16/` are diagnostic only.

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
