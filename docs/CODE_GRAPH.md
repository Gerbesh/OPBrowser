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
    J[op_js<br/>ECMAScript VM]
    N[op_net<br/>network stack]

    B --> E
    B --> W
    E --> D
    E --> H
    E --> C
    E --> J
    E --> N
    H --> D
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
    }

    class NativeBrowserWindow {
        -HWND hwnd
        +create(title)
        +hwnd()
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

    class ElementData {
        +String tag_name
        +Vec~Attribute~ attributes
    }

    class Tokenizer {
        -Vec~char~ input
        -usize cursor
        -State state
        +new(input)
        +tokenize()
    }

    class Token {
        <<enumeration>>
        StartTag
        EndTag
        Character
        Eof
    }

    class parse_document {
        <<function>>
        +parse_document(input) Document
    }

    Engine --> Document
    Engine --> Tokenizer
    Document --> Node
    Node --> ElementData
    Tokenizer --> Token
    Tokenizer --> parse_document
    parse_document --> Document
```

## Current ownership boundaries

- op_browser owns process bootstrap and eventually browser-level UI/session state.
- op_platform_win owns Windows-specific window/input/surface/process glue.
- op_engine owns orchestration between web-engine subsystems.
- op_html owns HTML tokenization and tree construction rules.
- op_dom owns document/node storage, element attributes, and DOM invariants.
- op_css will own parsing, cascade, computed style, and style data.
- op_js will own the original ECMAScript implementation.
- op_net will own navigation networking, HTTP(S), cache, cookies, and filtering.

## Active graph changes

The next graph expansion will connect:

op_dom::Document -> op_layout -> op_paint display list ->
op_platform_win paint surface.
