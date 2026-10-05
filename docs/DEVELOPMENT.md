# OPBrowser development environment

## Installed toolchain

Development host: Windows 11, x86-64.

Installed and verified:

- Rustup 1.29.1
- rustc 1.99.0, stable x86_64-pc-windows-msvc
- Cargo 1.99.0
- rustfmt
- Clippy
- rust-src
- Visual Studio Build Tools 2022 17.14.x with the C++ build tools workload
- Windows SDK installed through the Visual Studio C++ workload
- CMake 4.4.3
- Ninja 1.13.2
- Git 2.54.0
- Python 3.12.10

CMake is installed at:

C:\Program Files\CMake\bin\cmake.exe

Ninja is installed by WinGet at:

C:\Users\gerbe\AppData\Local\Microsoft\WinGet\Packages\Ninja-build.Ninja_Microsoft.Winget.Source_8wekyb3d8bbwe\ninja.exe

A newly opened terminal should receive updated WinGet/PATH environment state. The
project itself currently does not require CMake or Ninja for the Rust-only crates,
but they are installed now for future native dependencies and graphics tooling.

## Workspace crates

- op_browser: executable and browser-process bootstrap
- op_engine: engine orchestration
- op_dom: DOM data model
- op_html: HTML tokenizer/parser work
- op_css: CSS engine
- op_js: ECMAScript engine
- op_net: networking
- op_platform_win: Win32 platform integration

## Common commands

Format:

    cargo fmt --all

Check:

    cargo check --workspace

Lint:

    cargo clippy --workspace --all-targets -- -D warnings

Test:

    cargo test --workspace

Build debug browser:

    cargo build -p op_browser

Build release browser:

    cargo build -p op_browser --release

Run the browser:

    cargo run -p op_browser

Run the non-interactive Win32 startup smoke test:

    cargo run -p op_browser -- --smoke-test

Run the asynchronous address-input/worker/repaint smoke test (offline data URL):

    cargo run -p op_browser -- --navigation-smoke-test

Run the offline hyperlink-click/relative-file/worker/repaint smoke test:

    cargo run -p op_browser -- --link-smoke-test

Open the hyperlink example interactively:

    cargo run -p op_browser -- examples/navigation/index.html

Open an external site interactively:

    cargo run -p op_browser -- https://example.com

Verify external HTTPS through the same native input and worker path:

    cargo run -p op_browser -- --navigation-smoke-test https://example.com

Network tests use loopback TCP fixtures and do not require external access. WinHTTP
bindings reuse the existing windows-sys dependency; no new third-party package or
ready-made browser/JavaScript engine is added.

Debug executable:

    target\debug\op_browser.exe

Release executable:

    target\release\op_browser.exe

## Dependency rule

Do not add a browser engine, rendering engine, or ready-made JavaScript engine as a
dependency. OS APIs, generated Windows bindings, codecs, TLS, Unicode data and other
infrastructure libraries are allowed only when they do not replace OPBrowser's own
HTML, CSS, DOM, layout, rendering, JavaScript, or Web API implementation.
