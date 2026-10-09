# OPBrowser

[![CI](https://github.com/Gerbesh/OPBrowser/actions/workflows/ci.yml/badge.svg)](https://github.com/Gerbesh/OPBrowser/actions/workflows/ci.yml)
[![WPT static v1](https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2FGerbesh%2FOPBrowser%2Fmetrics%2Fwpt-static-v1-badge.json)](docs/COMPATIBILITY.md)
[![WPT positioning v1](https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2FGerbesh%2FOPBrowser%2Fmetrics%2Fwpt-positioning-v1-badge.json)](docs/COMPATIBILITY.md)
[![Test262 parser v1](https://img.shields.io/endpoint?url=https%3A%2F%2Fraw.githubusercontent.com%2FGerbesh%2FOPBrowser%2Fmetrics%2Ftest262-parser-v1-badge.json)](docs/COMPATIBILITY.md)

Public repository: https://github.com/Gerbesh/OPBrowser

**Documentation:** [Published Wiki](https://github.com/Gerbesh/OPBrowser/wiki) ·
[Current status](wiki/Current-Status.md) ·
[Project plan](docs/PROJECT_PLAN.md) ·
[Code Graph](docs/GENERATED_CODE_GRAPH.md) ·
[Code Slicer](docs/GENERATED_CODE_SLICES.md) ·
[Known WPT reference divergences](docs/KNOWN_TEST_DIVERGENCES.md).

Documentation snapshot (8 October 2026): frozen **WPT Static v1 strict
197/200 (98.5%)**, WPT-metadata-aware **198/200 (99%, separately reported)**,
WPT Positioning v1 **53/100 (53%)**, Test262 Parser v1 **523/1983 (26.37%)**
parse-only. These are *narrow pinned subsets*, not overall browser readiness.
Test262 runtime M4.18 adds a pinned and deterministic 289-case selection
across 25 language and built-in families. On the supported classic-script
harness subset, 65/179 (36.31%) pass and 110 unsupported cases are explicitly
SKIP. This is broader but still not a full Test262 or website score.
M4.19 adds String.prototype.charAt, Array.prototype.push/pop,
and SyntaxError inheritance for malformed JSON.parse. The pinned broad
runtime v2 reaches 78/179 (43.58%) with the same 110 explicit SKIPs.
The former 91-case arithmetic/equality suite remains 82/91 (90.11%);
the two scores are separate and must not be averaged.
Two Rec.2020 tests remain counted as failures but are classified as
known reference divergences against current CSS Color 4 gamma 2.4.

`python tools/code_intelligence.py --write` regenerates the source-backed crate
graph and curated code slices; `python tools/code_intelligence.py --check`
verifies their freshness and Wiki internal links in CI. The `wiki/`
directory remains the canonical source for the
[published GitHub Wiki](https://github.com/Gerbesh/OPBrowser/wiki).
Use `python tools/publish_wiki.py --target WIKI_CHECKOUT --write` to
synchronize after documentation updates.

M4.25 extends the **manually selected** original-source WPT DOM smoke
to v3: 13 selected upstream files, 10 attempted/passed, 3 explicit SKIP.
The 10 attempted files contain 11 synchronous test() callbacks,
all accounted for by a sticky-failure multi-test shim. A failing
callback cannot be overwritten by later successes. This is NOT the
official WPT testharness or a broad DOM conformance percentage.
The original JS/DOM engine now supports Element.hasAttribute(s)
and live Document/Element.getElementsByTagName with indexed/item
access across DOM edits and timers.

M4.24 adds live Element.children (numeric indices, item, basic namedItem),
childElementCount, firstElementChild, lastElementChild and element sibling
relationships. The pinned WPT DOM smoke now attempts and passes seven
manually selected original upstream fixtures, with three explicit skips.
The narrow sync harness is not official WPT or overall DOM conformance.

M4.23 adds Node.previousSibling/nextSibling/isConnected/contains,
nodeName/tagName/ownerDocument and Text.length, plus atomic
multi-token DOMTokenList mutations and semicolon-aware CSS style
declarations (quoted strings and balanced function parentheses).
The browser now runs its first **original pinned upstream WPT DOM
source fixture**, using a deliberately limited synchronous harness
adapter: 1 attempted PASS, 3 explicitly SKIP (4 manually selected
fixtures). This is **not** an official WPT score or a representative
DOM conformance result. The old Test262 Runtime v1/v2 baselines
remain separate and unchanged.

M4.22 adds live parentNode, firstChild/lastChild and stable childNodes
collections (length/index/item), replaceChild/remove, classList token
operations and a bounded live style object (display/cssText,
setProperty/getPropertyValue/removeProperty). These operate on actual
native DOM and author CSS, even across timer callbacks and transition
from newly created JS node handles to physical NodeIds. Nine new
end-to-end page tests verify the visible results.

M4.21 adds genuine Text nodes, insertBefore/removeChild,
setAttribute/getAttribute/removeAttribute, and live id/class/style changes.
Nested detach/reattach retains JS identity and updates lookup. Author CSS
is refreshed after timer/click mutations, keeping previously loaded linked
stylesheets and color profiles. Nine new native paint tests cover the slice.

M4.20 adds the first real dynamic DOM path: document.createElement(),
Element.appendChild(), element.id, and element.textContent now mutate the
authoritative DOM tree through page-owned bounded operations, with layout,
computed style and native paint after script/timer/click execution.
document.body is available when the HTML tree-builder creates it; dynamic
JS element identity survives getElementById and event dispatch. This is
not yet a full DOM API: removeChild/insertBefore/createTextNode, general
attributes and style mutation are still missing.

Status: early engine development. OPBrowser opens external HTTP/HTTPS HTML pages,
local files and HTML data URLs from a native address bar or command-line argument.
Its own tokenizer, tree builder, DOM, text layout and display list render the page
into a Win32 window. Text hyperlinks support current-window navigation, including
relative HTTP(S) and local-file links. PNG/JPEG/GIF/BMP images load from HTTP(S),
local files and data URLs, with dimensions, transparency, alt fallback and image links.
Initial embedded, inline and external author CSS plus block/flex/float layout and first relative/absolute/fixed positioning now reach native pixels. A standalone original JavaScript lexer/parser/bytecode/VM with objects, closures, exceptions, `this`, constructors, `arguments` and initial Error objects exists, and a first bounded inline `<script>` execution path can now update
DOM text by id before paint. Bounded relative/local and same-origin
external classic scripts now use the filtered network loader and execute
at parser boundaries. A first native `click` event slice now retains JS
handlers registered by `addEventListener` or `onclick`, allowing text
changes and reflow after user interaction. M4.5 adds capture-target-bubble and
listener removal plus event cancellation state. M4.6 runs classic scripts
during DOM tree construction. M4.7 adds bounded external classic defer
(after parsing, in order) and async (concurrent source fetching,
execution at parser/end-of-load polling points). An independent event loop,
document.write and broad Web APIs are **not** implemented. M4.8 now adds
host-owned document.readyState transitions and initial-load DOMContentLoaded/
window load event delivery after the parser/defer/async boundaries.
M4.9 adds page-owned setTimeout/clearTimeout with bounded task pumping
on the engine worker, so callbacks can repaint after presentation.
M4.10 adds repeating setInterval/clearInterval and bounded FIFO queueMicrotask
checkpoints. M4.11 adds bounded opFetchText(url, callback) for filtered
post-presentation text-network work. M4.12 introduces self-hosted Promise
reactions and a GET-only fetch()/Response.text() subset. M4.13 carries
real HTTP statuses, reason phrases, redirect URL, and filtered Headers
into Response; HTTP 404/500 fulfill with ok=false. M4.14 adds a bounded
Headers constructor, Request, safe GET RequestInit headers and manual
per-hop same-origin redirect checks, including redirect:error.
M4.16 adds a 91-case pinned Test262 classic-script runtime probe and
basic Object/Array/Boolean/Number/String/isNaN/isFinite globals.
See docs/COMPATIBILITY.md for the separate **59/91 (64.84%)** narrow
runtime result, which must not be confused with full JS conformance.
M4.15 adds strict native JSON.parse/stringify, async Response.json and
bounded self-hosted Promise.all/race/allSettled/any. Complete Fetch/CORS,
credentials, streams, caching, abort and ECMAScript Promise conformance
remain pending. See docs/PROGRESS_AUDIT.md for a scoped readiness estimate.

OPBrowser is an experimental Windows 11 browser built around an original web engine.

The project intentionally does **not** embed or fork Chromium/Blink, WebKit, Gecko,
Servo, V8, SpiderMonkey, JavaScriptCore, QuickJS, or another existing browser /
JavaScript engine.

Primary goals:

- low memory and CPU overhead;
- fast startup and responsive UI;
- minimum telemetry;
- built-in content blocking / ad-block integration;
- original HTML/CSS/DOM/layout/rendering/JavaScript engine;
- modern web-platform compatibility;
- built-in browser task manager;
- intelligent tab freezing, discarding and restoration.

Primary implementation language: **Rust**.

Current rendering path:

```text
HTTP(S) URL / local path / file: URL / data:text/html URL
  -> op_net RequestFilter + source loader (WinHTTP for transport/TLS only)
  -> op_html tokenizer/tree builder
  -> op_dom
  -> op_net bounded external stylesheet subresources
  -> op_css DOM-order author rules / selector matching / cascade / computed style
  -> op_layout CSS-aware flow + inline runs
  -> op_paint styled display list
  -> op_platform_win
  -> Win32 pixels
```

First JavaScript-to-DOM demonstration:
`target\\release\\op_browser.exe examples\\js\\dom-text.html`.
This executes two classic inline scripts through the original VM and
shows the resulting DOM text in the native window.

Interactive M4.3 demo: `target\\release\\op_browser.exe examples\\js\\click.html`.
Click the green button repeatedly to run a JavaScript event listener
loaded from sibling `click.js` and update the status without navigation.

Current usage examples:

```text
target\release\op_browser.exe "https://example.com"

target\release\op_browser.exe examples\hello.html
target\release\op_browser.exe examples\css\index.html
target\release\op_browser.exe examples\navigation\index.html
target\release\op_browser.exe "file:///C:/path/to/page.html"
target\release\op_browser.exe "data:text/html,%3Ch1%3EHello%3C%2Fh1%3E"
```

You can also start `target\release\op_browser.exe`, paste `https://example.com`
into the address bar, and press Enter or Go. Ctrl+L selects the address, F5 reloads,
Back/Forward traverse history, and the mouse wheel scrolls. Loads run on a worker
thread; errors appear in the status line and preserve the previous page/history.
Resizing the window rewraps the current page from retained DOM/image data without
refetching it. See [page reflow](wiki/Page-Reflow.md) for behavior and verification.
The start page is now styled with OPBrowser's supported CSS subset, so the release binary
shows text styling, display behavior and block margin/padding/background/borders immediately.
`examples\css\index.html` is a focused CSS fixture that loads a real external `theme.css`,
demonstrates source-order override and draws nested block boxes. Links remain blue/underlined with a hand cursor; clicking uses
the same loading/history path. The navigation example demonstrates a relative local link.

Initial network support requires Windows, an ASCII hostname (or an IPv4/bracketed
IPv6 literal) and an explicit `http://` or `https://` scheme. Documents support
UTF-8, Windows-1251, Windows-1252 and UTF-16. Encoding selection checks BOM,
transport charset and early HTML meta declarations; no declaration defaults to
strict UTF-8. The full HTML named-reference table and numeric references decode
in text and hrefs, including results containing two Unicode characters.
See [HTML text decoding](wiki/HTML-Text-Decoding.md) for exact limits. Requests
use system proxy/TLS settings, keep certificate validation enabled, follow at most
five redirects, reject HTTPS-to-HTTP redirects, and limit decompressed response bytes to 2 MiB.
Cookies and automatic authentication are disabled. No requests run until you supply
a document address. The initial native request-filter layer now runs before document,
stylesheet and image loads and supports a small Adblock-style network-rule subset,
exceptions, resource types, per-site allowlisting and counters; it does not yet ship a
public-list subscription/update UI or cosmetic filtering. See [source loading](wiki/Document-Source-Loading.md)
and [request filtering](wiki/Request-Filtering.md) for limits.
Image subrequests follow that document load on the same worker. Windows WIC performs
only raster decoding; OPBrowser owns resource policy, layout and painting. Images
share measured lines with text; GIF shows the first frame. Line layout supports
baseline alignment, wrapping, `<br>` and HTML whitespace. Initial CSS supports embedded
`<style>`, `style=""`, and bounded `<link rel="stylesheet">` from local/file/data/HTTP(S),
type/class/ID/universal selectors, attribute selectors, descendant/child/adjacent/general
sibling combinators, :root/:first-child/:last-child/:only-child/:empty/:link plus
`:is()`/`:where()`/`:not()`/`:nth-child()`/`:nth-last-child()` (including `of` filters)
and terminal `::before`/`::after`, cascade/inheritance,
`display` inline/block/none, #hex/all 148 opaque CSS named colors, legacy/modern rgb()/rgba()/hsl()/hsla(),
modern RGB/HSL/HWB and color(srgb ...)/color(srgb-linear ...) with alpha and used-value none components,
the separate `transparent` keyword,
relative/absolute font-size lengths, numeric/normal/bold font weight, inherited text-align,
real line-height geometry, italic/oblique font style, underline/line-through decoration,
white-space normal/nowrap/pre/pre-wrap/pre-line, letter/word spacing and text-transform
none/uppercase/lowercase/capitalize. Inline non-replaced elements also form real
background/padding/solid-border fragments that participate in wrapping and alignment.
Named lookup uses a compact 2,210-byte static table and allocation-free ASCII case-insensitive
binary search; source and generator are pinned.
Nested decorated inline ancestors remain visible around text, images, empty boxes and pseudos;
each wrapped line reserves all ancestor edges and paints outer backgrounds before inner ones.
CSS URL tokens preserve unquoted paths/data URLs and escaped characters; quoted url()
stays a function/string sequence. Generated content can mix text and URL images; the navigation
worker uses stylesheet-relative bases, shared image budgets/cache and retained resize resources.
DOM img padding/background/solid borders form atomic decorated boxes with edge-aware
line fitting and baseline geometry; nowrap suppresses soft image wrapping.
DOM images honor CSS width/height/min/max and box-sizing. HTML size attributes supply
defaults before author cascade; auto restores intrinsic sizing and one fixed dimension
preserves ratio. Percentage heights await containing-height propagation.
An inline before/after containing only one URL gets the same CSS sizes/decorations;
mixed content images keep intrinsic anonymous item sizing.
Block DOM/sole-URL images use intrinsic/CSS width, auto margins, precise border-box height
and adjacent vertical margin collapsing through one replaced-image geometry path.
Unavailable sole-URL images and DOM images without nonempty alt retain CSS geometry around
zero natural dimensions, without raster paint. Nonempty alt uses styled text fallback.
Author declarations retain their style/link/inline source node; the engine retains effective
external stylesheet addresses after redirects, preparing correct CSS resource bases.
`background` also supports the initial color-only shorthand subset. Inherited, case-sensitive
CSS custom properties (`--name`) now participate in the author cascade and `var(--name, fallback)`
substitution before supported value parsing, including generated `content`. `::before`/`::after`
content can concatenate quoted strings, `attr(name)`, `counter(name[, style])` and
`counters(name, separator[, style])`; initial `counter-reset`, `counter-set` and
`counter-increment` feed those values. Inherited `quotes` pairs and the four quote keywords
track document-order nesting; `<q>` receives default before/after quotation marks.
Generated content keeps inherited text styling and its own inline
background/padding/solid-border fragments. Generated `display:block` uses the same box model
as ordinary blocks, including empty decorated boxes. The block box model includes margin/padding shorthands and side longhands,
auto/negative/percentage margins, background color, independent solid/none border sides,
width/height with min/max, `box-sizing`, percentages, em/rem and CSS absolute length units.
Adjacent sibling vertical margins collapse. External CSS is merged with embedded rules in
DOM source order and retained across resize reflow. Parent/child margin collapse, definite
percentage-height propagation, broader custom-property grammar/registration and
language-aware automatic quotes, full
full HTML image-state/quirks-mode fallback semantics, relational selectors, sliced inline decoration edges,
advanced Color 4 spaces/functions,
`@import`, general media queries and CSS `url(...)` background resources are not implemented yet.
The standalone JavaScript engine foundation exists, but page script discovery/execution and DOM/Web API bindings remain unimplemented. Links receive a blue/underlined computed UA default; author CSS
controls their color and decoration through native painting. See [CSS foundation](wiki/CSS-Syntax-Foundation.md),
[CSS box model](wiki/CSS-Box-Model.md), [stylesheet loading](wiki/Stylesheet-Loading.md),
[inline layout](wiki/Inline-Layout.md) and [image loading](wiki/Image-Loading.md).

Custom-property cycles include self-references and references in unused fallbacks. An
iterative dependency graph resolves long chains; empty values remain valid. Substitution
is bounded to 16,384 tokens/256 KiB per value, 2 MiB retained custom values per element/pseudo
and 64 nested fallback levels. Over-budget values become invalid and permit consumer fallbacks.
Invalid computed var() winners keep their cascade priority and become unset for supported
properties; they cannot reveal older declarations. Malformed var() syntax is rejected earlier.
`:is()`/`:where()` discard invalid argument branches and keep supported selectors;
`:not()` and nth `of` lists remain strict. Nth filters count matching element siblings
once in either direction and add the maximum filter specificity. An+B parsing preserves
token sign/whitespace rules; functional selector nesting is limited to 64 levels.
Empty generated and ordinary inline boxes reserve padding/border edges and paint decorations
without creating text glyphs; wrapping, nowrap and alignment use the same line formatter.
Typed structural selectors first/last/only-of-type and nth-of-type/nth-last-of-type count
same-tag siblings, ignoring intervening other element types and text.

Product architecture is also moving earlier instead of waiting for a giant M5 rewrite.
`op_browser_core` now contains the UI-independent tab/lifecycle/discard-policy model,
including active/background/throttled/frozen/discarded/restoring states and protected-tab
rules. The visible Win32 product is still single-tab; native tab UI, renderer processes and
actual memory-pressure teardown/restoration are the next integration steps.

Compatibility measurement has started with a project-owned baseline command plus an
optional parse-only Test262 probe. These are regression/parse measurements, not invented
WPT percentages. See [compatibility measurement](docs/COMPATIBILITY.md).

See:

- [Project plan](docs/PROJECT_PLAN.md)
- [Requirements](docs/REQUIREMENTS.md)
- [Compatibility measurement](docs/COMPATIBILITY.md)
- [Code graph](docs/CODE_GRAPH.md)
- [Code slices](docs/CODE_SLICES.md)
- [Development log](docs/DEV_LOG.md)
- [Local wiki source](wiki/Home.md)
- [Language/platform ADR](docs/ADR-0001-language-and-platform.md)
- [Browser/renderer process ADR](docs/ADR-0002-process-model.md)
- [Windows text-shaping ADR](docs/ADR-0003-text-shaping.md)

The HTML named-reference data is derived from WHATWG and incorporated under
BSD-3-Clause; see [the third-party notice](third_party/WHATWG-HTML-LICENSE.txt).
Include this notice when distributing browser binaries.
