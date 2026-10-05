# Architecture

The browser is split into small Rust crates so platform UI and web-engine logic do
not collapse into one dependency knot.

Current crates:

- op_browser: application/browser-process entry point.
- op_engine: subsystem orchestration.
- op_platform_win: Win32 integration and current temporary GDI backend.
- op_dom: document/node storage and element attributes.
- op_html: HTML tokenizer and tree builder.
- op_css: CSS/style subsystem.
- op_layout: platform-neutral text/layout geometry.
- op_paint: platform-neutral display list.
- op_js: original ECMAScript runtime.
- op_net: networking and future request filtering.

The first visible renderer path is live:

HTML -> tokenizer -> tree builder -> DOM -> layout -> display list -> WM_PAINT -> pixels.

GDI is deliberately isolated inside op_platform_win. It is not responsible for HTML,
CSS, layout, or paint decisions. That means the Windows graphics backend can later be
replaced with DirectWrite/Direct2D/DirectComposition without rewriting the web engine.

The current tree builder and layout are early subsets, not complete WHATWG/CSS
implementations. Compatibility work will progressively replace subset behavior with
specification-defined algorithms.

Display-list storage is currently process-global because M1 has one browser window.
Multi-window and multi-process work will replace this with explicit per-window /
per-renderer ownership.

Multi-process isolation remains a planned architectural requirement.
