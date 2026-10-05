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

## S1 - Static local document to pixels

Status: IN PROGRESS.

Target path:

```text
HTML source bytes/string
  -> op_html::Tokenizer
  -> op_html::parse_document/tree builder
  -> op_dom::Document
  -> style defaults / op_css
  -> layout tree
  -> display list
  -> op_platform_win paint surface
  -> pixels in OPBrowser window
```

Implemented:

- tokenizer foundation
- DOM arena foundation
- DOM attributes
- initial tree builder
- mismatched-end-tag recovery for the current subset
- void-element handling for the current subset

Missing:

- layout primitives
- display list
- painting bridge

This is the current priority slice.

## S2 - URL navigation to static page

Status: PLANNED NEXT.

```text
address/navigation request
  -> URL parser
  -> op_net request
  -> HTTP(S) response
  -> MIME/encoding
  -> S1 static document pipeline
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
