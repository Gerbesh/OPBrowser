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

## R1.0: bounded CSS overflow clipping reaches native pixels

Real `overflow:hidden/clip` is now implemented for normal-flow
block descendant rendering and hit testing. Layout intersects
nested padding-box clipping ranges, the paint display list carries
them across stacking-context ordering, and the Win32 painter
clips background/text/image pixels and link bounds. Tests verify
visible versus clipped GDI pixels, nested scopes, CSS cascade
and actual mouse-click suppression outside the clipped region.

The original frozen WPT Static v1 remains 197/200 (exact) and
Positioning v1 53/100 without render errors. These are not
broad web-compatibility scores. **Still missing:** scrollTop,
per-container scroll offsets, wheel input for nested containers,
scrollbars, overflow-x/y longhands, and certain positioned/inline
fragment cases. overflow:scroll/auto deliberately stays visibly
unclipped until scrolling is implemented. No GC yet.

## M4.35: original WPT v8 and roadmap correction

The pinned original-source WPT DOM/Events v8 smoke selects
24 manually chosen HTML files: **21 attempted PASS, 0 FAIL,
3 explicit SKIP**. New fixtures test redispatch, event stop
flags set before dispatch, and omitted capture options.
Host document.documentElement and Event phase constants now work
through original DOM and paint, and dispatch respects pre-stopped
events. This sample does not measure broad web compatibility.

**P0 changed to working rendering/overflow scrolling and
a real original JS garbage collector.** Previously, too much
work went to narrowly passing EventTarget semantics while
visible CSS and long-session memory were incomplete. Current
object/environment arenas are append-only and have no GC.
M4.35 adds heap_usage() diagnostics, not reclamation.

The [active project plan](https://github.com/Gerbesh/OPBrowser/blob/main/docs/PROJECT_PLAN.md)
has been reduced from 1618 lines to a short priority roadmap;
historical details remain in the development log and Git.

## M4.34: measured original WPT Events progress

Verified upstream commit 97fe10c5d0e12e4a9d90f77b8db0602c64f3ad2d. Old v5 still passes 12/12 attempted, 3 SKIP. New frozen v7 selects 19 original HTML fixtures: **16 attempted PASS, 0 FAIL, 3 SKIP**, 39 untouched synchronous WPT callbacks. The four added upstream Events sources cover defaultPrevented, returnValue, post-dispatch srcElement and cancelBubble.

Original VM fixes: Function.prototype.call, Event.srcElement and propagation flag resets. The narrow WPT shim supports test context/step_func and sticky first failures. Not the official WPT harness or a global compliance score.

## M4.33a-b: EventTarget objects and EventListener.handleEvent

new EventTarget() now creates an original JS event emitter with
addEventListener/removeEventListener/dispatchEvent and correct
EventTarget.prototype ancestry; window/document/Elements/
AbortSignal also inherit from that prototype. Standalone targets
dispatch synchronously on a one-element path, with capture and
bubble listeners, cancellation and existing once/passive/signal
options. No external JS engine or browser runtime is embedded.

Listeners can be functions or objects with handleEvent(event).
An object listener sees its own object as this; handleEvent is
looked up when called, so replacement works without re-registering.
Object identity is used for remove/dedup; null/undefined callbacks
are no-ops. Bad handleEvent methods are reported without preventing
other callbacks. Ten VM and two native paint integrations pass.
EventTarget WebIDL/prototype descriptors, complete callback
coercion, Event subclasses and broad original WPT Events
conformance remain incomplete. Upstream WPT scores unchanged.

## M4.32b: trusted native clicks and Event.composed flags

Original native hit-tested clicks now produce events with
isTrusted=true, while programmatic Element.click() produces
isTrusted=false on the same original capture/target/bubble code
path. Both clicks are composed in the existing light DOM.
new Event(type,{composed:true}) retains that flag, with false by
default. Host document/window lifecycle and AbortSignal abort
events expose trusted host state and composed=false; legacy/new
scripted Event objects remain untrusted.

Original JS assignment can no longer forge the common read-only
event type/target/phase/trust/cancelation flags. Four VM and two
native integrations pass, including a click from Engine's actual
hit-tested layout region. No Shadow DOM, complete default
activation, full WebIDL descriptors or broad WPT conformance.
Pinned original WPT/Test262 scores are unchanged.

## M4.32a: Event.composedPath and cancelBubble

Original new Event(), document.createEvent() and native host events
now expose Event.composedPath(): an Array of the target-first
propagation route while dispatching and an empty Array outside
dispatch. Connected Element paths include all real HTML ancestors,
document and window. Document/window/AbortSignal targets get their
own bounded paths. Per-event state is cleared after dispatch
before queued microtasks execute.

Legacy event.cancelBubble now reads/writes the original propagation
stop state; false cannot undo a previously set stop. Native click
events also honor the legacy returnValue cancellation alias.
Five VM and two original page-to-pixel tests pass. Shadow DOM
retargeting, composed options across encapsulation, complete
Event WebIDL and broad WPT Events conformance remain unsupported.
The frozen upstream WPT baseline is unchanged.

## M4.31: stable EventTarget listener identity and handler ordering

Original Element, document, window and AbortSignal listeners now
carry registration identity tokens. Removing and re-adding the same
JS callback during a dispatch no longer accidentally fires the new
registration from an earlier event snapshot. The IDs carry across
JS-created Element virtual-to-native DOM binding and interact with
once/passive/AbortSignal listener options.

onclick callbacks now share registration-time ordering with Element
listeners; document.onreadystatechange, window.onload and
AbortSignal.onabort similarly use the target-phase listener order,
rather than always executing last. Removing/replacing properties
honors event snapshots. Eleven new JS VM tests and four native
DOM-paint integrations pass. Shadow DOM, full WebIDL, callback
listener objects, default activation and broad original upstream
WPT Events conformance remain unsupported. Previous WPT scores
are unchanged.

## M4.30b-d: DOMException, AbortSignal.any/timeout, fetch abort

OPBrowser now has original VM DOMException objects with readable
AbortError/TimeoutError reasons, legacy codes and toString().
AbortSignal.timeout (bounded to 60 seconds) runs through the
page-owned scheduler. AbortSignal.any composes up to 64 array-like
inputs and propagates the first abort reason through dependent
signals, cleaning up listener registrations.

The self-hosted Request and Promise-based fetch support RequestInit
signal. Aborting rejects the Promise with the same reason, deletes
queued work and discards late HTTP completions. A WinHTTP worker
already running is not forcibly interrupted and may continue until
completion. Five signal/exception VM tests, five fetch VM tests,
three cross-feature VM tests and one real delayed HTTP/DOM paint
integration pass. Existing WPT/Test262 baselines were not rerun.
Missing: complete iterable support, non-GET/CORS/streaming fetch,
physically interrupted network I/O and broad standards conformance.

## M4.30a: AbortController/AbortSignal and signal listener options

Original JavaScript now supports new AbortController(), the persistent
controller.signal with read-only aborted/reason state, abort(reason),
AbortSignal.abort(reason) and signal.throwIfAborted(). AbortSignal is
a bounded EventTarget supporting 'abort' listeners and onabort.
Element, Document and Window addEventListener accept {signal};
aborted signals suppress new registrations and abort() synchronously
removes existing signal-bound callbacks, even during active dispatch.
Seven VM and two real DOM/native paint tests pass, including
post-load timer cancellation. Limitations: default reason is a
string rather than DOMException; AbortSignal.timeout()/any(), fetch
cancellation, and complete WPT Events compatibility are pending.
Frozen original upstream compatibility metrics remain unchanged.

## M4.29b: connected Element events reach Document and Window

Original Element.dispatchEvent, Element.click and native hit-tested
clicks now travel through window capture, document capture, ancestor
capture, target, ancestor bubble, document bubble and window bubble.
Global capture runs even for nonbubbling events. Event.bubbles,
stopPropagation and stopImmediatePropagation retain their restrictions.
Detached and removed nodes do not deliver to document/window. Native
hit testing recognizes document/window-only click listeners and
replays resulting JavaScript DOM changes into the original native
layout and paint engine. Five VM and two native integration tests
cover these scenarios. No new dependency or third-party web engine.
AbortSignal, Shadow DOM, default actions and broad WPT conformance
remain unsupported; frozen original WPT counts are unchanged.

## M4.29a: document/window EventTarget

Document and window accept event listener option dictionaries with
capture, once and passive. Original lifecycle readystatechange,
DOMContentLoaded and load callbacks honor one-shot removal,
immediate propagation stops and isolated callback exceptions.
Both targets now expose dispatchEvent(new Event(...)) for custom
events; document dispatch performs window capture, document target
and optional window bubble. The original runtime retains bounded
recursion, cleanup and cancellation behavior. VM and native
DOM-paint integrations pass. Element-originating event paths do not
yet include window/document; AbortSignal and official asynchronous
WPT support remain pending. The frozen original WPT v5 numbers
were not changed.

## M4.28b: once/passive listeners and isolated event errors

Original Element.addEventListener accepts an options dictionary with
capture, once and passive, and removeEventListener accepts capture
matching. once callbacks are removed before nested dispatches; passive
listeners cannot cancel using preventDefault or legacy returnValue.
Normal callback exceptions are recorded in a bounded diagnostic
accessible through Engine.active_event_listener_errors, without
interrupting later listeners. Execution budgets still stop runaway
JavaScript. Original JS -> DOM -> native paint integration tests cover
timers and dynamically created nodes. The frozen original WPT DOM
smoke v5 sample and its counts are unchanged. Document/window
listener options and AbortSignal remain outside this milestone.

## M4.28a: stopImmediatePropagation

Native clicks and custom/legacy Event objects support stopImmediatePropagation(), which skips remaining callbacks on the same target and halts further propagation. Custom events reset this flag before each new dispatch. VM and native DOM/paint tests pass. once/passive options and exception isolation remain incomplete; the frozen WPT DOM/Events v5 sample is unchanged.

## M4.27: bounded compound selectors and dispatchEvent

Original Document/Element selectors now match descendant
and direct-child chains plus compound tag/class/id
and comma groups. Element.matches/closest share the
same bounded grammar. Event() / Element.dispatchEvent
support custom event types, capture/target/bubble,
preventDefault, stopPropagation and return-value semantics,
with legacy createEvent/initEvent. Seven new native page
integration tests pass.

Original pinned WPT DOM/Events smoke v5: 12 attempted
original HTML files passing, 3 explicit unsupported SKIP.
This remains manually scoped, not an overall DOM score.
