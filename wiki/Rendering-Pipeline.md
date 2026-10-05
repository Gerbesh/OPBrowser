# Rendering Pipeline

OPBrowser now has its first complete rendering slice.

## Current pipeline

```text
HTML string
  -> op_html::Tokenizer
  -> op_html::parse_document
  -> op_dom::Document
  -> op_layout::layout_document
  -> LayoutTree
  -> op_paint::build_display_list
  -> DisplayList
  -> op_platform_win
  -> WM_PAINT
  -> Win32 GDI
```

## Why GDI is acceptable here

GDI is only the temporary Windows pixel-output backend. It does not parse HTML, run
CSS, create layout, or decide what should be painted. Those decisions are owned by
OPBrowser crates.

The planned Windows rendering evolution is:

GDI bootstrap -> DirectWrite text -> Direct2D/Direct3D composition ->
DirectComposition where useful.

## Current layout subset

The M1 layout layer currently provides:

- body-root selection;
- hidden head/style/script content filtering;
- basic h1/h2/h3/p defaults;
- approximate word wrapping;
- vertical text flow;
- platform-neutral text boxes.

This is deliberately small. It exists to prove subsystem boundaries and the complete
path to pixels before expanding CSS/layout complexity.

## Paint smoke verification

NativeBrowserWindow::create calls UpdateWindow after ShowWindow. The WM_PAINT handler
sets an atomic painted-once flag. The --smoke-test mode fails if that flag is not set,
so CI verifies that the paint callback actually executed.
