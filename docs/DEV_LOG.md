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

## 2026-10-05 - Clickable text hyperlinks and relative navigation

- Preserved href byte ranges through nested inline labels, whitespace normalization
  and line wrapping. Normalized text in place to avoid another character buffer.
- Added platform-neutral LinkSpan metadata and default blue/underlined presentation.
  GDI measures actual label bounds for hand-cursor and scroll-aware click hit testing;
  ordinary text and toolbar areas do not activate links, and page replacement clears
  obsolete hit regions.
- Added FollowLink input and Engine::follow_link through the existing worker and
  commit-after-success history path. Effective document bases follow successful
  redirects on navigate/back/forward/reload without rewriting history requests.
- Added owned relative HTTP(S) reference resolution, dot-segment handling and relative
  local-file paths. Unsupported link schemes fail visibly without external launches.
  Fragment scrolling, HTML base/target/download behavior and character-reference
  decoding remain explicit limitations of this iteration.
- Added an external link on the start page and two local navigation example pages.
- Added --link-smoke-test and offline CI coverage for click -> relative URL -> worker
  -> destination pixels. Extended native tests with actual measured hit bounds,
  scrolling and stale-region removal, and loopback engine tests with initial/reload
  redirects and subsequent relative links.
- Verified external hyperlink navigation to https://example.com through a native
  click and the original engine.
- Final verification passed: rustfmt check, warning-free Clippy, all 39 workspace
  tests, startup/offline address/offline hyperlink smoke tests, release build and
  release hyperlink navigation to external HTTPS. Release EXE: 307,200 bytes,
  an increase of 23,552 bytes; no dependency packages were added.

## 2026-10-05 - Legacy document encodings and HTML character references

- Addressed the reported UnsupportedCharset windows-1251 failure with an owned,
  shared source decoder for UTF-8, UTF-16LE/BE, Windows-1251 and Windows-1252.
- Added compact WHATWG single-byte tables and charset label aliases without adding
  dependency packages. HTTP/file/data sources now share decoding before tokenization.
- Added BOM -> transport -> bounded early-meta selection, including Content-Type
  http-equiv declarations, comment/quoted-attribute skipping and strict Unicode errors.
  Kept the UTF-8 default and documented remaining sniffing/encoding limitations.
- Added original tokenizer consumption of common named/numeric references in text
  and attribute values, so escaped markup remains text and &amp; query separators
  in hrefs become the actual navigation address.
- Added initial raw-text/RCDATA context so script/style source remains literal and
  title/textarea text follows character-reference context rules.
- Added a Windows-1251 example, exact Cyrillic/paint/link regression tests, HTTP
  header/meta/BOM fixtures and native CI smoke coverage for legacy document loading.
- Fixed a discovered Windows fixture race: accepted loopback sockets explicitly
  return to blocking mode before reads instead of inheriting listener nonblocking state.
- Final verification passed: rustfmt check, warning-free Clippy, all 50 workspace
  tests, startup/offline address/offline hyperlink smoke tests, Windows-1251 native
  loading and hyperlink smoke tests, release build and release legacy hyperlink test.
  Release EXE: 317,440 bytes, an increase of 10,240 bytes; no dependency packages
  were added. The reported site's URL was not supplied; verification used controlled
  HTTP fixtures and the checked-in Cyrillic example.

## 2026-10-05 - Complete HTML named character references

- Replaced the common-name subset with all 2125 WHATWG names and 106 legacy
  spellings, so real pages decode accented letters, Greek/Cyrillic, arrows and math.
- Added one-/two-scalar results throughout text, RCDATA and all attribute-value
  states. Decoded markup remains literal and raw script/style source stays intact.
- Implemented original bounded prefix-range lookup with longest-match selection,
  legacy fallback and historical attribute ambiguity, without lookup allocation.
- Packed names/deduplicated UTF-8 results and eight-byte entries occupy 35,378 static
  bytes; lookup examines at most 31 input characters. No crate dependencies were added.
- Pinned normalized WHATWG source data with original SHA-256 and BSD-3-Clause notice;
  added an offline Python standard-library generator and CI freshness verification.
- Added exhaustive tests against all 2231 source spellings in text/attributes/RCDATA,
  an independent prefix oracle, and an example covering Unicode paint text/link spans
  and decoded query navigation, including a native hyperlink smoke check in CI.
- Final verification passed: rustfmt check, generated-table freshness check,
  warning-free workspace Clippy, all 56 workspace tests, startup/address smoke,
  full-table and Windows-1251 native hyperlink smoke tests, release build and
  release full-table hyperlink smoke. Release EXE: 353,280 bytes, an increase of
  35,840 bytes from 317,440; dependency packages are unchanged.

## 2026-10-05 - First raster image subresource pipeline

- Added original img resource orchestration and bounded HTTP/file/data binary loads
  using the effective post-redirect document address. Network pages cannot read
  local image files; HTTPS image downgrades are blocked and cookies/auth stay disabled.
- Added op_image with targeted Windows WIC/COM bindings for explicit Microsoft
  PNG/JPEG/GIF/BMP codecs, first-frame decoding and preallocation size/budget checks.
  The OS codec does not replace any HTML/DOM/CSS/layout/painting/JavaScript subsystem.
- Added page-local success/failure caching and shared immutable Arc BGRA pixels.
  Limits cover image nodes, source attempts, accepted encoded bytes and decoded
  buffers. Serial loads/decode run on the navigation worker before page publication.
- Added initial separate-line ImageBox placement, intrinsic/width/height dimensions,
  viewport fitting, alt fallback and inherited image-link metadata without failing
  document navigation/history when an image fails.
- Added visible-raster GDI AlphaBlend with transient DIB/DC cleanup and existing
  scroll-aware cursor/click regions, including replacement cleanup.
- Added generated local color/alpha/oversize/budget fixtures, image paint/link CI
  smokes and codec, HTTP redirect/cache/failure, budget, dimension and GDI pixel tests.
- Added 14 Windows binding/support/procedural-macro packages; no third-party codec
  or browser/JavaScript engine. Full inline image formatting, progressive results,
  animation and additional formats remain explicitly planned.
- Final verification passed: rustfmt check, named-table freshness check,
  warning-free workspace Clippy, all 68 workspace tests, startup/address/image
  paint/image-link/Windows-1251 smokes, release build and release image paint/link
  smokes. Release EXE: 389,120 bytes, an increase of 35,840 from 353,280 bytes.
  The page's owned decoded buffers are capped at 32 MiB; this is not a total-process
  or OS-codec working-memory cap.

## 2026-10-05 - Measured mixed text and image lines

- Replaced separate-line image placement with original mixed inline line building,
  baseline alignment, atomic image wrapping and line heights that contain tall images.
- Added TextMeasurer/TextMetrics and a worker-local Windows GDI font/DC cache using
  the same Segoe UI settings as painting. Portable helpers and GDI errors retain an
  approximate fallback. Existing windows-sys is reused; no package dependencies added.
- Split layout into flow grouping and inline line construction. Adjacent inline
  siblings share anonymous groups around block children; HTML spaces collapse,
  NBSP stays intact, repeated br produces blank lines and alt labels stay inline.
- Preserved UTF-8 href ranges through wrapping and added LayoutTree::order so text
  and image paint commands follow placement/source order. Emergency long-word
  splitting probes bounded prefixes instead of repeatedly measuring giant suffixes.
- Added deterministic geometry/whitespace/Unicode/long-word tests, real GDI extent
  and cache tests, a mixed-line example and native image paint/link CI smokes.
- Full CSS inline formatting, shaping/bidi/grapheme breaking and resize reflow
  remain planned; the new formatting behavior is explicitly an initial M1 subset.
- Final verification passed: rustfmt check, generated-table freshness check,
  warning-free workspace Clippy, all 77 workspace tests, startup/address/mixed-image
  paint/mixed-image-link/existing-image-link/Windows-1251-link native smokes,
  release build and release mixed-image paint/link smokes. Release EXE: 398,848
  bytes, an increase of 9,728 from 389,120; dependency packages are unchanged.
