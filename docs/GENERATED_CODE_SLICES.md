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

- [`op_net::load_document`](../crates/op_net/src/lib.rs#L156)
- [`op_html::Tokenizer`](../crates/op_html/src/lib.rs#L56)
- [`op_engine::render_html`](../crates/op_engine/src/lib.rs#L195)
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

- [`op_engine::navigate`](../crates/op_engine/src/lib.rs#L342)
- [`op_engine::reflow`](../crates/op_engine/src/lib.rs#L299)
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

- [`op_js::parse_script`](../crates/op_js/src/lib.rs#L18)
- [`op_js::JsRuntime`](../crates/op_js/src/lib.rs#L20)
- [`op_engine::Engine`](../crates/op_engine/src/lib.rs#L131)

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
- [`op_engine::parse_and_execute`](../crates/op_engine/src/scripts.rs#L141)
- [`op_js::install_dom_snapshot`](../crates/op_js/src/runtime.rs#L284)
- [`op_js::DomGetElementById`](../crates/op_js/src/runtime.rs#L61)
- [`op_dom::set_text_content`](../crates/op_dom/src/lib.rs#L197)
- [`op_css::compute_styles`](../crates/op_css/src/lib.rs#L20)
- [`op_engine::render`](../crates/op_engine/src/lib.rs#L112)
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

- [`op_engine::parse_and_execute`](../crates/op_engine/src/scripts.rs#L141)
- [`op_net::resolve_script_source`](../crates/op_net/src/scripts.rs#L14)
- [`op_net::load_script_for_page`](../crates/op_net/src/lib.rs#L145)
- [`op_net::load_script`](../crates/op_net/src/http.rs#L196)
- [`op_js::eval_script`](../crates/op_js/src/runtime.rs#L761)
- [`op_dom::set_text_content`](../crates/op_dom/src/lib.rs#L197)
- [`op_engine::prepare_source`](../crates/op_engine/src/lib.rs#L222)

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
- [`op_engine::click_at`](../crates/op_engine/src/lib.rs#L251)
- [`op_layout::click_regions`](../crates/op_layout/src/flow.rs#L71)
- [`op_engine::dispatch_click`](../crates/op_engine/src/scripts.rs#L573)
- [`op_js::dispatch_dom_click_path`](../crates/op_js/src/runtime.rs#L549)
- [`op_dom::set_text_content`](../crates/op_dom/src/lib.rs#L197)
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
- [`op_engine::parse_and_execute`](../crates/op_engine/src/scripts.rs#L141)
- [`op_engine::ParserScriptRunner`](../crates/op_engine/src/scripts.rs#L146)
- [`op_js::refresh_dom_snapshot`](../crates/op_js/src/runtime.rs#L536)
- [`op_js::eval_script`](../crates/op_js/src/runtime.rs#L761)
- [`op_dom::set_text_content`](../crates/op_dom/src/lib.rs#L197)
- [`op_engine::prepare_source`](../crates/op_engine/src/lib.rs#L222)

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
- [`op_engine::parse_and_execute`](../crates/op_engine/src/scripts.rs#L141)
- [`op_engine::prepare_fetch`](../crates/op_engine/src/scripts.rs#L340)
- [`op_net::load_script_for_page`](../crates/op_net/src/lib.rs#L145)
- [`op_engine::drain_ready`](../crates/op_engine/src/scripts.rs#L165)
- [`op_engine::evaluate_loaded`](../crates/op_engine/src/scripts.rs#L201)
- [`op_js::eval_script`](../crates/op_js/src/runtime.rs#L761)
- [`op_dom::set_text_content`](../crates/op_dom/src/lib.rs#L197)

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
- [`op_engine::advance_state`](../crates/op_engine/src/scripts.rs#L186)
- [`op_js::set_document_ready_state`](../crates/op_js/src/runtime.rs#L474)
- [`op_engine::dispatch_lifecycle`](../crates/op_engine/src/scripts.rs#L206)
- [`op_js::dispatch_lifecycle_event`](../crates/op_js/src/runtime.rs#L487)
- [`op_dom::set_text_content`](../crates/op_dom/src/lib.rs#L197)
- [`op_engine::prepare_source`](../crates/op_engine/src/lib.rs#L222)

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

- [`op_js::SetTimeout`](../crates/op_js/src/runtime.rs#L68)
- [`op_js::next_timer_wait`](../crates/op_js/src/runtime.rs#L380)
- [`op_browser::recv_timeout`](../crates/op_browser/src/main.rs#L132)
- [`op_engine::tick_timers`](../crates/op_engine/src/lib.rs#L300)
- [`op_js::run_due_timers`](../crates/op_js/src/runtime.rs#L392)
- [`op_dom::set_text_content`](../crates/op_dom/src/lib.rs#L197)
- [`op_engine::render`](../crates/op_engine/src/lib.rs#L112)
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

- [`op_js::SetInterval`](../crates/op_js/src/runtime.rs#L70)
- [`op_js::QueueMicrotask`](../crates/op_js/src/runtime.rs#L72)
- [`op_js::next_timer_wait`](../crates/op_js/src/runtime.rs#L380)
- [`op_browser::recv_timeout`](../crates/op_browser/src/main.rs#L132)
- [`op_js::run_due_timers`](../crates/op_js/src/runtime.rs#L392)
- [`op_js::drain_microtasks`](../crates/op_js/src/runtime.rs#L394)
- [`op_engine::tick_timers`](../crates/op_engine/src/lib.rs#L300)
- [`op_dom::set_text_content`](../crates/op_dom/src/lib.rs#L197)
- [`op_platform_win::present_reflow`](../crates/op_platform_win/src/lib.rs#L261)

Intervals and bounded FIFO microtasks; no Promise resolution jobs or network tasks after initial page paint.

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
