# OPBrowser

Public repository: https://github.com/Gerbesh/OPBrowser

Status: early engine development. OPBrowser opens external HTTP/HTTPS HTML pages,
local files and HTML data URLs from a native address bar or command-line argument.
Its own tokenizer, tree builder, DOM, text layout and display list render the page
into a Win32 window. CSS, images, JavaScript and clickable page links are not yet
implemented; this milestone displays static HTML text.

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
  -> op_layout
  -> op_paint display list
  -> op_platform_win
  -> Win32 pixels
```

Current usage examples:

```text
target\release\op_browser.exe "https://example.com"

target\release\op_browser.exe examples\hello.html
target\release\op_browser.exe "file:///C:/path/to/page.html"
target\release\op_browser.exe "data:text/html,%3Ch1%3EHello%3C%2Fh1%3E"
```

You can also start `target\release\op_browser.exe`, paste `https://example.com`
into the address bar, and press Enter or Go. Ctrl+L selects the address, F5 reloads,
Back/Forward traverse history, and the mouse wheel scrolls. Loads run on a worker
thread; errors appear in the status line and preserve the previous page/history.

Initial network support requires Windows, an ASCII hostname (or an IPv4/bracketed
IPv6 literal), an explicit `http://` or `https://` scheme, and UTF-8 HTML. Requests
use system proxy/TLS settings, keep certificate validation enabled, follow at most
five redirects, reject HTTPS-to-HTTP redirects, and limit decoded HTML to 2 MiB.
Cookies and automatic authentication are disabled. No requests run until you supply
a document address. See [source loading](wiki/Document-Source-Loading.md) for limits.

See:

- [Project plan](docs/PROJECT_PLAN.md)
- [Requirements](docs/REQUIREMENTS.md)
- [Code graph](docs/CODE_GRAPH.md)
- [Code slices](docs/CODE_SLICES.md)
- [Development log](docs/DEV_LOG.md)
- [Local wiki source](wiki/Home.md)
- [Language/platform ADR](docs/ADR-0001-language-and-platform.md)
