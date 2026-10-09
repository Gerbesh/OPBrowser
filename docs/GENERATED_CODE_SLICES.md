# Generated Code Slices

> Generated from `tools/code_slices.json` and validated against live Rust files.
> This is a curated vertical feature map, **not** an AST/call/data-flow slicer.

## S1 — HTML source to visible pixels

Status: **Static pipeline implemented**.

```text
op_net::load_document
op_html::Tokenizer
op_engine::render_html
op_layout::layout
op_paint::build_display_list
op_platform_win::paint_command
```

- [`op_net::load_document`](../crates/op_net/src/lib.rs#L251)
- [`op_html::Tokenizer`](../crates/op_html/src/lib.rs#L56)
- [`op_engine::render_html`](../crates/op_engine/src/lib.rs#L249)
- [`op_layout::layout`](../crates/op_layout/src/flow.rs#L15)
- [`op_paint::build_display_list`](../crates/op_paint/src/lib.rs#L86)
- [`op_platform_win::paint_command`](../crates/op_platform_win/src/lib.rs#L653)

This path lacks complete CSS, forms, DOM scripting and a GPU backend.

## S2 — Navigation and retained layout

Status: **Initial implementation**.

```text
op_engine::navigate
op_engine::reflow
op_platform_win::paint_window
```

- [`op_engine::navigate`](../crates/op_engine/src/lib.rs#L501)
- [`op_engine::reflow`](../crates/op_engine/src/lib.rs#L434)
- [`op_platform_win::paint_window`](../crates/op_platform_win/src/lib.rs#L566)

No full History API or single-page-application lifecycle.

## S3 — Author CSS to painted box

Status: **Partial CSS**.

```text
op_engine::load
op_css::compute_styles
op_layout::layout
op_paint::build_display_list
```

- [`op_engine::load`](../crates/op_engine/src/styles.rs#L105)
- [`op_css::compute_styles`](../crates/op_css/src/lib.rs#L20)
- [`op_layout::layout`](../crates/op_layout/src/flow.rs#L15)
- [`op_paint::build_display_list`](../crates/op_paint/src/lib.rs#L86)

CSS cascade, first-line and some grouping exist; many CSS properties and layout modes remain partial.

## S4 — Color-managed image to GDI

Status: **Implemented initial raster/ICC slice**.

```text
op_engine::load
op_image::convert_icc_rgb
op_platform_win::paint
```

- [`op_engine::load`](../crates/op_engine/src/images.rs#L24)
- [`op_image::convert_icc_rgb`](../crates/op_image/src/lib.rs#L386)
- [`op_platform_win::paint`](../crates/op_platform_win/src/raster.rs#L121)

Windows WIC performs codec/color conversion; OPBrowser owns resource policy and paint.

## S5 — Original JavaScript engine boundary

Status: **Standalone VM with first bounded page DOM binding**.

```text
op_js::parse_script
op_js::JsRuntime
op_engine::Engine
```

- [`op_js::parse_script`](../crates/op_js/src/lib.rs#L19)
- [`op_js::JsRuntime`](../crates/op_js/src/lib.rs#L21)
- [`op_engine::Engine`](../crates/op_engine/src/lib.rs#L176)

Classic inline and bounded same-origin external scripts now update textContent by id. Async/defer, document.write, module scripts and full DOM/Web APIs remain unsupported.

## S7 — Classic inline JavaScript to retained DOM and pixels

Status: **M4.6 parser-paused classic page script slice**.

```text
op_html::parse_document_with_script_hook
op_engine::parse_and_execute
op_js::install_dom_snapshot
op_js::DomGetElementById
op_dom::set_text_content
op_css::compute_styles
op_engine::render
op_paint::build_display_list
```

- [`op_html::parse_document_with_script_hook`](../crates/op_html/src/tree_builder.rs#L54)
- [`op_engine::parse_and_execute`](../crates/op_engine/src/scripts.rs#L330)
- [`op_js::install_dom_snapshot`](../crates/op_js/src/runtime.rs#L482)
- [`op_js::DomGetElementById`](../crates/op_js/src/runtime.rs#L74)
- [`op_dom::set_text_content`](../crates/op_dom/src/lib.rs#L267)
- [`op_css::compute_styles`](../crates/op_css/src/lib.rs#L20)
- [`op_engine::render`](../crates/op_engine/src/lib.rs#L157)
- [`op_paint::build_display_list`](../crates/op_paint/src/lib.rs#L86)

Bounded classic execution at script closing tags during DOM construction; basic external scripts now exist but no event loop, general DOM mutations or full HTML5test support.

## S8 — External classic JavaScript through filtered network to DOM

Status: **M4.6 parser-paused same-origin external JS**.

```text
op_engine::parse_and_execute
op_net::resolve_script_source
op_net::load_script_for_page
op_net::load_script
op_js::eval_script
op_dom::set_text_content
op_engine::prepare_source
```

- [`op_engine::parse_and_execute`](../crates/op_engine/src/scripts.rs#L330)
- [`op_net::resolve_script_source`](../crates/op_net/src/scripts.rs#L14)
- [`op_net::load_script_for_page`](../crates/op_net/src/lib.rs#L157)
- [`op_net::load_script`](../crates/op_net/src/http.rs#L197)
- [`op_js::eval_script`](../crates/op_js/src/runtime.rs#L461)
- [`op_dom::set_text_content`](../crates/op_dom/src/lib.rs#L267)
- [`op_engine::prepare_source`](../crates/op_engine/src/lib.rs#L276)

Only relative/local and same-origin HTTP(S) classic scripts in source order at parser pauses. Redirect origin checked after retrieval; async/defer/integrity not supported, no browser event loop.

## S9 — Native Win32 click through bubbling DOM listeners to repainted DOM

Status: **M4.5 capture-target-bubble and cancellation events**.

```text
op_platform_win::WM_LBUTTONUP
op_browser::NavigationEvent::Click
op_engine::click_at
op_layout::click_regions
op_engine::dispatch_click
op_js::dispatch_dom_click_path
op_dom::set_text_content
op_engine::compute_styles
```

- [`op_platform_win::WM_LBUTTONUP`](../crates/op_platform_win/src/lib.rs#L314)
- [`op_browser::NavigationEvent::Click`](../crates/op_browser/src/main.rs#L168)
- [`op_engine::click_at`](../crates/op_engine/src/lib.rs#L309)
- [`op_layout::click_regions`](../crates/op_layout/src/flow.rs#L71)
- [`op_engine::dispatch_click`](../crates/op_engine/src/scripts.rs#L769)
- [`op_js::dispatch_dom_click_path`](../crates/op_js/src/runtime.rs#L1531)
- [`op_dom::set_text_content`](../crates/op_dom/src/lib.rs#L267)
- [`op_engine::compute_styles`](../crates/op_engine/src/lib.rs#L3)

Only id-bearing block regions; capture/target/bubbling and listener cancellation work, but keyboard/default actions and full DOM hit-testing remain absent.

## S10 — Parser-closing classic script through retained DOM

Status: **M4.6 parser-paused classic scripting**.

```text
op_html::parse_document_with_script_hook
op_engine::parse_and_execute
op_engine::ParserScriptRunner
op_js::refresh_dom_snapshot
op_js::eval_script
op_dom::set_text_content
op_engine::prepare_source
```

- [`op_html::parse_document_with_script_hook`](../crates/op_html/src/tree_builder.rs#L54)
- [`op_engine::parse_and_execute`](../crates/op_engine/src/scripts.rs#L330)
- [`op_engine::ParserScriptRunner`](../crates/op_engine/src/scripts.rs#L335)
- [`op_js::refresh_dom_snapshot`](../crates/op_js/src/runtime.rs#L1409)
- [`op_js::eval_script`](../crates/op_js/src/runtime.rs#L461)
- [`op_dom::set_text_content`](../crates/op_dom/src/lib.rs#L267)
- [`op_engine::prepare_source`](../crates/op_engine/src/lib.rs#L276)

Eager tokenization; no document.write reentry, module scripts, independent async event loop or DOM lifecycle events.

## S11 — Bounded defer and async classic JavaScript scheduling

Status: **M4.7 concurrent source fetch with parser polling**.

```text
op_html::parse_document_with_script_hook
op_engine::parse_and_execute
op_engine::prepare_fetch
op_net::load_script_for_page
op_engine::drain_ready
op_engine::evaluate_loaded
op_js::eval_script
op_dom::set_text_content
```

- [`op_html::parse_document_with_script_hook`](../crates/op_html/src/tree_builder.rs#L54)
- [`op_engine::parse_and_execute`](../crates/op_engine/src/scripts.rs#L330)
- [`op_engine::prepare_fetch`](../crates/op_engine/src/scripts.rs#L531)
- [`op_net::load_script_for_page`](../crates/op_net/src/lib.rs#L157)
- [`op_engine::drain_ready`](../crates/op_engine/src/scripts.rs#L354)
- [`op_engine::evaluate_loaded`](../crates/op_engine/src/scripts.rs#L390)
- [`op_js::eval_script`](../crates/op_js/src/runtime.rs#L461)
- [`op_dom::set_text_content`](../crates/op_dom/src/lib.rs#L267)

Concurrent fetch and initial-load JS polling only; no event loop, DOMContentLoaded, module scripts or SRI.

## S12 — Page readyState and DOMContentLoaded/load lifecycle to paint

Status: **M4.8 initial-load DOM lifecycle subset**.

```text
op_html::parse_document_with_script_hook
op_engine::advance_state
op_js::set_document_ready_state
op_engine::dispatch_lifecycle
op_js::dispatch_lifecycle_event
op_dom::set_text_content
op_engine::prepare_source
```

- [`op_html::parse_document_with_script_hook`](../crates/op_html/src/tree_builder.rs#L54)
- [`op_engine::advance_state`](../crates/op_engine/src/scripts.rs#L375)
- [`op_js::set_document_ready_state`](../crates/op_js/src/runtime.rs#L1347)
- [`op_engine::dispatch_lifecycle`](../crates/op_engine/src/scripts.rs#L395)
- [`op_js::dispatch_lifecycle_event`](../crates/op_js/src/runtime.rs#L1360)
- [`op_dom::set_text_content`](../crates/op_dom/src/lib.rs#L267)
- [`op_engine::prepare_source`](../crates/op_engine/src/lib.rs#L276)

Initial-load-only readyState and DOMContentLoaded/load; timers and microtasks exist separately but no Promise or post-presentation async fetch.

## S13 — Retained JavaScript one-shot timeout to native reflow

Status: **M4.9 bounded page-owned timer tasks**.

```text
op_js::SetTimeout
op_js::next_timer_wait
op_browser::recv_timeout
op_engine::tick_timers
op_js::run_due_timers
op_dom::set_text_content
op_engine::render
op_platform_win::present_reflow
```

- [`op_js::SetTimeout`](../crates/op_js/src/runtime.rs#L99)
- [`op_js::next_timer_wait`](../crates/op_js/src/runtime.rs#L1253)
- [`op_browser::recv_timeout`](../crates/op_browser/src/main.rs#L132)
- [`op_engine::tick_timers`](../crates/op_engine/src/lib.rs#L435)
- [`op_js::run_due_timers`](../crates/op_js/src/runtime.rs#L1265)
- [`op_dom::set_text_content`](../crates/op_dom/src/lib.rs#L267)
- [`op_engine::render`](../crates/op_engine/src/lib.rs#L157)
- [`op_platform_win::present_reflow`](../crates/op_platform_win/src/lib.rs#L261)

Bounded timeout/interval and queueMicrotask jobs now exist; no Promise jobs, network event tasks or background throttling.

## S14 — Repeating timers and FIFO microtask checkpoints to native paint

Status: **M4.10 bounded intervals and VM microtasks**.

```text
op_js::SetInterval
op_js::QueueMicrotask
op_js::next_timer_wait
op_browser::recv_timeout
op_js::run_due_timers
op_js::drain_microtasks
op_engine::tick_timers
op_dom::set_text_content
op_platform_win::present_reflow
```

- [`op_js::SetInterval`](../crates/op_js/src/runtime.rs#L101)
- [`op_js::QueueMicrotask`](../crates/op_js/src/runtime.rs#L103)
- [`op_js::next_timer_wait`](../crates/op_js/src/runtime.rs#L1253)
- [`op_browser::recv_timeout`](../crates/op_browser/src/main.rs#L132)
- [`op_js::run_due_timers`](../crates/op_js/src/runtime.rs#L1265)
- [`op_js::drain_microtasks`](../crates/op_js/src/runtime.rs#L671)
- [`op_engine::tick_timers`](../crates/op_engine/src/lib.rs#L435)
- [`op_dom::set_text_content`](../crates/op_dom/src/lib.rs#L267)
- [`op_platform_win::present_reflow`](../crates/op_platform_win/src/lib.rs#L261)

Intervals and bounded FIFO microtasks; no Promise resolution jobs or network tasks after initial page paint.

## S15 — Nonstandard text IO callback after first native paint

Status: **M4.11 page-scoped filtered network completion tasks**.

```text
op_js::OpFetchText
op_js::take_text_requests
op_engine::dispatch_text_requests
op_net::load_text_for_page
op_net::load_text
op_engine::tick_timers
op_js::complete_text_request
op_dom::set_text_content
op_platform_win::present_reflow
```

- [`op_js::OpFetchText`](../crates/op_js/src/runtime.rs#L104)
- [`op_js::take_text_requests`](../crates/op_js/src/runtime.rs#L613)
- [`op_engine::dispatch_text_requests`](../crates/op_engine/src/lib.rs#L363)
- [`op_net::load_text_for_page`](../crates/op_net/src/lib.rs#L169)
- [`op_net::load_text`](../crates/op_net/src/http.rs#L236)
- [`op_engine::tick_timers`](../crates/op_engine/src/lib.rs#L435)
- [`op_js::complete_text_request`](../crates/op_js/src/runtime.rs#L623)
- [`op_dom::set_text_content`](../crates/op_dom/src/lib.rs#L267)
- [`op_platform_win::present_reflow`](../crates/op_platform_win/src/lib.rs#L261)

Nonstandard callback IO only, same-origin/64 KiB/8 requests; no Promise, fetch, CORS, streams or cancellation.

## S16 — Self-hosted Promise and fetch text GET to native pixels

Status: **M4.12 bounded same-origin Promise/Response text GET**.

```text
op_js::install_dom_snapshot
op_js::OpFetchText
op_js::take_text_requests
op_engine::dispatch_text_requests
op_net::load_text_for_page
op_engine::tick_timers
op_js::complete_text_request
op_js::drain_microtasks
op_dom::set_text_content
op_platform_win::present_reflow
```

- [`op_js::install_dom_snapshot`](../crates/op_js/src/runtime.rs#L482)
- [`op_js::OpFetchText`](../crates/op_js/src/runtime.rs#L104)
- [`op_js::take_text_requests`](../crates/op_js/src/runtime.rs#L613)
- [`op_engine::dispatch_text_requests`](../crates/op_engine/src/lib.rs#L363)
- [`op_net::load_text_for_page`](../crates/op_net/src/lib.rs#L169)
- [`op_engine::tick_timers`](../crates/op_engine/src/lib.rs#L435)
- [`op_js::complete_text_request`](../crates/op_js/src/runtime.rs#L623)
- [`op_js::drain_microtasks`](../crates/op_js/src/runtime.rs#L671)
- [`op_dom::set_text_content`](../crates/op_dom/src/lib.rs#L267)
- [`op_platform_win::present_reflow`](../crates/op_platform_win/src/lib.rs#L261)

Self-hosted Promise reactions, same-origin GET, synthetic 200/OK text-only Response; HTTP failures reject, no CORS, headers, streaming or abort.

## S17 — HTTP response metadata through own JS Headers to native pixels

Status: **M4.13 GET status and headers metadata**.

```text
op_net::load_text
op_net::LoadedTextResponse
op_net::load_text_response_for_page
op_engine::dispatch_text_requests
op_engine::tick_timers
op_js::TextResponse
op_js::complete_text_response_request
op_js::HeadersGet
op_js::drain_microtasks
op_dom::set_text_content
op_platform_win::present_reflow
```

- [`op_net::load_text`](../crates/op_net/src/http.rs#L236)
- [`op_net::LoadedTextResponse`](../crates/op_net/src/lib.rs#L41)
- [`op_net::load_text_response_for_page`](../crates/op_net/src/lib.rs#L175)
- [`op_engine::dispatch_text_requests`](../crates/op_engine/src/lib.rs#L363)
- [`op_engine::tick_timers`](../crates/op_engine/src/lib.rs#L435)
- [`op_js::TextResponse`](../crates/op_js/src/runtime.rs#L204)
- [`op_js::complete_text_response_request`](../crates/op_js/src/runtime.rs#L624)
- [`op_js::HeadersGet`](../crates/op_js/src/runtime.rs#L106)
- [`op_js::drain_microtasks`](../crates/op_js/src/runtime.rs#L671)
- [`op_dom::set_text_content`](../crates/op_dom/src/lib.rs#L267)
- [`op_platform_win::present_reflow`](../crates/op_platform_win/src/lib.rs#L261)

Bounded same-origin GET text; status/reason/headers/final URL with Set-Cookie hidden. No CORS, streams or public Headers constructor.

## S18 — Request Headers and guarded pre-connect redirect flow

Status: **M4.14 bounded GET request fields and origin checks**.

```text
op_js::read_headers_init
op_js::HeadersConstructor
op_js::allowed_request_headers
op_engine::dispatch_text_requests
op_net::load_text_response_for_page_with_options
op_net::load_text_with_options
op_net::load_with_headers
op_net::resolve_link
op_engine::tick_timers
op_js::complete_text_response_request
op_dom::set_text_content
op_platform_win::present_reflow
```

- [`op_js::read_headers_init`](../crates/op_js/src/runtime.rs#L1124)
- [`op_js::HeadersConstructor`](../crates/op_js/src/runtime.rs#L105)
- [`op_js::allowed_request_headers`](../crates/op_js/src/runtime.rs#L1223)
- [`op_engine::dispatch_text_requests`](../crates/op_engine/src/lib.rs#L363)
- [`op_net::load_text_response_for_page_with_options`](../crates/op_net/src/lib.rs#L190)
- [`op_net::load_text_with_options`](../crates/op_net/src/http.rs#L240)
- [`op_net::load_with_headers`](../crates/op_net/src/http.rs#L259)
- [`op_net::resolve_link`](../crates/op_net/src/links.rs#L5)
- [`op_engine::tick_timers`](../crates/op_engine/src/lib.rs#L435)
- [`op_js::complete_text_response_request`](../crates/op_js/src/runtime.rs#L624)
- [`op_dom::set_text_content`](../crates/op_dom/src/lib.rs#L267)
- [`op_platform_win::present_reflow`](../crates/op_platform_win/src/lib.rs#L261)

Same-origin GET text only, restricted request headers and <=5 checked redirect hops; no CORS, stream, credentials or POST.

## S19 — JSON parser through Promise reactions to native repaint

Status: **M4.15 bounded JSON and array-like Promise combinators**.

```text
op_js::parse
op_js::JsonParse
op_js::json_to_value
op_js::json_from_value
op_engine::dispatch_text_requests
op_engine::tick_timers
op_js::complete_text_response_request
op_js::drain_microtasks
op_dom::set_text_content
op_platform_win::present_reflow
```

- [`op_js::parse`](../crates/op_js/src/json.rs#L17)
- [`op_js::JsonParse`](../crates/op_js/src/runtime.rs#L111)
- [`op_js::json_to_value`](../crates/op_js/src/runtime.rs#L988)
- [`op_js::json_from_value`](../crates/op_js/src/runtime.rs#L1026)
- [`op_engine::dispatch_text_requests`](../crates/op_engine/src/lib.rs#L363)
- [`op_engine::tick_timers`](../crates/op_engine/src/lib.rs#L435)
- [`op_js::complete_text_response_request`](../crates/op_js/src/runtime.rs#L624)
- [`op_js::drain_microtasks`](../crates/op_js/src/runtime.rs#L671)
- [`op_dom::set_text_content`](../crates/op_dom/src/lib.rs#L267)
- [`op_platform_win::present_reflow`](../crates/op_platform_win/src/lib.rs#L261)

Bounded JSON parse/serialize, array-like combinators; no toJSON/reviver/replacer or true iterable protocol.

## S26 — Live NodeList and DOMTokenList style mutations to native pixels

Status: **M4.22 retained JS DOM references, ordered operations and CSS recascade**.

```text
op_engine::render_html
op_js::get_property
op_js::call_builtin
op_engine::apply_dom_operations
op_dom::replace_child
op_css::compute_styles
op_paint::build_display_list
```

- [`op_engine::render_html`](../crates/op_engine/src/lib.rs#L249)
- [`op_js::get_property`](../crates/op_js/src/runtime.rs#L2146)
- [`op_js::call_builtin`](../crates/op_js/src/runtime.rs#L2478)
- [`op_engine::apply_dom_operations`](../crates/op_engine/src/scripts.rs#L206)
- [`op_dom::replace_child`](../crates/op_dom/src/lib.rs#L194)
- [`op_css::compute_styles`](../crates/op_css/src/lib.rs#L20)
- [`op_paint::build_display_list`](../crates/op_paint/src/lib.rs#L86)

Limited bounded live NodeList, DOMTokenList and CSSStyleDeclaration only, not broad WPT DOM/CSSOM compliance.

## S25 — DOM removal and attributes trigger style recascade and native paint

Status: **M4.21 actual DOM and CSS update**.

```text
op_engine::render_html
op_js::call_builtin
op_js::take_dom_operations
op_engine::apply_dom_operations
op_dom::insert_before
op_css::collect_author_styles_with_linked
op_css::compute_styles
op_paint::build_display_list
```

- [`op_engine::render_html`](../crates/op_engine/src/lib.rs#L249)
- [`op_js::call_builtin`](../crates/op_js/src/runtime.rs#L2478)
- [`op_js::take_dom_operations`](../crates/op_js/src/runtime.rs#L1521)
- [`op_engine::apply_dom_operations`](../crates/op_engine/src/scripts.rs#L206)
- [`op_dom::insert_before`](../crates/op_dom/src/lib.rs#L124)
- [`op_css::collect_author_styles_with_linked`](../crates/op_css/src/lib.rs#L25)
- [`op_css::compute_styles`](../crates/op_css/src/lib.rs#L20)
- [`op_paint::build_display_list`](../crates/op_paint/src/lib.rs#L86)

Initial bounded DOM subset, not full WPT or CSSOM.

## S24 — DOM element creation and attachment through original VM to native paint

Status: **M4.20 original VM pending DOM operations and authoritative tree**.

```text
op_engine::render_html
op_js::call_builtin
op_js::take_dom_operations
op_engine::apply_dom_operations
op_dom::append_child
op_css::compute_styles
op_layout::layout_document_with_backgrounds_and_resources
op_paint::build_display_list
```

- [`op_engine::render_html`](../crates/op_engine/src/lib.rs#L249)
- [`op_js::call_builtin`](../crates/op_js/src/runtime.rs#L2478)
- [`op_js::take_dom_operations`](../crates/op_js/src/runtime.rs#L1521)
- [`op_engine::apply_dom_operations`](../crates/op_engine/src/scripts.rs#L206)
- [`op_dom::append_child`](../crates/op_dom/src/lib.rs#L123)
- [`op_css::compute_styles`](../crates/op_css/src/lib.rs#L20)
- [`op_layout::layout_document_with_backgrounds_and_resources`](../crates/op_layout/src/lib.rs#L304)
- [`op_paint::build_display_list`](../crates/op_paint/src/lib.rs#L86)

Initial bounded createElement/appendChild/id/textContent only, with missing general DOM methods.

## S23 — Native standard String and Array methods to DOM text repaint

Status: **M4.19 original VM builtins / SyntaxError**.

```text
op_engine::render_html
op_js::install_standard_primitives
op_js::call_builtin
op_js::get_property
op_dom::set_text_content
op_paint::build_display_list
```

- [`op_engine::render_html`](../crates/op_engine/src/lib.rs#L249)
- [`op_js::install_standard_primitives`](../crates/op_js/src/runtime.rs#L445)
- [`op_js::call_builtin`](../crates/op_js/src/runtime.rs#L2478)
- [`op_js::get_property`](../crates/op_js/src/runtime.rs#L2146)
- [`op_dom::set_text_content`](../crates/op_dom/src/lib.rs#L267)
- [`op_paint::build_display_list`](../crates/op_paint/src/lib.rs#L86)

Existing text DOM only; no insertion, lone UTF-16 surrogates or generic array-like methods.

## S22 — Page script typeof and ternary through native DOM repaint

Status: **M4.18 original JS syntax and standard static helpers**.

```text
op_js::tokenize
op_js::conditional
op_js::compile_program
op_engine::render_html
op_js::typeof_value
op_js::call_builtin
op_dom::set_text_content
op_paint::build_display_list
```

- [`op_js::tokenize`](../crates/op_js/src/lexer.rs#L76)
- [`op_js::conditional`](../crates/op_js/src/parser.rs#L725)
- [`op_js::compile_program`](../crates/op_js/src/bytecode.rs#L85)
- [`op_engine::render_html`](../crates/op_engine/src/lib.rs#L249)
- [`op_js::typeof_value`](../crates/op_js/src/runtime.rs#L878)
- [`op_js::call_builtin`](../crates/op_js/src/runtime.rs#L2478)
- [`op_dom::set_text_content`](../crates/op_dom/src/lib.rs#L267)
- [`op_paint::build_display_list`](../crates/op_paint/src/lib.rs#L86)

No complete modern ECMAScript, DOM or test harness support; v2 65/179 attempted with 110 explicit SKIP.

## S21 — Browser page boxed primitive and ToPrimitive to native paint

Status: **M4.17 implemented; Test262 Runtime v1 82/91**.

```text
op_engine::render_html
op_js::install_standard_primitives
op_js::box_primitive
op_js::construct_value
op_js::coerce_to_primitive
op_js::binary_with_coercion
op_js::apply_binary
op_paint::build_display_list
op_js::evaluate
```

- [`op_engine::render_html`](../crates/op_engine/src/lib.rs#L249)
- [`op_js::install_standard_primitives`](../crates/op_js/src/runtime.rs#L445)
- [`op_js::box_primitive`](../crates/op_js/src/runtime.rs#L826)
- [`op_js::construct_value`](../crates/op_js/src/runtime.rs#L2269)
- [`op_js::coerce_to_primitive`](../crates/op_js/src/runtime.rs#L845)
- [`op_js::binary_with_coercion`](../crates/op_js/src/runtime.rs#L891)
- [`op_js::apply_binary`](../crates/op_js/src/runtime.rs#L956)
- [`op_paint::build_display_list`](../crates/op_paint/src/lib.rs#L86)
- [`op_js::evaluate`](../crates/op_js/src/bin/test262_runtime_probe.rs#L170)

91-case legacy arithmetic/equality suite; no strict mode, eval, Date, Symbol, or modern ES full semantics.

## S6 — WPT image comparison and reporting

Status: **Strict and opt-in fuzzy reports**.

```text
op_browser::different_pixels
op_browser::within_wpt_fuzzy
op_platform_win::render_display_list_to_bgra
```

- [`op_browser::different_pixels`](../crates/op_browser/src/bin/wpt_probe.rs#L200)
- [`op_browser::within_wpt_fuzzy`](../crates/op_browser/src/bin/wpt_probe.rs#L133)
- [`op_platform_win::render_display_list_to_bgra`](../crates/op_platform_win/src/lib.rs#L61)

Strict Static: 197/200; metadata-aware Static: 198/200; two Rec.2020 fixtures remain known mismatches.

See [CODE_SLICES.md](CODE_SLICES.md) for full engineering notes.
