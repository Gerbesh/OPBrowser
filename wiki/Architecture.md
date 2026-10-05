# Architecture

The browser is split into small Rust crates so platform UI and web-engine logic do
not collapse into one dependency knot.

Current crates:

- op_browser: application/browser-process entry point.
- op_engine: subsystem orchestration.
- op_platform_win: Win32 integration.
- op_dom: document/node storage and element attributes.
- op_html: HTML tokenizer and tree builder.
- op_css: CSS/style subsystem.
- op_js: original ECMAScript runtime.
- op_net: networking and future request filtering.

The first parser path is now live:

HTML string -> op_html::Tokenizer -> op_html::parse_document -> op_dom::Document.

The current engineering priority continues the same vertical slice:

Document -> layout -> display list -> Win32 pixels.

The current tree builder is intentionally an early subset. It is not yet a complete
WHATWG HTML tree-construction implementation; compatibility work will progressively
replace subset behavior with specification-defined insertion modes and error recovery.

Multi-process isolation is a planned architectural requirement, not yet implemented.
