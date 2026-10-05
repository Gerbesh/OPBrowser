# OPBrowser Code Slices

Last updated: 2026-10-05

A code slice is an end-to-end path through the architecture that produces one
observable capability. This prevents isolated subsystems from becoming impressive
piles of code that never form a browser.

## S0 - Native process startup

Status: COMPLETE at M0 level.

Path:

```text
op_browser::main
  -> op_engine::Engine::start
  -> op_platform_win::NativeBrowserWindow::create
  -> Win32 CreateWindowExW
  -> NativeBrowserWindow::run_message_loop
```

Verification:

- cargo run -p op_browser -- --smoke-test
- manual native window launch

## S1 - Local/data document to pixels

Status: COMPLETE at initial M1 level.

Current working path:

```text
filesystem path / file: URL / data:text/html URL
  -> op_net::NetworkContext::load_document
  -> op_net::LoadedDocument
  -> op_engine::Engine::render_source
  -> op_html::Tokenizer
  -> op_html::parse_document/tree builder
  -> op_dom::Document
  -> op_layout::layout_document
  -> op_layout::LayoutTree/TextBox
  -> op_paint::build_display_list
  -> op_paint::DisplayList
  -> op_platform_win WM_PAINT/GDI backend
  -> pixels in OPBrowser window
```

Implemented:

- direct and relative filesystem paths
- Windows drive-path recognition
- file: URLs
- percent decoding for file/data URLs
- UTF-8 BOM stripping
- data:text/html with UTF-8 percent encoding
- data:text/html;base64
- tokenizer and DOM pipeline
- basic text layout and word wrapping
- display-list background/text commands
- Win32 paint backend
- synchronous UpdateWindow startup paint
- smoke verification that WM_PAINT executed

Verification:

- cargo run -p op_browser -- --smoke-test examples\hello.html
- cargo run -p op_browser -- --smoke-test "data:text/html,%3Ch1%3EData%20URL%20works%3C%2Fh1%3E"

## S2 - Navigation to static page

Status: NEXT.

```text
address/navigation request
  -> NavigationState/history
  -> source/URL parser
  -> op_net request
  -> local/data now; HTTP(S) next
  -> MIME/encoding
  -> S1 rendering pipeline
```

## S3 - CSS-styled document

Status: PLANNED.

```text
HTML + linked/inline CSS
  -> HTML DOM
  -> CSS parser
  -> selector matching
  -> cascade/computed style
  -> layout
  -> paint
```

## S4 - Scripted page

Status: PLANNED.

```text
<script>
  -> op_js lexer/parser
  -> bytecode
  -> VM
  -> Web IDL bindings
  -> DOM mutation/events
  -> style/layout invalidation
  -> repaint
```

## S5 - Managed background tab

Status: PLANNED.

```text
browser tab
  -> lifecycle scoring
  -> throttle/freeze
  -> discard snapshot
  -> process/RAM release
  -> restore
```

## S6 - Native content blocking

Status: PLANNED.

```text
resource request
  -> request classification
  -> filter engine
  -> allow/block/redirect decision
  -> op_net
  -> page-visible result/statistics
```

## Rule

When adding a major feature, either extend an existing slice or add a new slice.
A subsystem is not considered product-progress until it participates in an end-to-end
slice.
