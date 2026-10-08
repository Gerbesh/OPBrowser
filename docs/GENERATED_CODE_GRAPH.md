# Generated Code Graph

> Generated from Cargo manifests using `python tools/code_intelligence.py --write`.
> Edges represent local crate dependencies, not Rust function calls.

**12 crates; 22 local dependency edges.**

```mermaid
graph LR
    op_browser["op_browser"]
    op_browser_core["op_browser_core"]
    op_css["op_css"]
    op_dom["op_dom"]
    op_engine["op_engine"]
    op_html["op_html"]
    op_image["op_image"]
    op_js["op_js"]
    op_layout["op_layout"]
    op_net["op_net"]
    op_paint["op_paint"]
    op_platform_win["op_platform_win"]
    op_browser --> op_engine
    op_browser --> op_html
    op_browser --> op_paint
    op_browser --> op_platform_win
    op_css --> op_dom
    op_css --> op_html
    op_engine --> op_css
    op_engine --> op_dom
    op_engine --> op_html
    op_engine --> op_image
    op_engine --> op_js
    op_engine --> op_layout
    op_engine --> op_net
    op_engine --> op_paint
    op_html --> op_dom
    op_layout --> op_css
    op_layout --> op_dom
    op_layout --> op_html
    op_layout --> op_image
    op_paint --> op_image
    op_paint --> op_layout
    op_platform_win --> op_paint
```

## Sources

- [`op_browser`](../crates/op_browser/Cargo.toml)
- [`op_browser_core`](../crates/op_browser_core/Cargo.toml)
- [`op_css`](../crates/op_css/Cargo.toml)
- [`op_dom`](../crates/op_dom/Cargo.toml)
- [`op_engine`](../crates/op_engine/Cargo.toml)
- [`op_html`](../crates/op_html/Cargo.toml)
- [`op_image`](../crates/op_image/Cargo.toml)
- [`op_js`](../crates/op_js/Cargo.toml)
- [`op_layout`](../crates/op_layout/Cargo.toml)
- [`op_net`](../crates/op_net/Cargo.toml)
- [`op_paint`](../crates/op_paint/Cargo.toml)
- [`op_platform_win`](../crates/op_platform_win/Cargo.toml)

See [CODE_GRAPH.md](CODE_GRAPH.md) for architectural decisions and runtime ownership.
