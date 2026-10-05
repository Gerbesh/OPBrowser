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

The current milestone opens external HTTP/HTTPS HTML pages through a native address
bar or startup URL. It displays static text using the original engine, supports
Back/Forward/Reload and scrolling, and keeps the window responsive during loading.
Text hyperlinks are clickable, including relative HTTP(S) and local-file links.
PNG/JPEG/GIF/BMP images support bounded HTTP/file/data loading, dimensions, alpha,
alt fallback and image links. CSS and JavaScript remain future work.
