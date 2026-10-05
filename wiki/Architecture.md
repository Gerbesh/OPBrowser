# Architecture

The browser is split into small Rust crates so platform UI and web-engine logic do
not collapse into one dependency knot.

Current crates:

- op_browser: entry point, native-event routing and navigation worker/channels.
- op_engine: subsystem orchestration.
- op_platform_win: Win32 integration and current temporary GDI backend.
- op_dom: document/node storage and element attributes.
- op_html: HTML tokenizer and tree builder.
- op_css: CSS/style subsystem.
- op_layout: platform-neutral text/layout geometry.
- op_image: validated raster buffers and Windows WIC infrastructure codec adapter.
- op_paint: platform-neutral display list.
- op_js: original ECMAScript runtime.
- op_net: source loading, initial owned HTTP URL parsing, bounded WinHTTP transport
  and response validation; future cache/cookies/request filtering.

The first visible renderer path is live:

HTML -> tokenizer -> tree builder -> DOM -> layout -> display list -> WM_PAINT -> pixels.

GDI is deliberately isolated inside op_platform_win. It is not responsible for HTML,
CSS, layout, or paint decisions. That means the Windows graphics backend can later be
replaced with DirectWrite/Direct2D/DirectComposition without rewriting the web engine.

The current tree builder and layout are early subsets, not complete WHATWG/CSS
implementations. Compatibility work will progressively replace subset behavior with
specification-defined algorithms.

External HTML travels from address input through a worker-owned Engine and op_net
into the same original renderer. Structural containers preserve nested heading /
paragraph defaults. Windows WinHTTP supplies HTTP/TLS/proxy/framing/decompression
only; it is an OS infrastructure API, not a browser engine. TLS certificate checks
remain enabled. The UI owns window handles and updates them only on its own thread.

Layout preserves inline href metadata as LinkSpan byte ranges on each wrapped line.
Paint commands carry these ranges to GDI, which measures LinkRegion bounds for
native cursor/click hit testing. Engine tracks the last loaded document address
separately from history requests so link bases follow redirects during reload.

Display-list, scroll and measured link-region storage are currently process-global
because M1 has one browser window.
Multi-window and multi-process work will replace this with explicit per-window /
per-renderer ownership.

Multi-process isolation remains a planned architectural requirement.

Document decoding is owned by op_net::encoding: compact Windows-1251/1252 tables,
strict Unicode conversion, charset aliases and a bounded byte-level meta prescan.
HTTP, local-file and data loaders pass decoded Unicode to the original HTML parser.
op_html::references consumes all standard named and numeric character references inside
tokenizer text/attribute states; initial raw-text/RCDATA handling preserves the
context rules. Its Characters result carries one or two Unicode scalars. A sorted
eight-byte NamedEntry table and packed strings replace per-name pointers; prefix
ranges limit lookup to 31 input characters without allocation. A Python standard
library tool generates the table offline from pinned WHATWG data; Cargo builds use
the committed Rust output. No ready-made parsing/browser engine is involved.

Raster image subresources are coordinated by op_engine::images and loaded as binary
bytes by op_net::images. The new op_image crate uses targeted Windows WIC/COM bindings
to decode supported Microsoft raster formats into premultiplied BGRA. No codec
object crosses the worker/UI boundary. Arc pixels reach original ImageBox layout
and Image paint commands; a transient GDI DIB/DC draws them with alpha.
Image rectangles inherit anchor hrefs and use existing native hit testing.
See [Image Loading](Image-Loading.md) for budgets, source policy and current limits.

Layout now accepts a TextMeasurer interface. op_engine::text owns a per-render GDI
font/DC cache on Windows and returns numeric TextMetrics; no OS handle crosses
into layout or the UI result. op_layout::flow groups inline siblings around blocks,
and op_layout::inline constructs measured lines with a shared text/image baseline.
LayoutTree::order preserves TextBox/ImageBox source order in painting. Engine and
native painter share op_paint::TEXT_FONT_FAMILY; portable layout uses approximate
metrics. See [Inline Layout](Inline-Layout.md) for the supported formatting subset.
