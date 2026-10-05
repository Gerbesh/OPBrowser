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

## 2026-10-05 - HTML tree builder

- Added element attributes to the DOM model.
- Added op_html -> op_dom dependency for tree construction.
- Added parse_document(), which converts tokenizer output into op_dom::Document.
- Added an open-elements stack, current-parent selection, text coalescing, void
  element handling, and initial mismatched-end-tag recovery.
- Added regression tests for nested DOM construction, attributes, void elements, and
  malformed nesting.
- Verified rustfmt, Clippy, and all workspace tests.

## 2026-10-05 - First visible rendering pipeline

- Added op_layout and op_paint crates.
- Added initial layout tree/text-box model with heading/paragraph defaults and
  approximate word wrapping.
- Added platform-neutral display-list commands for background fills and text.
- Added Engine::render_html() to orchestrate HTML -> DOM -> layout -> paint.
- Extended op_platform_win with a WM_PAINT GDI backend consuming the display list.
- Kept GDI isolated as a temporary OS drawing backend rather than a web engine.
- Added an in-memory OPBrowser start page rendered entirely by the new pipeline.
- Added synchronous UpdateWindow startup painting.
- Strengthened --smoke-test so it fails unless WM_PAINT actually executed.
- Verified rustfmt, Clippy, all workspace tests, paint smoke, and release build.

## 2026-10-05 - GitHub Actions external blocker

- Confirmed the CI workflow is discovered on every public push.
- Inspected the failed GitHub check-run annotation directly.
- GitHub reports: "The job was not started because your account is locked due to a billing issue."
- No workflow steps execute, so these red runs are not code/test failures.
- Local rustfmt, Clippy, workspace tests, real WM_PAINT smoke test, and release build
  all pass for commit 20b9eea.
- Kept CI configured so remote execution can resume once the GitHub account billing
  lock is cleared.

## 2026-10-05 - Local and data document source loading

- Replaced the op_net placeholder with NetworkContext, LoadedDocument, SourceKind and
  typed LoadError handling.
- Added direct/relative filesystem path loading and Windows drive-path recognition.
- Added file: URL parsing with percent decoding.
- Added UTF-8 BOM stripping and strict UTF-8 validation.
- Added data:text/html loading with percent-encoded and base64 payload support.
- Explicitly reject HTTP(S) until the network-navigation slice is implemented.
- Added Engine::render_source() to connect source loading to the existing renderer.
- Added startup CLI source selection and source-derived window titles.
- Added examples/hello.html as a repository smoke fixture.
- Verified unit tests, Clippy, normal paint smoke, local-file paint smoke, and data-URL
  paint smoke.

## 2026-10-05 - Navigation history core

- Added NavigationEntry and NavigationState to the engine.
- Added navigate(), go_back(), go_forward() and reload() APIs.
- Navigation commits history only after successful loading/rendering.
- Back/forward reload historical requests without creating duplicate entries.
- Reload keeps history length and current index unchanged.
- New navigation after Back truncates the obsolete forward branch.
- Failed navigation leaves the previous history byte-for-byte unchanged.
- Startup external sources now enter navigation through Engine::navigate().
- Added regression coverage for all navigation invariants and no-target operations.

## 2026-10-05 - External HTTP(S) sites and native address navigation

- Completed the address-driven static-page slice so a user can open an external
  site such as https://example.com from the window or a startup URL.
- Added an owned HTTP URL subset parser and a private WinHTTP transport module in
  op_net. Reused windows-sys bindings; added no new third-party package or engine.
- Added system proxy and certificate-validated HTTPS, bounded redirects with
  HTTPS downgrade rejection, gzip/deflate and HTTP framing through the OS API.
- Added response status/MIME/charset validation, a 2 MiB decoded-body limit,
  operation timeouts and between-read elapsed checks. Cookies and automatic
  authentication are disabled; limits are documented rather than claiming full
  URL/encoding/web compatibility.
- Connected native address Enter/Go, Back/Forward/Reload, Ctrl+L and F5 to a
  worker-owned Engine over command/result channels. UI polling runs only while
  loading, and errors preserve the last displayed page and history.
- Added display-list replacement, native repaint, status feedback, mouse-wheel
  scrolling and structural-container block traversal for real static site markup.
- Added loopback transport and engine/history tests plus native button/Enter /
  repaint/scroll tests. Added --navigation-smoke-test for the asynchronous input-to-
  pixels path and included its offline data-URL form in CI.
- Verified external https://example.com through native Enter input, the navigation
  worker, own HTML/DOM/layout/paint pipeline and real WM_PAINT.
- Final verification passed: rustfmt check, warning-free workspace Clippy, all 35
  workspace tests, startup/local-file/offline navigation smoke tests, external HTTP
  navigation smoke, release build and release external HTTPS navigation smoke.
- Produced target/release/op_browser.exe (283,648 bytes on this build).
