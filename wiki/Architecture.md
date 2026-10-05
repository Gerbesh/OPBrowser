# Architecture

The browser is split into small Rust crates so platform UI and web-engine logic do
not collapse into one dependency knot.

Current crates:

- op_browser: application/browser-process entry point.
- op_engine: subsystem orchestration.
- op_platform_win: Win32 integration.
- op_dom: document/node storage.
- op_html: HTML tokenizer and tree builder.
- op_css: CSS/style subsystem.
- op_js: original ECMAScript runtime.
- op_net: networking and future request filtering.

Current engineering priority is the first vertical rendering slice:

HTML -> tokenizer -> tree builder -> DOM -> layout -> display list -> Win32 pixels.

Multi-process isolation is a planned architectural requirement, not yet implemented.
