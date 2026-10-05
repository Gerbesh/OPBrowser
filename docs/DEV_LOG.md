# OPBrowser Development Log

This file is append-only project history.

## 2026-10-05 - Project definition

- Defined OPBrowser as an independent Windows 11 browser with an original engine.
- Chose Rust as the primary implementation language.
- Explicitly prohibited Chromium/Blink, WebKit, Gecko, WebView2/CEF and ready-made
  JavaScript engines as page execution/rendering dependencies.
- Defined HTML5test, WPT, and Test262 as conformance targets.
- Defined privacy, task-manager, tab lifecycle, content blocking, security, and
  performance requirements.

## 2026-10-05 - Development environment

- Installed Rustup and stable Rust 1.99.0 for x86_64-pc-windows-msvc.
- Installed rustfmt, Clippy, and rust-src.
- Installed Visual Studio Build Tools 2022 with C++ workload and Windows SDK.
- Installed CMake 4.4.3 and Ninja 1.13.2.
- Reused existing Git 2.54.0 and Python 3.12.10.
- Added docs/DEVELOPMENT.md with verified toolchain and build commands.

## 2026-10-05 - M0 foundation

- Created a Cargo workspace with op_browser, op_engine, op_dom, op_html, op_css,
  op_js, op_net, and op_platform_win.
- Created a native Win32 window using windows-sys bindings only.
- Added the browser message loop and an engine lifecycle skeleton.
- Added --smoke-test for non-interactive native-window startup verification.
- Initialized the root Git repository and removed nested Cargo-created repositories.
- Verified formatting, Clippy, workspace tests, smoke startup, debug build, and
  release build.
- Produced target/release/op_browser.exe.

## 2026-10-05 - First engine primitives

- Replaced op_dom placeholder code with an arena-style DOM model using stable NodeId
  values, parent links, and child lists.
- Added safe node move behavior between parents and regression coverage.
- Replaced op_html placeholder code with an original HTML tokenizer state machine.
- Initial tokenizer handles text, start/end tags, tag-name normalization,
  quoted/unquoted/boolean attributes, duplicate attributes, and self-closing syntax.
- Added tokenizer regression tests.
- Replaced CSS/JS/network sample arithmetic placeholders with subsystem skeletons.

## 2026-10-05 - Project traceability policy

- Added AGENTS.md so documentation/verification/commit rules live in the repository.
- Added a maintained project plan.
- Added a maintained code graph and vertical code-slice inventory.
- Added a repository-local wiki source.
- Defined every coherent engineering update to end in tests, a Git commit, and a push
  to the public remote once configured.

## 2026-10-05 - Public repository and CI

- Created public GitHub repository: https://github.com/Gerbesh/OPBrowser
- Renamed the default local branch to main and configured origin/main tracking.
- Pushed the complete foundation as the first public commit.
- Added Windows GitHub Actions CI for rustfmt, Clippy, tests, Win32 smoke startup,
  and release build.
- Enabled the GitHub Wiki feature.
- Kept wiki/ in the main repository as the canonical wiki source; GitHub creates the
  separate wiki Git repository only after the first wiki page exists.
