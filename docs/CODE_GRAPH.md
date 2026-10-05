# OPBrowser Code Graph

Last updated: 2026-10-05

This document is the maintained human-readable code/dependency graph. It is updated
whenever crates, important types, or ownership boundaries change.

## Crate dependency graph

```mermaid
graph TD
    B[op_browser<br/>browser process bootstrap]
    E[op_engine<br/>engine orchestration]
    W[op_platform_win<br/>Win32 platform]
    D[op_dom<br/>DOM storage]
    H[op_html<br/>HTML tokenizer/tree builder]
    C[op_css<br/>CSS/style]
    L[op_layout<br/>layout]
    P[op_paint<br/>display list]
    J[op_js<br/>ECMAScript VM]
    N[op_net<br/>network stack]

    B --> E
    B --> W
    E --> D
    E --> H
    E --> C
    E --> L
    E --> P
    E --> J
    E --> N
    H --> D
    L --> D
    P --> L
    W --> P
```

No browser engine or ready-made JavaScript engine is below this graph.

## Current key types

```mermaid
classDiagram
    class Engine {
        -EngineState state
        +new()
        +start()
        +state()
        +render_html(html, width, height) DisplayList
    }

    class NativeBrowserWindow {
        -HWND hwnd
        +create(title, display_list)
        +hwnd()
        +painted_once()
        +run_message_loop()
    }

    class Document {
        -Vec~Node~ nodes
        -NodeId root
        +new()
        +root()
        +create_element()
        +create_element_with_attributes()
        +create_text()
        +append_child()
        +node()
        +children()
        +element()
    }

    class Node {
        +NodeKind kind
        +Option~NodeId~ parent
        +Vec~NodeId~ children
    }

    class Tokenizer {
        -Vec~char~ input
        -usize cursor
        -State state
        +new(input)
        +tokenize()
    }

    class LayoutTree {
        +i32 viewport_width
        +i32 content_height
        +Vec~TextBox~ text_boxes
    }

    class TextBox {
        +i32 x
        +i32 y
        +String text
        +i32 font_size
        +FontWeight weight
    }

    class DisplayList {
        +Vec~PaintCommand~ commands
    }

    class PaintCommand {
        <<enumeration>>
        FillRect
        Text
    }

    Tokenizer --> Document : parse_document
    Document --> LayoutTree : layout_document
    LayoutTree --> TextBox
    LayoutTree --> DisplayList : build_display_list
    DisplayList --> PaintCommand
    Engine --> DisplayList
    NativeBrowserWindow --> DisplayList
```

## Current ownership boundaries

- op_browser owns process bootstrap and eventually browser-level UI/session state.
- op_platform_win owns Windows-specific window/input/surface/process glue and consumes
  platform-neutral display lists.
- op_engine owns orchestration between web-engine subsystems.
- op_html owns HTML tokenization and tree construction rules.
- op_dom owns document/node storage, element attributes, and DOM invariants.
- op_layout owns current text-flow geometry and will grow into full layout.
- op_paint owns platform-neutral paint commands/display lists.
- op_css will own parsing, cascade, computed style, and style data.
- op_js will own the original ECMAScript implementation.
- op_net will own navigation networking, HTTP(S), cache, cookies, and filtering.

## Temporary architectural constraints

- The initial Windows renderer uses GDI as an OS drawing backend.
- Display-list storage is currently process-global because M1 has one window.
- Later browser/window isolation will move display-list state to per-window/per-renderer
  ownership and replace the temporary GDI backend with the planned DirectWrite /
  Direct2D / DirectComposition path.

## Active graph changes

The next graph expansion will connect:

navigation/local source -> op_net/source loader -> HTML input -> current rendering slice.
