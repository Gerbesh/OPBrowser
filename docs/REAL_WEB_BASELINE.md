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
