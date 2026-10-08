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
