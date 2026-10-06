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

## 2026-10-05 - Retained-page resize reflow

- Added a single active PreparedDocument with DOM, effective address/MIME and shared
  image resources. Reflow rebuilds layout/paint without fetching, decoding or
  changing history. Successful navigation/back/forward/reload replaces the snapshot;
  failed loading preserves it. The startup page retains DOM without a history entry.
- Added WM_SIZE debounce, worker reflow commands and viewport-tagged results.
  Stale-width geometry is discarded and the latest size is requested after busy
  work. Reflow presentation preserves address edits, clamps scroll and rebuilds
  measured text/image hit regions; navigation still resets scroll normally.
- Added a resize example and bounded offline native smoke that changes width during
  an older in-flight reflow, checks final wrapping/raster paint and clicks an image
  link. Added source-deletion/Arc-reuse/history tests and extended native tests for
  resize debounce, scroll preservation/clamping and stale hit regions.
- Parallel checks exposed transient implausible GDI widths for short words on this
  installation. A shared per-operation GDI font gate removes the observed fault;
  OS cause is not established. A 128-render concurrent regression checks stability.
- The current DOM now remains alive beyond loading; image pixels stay Arc-shared.
  This is not a history page cache, incremental layout or semantic scroll anchoring.
  No dependency packages were added; op_browser adds an internal op_paint dependency
  for smoke geometry assertions.
- Final verification passed: rustfmt check, warning-free workspace Clippy, all 80
  workspace tests, generated-table freshness, startup/address/resize/mixed-image
  paint/mixed-image-link/Windows-1251-link native smokes, release build and release
  resize/image paint smokes. Release EXE: 417,792 bytes, an increase of 18,944 from
  398,848; dependency packages are unchanged. The new resize smoke also verifies
  an actual hyperlink destination after the geometry update.

## 2026-10-06 - CSS syntax and core stylesheet model

- Replaced the op_css placeholder with an original tokenizer and parser implemented
  without new dependency packages or a ready-made CSS engine.
- Added positioned tokens for identifiers, hashes, strings/escapes, numbers,
  percentages, dimensions, functions and CSS punctuation; comments are consumed and
  malformed comments/strings produce recoverable errors.
- Added Stylesheet, StyleRule, Declaration, Selector, CompoundSelector, SimpleSelector
  and Specificity data models. The initial selector parser supports type/universal,
  class/ID, selector lists and descendant/child combinators.
- Added declaration-list parsing for future style attributes, case normalization for
  ordinary property names, custom-property case preservation and !important extraction.
- Unsupported at-rules/selectors are explicit initial limitations and do not poison
  following valid rules. CSS is intentionally not wired to layout in this update.
- Added eight focused CSS tests. Final verification passed workspace rustfmt,
  warning-free workspace Clippy, all 88 workspace tests and the native browser
  startup/paint smoke.

## 2026-10-06 - Author style collection and selector matching

- Added op_css -> op_dom ownership for the style-resolution boundary and op_html only as
  an op_css test dependency; no third-party package was added.
- Added right-to-left selector matching for the existing type/universal/class/ID,
  compound, descendant and child subset.
- Added collection of CSS style elements and inline style attributes into a per-NodeId
  StyleMap. MatchedDeclaration retains specificity, source order, !important and
  stylesheet-vs-inline source for the upcoming cascade instead of choosing winners early.
- Selector-list declarations are stored once with the highest specificity among matching
  selectors. CSS parse errors retain their source NodeId; non-CSS style types are skipped.
- PreparedDocument now retains StyleCollection beside DOM/images. Engine exposes the
  active style map/errors for diagnostics and resize reflow keeps the same collection.
- Added five op_css matching/collection tests and one engine retention/reflow test.
  CSS still does not affect layout or pixels; cascade/computed values are the next step.
- Final verification passed workspace rustfmt, warning-free workspace Clippy, all 94
  workspace tests and the native browser startup/paint smoke.

## 2026-10-06 - Initial cascade, inheritance and computed styles

- Added ComputedStyleMap and ComputedStyle for the first resolved property subset:
  display, color, font-size and font-weight.
- Added author cascade precedence across !important, inline-vs-stylesheet source,
  specificity and source order. Invalid/unsupported values for a supported property are
  filtered before winner selection so lower-priority valid declarations remain usable.
- Added inheritance for color/font-size/font-weight and global inherit/initial/unset
  handling; display remains non-inherited unless explicitly set to inherit.
- Added initial value parsing for inline/block/none, bounded px font sizes,
  normal/bold/400/700 weights, a small named-color set and #RGB(A)/#RRGGBB(AA).
- Refined CSS hash tokenization so digit-leading values such as #123456 are accepted
  for hexadecimal colors while an ID-type flag keeps non-identifier hashes such as #123
  from being accepted as ID selectors.
- Added temporary UA defaults matching current M1 block/hidden behavior and heading
  typography so the next layout integration can preserve pages without author CSS.
- PreparedDocument now retains ComputedStyleMap beside DOM/images/author styles and
  resize reflow reuses it without reparsing, rematching or recascading CSS.
- Added six focused computed-style tests, two hash/selector conformance regressions and
  extended the engine retention test. CSS still does not change pixels until layout/paint
  consume ComputedStyleMap.
- Final verification passed workspace rustfmt, warning-free workspace Clippy, all 102
  workspace tests and the native browser startup/paint smoke.

## 2026-10-06 - Computed CSS reaches layout and native paint

- Connected the retained ComputedStyleMap to op_layout and op_paint so author CSS now
  changes native Win32 pixels instead of stopping after style resolution.
- display:none skips element subtrees, display:block creates a flow boundary, and
  display:inline remains in the surrounding inline flow.
- Reworked inline line construction to preserve per-run computed font size, weight and
  RGBA text color. Differently sized runs share one measured baseline with images.
- TextBox now carries color; op_paint composites alpha text colors over the current white
  page background and emits the resulting native text color.
- Preserved the existing LinkSpan/hit-testing path and native default blue for hyperlink
  glyphs/underlines; author link color remains an explicit temporary limitation.
- Added an end-to-end engine regression proving CSS size/weight/color/display reach paint,
  plus a paint regression for alpha compositing. Workspace total is now 104 tests.
- Added `examples/css/index.html` and styled the built-in start page so the supported CSS
  subset is visible immediately in a normal release launch.
- Updated project plan, code graph/slices, README, development workflow and CSS/rendering/
  inline-layout wiki pages for the new end-to-end rendering path.
- Final verification passed rustfmt, warning-free workspace Clippy, all 104 workspace
  tests, normal startup smoke and a CSS-demo smoke producing 13 paint commands.
- Rebuilt `target/release/op_browser.exe`; this build is 495,616 bytes.

## 2026-10-06 - External author stylesheet loading

- Added bounded `<link rel="stylesheet">` discovery during page preparation and connected
  loaded external CSS to the existing author cascade at each link node's DOM source order.
- Added local/file/data/HTTP(S) stylesheet loading in op_net with a dedicated Stylesheet
  resource kind, CSS Accept header, MIME validation, redirect/source policy and byte errors.
- Added CSS text decoding with BOM -> transport/data charset -> initial `@charset` -> UTF-8
  selection using the owned UTF-8/UTF-16/Windows-1251/Windows-1252 decoder.
- External stylesheet failures are nonfatal; duplicate resolved sources reuse fetched CSS.
  Current document bounds are 32 link nodes, 8 requests, 1 MiB per stylesheet, 2 MiB
  decoded CSS total and a 10 second stylesheet-discovery guard.
- The initial link activation subset accepts normal stylesheet rels for empty/all/screen
  media, skips alternate/disabled/print and rejects non-CSS type attributes.
- Added local and loopback-HTTP end-to-end regressions proving external CSS reaches native
  paint, later embedded rules override linked CSS, and resize reflow retains styles after
  the original local HTML/CSS files have been deleted.
- Updated `examples/css/index.html` to load the new `examples/css/theme.css` fixture and
  demonstrate source-order override behavior; the start page now advertises external CSS.
- Added `wiki/Stylesheet-Loading.md` and updated project plan, code graph/slices, README,
  development instructions, CSS foundation, rendering pipeline and wiki navigation.
- Deliberate limits remain: no `@import`, general media queries, `<base>` stylesheet URL
  semantics, CSS `url(...)` resources, integrity/CORS/CSP handling or cross-document cache.
- Final verification passed rustfmt, warning-free workspace Clippy, all 112 workspace tests,
  normal startup smoke and the external-CSS demo smoke with 14 paint commands.
- Rebuilt `target/release/op_browser.exe`; this build is 518,144 bytes.

## 2026-10-06 - Initial CSS block box model

- Extended ComputedStyle with non-inherited background-color, margin/padding edges and a
  uniform initial border value while preserving the existing text/display inheritance path.
- Added `margin` and `padding` shorthands with 1-4 nonnegative px/zero values, plus
  `background-color`, `border: none`, and `<px> solid <color>` parsing/cascade support.
- Moved temporary h1-h6 and paragraph/list vertical spacing into computed UA margins so
  author margins and browser defaults now enter the same block geometry path.
- Reworked op_layout block flow to compute margin -> border -> padding -> content geometry,
  propagate the reduced content width to nested blocks/inline lines/images, and emit
  BoxDecoration records for visible block backgrounds and solid borders.
- op_paint now expands BoxDecoration into a background FillRect and four solid-border side
  FillRects before text/images; Win32 remains a native drawing backend rather than CSS logic.
- Added deterministic computed-style, layout, paint and engine regressions for shorthand
  expansion/global keywords, UA-margin override, nested content coordinates and final paint.
- Updated the built-in start page and external CSS demo with visible nested box-model panels;
  the CSS demo smoke now produces 26 paint commands.
- Added `wiki/CSS-Box-Model.md` and updated project plan, code graph/slices, README,
  development instructions, CSS/rendering/inline-layout wiki pages and wiki navigation.
- Current limits are explicit: ordinary non-replaced block boxes only, no margin collapsing,
  side longhands, width/height, auto/percent/em sizing, negative margins, inline box
  fragments/decorations, replaced-element box decoration, radii/shadows or full stacking.
- Final verification passed rustfmt, warning-free workspace Clippy, all 117 workspace tests,
  normal startup smoke, the 26-command CSS box demo smoke and optimized release build.
- Rebuilt `target/release/op_browser.exe`; this build is 528,384 bytes.

## 2026-10-06 - Expanded CSS block sizing and margin collapse

- Replaced the initial px-only uniform box representation with owned CSS used-value types
  for length/percentage, auto margins, independent margin/padding edges, independent border
  edges and content-box/border-box sizing.
- Added margin/padding side longhands, border side shorthands, border-width/style/color and
  all corresponding side longhands. Shorthand and longhand candidates compete through the
  normal cascade key instead of a fixed application order.
- Added width/height, min/max width/height and box-sizing values. Used block width now handles
  percentages, auto horizontal margins, min/max constraints, negative margins and centered
  fixed/percentage-width blocks.
- Expanded length parsing to px, percent, em/rem and CSS absolute units (in/cm/mm/Q/pt/pc),
  plus font-size percentage/absolute/relative keywords. Current rem uses the initial root
  baseline; percentage heights remain auto-like without a definite containing height.
- Added border width keywords thin/medium/thick, currentColor, per-side widths/colors and
  side-specific display-list FillRect painting. Border styles currently support none/solid.
- Added adjacent sibling vertical margin collapsing with CSS positive/negative arithmetic.
  Whitespace-only text between block siblings no longer creates an anonymous line that would
  break collapse. Parent/child and empty-block collapsing remain explicit later work.
- Added deterministic CSS, layout, paint and engine regressions for cascade competition,
  unit conversion, percent/min/max width, auto centering, border-box height, per-side paint,
  negative margins and sibling margin collapse.
- Updated the built-in start page and CSS demo with centered percentage boxes, min/max width,
  percent/em padding, box-sizing, independent border sides and a visible margin-collapse case.
  The CSS demo smoke now produces 35 paint commands.
- Updated project plan, code graph/slices, README, development docs and CSS/rendering/layout
  wiki pages to describe the expanded behavior and remaining limits precisely.
- Final verification passed rustfmt, warning-free workspace Clippy, all 123 workspace tests,
  normal startup smoke, the 35-command expanded CSS demo smoke and optimized release build.
- Rebuilt `target/release/op_browser.exe`; this build is 548,352 bytes.

## 2026-10-06 - Expanded CSS selector matching

- Extended the selector AST with attribute selectors, attribute match operators and explicit
  ASCII case flags, plus adjacent/general sibling combinators and initial pseudo-class nodes.
- Added `[attr]`, `=`, `~=`, `|=`, `^=`, `$=`, `*=` matching with `i`/`s` flags. Attribute
  names follow HTML ASCII-insensitive lookup while values remain case-sensitive unless `i`.
- Added `+` and `~` right-to-left matching over element siblings, correctly ignoring text
  nodes between elements instead of treating DOM indentation as a CSS sibling.
- Added `:root`, `:first-child`, `:last-child`, `:only-child`, `:empty` and `:link` matching.
  Attributes and pseudo-classes contribute class-column specificity.
- Added parser/matcher regressions for every attribute operator, flags, sibling relations,
  structural pseudos, :empty whitespace semantics and specificity, plus an engine regression
  proving the expanded selectors reach native display-list text/background output.
- Updated the CSS demo with attribute + adjacent sibling, general sibling + suffix matching
  and a visible :empty block; the demo smoke now produces 44 paint commands.
- Updated the start page, project plan, code graph/slices, README and CSS syntax wiki with the
  expanded selector surface and explicit remaining functional-pseudo/pseudo-element limits.
- Final verification passed rustfmt, warning-free workspace Clippy, all 127 workspace tests,
  normal startup smoke, the 44-command selector demo smoke and optimized release build.
- Rebuilt `target/release/op_browser.exe`; this build is 564,224 bytes.

## 2026-10-06 - Functional CSS colors

- Reworked supported color parsing through one shared CssColor value path used by text,
  backgrounds and border color/shorthand parsing instead of property-specific color logic.
- Added legacy comma and modern space/slash `rgb()`/`rgba()` syntax with numeric or percentage
  channels, number/percentage alpha and CSS-style output clamping.
- Added legacy/modern `hsl()`/`hsla()` with percentage saturation/lightness, alpha and hue in
  unitless degrees, deg, grad, rad or turn; HSL is converted to sRGB computed channels.
- Expanded named colors to the CSS basic set plus aliases and `rebeccapurple`; transparent
  and existing #RGB(A)/#RRGGBB(AA) remain supported.
- Updated border component splitting so functional colors remain one top-level value inside
  `border`, per-side border shorthands and one-to-four `border-color` lists.
- Added computed-style regressions covering legacy/modern functional syntax, clamping,
  functional border shorthands/lists, alpha and multiple hue units, plus a full engine test
  proving rgb/hsl reach text/background/border native display-list output.
- Updated the built-in start page and CSS demo with visible rgb/rgba/hsl functional colors;
  the CSS demo smoke now produces 52 paint commands.
- Updated project plan, code slices/graph, README and CSS syntax wiki with the shared functional
  color path and remaining advanced Color 4 / layered alpha limitations.
- Final verification passed rustfmt, warning-free workspace Clippy, all 129 workspace tests,
  normal startup smoke, the 52-command functional-color demo smoke and optimized release build.
- Rebuilt `target/release/op_browser.exe`; this build is 571,904 bytes.

## 2026-10-06 - Text alignment and line-height geometry

- Added inherited `text-align` values start/end/left/right/center to ComputedStyle and
  carried alignment into the inline formatter instead of treating it as paint-only metadata.
- Each completed wrapped/explicit-break line now computes its remaining content-box width and
  applies the requested horizontal alignment before TextBox/ImageBox placement.
- Added inherited `line-height` support for normal, unitless multipliers, percentages and
  CSS length units. Unitless values remain multipliers across inheritance; percentages and
  lengths compute to px and now drive the actual line strut/ascent/descent geometry.
- Expanded font-weight parsing to numeric 1-1000 plus bolder/lighter. The current GDI text
  backend still exposes only normal/bold faces, so weights are deliberately mapped onto those
  two available rendering buckets until variable/multiweight font selection is implemented.
- Added deterministic computed-style and layout regressions plus an Engine display-list test
  proving center/right x coordinates and an exact 40px line-height survive to native paint.
- Updated the built-in start page and CSS demo with centered/right-aligned multi-line panels,
  unitless/fixed line-height and numeric font weights; the demo smoke now emits 66 commands.
- Updated project plan, code graph/slices, README and CSS/rendering/inline-layout wiki pages.
- Final verification passed rustfmt, warning-free workspace Clippy, all 132 workspace tests,
  normal startup smoke, the 66-command CSS demo smoke and optimized release build.
- Rebuilt `target/release/op_browser.exe`; this build is 576,000 bytes.

## 2026-10-06 - Font style, text decoration and white-space modes

- Added inherited `font-style: normal|italic|oblique`; italic/oblique now select the same
  italic GDI face during both native measurement and paint, with worker font-cache keys
  extended by style so text geometry matches the rendered face.
- Added initial `text-decoration` / `text-decoration-line` handling for `none`, `underline`
  and `line-through` combinations. TextBox and PaintCommand now carry decoration flags;
  Win32 draws measured decoration segments for normal/link runs instead of keeping the state
  as parser-only metadata.
- Added `white-space: normal|nowrap|pre|pre-wrap|pre-line` to the owned line formatter.
  Newline preservation, ASCII-space collapsing/preservation and soft wrapping now depend on
  the computed mode; preserved tabs currently expand to four spaces.
- Added UA defaults for b/strong, i/em, u, s/strike/del and pre so common semantic HTML uses
  the same computed-style path without separate renderer special cases.
- Added deterministic computed-style/layout/Engine regressions covering inheritance, UA
  defaults, nowrap, preserved spaces/newlines, italic and decoration display-list flags.
  Native Windows tests continue to pass with italic font creation and decoration painting.
- Updated the built-in start page and CSS demo with italic, underline/line-through, pre-wrap
  and nowrap examples; the CSS demo smoke now emits 85 paint commands.
- Updated project plan, code slices/graph, README and CSS/rendering/inline-layout wiki pages.
- Deliberate limits remain: oblique maps to italic, decoration color/style/thickness are not
  implemented, decoration propagation is simplified, tab-size is fixed, inline replaced
  elements do not fully honor nowrap, and full Unicode/grapheme/bidi breaking remains later.
- Final verification passed rustfmt, warning-free workspace Clippy, all 135 workspace tests,
  native Windows tests, normal startup smoke, the 85-command CSS demo smoke and release build.
- Rebuilt `target/release/op_browser.exe`; this build is 583,680 bytes.

## 2026-10-06 - Text transform and character/word spacing

- Added inherited `letter-spacing` and `word-spacing` computed values with normal/global
  keywords plus signed CSS lengths; word-spacing also accepts percentages against the
  current computed font size in this initial subset.
- Added inherited `text-transform: none|uppercase|lowercase|capitalize`. Transform runs
  before measurement and final TextBox/link-span construction, so Unicode expansions such
  as `ß -> SS` produce valid transformed UTF-8 link byte ranges instead of stale offsets.
- Inline layout now includes letter/word spacing in measured run widths, wrapping, text-align
  offsets and final TextBox geometry. Positive and negative spacing therefore affect real
  line breaking rather than only painter metadata.
- PaintCommand carries the same spacing values. Win32 draws nonzero-spaced text per Unicode
  scalar while keeping returned segment/link widths anchored to whole-run GDI measurement
  plus the CSS spacing adjustment; decoration widths follow the spaced segment bounds.
- Added computed/layout/Engine regressions for inheritance, em/%/negative spacing,
  uppercase/lowercase/capitalize, transformed Unicode text and link byte-range preservation.
- Updated the built-in start page and CSS demo with uppercase/capitalize plus positive and
  negative spacing examples; the CSS demo smoke now emits 99 paint commands.
- Updated project plan, code graph/slices, README and CSS/rendering/inline-layout wiki pages.
- Deliberate limits remain: capitalize uses whitespace word starts instead of full locale/
  context-sensitive CSS rules, word spacing targets processed ASCII spaces, and per-scalar
  GDI paint can differ slightly from whole-run kerning used as the layout width baseline.
- Final verification passed rustfmt, warning-free workspace Clippy, all 138 workspace tests,
  native Windows tests, normal startup smoke, the 99-command CSS demo smoke and release build.
- Rebuilt `target/release/op_browser.exe`; this build is 596,992 bytes.

## 2026-10-06 - Inline box fragments

- Added initial inline fragment geometry for non-replaced inline elements with computed
  `background-color`, padding and solid per-side borders. Horizontal padding/borders now
  participate in fitting, wrapping and text-align instead of painting over neighboring text.
- Wrapped decorated spans emit independent per-line BoxDecoration records through the same
  platform-neutral block decoration path, so op_paint/Win32 did not need a second bespoke
  inline painting backend.
- Vertical padding/borders enlarge the safe line box in this initial implementation to avoid
  fragment paint overlapping adjacent lines; text glyph baselines remain aligned normally.
- Inline box identity includes the source NodeId, so adjacent elements with identical CSS stay
  separate. Undecorated nested inline text such as b/em/a keeps the outer active decoration.
- Added deterministic layout regressions for wrapping fragments, exact left border+padding
  geometry, nested bold continuity and distinct adjacent equal-style spans, plus an Engine
  regression proving background/border FillRects and text offsets reach the display list.
- Updated the built-in start page and CSS demo with padded bordered inline chips; the CSS demo
  smoke now emits 119 paint commands.
- Updated project plan, README, code graph/slices and CSS/rendering/inline-layout wiki pages.
- Deliberate limits remain: only one decorated inline ancestor is represented at once, a nested
  decorated inline replaces the outer decoration for that nested run, inline images do not yet
  inherit fragment decoration, and wrapped fragments currently clone horizontal edges rather
  than implementing `box-decoration-break: slice` semantics.
- Final verification passed rustfmt, warning-free workspace Clippy, all 141 workspace tests,
  normal startup smoke, the 119-command CSS demo smoke and optimized release build.
- Rebuilt `target/release/op_browser.exe`; this build is 607,232 bytes.

## 2026-10-06 - Functional pseudo-classes and background shorthand

- Added recursive functional selector AST/matching for `:is()`/`:where()`/`:not()` using the
  existing right-to-left Selector engine instead of a separate special-case matcher.
- Implemented specificity rules for the new functions: `:is()` and `:not()` contribute the
  maximum specificity of their selector arguments, while `:where()` contributes zero.
- Added `:nth-child(An+B)` parsing/matching for integers, odd/even, `n`, signed coefficients
  and offsets such as `2n+1` and `-n+4`; matching counts element siblings only.
- Fixed selector-list comma splitting so commas nested inside functional pseudos no longer
  split the outer author selector list.
- Added a color-only `background` shorthand subset supporting CSS colors, `transparent`,
  `none` and global keywords. It competes with `background-color` through the normal cascade
  key instead of declaration-type application order.
- Added parser, specificity, matcher, computed-cascade and Engine regressions. The Engine test
  combines `:is + :not + :nth-child + :where` with `background:#eef2ff` and proves selected
  inline fragments reach native display-list text/background commands.
- Updated the built-in start page and CSS demo with functional-selector/background shorthand
  examples; the CSS demo smoke now emits 133 paint commands.
- Updated README, project plan, code graph/slices and CSS/rendering wiki pages.
- Deliberate limits remain: functional selector lists are currently strict rather than
  forgiving, `:nth-child(... of selector)` is not parsed, pseudo-elements are not implemented,
  and `background` currently accepts only a color/none/global value rather than image/position/
  repeat/size/layer syntax.
- Final verification passed rustfmt, warning-free workspace Clippy, all 145 workspace tests,
  normal startup smoke, the 133-command CSS demo smoke and optimized release build.
- Rebuilt `target/release/op_browser.exe`; this build is 615,936 bytes.

## 2026-10-06 - Generated ::before / ::after content

- Added terminal `::before` and `::after` pseudo-element selector targets. They contribute
  type-level specificity and must terminate the selector instead of masquerading as DOM nodes.
- Split author style storage into host NodeId buckets and `(NodeId, PseudoElement)` buckets, so
  mixed rules such as `.note, .note::before` cascade independently for the real element and
  generated pseudo target.
- Added `ComputedPseudoStyle { style, content }`. Generated pseudos inherit host text properties,
  then apply their own author declarations through the normal importance/specificity/source-order
  cascade.
- Added the first `content` value subset: one or more quoted strings concatenate into generated
  text; `none`/`normal` suppress generation. Invalid unsupported values do not hide a lower valid
  declaration.
- Routed generated text through the existing inline formatter before/after real DOM children.
  It therefore shares wrapping, line-height, transforms, spacing, colors and native text paint.
- Pseudo inline decoration uses `(NodeId, PseudoElement)` identity, so its background/padding/
  solid border stays separate from an identically styled host fragment while reusing the normal
  BoxDecoration -> FillRect paint path.
- Generated text inside a link inherits its href and the existing hit-test path. Initial
  `display:block` support forces a line boundary; full virtual block-box geometry is deferred.
- Tightened functional pseudo parsing so pseudo-elements inside `:is()`/`:where()`/`:not()` are
  explicitly rejected rather than silently matched as the host element in the current strict
  selector-list subset.
- Added parser, author-bucket, computed-style, layout-geometry and Engine display-list regressions,
  including exact generated ordering (`before < DOM text < after`) and pseudo padding/border paint.
- Updated the built-in start page and CSS demo with visible generated-content examples; the CSS
  demo smoke now emits 141 paint commands.
- Updated README, project plan, code graph/slices and CSS/rendering/inline-layout wiki pages.
- Deliberate limits remain: `content:attr()`/counters/quotes/images are not implemented, empty
  generated strings do not yet materialize a decorated box, replaced elements do not receive
  generated pseudos, and generated `display:block` is not yet a full virtual block formatting box.
- Final verification passed rustfmt, warning-free workspace Clippy, all 150 workspace tests,
  normal startup smoke, the 141-command CSS demo smoke and optimized release build.
- Rebuilt `target/release/op_browser.exe`; this build is 626,688 bytes.

## 2026-10-06 - CSS custom properties and var()

- Added inherited, case-sensitive CSS custom properties using the existing author cascade,
  including `!important`, specificity and source-order winner selection for `--name` values.
- Added per-element and per-pseudo `CustomPropertyMap` snapshots in `ComputedStyleMap` so
  computed custom values remain available for inheritance, diagnostics and resize reflow.
- Custom property values are resolved on the element where they are computed. An inherited
  value such as `--frozen:var(--accent)` therefore keeps the parent's resolved token value even
  when a descendant overrides `--accent`.
- Added recursive `var(--name, fallback)` substitution with nested fallbacks, arbitrary token
  payloads and simple dependency-cycle invalidation before supported normal-property parsing.
- `initial` removes a custom property; `inherit` and `unset` reuse the parent's computed custom
  value in this initial subset.
- `var()` now feeds the existing parsers for colors, lengths, shorthands, borders, padding,
  sizing, font properties and generated `content` instead of requiring property-specific hooks.
- Pseudo-elements inherit the host custom-property environment and may add/override their own
  custom properties before resolving `content` and normal pseudo styles.
- Added regressions for custom-property cascade/`!important`, resolved-value inheritance,
  nested fallbacks, simple cycles, pseudo inheritance/overrides and full Engine display-list
  propagation through text/background/border paint.
- Updated the built-in start page and `examples/css` with visible inherited-variable examples;
  the CSS demo smoke now emits 149 paint commands.
- Updated README, project plan, code graph/slices and CSS/rendering wiki documentation.
- Deliberate limits remain: unresolved `var()` declarations currently reuse OPBrowser's existing
  invalid-value filtering and may expose a lower valid candidate instead of full CSS
  invalid-at-computed-value-time behavior; complete dependency-graph cycle semantics are also
  deferred.
- Final verification passed rustfmt, warning-free workspace Clippy, all 154 workspace tests,
  normal startup smoke, the 149-command CSS demo smoke and optimized release build. The first
  release attempt was blocked only because an older `target/release/op_browser.exe` process was
  still running; that workspace process was stopped and the rebuild then passed.
- Rebuilt `target/release/op_browser.exe`; this build is 655,360 bytes.

## 2026-10-06 - Generated attr() and CSS counters

- Expanded generated `content` beyond quoted strings with `attr(name)`, `counter()` and
  `counters()` function parsing. Mixed strings/functions concatenate into one final pseudo text
  payload before layout.
- `attr()` reads the originating element's DOM attributes case-insensitively for HTML names;
  a missing attribute contributes an empty string in this initial subset.
- Added initial `counter-reset`, `counter-set` and `counter-increment` parsing after custom-property
  `var()` substitution. Counter properties use the existing author cascade winner order and accept
  explicit integer values with CSS-like reset/set/increment defaults.
- Added a scoped CounterContext with per-name value stacks. Resets created by an element remain
  visible to following siblings in that parent group, descendant-created scopes are truncated
  when leaving the group, while increments to existing outer counters remain visible.
- `::before` counter/content work now runs before child traversal and `::after` runs after completed
  child traversal, so after-content can observe counter increments performed by descendants.
- Added generated counter formatting for decimal, decimal-leading-zero, lower/upper alpha/latin and
  lower/upper roman styles. `counters(name, separator)` joins all active reset-stack values.
- Added CSS regressions for attribute content, sibling/nested counter scopes, `counter-set`,
  `counters()` and formatter edge cases plus an Engine display-list regression proving attr/counter
  text reaches native painting with pseudo colors/weight intact.
- Updated the built-in start page and `examples/css` with visible counter demos; the CSS demo smoke
  now emits 168 paint commands.
- Updated README, project plan, code graph/slices and CSS/inline/rendering wiki documentation.
- Deliberate limits remain: typed/fallback `attr()` syntax, custom `@counter-style`, quote keywords,
  generated `url()` images and the remaining counter-scope edge cases are deferred. Generated
  `display:block` is still an initial line-boundary approximation rather than a full pseudo block.
- Final verification passed rustfmt, warning-free workspace Clippy, all 158 workspace tests,
  normal startup smoke, the 168-command CSS demo smoke and optimized release build. The first
  release attempt was blocked only by a running `target/release/op_browser.exe`; that workspace
  process was stopped and the rebuild then passed.
- Rebuilt `target/release/op_browser.exe`; this build is 676,864 bytes.

## 2026-10-06 - CSS quotation marks and generated-state visibility

- Added retained ComputedQuotes host/pseudo values with inherited auto/none/string-pair
  cascade and var() substitution. Auto currently uses deterministic English Unicode pairs;
  language-specific selection is deferred.
- Added generated open-quote/close-quote/no-open-quote/no-close-quote commands, shared
  document-order nesting, deepest-pair repetition and safe unmatched closing behavior.
- Added UA before/after content for HTML q elements, overridable by author content/style.
- Content candidate validation is separate from quote emission, so losing or invalid
  declarations cannot change nesting. Pseudo counter operations now require emitted content.
- Hidden subtrees, hidden/absent pseudos and unsupported img/br pseudos no longer mutate
  generated quote/counter state that can affect later visible content.
- Added four CSS regressions for inheritance/overrides, var(), cascade, nesting, none,
  silent commands, underflow and hidden-state exclusion, plus an Engine regression proving
  exact Unicode order, generated link spans/colors and stable retained resize reflow.
- Added visible nested quotation examples to the startup page and external CSS demo;
  updated README, plan, code graph/slices and CSS/inline/rendering wiki pages.
- Final verification passed rustfmt, warning-free workspace Clippy, all 163 workspace tests,
  normal startup smoke, the 175-command CSS demo smoke and optimized workspace release build.

## 2026-10-06 - Real generated block geometry

- Replaced pseudo display:block line-boundary approximation with shared BlockContent handling
  for ordinary DOM children and retained generated text, without creating synthetic DOM nodes.
- Generated blocks honor dimensions/min/max, percentage widths, box-sizing, auto/negative
  margins, padding, independent solid borders, background and normal inline wrapping/alignment.
- Empty generated block strings now create real sized/decorated boxes without invented glyphs;
  empty inline decoration and generated image content remain later work.
- Corrected ordinary/generated definite block-height geometry: overflowing text no longer
  expands the prescribed flow/background/border height. Existing parent/child and empty-block
  margin collapsing and percentage-height limitations remain explicit.
- Added three layout regressions for precise geometry, empty min/max boxes, adjacent pseudo
  margin collapse and overflowing definite heights, plus an Engine regression for FillRect,
  pseudo link spans and stable retained resize reflow.
- Updated built-in start page, external CSS demo, README, plan, graph/slices and relevant wiki.
- Final verification passed rustfmt, warning-free workspace Clippy, all 167 workspace tests,
  startup/CSS demo native paint smokes and optimized workspace release build.

## 2026-10-06 - Custom-property dependency cycles and expansion budgets

- Added op_css::custom as the owned dependency/substitution module. A directed graph includes
  all var() references, including unused fallback branches; iterative Kosaraju SCC traversal
  invalidates exactly cyclic components before dependency-order value resolution.
- Self-referencing variables cannot rescue themselves using a fallback. Noncyclic consumers
  can still recover from an invalid dependency with their own fallback. Resolved inherited
  token streams remain frozen and do not create false descendant cycles.
- Removed recursive dependency resolution, allowing 10,001-variable chains without native
  stack growth. Fallback nesting remains separately bounded at 64 levels.
- Added pre-clone expansion limits: 16,384 tokens and 256 KiB token storage per value, with
  2 MiB retained resolved custom-value storage per element/pseudo. Over-budget values become
  invalid and consumer fallbacks remain usable. No infrastructure dependency was added.
- Accepted valid empty custom values (including important empty values), retained empty-token
  substitution semantics, and rejected reserved bare -- names / empty normal declarations.
- Added graph/cycle/inheritance/empty-value regressions, long-chain/exponential/payload/total
  storage/fallback-depth tests and an Engine paint/reflow regression for recovered values.
- Updated README, project plan, graph/slices and CSS/rendering wiki documentation.
- Normal-property invalid-at-computed-value-time winner behavior remains the next separate
  cascade correction; this update does not claim complete custom-property conformance.
- Final verification passed rustfmt, warning-free workspace Clippy, all 175 workspace tests,
  startup/CSS demo native paint smokes and optimized workspace release build.

## 2026-10-06 - Invalid computed var() values preserve cascade priority

- Computed MatchedDeclaration copies now retain value_from_var priority. Missing variables,
  empty substitutions, over-budget expansion and wrong property grammar become unset instead
  of exposing older declarations; inherited/non-inherited resolution reuses existing rules.
- Applied the rule to ordinary supported properties, margin/padding/border/background and
  text-decoration shorthand components, generated content, quotes and counter operations.
  Later component declarations still override earlier invalid shorthands by normal priority.
- Validated var() syntax before author cascade, including malformed unused fallback branches.
  Literal parse-time invalid values retain their existing lower-valid-value behavior.
- Added inheritance/initial-value, important/inline/source-order, empty-value, shorthand,
  pseudo-state and parser regressions. A table-driven check compares all 52 ordinary supported
  style property names against explicit unset for missing/wrong-type/empty substitutions.
- Added Engine paint/reflow coverage and a visible external CSS demo explaining recovered
  inherited color and initial transparent background. Updated plan, README, graph/slices/wiki.
- Broader custom-property grammar, registration/animation taint and unsupported CSS properties
  remain outside this milestone; no complete CSS conformance claim is made.
- Final verification passed rustfmt, warning-free workspace Clippy, all 181 workspace tests,
  startup/CSS demo native paint smokes and optimized workspace release build.
