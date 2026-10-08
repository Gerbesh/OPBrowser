# Project Overview

OPBrowser exists to explore whether a modern Windows browser can be built with a
small, explicit architecture rather than embedding Chromium or another existing web
engine.

Core goals:

- native performance and low idle overhead;
- aggressive but safe tab memory management;
- minimal/no default telemetry;
- integrated content blocking;
- measurable standards conformance;
- clear process/resource accounting;
- original HTML/CSS/DOM/layout/rendering/JavaScript implementation.

The project starts on Windows 11 x86-64 using Rust and the MSVC toolchain.

The current milestone can open local/file/data and bounded HTTP(S) HTML,
paint its own DOM/CSS layout into a Win32 window, navigate links, scroll,
and reflow retained pages. The CSS parser/cascade, box/inline/flex/table
layout and positioned painting are functional **partial implementations**,
not future ideas. Raster images and ICC color conversions use Windows
infrastructure codecs only; layout and paint remain OPBrowser-owned.

A separate original JavaScript lexer/parser/bytecode/VM supports a useful
language subset, but the VM **does not yet run page scripts** or bind to
DOM/Web APIs. Tabs, the task manager, automatic tab freezing, comprehensive
content blocking and full web compatibility are still roadmap work.

See [Current Status](Current-Status.md), [Roadmap](Roadmap.md),
[Code Graph](Code-Graph.md), [Code Slicer](Code-Slicer.md) and
[Known Test Divergences](Known-Test-Divergences.md).
