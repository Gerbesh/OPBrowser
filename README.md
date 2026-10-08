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
Two Rec.2020 tests remain counted as failures but are classified as
known reference divergences against current CSS Color 4 gamma 2.4.

`python tools/code_intelligence.py --write` regenerates the source-backed crate
graph and curated code slices; `python tools/code_intelligence.py --check`
verifies their freshness and Wiki internal links in CI. The `wiki/`
directory remains the canonical source for the
[published GitHub Wiki](https://github.com/Gerbesh/OPBrowser/wiki).
Use `python tools/publish_wiki.py --target WIKI_CHECKOUT --write` to
synchronize after documentation updates.

Status: early engine development. OPBrowser opens external HTTP/HTTPS HTML pages,
local files and HTML data URLs from a native address bar or command-line argument.
Its own tokenizer, tree builder, DOM, text layout and display list render the page
into a Win32 window. Text hyperlinks support current-window navigation, including
relative HTTP(S) and local-file links. PNG/JPEG/GIF/BMP images load from HTTP(S),
local files and data URLs, with dimensions, transparency, alt fallback and image links.
Initial embedded, inline and external author CSS plus block/flex/float layout and first relative/absolute/fixed positioning now reach native pixels. A standalone original JavaScript lexer/parser/bytecode/VM with objects, closures, exceptions, `this`, constructors, `arguments` and initial Error objects exists, but page `<script>` execution, DOM bindings and Web APIs are not connected yet.

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
