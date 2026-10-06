# Page Reflow

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
