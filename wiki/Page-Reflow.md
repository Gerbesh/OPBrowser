# Page Reflow

## CSS recascade after JavaScript focus (20 October 2026)

At the paint boundary, `PreparedDocument::render` consumes one-shot `JsRuntime::take_programmatic_focus_change` if `element.focus()`/`blur()` was executed by an inline script or DOM event callback. The Rust host resolves the JS node index to a live generation-safe NodeId, updates original `StyleInteractionState` and recascades `:focus/:focus-within` before native layout and paint. The JS VM owns the synchronous focus/blur event dispatch already, so this host sync never duplicates events. No new ready-made rendering dependency.

## Keyboard scroll into view without widening click targets (20 October 2026)

Original layout stores `focus_regions` copied after CSS positioning, nested scroll translation and inline glyph placement but before ancestor overflow clipping; `click_regions` remain clipped. Tab walks CSS scrollport ancestors from the inner container outward, using each container's computed min/max vertical/horizontal scroll offsets and rerendering original layout to find new target coordinates. The resulting document top/bottom bounds are sent once with the worker repaint to the Win32 main viewport, whose existing SCROLL_Y scrolls only if the target lies outside the visible page. Tests validate nested auto overflow, near-bottom/root scrolling, reverse Tab and absence of jitter.

## CSS focus-visible and fallback ring from keyboard focus (20 October 2026)

Original retained `StyleInteractionState` now receives focused NodeIds from DOM/tabindex-based native keyboard movement. Recomputing the author cascade activates `:focus`, `:focus-visible` and ancestor `:focus-within`. For a keyboard-focused node with a cached native hit region, OPBrowser draws a 2px blue rectangle after the existing page display list as a fallback indicator independent of site styling. Mouse click switches focus visibility off; navigation creates a fresh focus state. Automatic scrolling and exact outline clipping or rings for targets with no hit box are still unsupported.

## Original DOM hit order now guards painted hyperlinks (20 October 2026)

The Win32 painter still computes glyph/image hyperlink rectangles from the display list, but clicks now include the actual document coordinates and must pass the original Rust DOM hit map, computed `pointer-events` eligibility and matching ancestor `a[href]`. Regular image elements get generation-safe rectangles from the exact inline raster placement. Higher-z overlays block underlying navigation unless marked `pointer-events:none`. Multiple painted link rectangles are examined last-painted-first. The native display list remains the only renderer; browser engine CSS/layout/DOM are still first-party.

## Stacking-context-aware hit precedence (20 October 2026)

`LayoutTree.paint_groups` already drives the original raster compositing order. Each `ClickRegion` now optionally records the same `PaintKey`, including inline text/fragment positions. The engine sorts regions per completed render by nested context precedence and connected DOM order, then reverse-scans the cached list on hover/press. DOM click hit testing applies the same order to its fresh layout, so a smaller obscured element no longer steals focus/clicks from the higher-z visual element. Region creation, transform offsets, nested scrolling and clipping continue to come from original layout.

## Native hit regions follow overflow and positioned transforms (19 October 2026)

The original layout now exposes hit rectangles for all ordinary block elements, plus inline text/link spans mapped onto the text painter's measured glyph runs. Click regions move with relative-position transforms and nested scroll offsets and are clipped by overflow scopes; creating these regions does not split text into new line boxes. The engine reuses the last hit map for mouse hover and press and recalculates styles only when needed. CSS stacking-aware topmost-hit selection and inline atomic targets are not yet complete.

## CSS hover/focus recascade without unnecessary layout (19 October 2026)

Native Win32 motion is translated to document coordinates and hit-tested against the ID-bearing block rectangles cached by the previous OPBrowser layout. When the target changes, the browser recomputes the original CSS cascade and redraws; moving inside the same target returns no new frame. Pointer-leave clears hover, and queued moves while layout is busy collapse to the latest position. Mouse clicks on supported focusable blocks update `:focus`. Full hit-testing for inline and ID-less elements remains unsupported.


## Screen media queries update computed styles on resize (19 October 2026)

The authoritative page now tracks the viewport used to compute its author stylesheet. Changing width or height through native reflow triggers a fresh original CSS cascade for supported screen `@media` queries before layout and painting. Identical viewport renders avoid this extra refresh. Tests verify that a CSS-hidden paragraph becomes visible when the window crosses a min/max-width breakpoint; no page source or images are refetched solely for the style recascade.


Changing the window width now rewraps text and images without loading the document
or image sources again. Height changes update the background and scroll bounds.
The same path also supports the in-memory startup page.

## Retained state

Engine owns one private PreparedDocument: parsed DOM, effective address, MIME type
and per-node Arc image resources. Successful navigate/back/forward/reload replaces
this snapshot; failed loading leaves it and history unchanged. Reflow borrows the
snapshot and calls only font measurement, original layout and display-list building.
It never changes the link base or creates a history entry. Stateless render_source
continues to return a page without retaining it; set_html_page initializes the
startup snapshot with an empty history.
PreparedDocument also retains effective external stylesheet addresses by source NodeId;
author declaration provenance survives matching and computed substitution.
Generated before/after image Arc resources are retained by host/pseudo/content-item index;
ordinary and generated uses of the same URL keep shared pixels after source files disappear.

Loaded HTML bytes/text are dropped after parsing. DOM strings/node storage remain
alive for the current page and add memory beyond the display list. Shared decoded
pixels are reused, not copied for each resize. Existing resource budgets remain;
the 32 MiB decoded-image budget is not a total-process memory cap. Old active DOM
and resource-map ownership ends on successful replacement, while any old paint
commands keep their Arc references until the UI replaces them.

This is a current-page snapshot. Back/Forward and Reload still fetch their source;
there is no history page cache. Reload explicitly requests fresh content.

## Native event path

WM_SIZE moves controls immediately and resets a separate 120 ms debounce timer.
Minimized messages do not schedule it. The timer emits NavigationEvent::Resize,
which sends the current viewport dimensions to the existing navigation worker.
Repeated sizes are skipped when already presented. A busy worker remains serialized;
on completion, the UI compares requested and current viewport dimensions. Stale
geometry is discarded and a reflow for the latest size is requested. Navigation
completion still commits history, but its display waits for current-width geometry.

The normal 30 ms result-poll timer runs only while a worker command is active.
Navigation controls/shortcuts remain disabled during that command. Paint, close,
address editing and scrolling remain available. The status is Layout during idle
resize work; a prior load error survives subsequent reflow.

present_reflow keeps the address edit/title, preserves the pixel scroll offset
within the new content bounds and clears/rebuilds text/image hit regions before
immediate painting. New navigation still resets scroll and shows its effective
address. Semantic scroll anchoring and incremental relayout are future work.

## GDI concurrency

Parallel regression tests exposed occasional implausible short-word widths during
concurrent GDI font operations on this Windows installation. Worker metric calls
and native text drawing now share op_paint::GDI_TEXT_LOCK for font realization,
use and cleanup. Networking and original layout never hold the gate. Repeated
parallel tests and a 128-render concurrent comparison pass with the gate; the OS
root cause has not been established. Microsoft documents that access to shared
GDI objects requires application synchronization in
[Multiple Threads and GDI Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/multiple-threads-and-gdi-objects).

## Verification

    cargo run -p op_browser -- examples/navigation/resize.html
    cargo run -p op_browser -- --resize-smoke-test

The offline smoke resizes the real window repeatedly, then changes width while
the worker has an older reflow request. It requires final 320-pixel wrapping,
raster painting, a native image-link click and destination painting/history.
A ten-second watchdog bounds failure. CI runs this alongside existing smokes.

Engine tests remove the loaded HTML/PNG files, narrow/restore the retained page,
assert shared pixel identity and identical restored commands, then check failed
Reload/link rollback. Startup/history tests verify successful Back/Forward/Reload
snapshot replacement. Native tests cover debounce, address edits, scroll
preservation/clamping and stale link cleanup. Concurrent font tests compare repeated
multi-thread renders against an independently rendered baseline.
