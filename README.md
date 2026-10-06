# OPBrowser

Public repository: https://github.com/Gerbesh/OPBrowser

Status: early engine development. OPBrowser opens external HTTP/HTTPS HTML pages,
local files and HTML data URLs from a native address bar or command-line argument.
Its own tokenizer, tree builder, DOM, text layout and display list render the page
into a Win32 window. Text hyperlinks support current-window navigation, including
relative HTTP(S) and local-file links. PNG/JPEG/GIF/BMP images load from HTTP(S),
local files and data URLs, with dimensions, transparency, alt fallback and image links.
Initial embedded, inline and external author CSS plus a block-level box model now reach native pixels; JavaScript remains future work.

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
  -> op_net source loader (WinHTTP for transport/TLS only)
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
a document address. See [source loading](wiki/Document-Source-Loading.md) for limits.
Image subrequests follow that document load on the same worker. Windows WIC performs
only raster decoding; OPBrowser owns resource policy, layout and painting. Images
share measured lines with text; GIF shows the first frame. Line layout supports
baseline alignment, wrapping, `<br>` and HTML whitespace. Initial CSS supports embedded
`<style>`, `style=""`, and bounded `<link rel="stylesheet">` from local/file/data/HTTP(S),
type/class/ID/universal selectors, attribute selectors, descendant/child/adjacent/general
sibling combinators, :root/:first-child/:last-child/:only-child/:empty/:link, cascade/inheritance,
`display` inline/block/none, #hex/basic named colors, legacy/modern rgb()/rgba()/hsl()/hsla(),
relative/absolute font-size lengths, numeric/normal/bold font weight, inherited text-align
and real line-height line-box geometry. The block box model includes
margin/padding shorthands and side longhands,
auto/negative/percentage margins, background color, independent solid/none border sides,
width/height with min/max, `box-sizing`, percentages, em/rem and CSS absolute length units.
Adjacent sibling vertical margins collapse. External CSS is merged with embedded rules in
DOM source order and retained across resize reflow. Parent/child margin collapse, definite
percentage-height propagation, functional pseudos/pseudo-elements, inline box decorations,
advanced Color 4 spaces/functions, `@import`, general media queries, CSS `url(...)` resources
and JavaScript
are not implemented yet;
link glyphs still use the native default blue. See [CSS foundation](wiki/CSS-Syntax-Foundation.md),
[CSS box model](wiki/CSS-Box-Model.md), [stylesheet loading](wiki/Stylesheet-Loading.md),
[inline layout](wiki/Inline-Layout.md) and [image loading](wiki/Image-Loading.md).

See:

- [Project plan](docs/PROJECT_PLAN.md)
- [Requirements](docs/REQUIREMENTS.md)
- [Code graph](docs/CODE_GRAPH.md)
- [Code slices](docs/CODE_SLICES.md)
- [Development log](docs/DEV_LOG.md)
- [Local wiki source](wiki/Home.md)
- [Language/platform ADR](docs/ADR-0001-language-and-platform.md)

The HTML named-reference data is derived from WHATWG and incorporated under
BSD-3-Clause; see [the third-party notice](third_party/WHATWG-HTML-LICENSE.txt).
Include this notice when distributing browser binaries.
