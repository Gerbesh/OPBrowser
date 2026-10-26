# Generated Code Slices

> Generated from `tools/code_slices.json` and validated against live Rust files.
> This is a curated vertical feature map, **not** an AST/call/data-flow slicer.

## S122 — Native textarea visual-row wheel, scrollbar track and click priority

Status: **First-party renderer DOM wheel/track integration, focused/unfocused caret, scroll limits and Windows startup tested**.

```text
op_platform_win::WM_MOUSEWHEEL
op_browser::NavigationEvent::Scroll
op_engine::scroll_textarea_at
op_engine::textarea_wheel_row
op_engine::TextareaScrollbar
op_engine::click_textarea_scrollbar_at
op_engine::textarea_scrollbar_track_click_jumps_without_dispatching_dom_click
```

- [`op_platform_win::WM_MOUSEWHEEL`](../crates/op_platform_win/src/lib.rs#L696)
- [`op_browser::NavigationEvent::Scroll`](../crates/op_browser/src/main.rs#L263)
- [`op_engine::scroll_textarea_at`](../crates/op_engine/src/lib.rs#L656)
- [`op_engine::textarea_wheel_row`](../crates/op_engine/src/forms.rs#L315)
- [`op_engine::TextareaScrollbar`](../crates/op_engine/src/forms.rs#L339)
- [`op_engine::click_textarea_scrollbar_at`](../crates/op_engine/src/lib.rs#L616)
- [`op_engine::textarea_scrollbar_track_click_jumps_without_dispatching_dom_click`](../crates/op_engine/src/lib.rs#L3157)

No captured scrollbar thumb dragging/touch, standards-grade CSS control clipping, IME/BiDi or grapheme-aware editing.

## S121 — Shared measured textarea visual-line wrapping from DOM to native glyphs, clicks and selections

Status: **Original op_layout/op_engine Unicode visual rows, wrapped caret/selection/click integration and Win32 smoke tested**.

```text
op_layout::fn textarea_visual_lines
op_layout::fn textarea_atomic
op_engine::wrapped_textarea_caret_at_point
op_engine::wrapped_textarea_selection_fragments
op_engine::textarea_soft_wrap_scroll_click_and_unicode_insert_keep_full_dom_value
```

- [`op_layout::fn textarea_visual_lines`](../crates/op_layout/src/lib.rs#L682)
- [`op_layout::fn textarea_atomic`](../crates/op_layout/src/flow.rs#L1667)
- [`op_engine::wrapped_textarea_caret_at_point`](../crates/op_engine/src/forms.rs#L394)
- [`op_engine::wrapped_textarea_selection_fragments`](../crates/op_engine/src/forms.rs#L416)
- [`op_engine::textarea_soft_wrap_scroll_click_and_unicode_insert_keep_full_dom_value`](../crates/op_engine/src/lib.rs#L3314)

Bounded scalar/space-first wrap, not full Unicode bidi/grapheme CSS, IME, textarea wheel scrolling or accurate CSS padding/clipping.

## S120 — CSS-sized atomic textarea, logical row viewport, click and Unicode DOM editing

Status: **First-party layout and native engine regression tests cover fixed CSS hit box, scrolling explicit rows, mouse Unicode editing; workspace and Win32 smoke verified**.

```text
op_html::textarea
op_layout::fn textarea_atomic
op_layout::InlineAtomic
op_engine::reveal_textarea_row
op_engine::text_caret_at
op_engine::textarea_atomic_viewport_follows_caret_and_click_maps_to_scrolled_row
```

- [`op_html::textarea`](../crates/op_html/src/tree_builder.rs#L2147)
- [`op_layout::fn textarea_atomic`](../crates/op_layout/src/flow.rs#L1667)
- [`op_layout::InlineAtomic`](../crates/op_layout/src/inline.rs#L338)
- [`op_engine::reveal_textarea_row`](../crates/op_engine/src/forms.rs#L290)
- [`op_engine::text_caret_at`](../crates/op_engine/src/lib.rs#L433)
- [`op_engine::textarea_atomic_viewport_follows_caret_and_click_maps_to_scrolled_row`](../crates/op_engine/src/lib.rs#L3414)

No shared visual-line wrapping, wheel scrollbars, IME, bidi/grapheme movement or full CSS padding fidelity.

## S116 — HTML input size and textarea cols/rows into measured first-party form hit geometry

Status: **HTML dimension hints feed source-owned atomic controls; original glyph rows and native hit/Unicode/clipboard regressions pass**.

```text
op_html::input
op_layout::fn textarea_atomic
op_layout::fn text_input_atomic
op_layout::form_size_and_cols_hints_change_native_inline_control_width
op_engine::PreparedDocument
```

- [`op_html::input`](../crates/op_html/src/tree_builder.rs#L53)
- [`op_layout::fn textarea_atomic`](../crates/op_layout/src/flow.rs#L1667)
- [`op_layout::fn text_input_atomic`](../crates/op_layout/src/flow.rs#L1811)
- [`op_layout::form_size_and_cols_hints_change_native_inline_control_width`](../crates/op_layout/src/lib.rs#L2062)
- [`op_engine::PreparedDocument`](../crates/op_engine/src/lib.rs#L282)

Atomic sizing works, but full soft-wrap, Unicode shaping, IME and textarea scrollbar behavior are incomplete.

## S115 — Cancelable Ctrl+C/X/V from original DOM to Windows Unicode clipboard and pixels

Status: **Original JS keydown, Unicode scalar selection, Win32 clipboard encoding and DOM edit regressions pass**.

```text
op_browser::keyboard_default_prevented
op_platform_win::write_clipboard_text
op_platform_win::read_clipboard_text
op_engine::selected_clipboard_text
op_engine::DeleteSelection
op_js::sync_native_control_selection
```

- [`op_browser::keyboard_default_prevented`](../crates/op_browser/src/main.rs#L320)
- [`op_platform_win::write_clipboard_text`](../crates/op_platform_win/src/lib.rs#L189)
- [`op_platform_win::read_clipboard_text`](../crates/op_platform_win/src/lib.rs#L226)
- [`op_engine::selected_clipboard_text`](../crates/op_engine/src/lib.rs#L1936)
- [`op_engine::DeleteSelection`](../crates/op_engine/src/forms.rs#L32)
- [`op_js::sync_native_control_selection`](../crates/op_js/src/runtime.rs#L3133)

No ClipboardEvent/custom MIME support, IME, undo or physical OS clipboard roundtrip automated test.

## S114 — Captured Win32 mouse drag to original JS form selection and pixels

Status: **Native capture, browser queue, UTF-16 control editing and rendering tested**.

```text
op_platform_win::NavigationEvent::PointerMove
op_browser::NavigationEvent::PointerMove
op_engine::drag_text_selection_at
op_js::sync_native_control_selection
op_engine::selection_fragments
```

- [`op_platform_win::NavigationEvent::PointerMove`](../crates/op_platform_win/src/lib.rs#L650)
- [`op_browser::NavigationEvent::PointerMove`](../crates/op_browser/src/main.rs#L98)
- [`op_engine::drag_text_selection_at`](../crates/op_engine/src/lib.rs#L2299)
- [`op_js::sync_native_control_selection`](../crates/op_js/src/runtime.rs#L3133)
- [`op_engine::selection_fragments`](../crates/op_engine/src/forms.rs#L525)

Provisional glyph hit geometry, no soft wrapping, bidi/graphemes, IME or clipboard.

## S94 — Original nested-await expressions through receiver-safe JS VM into pixels

Status: **Bounded eager await expressions, GC and actual DOM-to-native-paint integration tested**.

```text
op_js::Await
op_js::split_statement
op_js::CallWithReceiver
op_js::Instruction::Call
op_engine::expression_await_updates_real_native_paint
```

- [`op_js::Await`](../crates/op_js/src/parser.rs#L153)
- [`op_js::split_statement`](../crates/op_js/src/await_lowering.rs#L497)
- [`op_js::CallWithReceiver`](../crates/op_js/src/bytecode.rs#L883)
- [`op_js::Instruction::Call`](../crates/op_js/src/runtime.rs#L6191)
- [`op_engine::expression_await_updates_real_native_paint`](../crates/op_engine/tests/scripts.rs#L55)

Conditional branches, short-circuit RHS, loops and try/finally await still unsupported; public Reddit blank and YouTube textless.

## S86 — Compound and comma JS expressions into browser DOM

Status: **Original VM, full expression and native DOM integration tested**.

```text
op_js::PlusEqual
op_js::CompoundAssignment
op_js::Sequence
op_js::Dup2
op_js::Dup2
op_engine::parse_and_execute
```

- [`op_js::PlusEqual`](../crates/op_js/src/lexer.rs#L53)
- [`op_js::CompoundAssignment`](../crates/op_js/src/parser.rs#L210)
- [`op_js::Sequence`](../crates/op_js/src/parser.rs#L209)
- [`op_js::Dup2`](../crates/op_js/src/bytecode.rs#L82)
- [`op_js::Dup2`](../crates/op_js/src/runtime.rs#L6409)
- [`op_engine::parse_and_execute`](../crates/op_engine/src/scripts.rs#L413)

Only += and -= compound operations; comma operator only in explicitly supported expression contexts; real YouTube still displays no text.

## S85 — Real regex literals through original VM into native DOM

Status: **Bounded regexp subset and parser-DOM integration tested**.

```text
op_js::RegExp
op_js::RegExp
op_js::CreateRegExp
op_js::build_js_regex
op_engine::parse_and_execute
```

- [`op_js::RegExp`](../crates/op_js/src/lexer.rs#L14)
- [`op_js::RegExp`](../crates/op_js/src/parser.rs#L136)
- [`op_js::CreateRegExp`](../crates/op_js/src/bytecode.rs#L65)
- [`op_js::build_js_regex`](../crates/op_js/src/runtime.rs#L1806)
- [`op_engine::parse_and_execute`](../crates/op_engine/src/scripts.rs#L413)

A finite automata regular-expression backend is not a full JavaScript RegExp implementation; unsupported patterns and flags reject.

## S84 — Original for-in parsing through real YouTube configuration script

Status: **Own-key VM subset and HTML-to-DOM integration tested**.

```text
op_js::ForIn
op_js::ToForInKeys
op_js::ToForInKeys
op_engine::bounded_js_error_context
op_engine::parse_and_execute
```

- [`op_js::ForIn`](../crates/op_js/src/parser.rs#L40)
- [`op_js::ToForInKeys`](../crates/op_js/src/bytecode.rs#L70)
- [`op_js::ToForInKeys`](../crates/op_js/src/runtime.rs#L6068)
- [`op_engine::bounded_js_error_context`](../crates/op_engine/src/scripts.rs#L536)
- [`op_engine::parse_and_execute`](../crates/op_engine/src/scripts.rs#L413)

Own keys only, sorted rather than insertion ordered. Real YouTube for initializer passes parser, but runtime failure and regex/paint gaps remain.

## S83 — Bounded synchronous for-of into live DOM mutations

Status: **Original VM and DOM integration tested; iterable subset**.

```text
op_js::for_statement
op_js::for_of_statement
op_js::ToForOfValues
op_engine::parse_and_execute
```

- [`op_js::for_statement`](../crates/op_js/src/parser.rs#L349)
- [`op_js::for_of_statement`](../crates/op_js/src/bytecode.rs#L173)
- [`op_js::ToForOfValues`](../crates/op_js/src/runtime.rs#L6016)
- [`op_engine::parse_and_execute`](../crates/op_engine/src/scripts.rs#L413)

Snapshots arrays, strings and selected DOM lists; no generic iterator protocol or live array mutation semantics.

## S82 — Lazy nullish coalescing into page text

Status: **Original JS semantics and parser-DOM integration tested**.

```text
op_js::QuestionQuestion
op_js::nullish
op_js::JumpIfNullish
op_js::JumpIfNullish
op_engine::parse_and_execute
```

- [`op_js::QuestionQuestion`](../crates/op_js/src/lexer.rs#L95)
- [`op_js::nullish`](../crates/op_js/src/parser.rs#L1296)
- [`op_js::JumpIfNullish`](../crates/op_js/src/bytecode.rs#L90)
- [`op_js::JumpIfNullish`](../crates/op_js/src/runtime.rs#L6455)
- [`op_engine::parse_and_execute`](../crates/op_engine/src/scripts.rs#L413)

Five live public sites remain visually incomplete; grammar and expression compatibility are partial.

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
- [`op_engine::render_html`](../crates/op_engine/src/lib.rs#L1732)
- [`op_layout::layout`](../crates/op_layout/src/flow.rs#L15)
- [`op_paint::build_display_list`](../crates/op_paint/src/lib.rs#L100)
- [`op_platform_win::paint_command`](../crates/op_platform_win/src/lib.rs#L1151)

This path lacks complete CSS, forms, DOM scripting and a GPU backend.

## S2 — Navigation and retained layout

Status: **Initial implementation**.

```text
op_engine::navigate
op_engine::reflow
op_platform_win::paint_window
```

- [`op_engine::navigate`](../crates/op_engine/src/lib.rs#L2871)
- [`op_engine::reflow`](../crates/op_engine/src/lib.rs#L2795)
- [`op_platform_win::paint_window`](../crates/op_platform_win/src/lib.rs#L1064)

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
- [`op_css::compute_styles`](../crates/op_css/src/lib.rs#L21)
- [`op_layout::layout`](../crates/op_layout/src/flow.rs#L15)
- [`op_paint::build_display_list`](../crates/op_paint/src/lib.rs#L100)

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

- [`op_js::parse_script`](../crates/op_js/src/lib.rs#L20)
- [`op_js::JsRuntime`](../crates/op_js/src/lib.rs#L24)
- [`op_engine::Engine`](../crates/op_engine/src/lib.rs#L1586)

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
- [`op_engine::parse_and_execute`](../crates/op_engine/src/scripts.rs#L413)
- [`op_js::install_dom_snapshot`](../crates/op_js/src/runtime.rs#L1143)
- [`op_js::DomGetElementById`](../crates/op_js/src/runtime.rs#L151)
- [`op_dom::set_text_content`](../crates/op_dom/src/lib.rs#L354)
- [`op_css::compute_styles`](../crates/op_css/src/lib.rs#L21)
- [`op_engine::render`](../crates/op_engine/src/lib.rs#L1040)
- [`op_paint::build_display_list`](../crates/op_paint/src/lib.rs#L100)

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

- [`op_engine::parse_and_execute`](../crates/op_engine/src/scripts.rs#L413)
- [`op_net::resolve_script_source`](../crates/op_net/src/scripts.rs#L14)
- [`op_net::load_script_for_page`](../crates/op_net/src/lib.rs#L157)
- [`op_net::load_script`](../crates/op_net/src/http.rs#L197)
- [`op_js::eval_script`](../crates/op_js/src/runtime.rs#L1057)
- [`op_dom::set_text_content`](../crates/op_dom/src/lib.rs#L354)
- [`op_engine::prepare_source`](../crates/op_engine/src/lib.rs#L1761)

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

- [`op_platform_win::WM_LBUTTONUP`](../crates/op_platform_win/src/lib.rs#L530)
- [`op_browser::NavigationEvent::Click`](../crates/op_browser/src/main.rs#L138)
- [`op_engine::click_at`](../crates/op_engine/src/lib.rs#L1807)
- [`op_layout::click_regions`](../crates/op_layout/src/flow.rs#L76)
- [`op_engine::dispatch_click`](../crates/op_engine/src/scripts.rs#L1049)
- [`op_js::dispatch_dom_click_path`](../crates/op_js/src/runtime.rs#L3636)
- [`op_dom::set_text_content`](../crates/op_dom/src/lib.rs#L354)
- [`op_engine::compute_styles`](../crates/op_engine/src/lib.rs#L4)

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
- [`op_engine::parse_and_execute`](../crates/op_engine/src/scripts.rs#L413)
- [`op_engine::ParserScriptRunner`](../crates/op_engine/src/scripts.rs#L418)
- [`op_js::refresh_dom_snapshot`](../crates/op_js/src/runtime.rs#L3076)
- [`op_js::eval_script`](../crates/op_js/src/runtime.rs#L1057)
- [`op_dom::set_text_content`](../crates/op_dom/src/lib.rs#L354)
- [`op_engine::prepare_source`](../crates/op_engine/src/lib.rs#L1761)

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
- [`op_engine::parse_and_execute`](../crates/op_engine/src/scripts.rs#L413)
- [`op_engine::prepare_fetch`](../crates/op_engine/src/scripts.rs#L686)
- [`op_net::load_script_for_page`](../crates/op_net/src/lib.rs#L157)
- [`op_engine::drain_ready`](../crates/op_engine/src/scripts.rs#L438)
- [`op_engine::evaluate_loaded`](../crates/op_engine/src/scripts.rs#L474)
- [`op_js::eval_script`](../crates/op_js/src/runtime.rs#L1057)
- [`op_dom::set_text_content`](../crates/op_dom/src/lib.rs#L354)

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
- [`op_engine::advance_state`](../crates/op_engine/src/scripts.rs#L459)
- [`op_js::set_document_ready_state`](../crates/op_js/src/runtime.rs#L2538)
- [`op_engine::dispatch_lifecycle`](../crates/op_engine/src/scripts.rs#L479)
- [`op_js::dispatch_lifecycle_event`](../crates/op_js/src/runtime.rs#L2932)
- [`op_dom::set_text_content`](../crates/op_dom/src/lib.rs#L354)
- [`op_engine::prepare_source`](../crates/op_engine/src/lib.rs#L1761)

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

- [`op_js::SetTimeout`](../crates/op_js/src/runtime.rs#L209)
- [`op_js::next_timer_wait`](../crates/op_js/src/runtime.rs#L2421)
- [`op_browser::recv_timeout`](../crates/op_browser/src/main.rs#L206)
- [`op_engine::tick_timers`](../crates/op_engine/src/lib.rs#L2796)
- [`op_js::run_due_timers`](../crates/op_js/src/runtime.rs#L2438)
- [`op_dom::set_text_content`](../crates/op_dom/src/lib.rs#L354)
- [`op_engine::render`](../crates/op_engine/src/lib.rs#L1040)
- [`op_platform_win::present_reflow`](../crates/op_platform_win/src/lib.rs#L477)

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

- [`op_js::SetInterval`](../crates/op_js/src/runtime.rs#L211)
- [`op_js::QueueMicrotask`](../crates/op_js/src/runtime.rs#L213)
- [`op_js::next_timer_wait`](../crates/op_js/src/runtime.rs#L2421)
- [`op_browser::recv_timeout`](../crates/op_browser/src/main.rs#L206)
- [`op_js::run_due_timers`](../crates/op_js/src/runtime.rs#L2438)
- [`op_js::drain_microtasks`](../crates/op_js/src/runtime.rs#L1477)
- [`op_engine::tick_timers`](../crates/op_engine/src/lib.rs#L2796)
- [`op_dom::set_text_content`](../crates/op_dom/src/lib.rs#L354)
- [`op_platform_win::present_reflow`](../crates/op_platform_win/src/lib.rs#L477)

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

- [`op_js::OpFetchText`](../crates/op_js/src/runtime.rs#L214)
- [`op_js::take_text_requests`](../crates/op_js/src/runtime.rs#L1418)
- [`op_engine::dispatch_text_requests`](../crates/op_engine/src/lib.rs#L2724)
- [`op_net::load_text_for_page`](../crates/op_net/src/lib.rs#L169)
- [`op_net::load_text`](../crates/op_net/src/http.rs#L236)
- [`op_engine::tick_timers`](../crates/op_engine/src/lib.rs#L2796)
- [`op_js::complete_text_request`](../crates/op_js/src/runtime.rs#L1428)
- [`op_dom::set_text_content`](../crates/op_dom/src/lib.rs#L354)
- [`op_platform_win::present_reflow`](../crates/op_platform_win/src/lib.rs#L477)

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

- [`op_js::install_dom_snapshot`](../crates/op_js/src/runtime.rs#L1143)
- [`op_js::OpFetchText`](../crates/op_js/src/runtime.rs#L214)
- [`op_js::take_text_requests`](../crates/op_js/src/runtime.rs#L1418)
- [`op_engine::dispatch_text_requests`](../crates/op_engine/src/lib.rs#L2724)
- [`op_net::load_text_for_page`](../crates/op_net/src/lib.rs#L169)
- [`op_engine::tick_timers`](../crates/op_engine/src/lib.rs#L2796)
- [`op_js::complete_text_request`](../crates/op_js/src/runtime.rs#L1428)
- [`op_js::drain_microtasks`](../crates/op_js/src/runtime.rs#L1477)
- [`op_dom::set_text_content`](../crates/op_dom/src/lib.rs#L354)
- [`op_platform_win::present_reflow`](../crates/op_platform_win/src/lib.rs#L477)

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
- [`op_engine::dispatch_text_requests`](../crates/op_engine/src/lib.rs#L2724)
- [`op_engine::tick_timers`](../crates/op_engine/src/lib.rs#L2796)
- [`op_js::TextResponse`](../crates/op_js/src/runtime.rs#L362)
- [`op_js::complete_text_response_request`](../crates/op_js/src/runtime.rs#L1429)
- [`op_js::HeadersGet`](../crates/op_js/src/runtime.rs#L216)
- [`op_js::drain_microtasks`](../crates/op_js/src/runtime.rs#L1477)
- [`op_dom::set_text_content`](../crates/op_dom/src/lib.rs#L354)
- [`op_platform_win::present_reflow`](../crates/op_platform_win/src/lib.rs#L477)

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

- [`op_js::read_headers_init`](../crates/op_js/src/runtime.rs#L2292)
- [`op_js::HeadersConstructor`](../crates/op_js/src/runtime.rs#L215)
- [`op_js::allowed_request_headers`](../crates/op_js/src/runtime.rs#L2391)
- [`op_engine::dispatch_text_requests`](../crates/op_engine/src/lib.rs#L2724)
- [`op_net::load_text_response_for_page_with_options`](../crates/op_net/src/lib.rs#L190)
- [`op_net::load_text_with_options`](../crates/op_net/src/http.rs#L240)
- [`op_net::load_with_headers`](../crates/op_net/src/http.rs#L259)
- [`op_net::resolve_link`](../crates/op_net/src/links.rs#L5)
- [`op_engine::tick_timers`](../crates/op_engine/src/lib.rs#L2796)
- [`op_js::complete_text_response_request`](../crates/op_js/src/runtime.rs#L1429)
- [`op_dom::set_text_content`](../crates/op_dom/src/lib.rs#L354)
- [`op_platform_win::present_reflow`](../crates/op_platform_win/src/lib.rs#L477)

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
- [`op_js::JsonParse`](../crates/op_js/src/runtime.rs#L221)
- [`op_js::json_to_value`](../crates/op_js/src/runtime.rs#L2144)
- [`op_js::json_from_value`](../crates/op_js/src/runtime.rs#L2182)
- [`op_engine::dispatch_text_requests`](../crates/op_engine/src/lib.rs#L2724)
- [`op_engine::tick_timers`](../crates/op_engine/src/lib.rs#L2796)
- [`op_js::complete_text_response_request`](../crates/op_js/src/runtime.rs#L1429)
- [`op_js::drain_microtasks`](../crates/op_js/src/runtime.rs#L1477)
- [`op_dom::set_text_content`](../crates/op_dom/src/lib.rs#L354)
- [`op_platform_win::present_reflow`](../crates/op_platform_win/src/lib.rs#L477)

Bounded JSON parse/serialize, array-like combinators; no toJSON/reviver/replacer or true iterable protocol.

## S55 — Original flex basis, weighted shrinking and shorthand to native pixels

Status: **R2c row-only flex-shrink, flex-basis, shorthand cascade and min-width redistribution**.

```text
op_css::parse_flex_shorthand
op_css::parse_flex_basis
op_layout::flex_row_atomic
op_layout::reflow_flex_item_to_outer_width
op_platform_win::render_display_list_to_bgra
```

- [`op_css::parse_flex_shorthand`](../crates/op_css/src/computed.rs#L1830)
- [`op_css::parse_flex_basis`](../crates/op_css/src/computed.rs#L1861)
- [`op_layout::flex_row_atomic`](../crates/op_layout/src/flow.rs#L1252)
- [`op_layout::reflow_flex_item_to_outer_width`](../crates/op_layout/src/flow.rs#L2223)
- [`op_platform_win::render_display_list_to_bgra`](../crates/op_platform_win/src/lib.rs#L135)

Horizontal row flex only, no full automatic min-content, column direction, cross-axis alignment/order, Grid or complete Flexbox conformance.

## S54 — Original CSS flex-grow and justify-content to GDI pixel coordinates

Status: **R2b positive weighted flex growth and six main-axis justification values**.

```text
op_css::parse_flex_grow
op_css::parse_justify_content
op_layout::flex_row_atomic
op_paint::build_display_list
op_platform_win::render_display_list_to_bgra
```

- [`op_css::parse_flex_grow`](../crates/op_css/src/computed.rs#L1848)
- [`op_css::parse_justify_content`](../crates/op_css/src/computed.rs#L1821)
- [`op_layout::flex_row_atomic`](../crates/op_layout/src/flow.rs#L1252)
- [`op_paint::build_display_list`](../crates/op_paint/src/lib.rs#L100)
- [`op_platform_win::render_display_list_to_bgra`](../crates/op_platform_win/src/lib.rs#L135)

Positive growth and row-direction alignment only; no shrink/basis/shorthand/vertical alignment/direction/order/full min-content.

## S53 — Computed flex-wrap and gap to original multi-row GDI pixels

Status: **R2a bounded row-direction flex-wrap and non-negative CSS gap shorthand**.

```text
op_css::parse_flex_wrap
op_css::parse_flex_gap
op_layout::flex_row_atomic
op_paint::build_display_list
op_platform_win::render_display_list_to_bgra
```

- [`op_css::parse_flex_wrap`](../crates/op_css/src/computed.rs#L1805)
- [`op_css::parse_flex_gap`](../crates/op_css/src/computed.rs#L1813)
- [`op_layout::flex_row_atomic`](../crates/op_layout/src/flow.rs#L1252)
- [`op_paint::build_display_list`](../crates/op_paint/src/lib.rs#L100)
- [`op_platform_win::render_display_list_to_bgra`](../crates/op_platform_win/src/lib.rs#L135)

Intrinsic-width row flex only; no grow/shrink, direction, item order, alignment, percent gap, Grid or general Flexbox conformance.

## S52 — Captured Win32 scrollbar thumb movement to nested original scrollTop

Status: **R1.2c capture, busy-worker pointer coalescing, original layout mapping and clamped scrollTop**.

```text
op_platform_win::navigation_event
op_browser::queue_thumb_event
op_engine::start_scrollbar_drag_at
op_engine::move_scrollbar_drag_to
op_layout::scroll_top_for_thumb_drag
```

- [`op_platform_win::navigation_event`](../crates/op_platform_win/src/lib.rs#L572)
- [`op_browser::queue_thumb_event`](../crates/op_browser/src/main.rs#L86)
- [`op_engine::start_scrollbar_drag_at`](../crates/op_engine/src/lib.rs#L1808)
- [`op_engine::move_scrollbar_drag_to`](../crates/op_engine/src/lib.rs#L1844)
- [`op_layout::scroll_top_for_thumb_drag`](../crates/op_layout/src/lib.rs#L386)

Vertical overlay thumb dragging only, no horizontal axes, reserved gutter or complete complex clipping.

## S51 — Original vertical scrollbar geometry through GDI pixels and track hit testing

Status: **R1.2b native vertical overlay scrollbar pixels and track click routing**.

```text
op_layout::scrollbar_geometry
op_layout::scroll_top_at_track_point
op_paint::build_display_list
op_platform_win::paint_command
op_platform_win::scrollbar_at_document_point
op_engine::click_at
```

- [`op_layout::scrollbar_geometry`](../crates/op_layout/src/lib.rs#L260)
- [`op_layout::scroll_top_at_track_point`](../crates/op_layout/src/lib.rs#L400)
- [`op_paint::build_display_list`](../crates/op_paint/src/lib.rs#L100)
- [`op_platform_win::paint_command`](../crates/op_platform_win/src/lib.rs#L1151)
- [`op_platform_win::scrollbar_at_document_point`](../crates/op_platform_win/src/lib.rs#L961)
- [`op_engine::click_at`](../crates/op_engine/src/lib.rs#L1807)

Vertical overlay scrollbars only; no drag, horizontal overflow-x/y, reserved gutters or complete complex positioned clipping.

## S50 — Layout-derived scroll maxima to bounded JavaScript scrollTop after rendering

Status: **R1.2a post-layout JS and no-script vertical scroll reconciliation**.

```text
op_layout::max_scroll_top
op_engine::render
op_js::sync_dom_scroll_limits
op_js::set_dom_scroll_top
op_engine::reflow
```

- [`op_layout::max_scroll_top`](../crates/op_layout/src/flow.rs#L1389)
- [`op_engine::render`](../crates/op_engine/src/lib.rs#L1040)
- [`op_js::sync_dom_scroll_limits`](../crates/op_js/src/runtime.rs#L3169)
- [`op_js::set_dom_scroll_top`](../crates/op_js/src/runtime.rs#L3184)
- [`op_engine::reflow`](../crates/op_engine/src/lib.rs#L2795)

JS range readback synchronizes after render/reflow, not synchronously in parser-time scripts; vertical normal-flow only, no scrollbars/overflow-x/y.

## S49 — Conservative original JS object and captured-environment reachability at idle page checkpoints

Status: **G1.0 read-only root tracer and engine memory-report integration; no GC**.

```text
op_js::trace_heap_reachability
op_js::TraceWork
op_js::trace_js_value
op_engine::active_script_heap_reachability
```

- [`op_js::trace_heap_reachability`](../crates/op_js/src/runtime.rs#L5102)
- [`op_js::TraceWork`](../crates/op_js/src/runtime.rs#L549)
- [`op_js::trace_js_value`](../crates/op_js/src/runtime.rs#L554)
- [`op_engine::active_script_heap_reachability`](../crates/op_engine/src/lib.rs#L1692)

Read-only conservative mark at idle checkpoint; no native activation stack roots, arena sweeping, generation-safe slots, or automatic GC.

## S48 — Original JS scrollTop and nested Win32 wheel to GDI pixels and scroll-aware hit targets

Status: **R1.1 vertical scroll offsets, programmatic and nested native wheel with document fallback**.

```text
op_js::set_dom_scroll_top
op_layout::layout_document_with_scrolling
op_layout::ScrollContainer
op_engine::scroll_at
op_paint::clip_commands_since
op_platform_win::NavigationEvent
op_browser::LoadResult
```

- [`op_js::set_dom_scroll_top`](../crates/op_js/src/runtime.rs#L3184)
- [`op_layout::layout_document_with_scrolling`](../crates/op_layout/src/lib.rs#L575)
- [`op_layout::ScrollContainer`](../crates/op_layout/src/flow.rs#L239)
- [`op_engine::scroll_at`](../crates/op_engine/src/lib.rs#L2584)
- [`op_paint::clip_commands_since`](../crates/op_paint/src/lib.rs#L243)
- [`op_platform_win::NavigationEvent`](../crates/op_platform_win/src/lib.rs#L65)
- [`op_browser::LoadResult`](../crates/op_browser/src/main.rs#L72)

Bounded vertical-only scrolling; no visible scrollbars, horizontal overflow longhands, full JS max-scroll clamping or broad overflow WPT conformance.

## S47 — Original CSS overflow hidden/clip through layout, paint, GDI pixels and native clicks

Status: **R1.0 verified nested GDI clipping and clipped native click hit testing**.

```text
op_css::parse_overflow
op_layout::collect_overflow_clips
op_layout::ClipRect
op_paint::clip_commands_since
op_platform_win::paint_command
op_engine::click_at
```

- [`op_css::parse_overflow`](../crates/op_css/src/computed.rs#L1995)
- [`op_layout::collect_overflow_clips`](../crates/op_layout/src/flow.rs#L101)
- [`op_layout::ClipRect`](../crates/op_layout/src/lib.rs#L187)
- [`op_paint::clip_commands_since`](../crates/op_paint/src/lib.rs#L243)
- [`op_platform_win::paint_command`](../crates/op_platform_win/src/lib.rs#L1151)
- [`op_engine::click_at`](../crates/op_engine/src/lib.rs#L1807)

Only overflow hidden/clip block descendant clipping; no scrollTop/wheel/nested offsets, axis longhands or comprehensive positioned/inline clipping.

## S46 — Original VM object/environment allocation observability before garbage collection

Status: **M4.35 non-collecting heap_usage diagnostic and explicit GC design**.

```text
op_js::heap_usage
op_js::allocate_object_with_function
op_js::allocate_environment
op_engine::execute_retained
```

- [`op_js::heap_usage`](../crates/op_js/src/runtime.rs#L5711)
- [`op_js::allocate_object_with_function`](../crates/op_js/src/runtime.rs#L1202)
- [`op_js::allocate_environment`](../crates/op_js/src/runtime.rs#L6420)
- [`op_engine::execute_retained`](../crates/op_engine/src/scripts.rs#L857)

Append-only arenas; diagnostic counts include unreachable slots; no collection, no long-session memory plateau.

## S45 — Parser documentElement and pre-dispatch propagation through original WPT

Status: **M4.35 pinned original WPT DOM/Events v8 21/21 attempted PASS 3 SKIP**.

```text
op_engine::sync_dom_tree
op_js::set_dom_document_element_node
op_js::dispatch_custom_event_inner
op_js::finish_event_dispatch
op_engine::instrument
op_paint::PaintCommand
```

- [`op_engine::sync_dom_tree`](../crates/op_engine/src/scripts.rs#L160)
- [`op_js::set_dom_document_element_node`](../crates/op_js/src/runtime.rs#L3500)
- [`op_js::dispatch_custom_event_inner`](../crates/op_js/src/runtime.rs#L3954)
- [`op_js::finish_event_dispatch`](../crates/op_js/src/runtime.rs#L2976)
- [`op_engine::instrument`](../crates/op_engine/src/bin/wpt_dom_probe.rs#L32)
- [`op_paint::PaintCommand`](../crates/op_paint/src/lib.rs#L37)

Narrow pinned original synchronous WPT selection; not representative DOM/Event/visual readiness.

## S44 — Real pinned WPT Events source through original Function.call and native render verdict

Status: **M4.34 original v7 16/16 selected attempted PASS, 3 explicit SKIP**.

```text
op_engine::instrument
op_js::FunctionCall
op_js::call_value
op_js::DomInitEvent
op_js::finish_event_dispatch
op_js::get_property
op_engine::run
op_paint::PaintCommand
```

- [`op_engine::instrument`](../crates/op_engine/src/bin/wpt_dom_probe.rs#L32)
- [`op_js::FunctionCall`](../crates/op_js/src/runtime.rs#L149)
- [`op_js::call_value`](../crates/op_js/src/runtime.rs#L1468)
- [`op_js::DomInitEvent`](../crates/op_js/src/runtime.rs#L183)
- [`op_js::finish_event_dispatch`](../crates/op_js/src/runtime.rs#L2976)
- [`op_js::get_property`](../crates/op_js/src/runtime.rs#L2552)
- [`op_engine::run`](../crates/op_engine/src/bin/wpt_dom_probe.rs#L101)
- [`op_paint::PaintCommand`](../crates/op_paint/src/lib.rs#L37)

Manually selected synchronous WPT smoke only; 19 pinned files, 16 attempted, 3 explicit SKIP; not full conformance.

## S43 — Original EventTarget constructor and handleEvent listeners to native paint

Status: **M4.33a-b standalone EventTarget, shared prototype and callback object delivery**.

```text
op_js::EventTargetConstructor
op_js::is_global_event_target
op_js::dispatch_global_custom_event
op_js::call_isolated_registered_listener
op_js::deliver_lifecycle_listeners
op_js::deliver_element_listeners
op_engine::apply_dom_operations
op_paint::build_display_list
```

- [`op_js::EventTargetConstructor`](../crates/op_js/src/runtime.rs#L199)
- [`op_js::is_global_event_target`](../crates/op_js/src/runtime.rs#L2781)
- [`op_js::dispatch_global_custom_event`](../crates/op_js/src/runtime.rs#L2775)
- [`op_js::call_isolated_registered_listener`](../crates/op_js/src/runtime.rs#L2905)
- [`op_js::deliver_lifecycle_listeners`](../crates/op_js/src/runtime.rs#L2811)
- [`op_js::deliver_element_listeners`](../crates/op_js/src/runtime.rs#L3878)
- [`op_engine::apply_dom_operations`](../crates/op_engine/src/scripts.rs#L224)
- [`op_paint::build_display_list`](../crates/op_paint/src/lib.rs#L100)

Bounded original EventTarget prototype and callback objects; no full WebIDL descriptors, Event subclasses, Shadow DOM or broad pinned WPT Events conformance.

## S42 — Trusted original layout-hit-tested click versus programmatic Element.click through native pixels

Status: **M4.32b original click trust marker and EventInit.composed/readonly flags**.

```text
op_engine::click_at
op_js::dispatch_dom_click_path_with_trust
op_js::DomEventConstructor
op_js::dispatch_lifecycle_event
op_js::dispatch_abort_signal_event
op_js::set_property
op_engine::apply_dom_operations
op_paint::build_display_list
```

- [`op_engine::click_at`](../crates/op_engine/src/lib.rs#L1807)
- [`op_js::dispatch_dom_click_path_with_trust`](../crates/op_js/src/runtime.rs#L3708)
- [`op_js::DomEventConstructor`](../crates/op_js/src/runtime.rs#L181)
- [`op_js::dispatch_lifecycle_event`](../crates/op_js/src/runtime.rs#L2932)
- [`op_js::dispatch_abort_signal_event`](../crates/op_js/src/runtime.rs#L2738)
- [`op_js::set_property`](../crates/op_js/src/runtime.rs#L5864)
- [`op_engine::apply_dom_operations`](../crates/op_engine/src/scripts.rs#L224)
- [`op_paint::build_display_list`](../crates/op_paint/src/lib.rs#L100)

Light-DOM events only; no Shadow DOM retargeting, trusted default activation, WebIDL property descriptors or broad WPT Events conformance.

## S41 — Event composedPath and cancelBubble during original DOM dispatch through native paint

Status: **M4.32a active event path snapshots, cleanup and legacy propagation alias**.

```text
op_js::install_element_event_path
op_js::install_global_event_path
op_js::EventComposedPath
op_js::dispatch_custom_event_inner
op_js::dispatch_dom_click_path
op_js::finish_event_dispatch
op_engine::apply_dom_operations
op_paint::build_display_list
```

- [`op_js::install_element_event_path`](../crates/op_js/src/runtime.rs#L3865)
- [`op_js::install_global_event_path`](../crates/op_js/src/runtime.rs#L2973)
- [`op_js::EventComposedPath`](../crates/op_js/src/runtime.rs#L195)
- [`op_js::dispatch_custom_event_inner`](../crates/op_js/src/runtime.rs#L3954)
- [`op_js::dispatch_dom_click_path`](../crates/op_js/src/runtime.rs#L3636)
- [`op_js::finish_event_dispatch`](../crates/op_js/src/runtime.rs#L2976)
- [`op_engine::apply_dom_operations`](../crates/op_engine/src/scripts.rs#L224)
- [`op_paint::build_display_list`](../crates/op_paint/src/lib.rs#L100)

Bounded connected light-DOM paths only; no Shadow DOM composed retargeting, full Event WebIDL descriptors or broad WPT Events conformance.

## S40 — Chronological onclick and lifecycle property handler delivery to original native paint

Status: **M4.31 owned EventTarget property slots share normal registration order**.

```text
op_js::lifecycle_handler_property
op_js::deliver_lifecycle_listeners
op_js::deliver_lifecycle_target
op_js::deliver_element_listeners
op_engine::apply_dom_operations
op_paint::build_display_list
```

- [`op_js::lifecycle_handler_property`](../crates/op_js/src/runtime.rs#L2790)
- [`op_js::deliver_lifecycle_listeners`](../crates/op_js/src/runtime.rs#L2811)
- [`op_js::deliver_lifecycle_target`](../crates/op_js/src/runtime.rs#L2917)
- [`op_js::deliver_element_listeners`](../crates/op_js/src/runtime.rs#L3878)
- [`op_engine::apply_dom_operations`](../crates/op_engine/src/scripts.rs#L224)
- [`op_paint::build_display_list`](../crates/op_paint/src/lib.rs#L100)

Bounded original property handler ordering; no complete WHATWG EventTarget/HTML activation or broad WPT Events coverage.

## S39 — Stable EventTarget callback registration identity through original DOM/native paint

Status: **M4.31 listener tokens distinguish a removed/readded function during dispatch**.

```text
op_js::version_listener_options
op_js::deliver_element_listeners
op_js::deliver_lifecycle_listeners
op_js::bind_dom_node
op_engine::apply_dom_operations
op_paint::build_display_list
```

- [`op_js::version_listener_options`](../crates/op_js/src/runtime.rs#L2630)
- [`op_js::deliver_element_listeners`](../crates/op_js/src/runtime.rs#L3878)
- [`op_js::deliver_lifecycle_listeners`](../crates/op_js/src/runtime.rs#L2811)
- [`op_js::bind_dom_node`](../crates/op_js/src/runtime.rs#L3290)
- [`op_engine::apply_dom_operations`](../crates/op_engine/src/scripts.rs#L224)
- [`op_paint::build_display_list`](../crates/op_paint/src/lib.rs#L100)

Original JS function callbacks only; no listener object interface or full WPT/DOM EventTarget conformance.

## S38 — AbortSignal cancels Promise fetch queues and late network callbacks to original paint

Status: **M4.30c-d original native-signal fetch rejection and late network suppression**.

```text
op_js::abort_signal
op_js::take_text_requests
op_engine::dispatch_text_requests
op_js::complete_text_response_request
op_engine::tick_timers
op_engine::apply_dom_operations
op_paint::build_display_list
```

- [`op_js::abort_signal`](../crates/op_js/src/runtime.rs#L2454)
- [`op_js::take_text_requests`](../crates/op_js/src/runtime.rs#L1418)
- [`op_engine::dispatch_text_requests`](../crates/op_engine/src/lib.rs#L2724)
- [`op_js::complete_text_response_request`](../crates/op_js/src/runtime.rs#L1429)
- [`op_engine::tick_timers`](../crates/op_engine/src/lib.rs#L2796)
- [`op_engine::apply_dom_operations`](../crates/op_engine/src/scripts.rs#L224)
- [`op_paint::build_display_list`](../crates/op_paint/src/lib.rs#L100)

Buffered same-origin GET; JS callbacks canceled and late completions dropped but blocking WinHTTP work may finish; no streaming, CORS or non-GET.

## S37 — DOMException AbortSignal timeout/any composition via page-owned task loop

Status: **M4.30b-d original DOMException, bounded timeout scheduler and signal followers**.

```text
op_js::new_dom_exception
op_js::new_abort_signal
op_js::abort_signal
op_js::next_timer_wait
op_js::run_due_timers
op_engine::tick_timers
op_js::deliver_lifecycle_listeners
```

- [`op_js::new_dom_exception`](../crates/op_js/src/runtime.rs#L2453)
- [`op_js::new_abort_signal`](../crates/op_js/src/runtime.rs#L2596)
- [`op_js::abort_signal`](../crates/op_js/src/runtime.rs#L2454)
- [`op_js::next_timer_wait`](../crates/op_js/src/runtime.rs#L2421)
- [`op_js::run_due_timers`](../crates/op_js/src/runtime.rs#L2438)
- [`op_engine::tick_timers`](../crates/op_engine/src/lib.rs#L2796)
- [`op_js::deliver_lifecycle_listeners`](../crates/op_js/src/runtime.rs#L2811)

No generic iterable AbortSignal.any, timeout >60 seconds, or broad WebIDL DOMException conformance.

## S36 — AbortController signal listener cancellation through original DOM and native paint

Status: **M4.30a bounded native AbortController/AbortSignal and listener options.signal**.

```text
op_js::new_abort_signal
op_js::listener_abort_signal
op_js::remove_aborted_signal_listeners
op_js::dispatch_abort_signal_event
op_js::deliver_element_listeners
op_js::deliver_lifecycle_listeners
op_engine::apply_dom_operations
op_paint::build_display_list
```

- [`op_js::new_abort_signal`](../crates/op_js/src/runtime.rs#L2596)
- [`op_js::listener_abort_signal`](../crates/op_js/src/runtime.rs#L2637)
- [`op_js::remove_aborted_signal_listeners`](../crates/op_js/src/runtime.rs#L2655)
- [`op_js::dispatch_abort_signal_event`](../crates/op_js/src/runtime.rs#L2738)
- [`op_js::deliver_element_listeners`](../crates/op_js/src/runtime.rs#L3878)
- [`op_js::deliver_lifecycle_listeners`](../crates/op_js/src/runtime.rs#L2811)
- [`op_engine::apply_dom_operations`](../crates/op_engine/src/scripts.rs#L224)
- [`op_paint::build_display_list`](../crates/op_paint/src/lib.rs#L100)

No DOMException default reason, AbortSignal.timeout/any, signal cancellation of fetch/timers, or broad WPT Events conformance.

## S35 — Connected Element event capture/target/bubble through document/window to native paint

Status: **M4.29b bounded original Element global propagation and native click handling**.

```text
op_engine::sync_dom_tree
op_js::element_event_path
op_js::path_reaches_document
op_js::deliver_element_global_capture
op_js::dispatch_custom_event_inner
op_js::dispatch_dom_click_path
op_js::has_dom_click_path_listener
op_engine::dispatch_click
op_paint::build_display_list
```

- [`op_engine::sync_dom_tree`](../crates/op_engine/src/scripts.rs#L160)
- [`op_js::element_event_path`](../crates/op_js/src/runtime.rs#L3223)
- [`op_js::path_reaches_document`](../crates/op_js/src/runtime.rs#L3213)
- [`op_js::deliver_element_global_capture`](../crates/op_js/src/runtime.rs#L3641)
- [`op_js::dispatch_custom_event_inner`](../crates/op_js/src/runtime.rs#L3954)
- [`op_js::dispatch_dom_click_path`](../crates/op_js/src/runtime.rs#L3636)
- [`op_js::has_dom_click_path_listener`](../crates/op_js/src/runtime.rs#L3680)
- [`op_engine::dispatch_click`](../crates/op_engine/src/scripts.rs#L1049)
- [`op_paint::build_display_list`](../crates/op_paint/src/lib.rs#L100)

64-element ancestry budget, no Shadow DOM, default activation, AbortSignal or broad WPT conformance.

## S34 — Document/window lifecycle and custom EventTarget through original native paint

Status: **M4.29a original document/window listener options, dispatchEvent and error isolation**.

```text
op_js::deliver_lifecycle_listeners
op_js::dispatch_lifecycle_event
op_js::dispatch_global_custom_event
op_js::call_isolated_event_handler
op_engine::apply_dom_operations
op_dom::set_text_content
op_paint::build_display_list
```

- [`op_js::deliver_lifecycle_listeners`](../crates/op_js/src/runtime.rs#L2811)
- [`op_js::dispatch_lifecycle_event`](../crates/op_js/src/runtime.rs#L2932)
- [`op_js::dispatch_global_custom_event`](../crates/op_js/src/runtime.rs#L2775)
- [`op_js::call_isolated_event_handler`](../crates/op_js/src/runtime.rs#L2869)
- [`op_engine::apply_dom_operations`](../crates/op_engine/src/scripts.rs#L224)
- [`op_dom::set_text_content`](../crates/op_dom/src/lib.rs#L354)
- [`op_paint::build_display_list`](../crates/op_paint/src/lib.rs#L100)

Element event paths do not yet include document/window; no AbortSignal or official asynchronous WPT harness.

## S33 — Element event listener once/passive error isolation to native pixels

Status: **M4.28b bounded original Element addEventListener options and diagnostics**.

```text
op_js::DomListenerOptions
op_js::deliver_element_listeners
op_js::call_isolated_event_handler
op_engine::active_event_listener_errors
op_engine::apply_dom_operations
op_paint::build_display_list
```

- [`op_js::DomListenerOptions`](../crates/op_js/src/runtime.rs#L431)
- [`op_js::deliver_element_listeners`](../crates/op_js/src/runtime.rs#L3878)
- [`op_js::call_isolated_event_handler`](../crates/op_js/src/runtime.rs#L2869)
- [`op_engine::active_event_listener_errors`](../crates/op_engine/src/lib.rs#L1701)
- [`op_engine::apply_dom_operations`](../crates/op_engine/src/scripts.rs#L224)
- [`op_paint::build_display_list`](../crates/op_paint/src/lib.rs#L100)

Element listener dictionaries only. No AbortSignal or document/window options, official async WPT harness or broad event conformance.

## S32 — Immediate event propagation stop through native DOM pixels

Status: **M4.28a bounded stopImmediatePropagation in original JS**.

```text
op_js::dispatch_dom_click_path
op_js::dispatch_custom_event_inner
op_js::event_immediate_stopped
op_engine::apply_dom_operations
op_paint::build_display_list
```

- [`op_js::dispatch_dom_click_path`](../crates/op_js/src/runtime.rs#L3636)
- [`op_js::dispatch_custom_event_inner`](../crates/op_js/src/runtime.rs#L3954)
- [`op_js::event_immediate_stopped`](../crates/op_js/src/runtime.rs#L2871)
- [`op_engine::apply_dom_operations`](../crates/op_engine/src/scripts.rs#L224)
- [`op_paint::build_display_list`](../crates/op_paint/src/lib.rs#L100)

Immediate-stop control only; once/passive and listener exception isolation remain incomplete. External WPT baseline unchanged.

## S31 — Compound DOM selector chains and typed Event dispatch to native repaint

Status: **M4.27 bounded original selectors and EventTarget with pinned WPT Events v5**.

```text
op_js::selector_chain_matches
op_js::query_descendants
op_js::dispatch_custom_event
op_engine::apply_dom_operations
op_dom::append_child
op_paint::build_display_list
```

- [`op_js::selector_chain_matches`](../crates/op_js/src/runtime.rs#L4424)
- [`op_js::query_descendants`](../crates/op_js/src/runtime.rs#L4465)
- [`op_js::dispatch_custom_event`](../crates/op_js/src/runtime.rs#L3909)
- [`op_engine::apply_dom_operations`](../crates/op_engine/src/scripts.rs#L224)
- [`op_dom::append_child`](../crates/op_dom/src/lib.rs#L194)
- [`op_paint::build_display_list`](../crates/op_paint/src/lib.rs#L100)

Bounded partial Selectors and EventTarget only; pinned WPT DOM/Events v5 is 12 chosen attempted/pass and 3 skips, not a conformance rate.

## S30 — JS in and scoped DOM selectors with programmatic click to native repaint

Status: **M4.26 original VM selectors and event dispatch with WPT DOM v4**.

```text
op_js::comparison
op_js::binary_with_coercion
op_js::query_descendants
op_js::dispatch_dom_click_path
op_engine::apply_dom_operations
op_dom::append_child
op_paint::build_display_list
```

- [`op_js::comparison`](../crates/op_js/src/parser.rs#L1435)
- [`op_js::binary_with_coercion`](../crates/op_js/src/runtime.rs#L2021)
- [`op_js::query_descendants`](../crates/op_js/src/runtime.rs#L4465)
- [`op_js::dispatch_dom_click_path`](../crates/op_js/src/runtime.rs#L3636)
- [`op_engine::apply_dom_operations`](../crates/op_engine/src/scripts.rs#L224)
- [`op_dom::append_child`](../crates/op_dom/src/lib.rs#L194)
- [`op_paint::build_display_list`](../crates/op_paint/src/lib.rs#L100)

Partial in, simple selectors, static query lists, bounded click only. 11 pinned manually chosen WPT files pass; 3 SKIP. Not full WPT.

## S29 — Live DOM tag search and pinned multi-test WPT DOM smoke to native pixels

Status: **M4.25 bounded tag collection and fail-sticky testharness shim**.

```text
op_engine::instrument
op_engine::run
op_engine::set_html_page
op_js::call_builtin
op_js::elements_by_tag
op_engine::apply_dom_operations
op_paint::build_display_list
```

- [`op_engine::instrument`](../crates/op_engine/src/bin/wpt_dom_probe.rs#L32)
- [`op_engine::run`](../crates/op_engine/src/bin/wpt_dom_probe.rs#L101)
- [`op_engine::set_html_page`](../crates/op_engine/src/lib.rs#L1766)
- [`op_js::call_builtin`](../crates/op_js/src/runtime.rs#L6736)
- [`op_js::elements_by_tag`](../crates/op_js/src/runtime.rs#L2002)
- [`op_engine::apply_dom_operations`](../crates/op_engine/src/scripts.rs#L224)
- [`op_paint::build_display_list`](../crates/op_paint/src/lib.rs#L100)

Ten manually selected original source WPT fixtures, 11 test callbacks, 3 skips; partial live tag collections, not full DOM.

## S28 — Live HTMLCollection and original pinned WPT DOM fixtures through native pixels

Status: **M4.24 limited WPT DOM smoke v2 and retained Element.children**.

```text
op_engine::instrument
op_engine::run
op_js::get_property
op_js::dom_html_collection_object
op_engine::apply_dom_operations
op_dom::remove_child
op_paint::build_display_list
```

- [`op_engine::instrument`](../crates/op_engine/src/bin/wpt_dom_probe.rs#L32)
- [`op_engine::run`](../crates/op_engine/src/bin/wpt_dom_probe.rs#L101)
- [`op_js::get_property`](../crates/op_js/src/runtime.rs#L2552)
- [`op_js::dom_html_collection_object`](../crates/op_js/src/runtime.rs#L4295)
- [`op_engine::apply_dom_operations`](../crates/op_engine/src/scripts.rs#L224)
- [`op_dom::remove_child`](../crates/op_dom/src/lib.rs#L286)
- [`op_paint::build_display_list`](../crates/op_paint/src/lib.rs#L100)

Seven manually scoped real WPT DOM fixtures, three explicit skips; no official WPT compliance score.

## S27 — Pinned upstream WPT DOM fixture through original JS and native paint

Status: **M4.23 fixed original WPT file with scoped synchronous harness adapter**.

```text
op_engine::instrument
op_engine::run
op_engine::set_html_page
op_js::get_property
op_engine::apply_dom_operations
op_dom::remove_child
op_paint::build_display_list
```

- [`op_engine::instrument`](../crates/op_engine/src/bin/wpt_dom_probe.rs#L32)
- [`op_engine::run`](../crates/op_engine/src/bin/wpt_dom_probe.rs#L101)
- [`op_engine::set_html_page`](../crates/op_engine/src/lib.rs#L1766)
- [`op_js::get_property`](../crates/op_js/src/runtime.rs#L2552)
- [`op_engine::apply_dom_operations`](../crates/op_engine/src/scripts.rs#L224)
- [`op_dom::remove_child`](../crates/op_dom/src/lib.rs#L286)
- [`op_paint::build_display_list`](../crates/op_paint/src/lib.rs#L100)

Exactly one original upstream WPT file attempted; limited harness, three explicit skips, not a full WPT score.

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

- [`op_engine::render_html`](../crates/op_engine/src/lib.rs#L1732)
- [`op_js::get_property`](../crates/op_js/src/runtime.rs#L2552)
- [`op_js::call_builtin`](../crates/op_js/src/runtime.rs#L6736)
- [`op_engine::apply_dom_operations`](../crates/op_engine/src/scripts.rs#L224)
- [`op_dom::replace_child`](../crates/op_dom/src/lib.rs#L267)
- [`op_css::compute_styles`](../crates/op_css/src/lib.rs#L21)
- [`op_paint::build_display_list`](../crates/op_paint/src/lib.rs#L100)

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

- [`op_engine::render_html`](../crates/op_engine/src/lib.rs#L1732)
- [`op_js::call_builtin`](../crates/op_js/src/runtime.rs#L6736)
- [`op_js::take_dom_operations`](../crates/op_js/src/runtime.rs#L3626)
- [`op_engine::apply_dom_operations`](../crates/op_engine/src/scripts.rs#L224)
- [`op_dom::insert_before`](../crates/op_dom/src/lib.rs#L195)
- [`op_css::collect_author_styles_with_linked`](../crates/op_css/src/lib.rs#L28)
- [`op_css::compute_styles`](../crates/op_css/src/lib.rs#L21)
- [`op_paint::build_display_list`](../crates/op_paint/src/lib.rs#L100)

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

- [`op_engine::render_html`](../crates/op_engine/src/lib.rs#L1732)
- [`op_js::call_builtin`](../crates/op_js/src/runtime.rs#L6736)
- [`op_js::take_dom_operations`](../crates/op_js/src/runtime.rs#L3626)
- [`op_engine::apply_dom_operations`](../crates/op_engine/src/scripts.rs#L224)
- [`op_dom::append_child`](../crates/op_dom/src/lib.rs#L194)
- [`op_css::compute_styles`](../crates/op_css/src/lib.rs#L21)
- [`op_layout::layout_document_with_backgrounds_and_resources`](../crates/op_layout/src/lib.rs#L555)
- [`op_paint::build_display_list`](../crates/op_paint/src/lib.rs#L100)

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

- [`op_engine::render_html`](../crates/op_engine/src/lib.rs#L1732)
- [`op_js::install_standard_primitives`](../crates/op_js/src/runtime.rs#L1041)
- [`op_js::call_builtin`](../crates/op_js/src/runtime.rs#L6736)
- [`op_js::get_property`](../crates/op_js/src/runtime.rs#L2552)
- [`op_dom::set_text_content`](../crates/op_dom/src/lib.rs#L354)
- [`op_paint::build_display_list`](../crates/op_paint/src/lib.rs#L100)

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

- [`op_js::tokenize`](../crates/op_js/src/lexer.rs#L102)
- [`op_js::conditional`](../crates/op_js/src/parser.rs#L1256)
- [`op_js::compile_program`](../crates/op_js/src/bytecode.rs#L110)
- [`op_engine::render_html`](../crates/op_engine/src/lib.rs#L1732)
- [`op_js::typeof_value`](../crates/op_js/src/runtime.rs#L1885)
- [`op_js::call_builtin`](../crates/op_js/src/runtime.rs#L6736)
- [`op_dom::set_text_content`](../crates/op_dom/src/lib.rs#L354)
- [`op_paint::build_display_list`](../crates/op_paint/src/lib.rs#L100)

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

- [`op_engine::render_html`](../crates/op_engine/src/lib.rs#L1732)
- [`op_js::install_standard_primitives`](../crates/op_js/src/runtime.rs#L1041)
- [`op_js::box_primitive`](../crates/op_js/src/runtime.rs#L1833)
- [`op_js::construct_value`](../crates/op_js/src/runtime.rs#L6269)
- [`op_js::coerce_to_primitive`](../crates/op_js/src/runtime.rs#L1852)
- [`op_js::binary_with_coercion`](../crates/op_js/src/runtime.rs#L2021)
- [`op_js::apply_binary`](../crates/op_js/src/runtime.rs#L2112)
- [`op_paint::build_display_list`](../crates/op_paint/src/lib.rs#L100)
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
- [`op_platform_win::render_display_list_to_bgra`](../crates/op_platform_win/src/lib.rs#L135)

Strict Static: 197/200; metadata-aware Static: 198/200; two Rec.2020 fixtures remain known mismatches.

## S68 — All-collection GC pauses and native DOM stress

Status: **G3 partial: bounded pause histogram, timer/DOM profiles, native arena occupancy**.

```text
op_js::collect_if_needed_at_idle
op_js::JsGcPauseHistogram
op_engine::active_script_gc_telemetry
op_engine::active_document_node_slots
op_engine::main
```

- [`op_js::collect_if_needed_at_idle`](../crates/op_js/src/runtime.rs#L5623)
- [`op_js::JsGcPauseHistogram`](../crates/op_js/src/runtime.rs#L475)
- [`op_engine::active_script_gc_telemetry`](../crates/op_engine/src/lib.rs#L1680)
- [`op_engine::active_document_node_slots`](../crates/op_engine/src/lib.rs#L1655)
- [`op_engine::main`](../crates/op_engine/src/bin/gc_stress_probe.rs#L107)

Percentiles are bucket upper bounds. Native Document nodes are not reclaimed; no 30-minute Windows plateau or real-network/navigation proof.

## S69 — Native DOM binding and recoverable staging quota

Status: **G3 partial: preserve connectivity, canonical native tree edges and bounded uncommitted identities**.

```text
op_js::allocate_virtual_dom_node
op_engine::apply_dom_operations
op_js::bind_dom_node
op_js::collect_if_needed_at_idle
```

- [`op_js::allocate_virtual_dom_node`](../crates/op_js/src/runtime.rs#L3544)
- [`op_engine::apply_dom_operations`](../crates/op_engine/src/scripts.rs#L224)
- [`op_js::bind_dom_node`](../crates/op_js/src/runtime.rs#L3290)
- [`op_js::collect_if_needed_at_idle`](../crates/op_js/src/runtime.rs#L5623)

Authoritative native Document storage remains append-only; staging quota recovery does not reclaim the native arena.

## S70 — Native DOM tracing, generational recycling and weak cache cleanup

Status: **G3 partial: JS-validated native tree sweeping with parser lifetime barrier**.

```text
op_engine::apply_dom_operations
op_engine::collect_at_idle
op_js::native_dom_roots
op_dom::collect_garbage
op_js::forget_collected_native_nodes
op_engine::prune_native_resources
op_engine::main
```

- [`op_engine::apply_dom_operations`](../crates/op_engine/src/scripts.rs#L224)
- [`op_engine::collect_at_idle`](../crates/op_engine/src/scripts.rs#L380)
- [`op_js::native_dom_roots`](../crates/op_js/src/runtime.rs#L3566)
- [`op_dom::collect_garbage`](../crates/op_dom/src/lib.rs#L427)
- [`op_js::forget_collected_native_nodes`](../crates/op_js/src/runtime.rs#L3583)
- [`op_engine::prune_native_resources`](../crates/op_engine/src/lib.rs#L987)
- [`op_engine::main`](../crates/op_engine/src/bin/gc_stress_probe.rs#L107)

Preserves complete retained native components; no mid-parser or in-flight native JS collection. Short synthetic Linux profile is not the 30-minute Windows/full browsing gate. Native pauses exclude subsequent metadata/cache pruning.

## S74 — Opt-in live public websites through original browser render path

Status: **Diagnostics implemented, modern web conformance NOT established**.

```text
op_browser::run
op_engine::navigate
op_engine::tick_timers
op_platform_win::render_display_list_to_bgra
op_browser::bmp_from_bgra
```

- [`op_browser::run`](../crates/op_browser/src/bin/live_site_probe.rs#L117)
- [`op_engine::navigate`](../crates/op_engine/src/lib.rs#L2871)
- [`op_engine::tick_timers`](../crates/op_engine/src/lib.rs#L2796)
- [`op_platform_win::render_display_list_to_bgra`](../crates/op_platform_win/src/lib.rs#L135)
- [`op_browser::bmp_from_bgra`](../crates/op_browser/src/bin/live_site_probe.rs#L68)

Only anonymous direct navigation, 1-second settles, screenshots and aggregate counters. Live-site content changes, sites may block automated loading, and this is not an interaction or visual-fidelity pass.

## S75 — Public-site JavaScript diagnostics drive conforming bitwise operators

Status: **Bounded per-page diagnostics implemented; bitwise expression subset tested, sites remain incomplete**.

```text
op_engine::ParserScriptRunner
op_engine::active_script_diagnostics
op_browser::script_diag_
op_js::Caret
op_js::bitwise_xor
op_js::to_int32
op_js::apply_binary
```

- [`op_engine::ParserScriptRunner`](../crates/op_engine/src/scripts.rs#L418)
- [`op_engine::active_script_diagnostics`](../crates/op_engine/src/lib.rs#L1649)
- [`op_browser::script_diag_`](../crates/op_browser/src/bin/live_site_probe.rs#L204)
- [`op_js::Caret`](../crates/op_js/src/lexer.rs#L84)
- [`op_js::bitwise_xor`](../crates/op_js/src/parser.rs#L1396)
- [`op_js::to_int32`](../crates/op_js/src/runtime.rs#L11530)
- [`op_js::apply_binary`](../crates/op_js/src/runtime.rs#L2112)

Only initial parser-script diagnostics are enumerated; callbacks/timers require broader tracing. Successful parsing of individual bitwise operators does not imply the modern Google/Pikabu scripts now work.

See [CODE_SLICES.md](CODE_SLICES.md) for full engineering notes.
