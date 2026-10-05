# OPBrowser

OPBrowser is an experimental Windows 11 browser built around an original web engine.

The project intentionally does **not** embed or fork Chromium/Blink, WebKit, Gecko,
Servo, V8, SpiderMonkey, JavaScriptCore, QuickJS, or another existing browser /
JavaScript engine.

Primary goals:

- low memory and CPU overhead;
- fast startup and responsive UI;
- minimum telemetry;
- built-in content blocking / ad-block integration;
- original HTML/CSS/DOM/layout/rendering/JavaScript engine;
- modern web-platform compatibility;
- built-in browser task manager;
- intelligent tab freezing, discarding and restoration.

Primary implementation language: **Rust**.

See:

- docs/REQUIREMENTS.md
- docs/ADR-0001-language-and-platform.md
