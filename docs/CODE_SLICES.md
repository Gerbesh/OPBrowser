# OPBrowser Code Slices

Last updated: 2026-10-05

A code slice is an end-to-end path through the architecture that produces one
observable capability. This prevents isolated subsystems from becoming impressive
piles of code that never form a browser.

## S0 - Native process startup

Status: COMPLETE at M0 level.

```text
op_browser::main
  -> Engine::start
  -> NativeBrowserWindow::create
  -> Win32 CreateWindowExW
  -> NativeBrowserWindow::run_message_loop
```

## S1 - Local/data document to pixels

Status: COMPLETE at initial M1 level.

```text
filesystem path / file: URL / data:text/html URL
  -> NetworkContext::load_document
  -> LoadedDocument
  -> Engine::navigate
  -> HTML tokenizer/tree builder
  -> DOM
  -> layout
  -> display list
  -> Win32 WM_PAINT/GDI
  -> pixels
```

Verified with examples\hello.html and data:text/html startup smoke tests.

## S2 - Navigation to static page

Status: IN PROGRESS.

Working core:

```text
Engine::navigate(request)
  -> load + render succeeds
  -> NavigationState::commit_navigation
  -> NavigationEntry

Engine::go_back / go_forward
  -> historical request
  -> reload source
  -> move current_index only after success

Engine::reload
  -> current historical request
  -> reload/render
  -> history unchanged
```

Implemented:

- navigation entries with original request, normalized address and MIME type
- current index
- can_go_back / can_go_forward
- back / forward
- reload
- forward-branch truncation after new navigation
- failed-navigation rollback by commit-after-success design

Still missing for a user-visible browser navigation slice:

- native window events/address input
- display-list replacement + invalidation after navigation
- URL parser for network navigation
- HTTP(S)

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
