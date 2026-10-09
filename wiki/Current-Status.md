# Current Project Status

Last reviewed: **9 October 2026**. The [published GitHub Wiki](https://github.com/Gerbesh/OPBrowser/wiki) tracks the main repository's `wiki/` sources. This is a development snapshot, not a
claim that OPBrowser is ready for everyday browsing.

## Working today

- Native Windows 11 browser window with address field, navigation, back/forward,
  reload, scrolling and retained page reflow.
- Independent HTML tokenizer/tree builder, DOM, CSS tokenizer/cascade, text
  layout, display-list painter and native GDI presentation.
- Local/file/data and bounded HTTP(S) document loading; author stylesheets,
  images, basic SVG text, CSS backgrounds, relative/absolute/fixed layout,
  floats, basic flex/table formatting, selected compositing/filter effects.
- Windows WIC-backed raster decoding and ICC color conversion.
- Independent JavaScript lexer/parser/bytecode VM now executes bounded
  classic inline **and same-origin external** scripts in document order,
  supporting `document.getElementById` and textContent updates before
  CSS/layout. Native block clicks execute `addEventListener` or
  `onclick` handlers across capture/target/bubble phases and support
  `removeEventListener`, `stopPropagation` and `preventDefault`
  event state. Classic scripts run during tree construction at script
  closing tags, with retained DOM snapshots refreshed as markup arrives.
  Bounded external classic defer/async now fetch concurrently; defer runs
  in document order after parsing; async runs at parser/load polling
  points. An independent event loop, document.write, native default-action
  cancellation and broad Web APIs remain absent. document.readyState now
  tracks loading/interactive/complete, with DOMContentLoaded after defer
  and window load after initial async completion.
- Network request filtering foundation; full adblock UI/subscription pipeline
  is **not** implemented.

## Verified compatibility snapshot

| Metric | Result | Meaning |
| --- | --- | --- |
| WPT Static v1 strict | **197/200 (98.5%)** | Fixed subset, exact RGB comparison |
| WPT Static v1 WPT-authored fuzzy | **198/200 (99%)** | Separate opt-in report |
| WPT Positioning v1 strict | **53/100 (53%)** | Fixed 100 reftests |
| Test262 Parser v1 | **523/1983 (26.37%)** | Parse-only sample, not JS runtime coverage |

The two outstanding Rec.2020 cases have documented **known
legacy-reference divergence** status and remain counted as failures.
See [Known Test Divergences](Known-Test-Divergences.md).

## Biggest gaps

Complete page-script lifecycle, broad DOM/Web API bindings, events, modern JavaScript runtime
coverage, modern layout modes, complete CSS painting/stacking, browser tabs,
tab freezing, browser task manager, settings and full content blocker remain
major workstreams. The 98.5% metric measures only **200 selected static tests**.

See [Project Overview](Project-Overview.md), [Architecture](Architecture.md),
[Code Graph](Code-Graph.md), [Code Slicer](Code-Slicer.md), and
[Development Workflow](Development-Workflow.md).

## M4.8: lifecycle timing

Initial document.readyState and DOMContentLoaded/load sequencing is
available with bounded document/window listeners; this does not yet
imply a full browser event loop or resource-aware load timing.

## M4.9: timeouts and a page worker task pump

OPBrowser now keeps a bounded setTimeout/clearTimeout task queue on the
page's retained JS VM and wakes the browser worker when one-shot
callbacks are due. Callback DOM changes can repaint the page without
native input. Script tasks are still single-threaded within the page.
No setInterval, Promise microtasks or general browser task loop yet.

## M4.10: intervals and queued microtasks

setInterval/clearInterval are available in the window/global scope,
using the same handles as setTimeout/clearTimeout. An interval may
cancel itself. queueMicrotask(function) runs callbacks in FIFO order
after the current script/event/timer task, before the next timer.
Recursive enqueuing is bounded and cannot monopolize the worker
indefinitely. Promise and async network event jobs remain unsupported.

## M4.11: post-presentation text completion tasks

The browser now has a small experimental opFetchText(url, callback)
host API. Network IO is performed on bounded background workers while
the page's own JS thread handles completion callbacks and DOM updates.
Same-origin filtering, size and MIME limits apply to text resources.
Callbacks cannot leak into newly navigated pages. This is not standard
fetch/Promise/Response, and network tasks do not yet support aborting,
CORS, streaming or credentials.

## M4.12–M4.13: Promise reactions and real fetch response metadata

The original JS VM implements a bounded self-hosted Promise subset,
queueMicrotask reaction checkpoints and a same-origin GET fetch().
Real HTTP status, statusText, final URL, redirected flag and filtered
Headers.get/has are available on Response. HTTP 404/500 fulfill with
ok=false, and response.text() remains one-use and Promise-returning.
Set-Cookie is hidden from JS. Navigation generation and request/size
policies still apply; this is not full Fetch/CORS or ECMAScript
Promise conformance.

## M4.14: bounded Request, Headers and per-hop redirect protection

A browser page can now create Headers, call get/has/set/append/delete,
create Request(url, init), and pass Request into fetch. Whitelisted
custom GET headers reach WinHTTP and local test servers. Text/fetch
redirects are checked *before* visiting each new address, preventing
cross-origin intermediate hops. redirect:error is supported. No general
CORS, body upload, credentials, caching, streams or abort support yet.

## M4.15: strict JSON and Promise combinators

Native JSON.parse/stringify, Response.json and Promise.all/race/
allSettled/any now work in the original VM. A network-loaded JSON
object is verified to repaint native text pixels through Promise
microtask callbacks. This does not imply full Fetch/ECMAScript support.

Overall readiness toward a production-quality independent modern
browser is approximately 10-15% as a qualitative engineering opinion,
not a measured compatibility score. Pinned external subsets remain
WPT Static 197/200, Positioning 53/100, Test262 Parser 523/1983.

## M4.16: executable Test262 baseline

A separate pinned runtime suite now executes original VM scripts
instead of merely parsing them. The narrow arithmetic/equality
classic-script manifest passes 59/91 cases (64.84%), with 32 failed
and zero skipped in that sample. The original JS interpreter gained
callable Boolean/Number/String/Object/Array, isNaN/isFinite, Number
constants and numeric radix conversions.

This result covers only those 91 files. Full JS Runtime Test262
conformance, strict/module semantics, boxing and core Web APIs remain
unmeasured or incomplete. See docs/COMPATIBILITY.md for scope.

## M4.17: boxed primitives, conversion and operators

new Boolean/Number/String now create real objects, and custom
valueOf/toString are invoked during binary operand conversion.
instanceof, void and sloppy undeclared-variable assignment
also work. The same pinned 91 upstream Test262 classic runtime
fixtures improved from 59/91 to 82/91 (90.11%) without skips.
This is a narrow legacy arithmetic/equality sample, not overall
JS conformance. Eval, Date and Symbol are still missing.

## M4.18: broader pinned runtime compatibility sample

Beyond the narrow legacy arithmetic/equality 82/91 runtime score,
the new 25-family Test262 Runtime v2 samples 289 cases. 179 can be
attempted by the currently limited harness, 110 are explicitly
skipped, and 65/179 (36.31%) pass. On this fixed selection the previous
baseline was 39/179 (21.79%). This is a more useful view of modern
JavaScript gaps, but still not a comprehensive conformity score.

Native typeof, ternary expressions, Array.isArray/of, Number.isNaN/
isFinite and Object.is now work and are tested through native page
rendering. Modern application compatibility remains substantially
behind the narrow static CSS metrics.

## M4.19: more standard JavaScript primitives

Native String.charAt and bounded Array.push/pop now work, and JSON.parse
errors have proper SyntaxError inheritance. Locked Test262 Runtime v2
rose from 65/179 to 78/179 (43.58%) with 110 unchanged SKIPs and
101 FAIL. Narrow Runtime v1 remains 82/91. Full modern JS support and
real dynamic DOM node insertion are still unfinished.

## M4.20: genuine dynamic DOM nodes and click-driven repaint

OPBrowser can now create an element in its own JS VM with
document.createElement, assign id/textContent, append it to a real
document.body/element and paint the result. Repeated lookups
preserve JS object identity; dynamically added click listeners and
timer mutations work after host replay. The authoritative op_dom
prevents parent/ancestor cycles. The v1/v2 Test262 runtime baseline
is unchanged at 82/91 and 78/179 attempted (110 v2 SKIPs).
General DOM removal, new Text nodes, attributes and style mutation
are still missing.

## M4.21: real DOM Text/removal/attributes

The native VM now creates Text nodes, moves and removes real
elements, changes attributes and recascades styles after callbacks.
Nine end-to-end page tests pass, including same-script nested
ID lookup on detach/reinsert. Test262 Runtime v1 82/91, v2
78/179 attempted (110 skipped), unchanged and limited in scope.

## M4.22: live DOM collections and CSS interfaces

DOM Elements/Text now have limited live parentNode/childNodes/
firstChild/lastChild semantics; classList and element.style
update real author attributes and native CSS. replaceChild
and remove can reorder/retire real nodes. Nine new
script-to-native-pixels scenarios are covered, including
timer callbacks and synthetic-to-physical node identity.
Full DOM/CSSOM/WPT compatibility is still unmeasured and partial.

## M4.23: first pinned upstream WPT DOM smoke

Live Node sibling/connectivity properties and Element.contains
work in the original JS/DOM pipeline. classList supports atomic
multi-token add/remove; style parsing protects quoted semicolons
and balanced functions. Three new native DOM rendering tests pass.

A limited adapter executes unchanged assertions from the pinned
upstream WPT Node-childNodes-cache.html fixture: one attempted/pass,
three manually chosen fixtures explicitly unsupported/skipped.
No representative DOM WPT conformance percentage exists yet.

## M4.24: seven original WPT DOM fixtures and live Element.children

Manual pinned WPT DOM v2 selection has seven attempted/passing
original source files and three explicit unsupported SKIPs.
Native Element.children now exposes a live HTMLCollection with
basic indices/item/namedItem and physical node identity.
No official WPT DOM conformance score is claimed.

## M4.25: ten original WPT DOM fixtures and live tag lookup

Live Document/Element.getElementsByTagName and hasAttribute(s)
now operate on the authoritative original DOM; added native
script/timer repaint regressions. The manually selected WPT DOM
smoke v3 attempts/passes ten original pinned HTML fixtures,
eleven test() callbacks in total, with three explicit skips.
Failure status cannot be erased by later tests. Broader WPT
conformance and async harness are still incomplete.

## M4.26: scoped selectors, native programmatic clicks and WPT v4

Original JS now supports a bounded in operator, simple DOM
querySelector/querySelectorAll with static result lists, and
element.click routed through native capture/target/bubble event
handlers. Five new native rendering integrations pass, including
deferred mutations and saved object identity. Pinned manually
selected original DOM WPT smoke v4: 11/11 attempted files PASS,
3 SKIP; a narrow sample, not overall WPT conformance.
