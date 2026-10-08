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

## 2026-10-06 - Forgiving selector lists and filtered nth sibling indexing

- Added NthSelector expression/of/from_end AST data and nth-last-child support. Optional strict
  of selector lists use the existing complex-selector matcher to filter inclusive element
  siblings before indexing, counting union matches only once and ignoring text nodes.
- Nth specificity adds one pseudo-class and the maximum filter argument specificity,
  independent of which branch matches. Negative/zero coefficients and reverse indexing work.
- Added forgiving is/where branch recovery, including unsupported/malformed/pseudo-element
  branches and empty/all-invalid lists that match nothing. Not/of/top-level lists stay strict.
- Replaced An+B whitespace concatenation with token-aware integer/n-ident/n-dimension grammar;
  malformed signs, separated coefficients, decimals, exponent forms and overflow are rejected.
- Bounded selector function nesting at 64 levels before recursive parsing/matching.
- Added parser, specificity, strict/forgiving recovery, An+B boundary and computed matching
  regressions plus Engine generated-text/native-paint/retained-reflow verification.
- Added a visible external CSS demo and updated README, plan, graph/slices and CSS/rendering wiki.
- Final verification passed rustfmt, warning-free workspace Clippy, all 186 workspace tests,
  startup smoke, the 195-command CSS demo smoke and optimized workspace release build.

## 2026-10-06 - Empty generated and ordinary inline decorations

- Added EmptyInline items to the original line formatter. Decorated empty pseudo strings and
  ordinary childless inline elements reserve horizontal padding/border edges, contribute
  font/vertical-edge line metrics, and emit BoxDecoration without fabricated TextBox glyphs.
- Empty boxes wrap as indivisible edge payloads, honor nowrap and participate in text-align.
  Their node/pseudo identities preserve separate before/after decorations.
- Undecorated empty pseudos do not introduce phantom line items; empty descendants inheriting
  an ancestor decoration do not duplicate that ancestor's edges. Whitespace-only decorated
  content and simultaneous nested decoration stacks remain later work.
- Added four layout regressions for exact edge positioning/alignment/line extents, wrapping/
  nowrap, standalone decorated lines and ordinary empty spans, plus Engine FillRect/reflow
  verification that no empty Text paint commands are introduced.
- Added a visible external CSS demo and updated README, plan, graph/slices and relevant wiki.
- Final verification passed rustfmt, warning-free workspace Clippy, all 191 workspace tests,
  startup smoke, the 213-command CSS demo smoke and optimized workspace release build.

## 2026-10-06 - Typed structural CSS selectors

- Added first-of-type, last-of-type and only-of-type matching over same-tag inclusive element
  siblings, excluding text and intervening other element types from typed positions.
- Added nth-of-type/nth-last-of-type through NthSelector same_type metadata, sharing the
  existing token-aware An+B parser, i64 indexing arithmetic and forward/reverse counting.
- All five contribute ordinary pseudo-class specificity. Typed nth functions reject of lists;
  namespace-aware matching remains outside the current HTML-only selector subset.
- Added parser/specificity/invalid-argument and mixed-sibling computed-style regressions plus
  Engine tests for pseudo text, color/weight and stable retained resize reflow.
- Added an external CSS demo and updated README, plan, graph/slices and CSS/rendering wiki.
- Final verification passed rustfmt, warning-free workspace Clippy, all 194 workspace tests,
  startup smoke, the 224-command CSS demo smoke and optimized workspace release build.

## 2026-10-06 - Computed hyperlink presentation

- Moved blue/underline link defaults into computed UA styling for anchors with href, before
  author cascade; anchors without href inherit ordinary text presentation.
- GDI now paints linked glyphs and decorations with the same computed color/flags as other
  text. LinkSpan and measured click regions retain their existing navigation behavior.
- Added UA/author/inheritance, generated/nested link text and retained reflow regressions;
  real GDI pixel tests verify red glyphs, optional underline and retained hit regions.
- Updated mixed image/link layout expectations for distinct presentation runs, preserving
  exact shared baselines, contiguous advances and linked failed-image alt text.
- Added a visible author-styled link/pseudo prefix to the CSS demo and updated documentation.
- Final verification passed rustfmt, warning-free workspace Clippy, all 197 workspace tests,
  startup/CSS demo smoke (225 paint commands), native styled-link navigation smoke and the
  optimized workspace release build.

## 2026-10-06 - CSS URL tokenization foundation

- Added owned Url/BadUrl tokens, separating unquoted URL syntax from quoted url()/String
  functions. URL payloads preserve address/data punctuation and decode CSS escapes.
- Added bounded recovery through unescaped closing parentheses, rejection of invalid URL
  characters/whitespace/newline escapes and EOF value retention with byte-offset diagnostics.
- BadUrl/BadString now invalidate complete declarations, including custom properties and
  unused var() fallbacks. URL payloads count toward existing custom-value storage limits.
- Added span/escape/data/empty/quoted/bad/EOF recovery and declaration/custom-budget tests.
  Updated syntax documentation, key-type graph, feature slice and project plan.
- Generated URL image loading and stylesheet-relative resource bases remain next steps.
- Final verification passed rustfmt, warning-free workspace Clippy, all 202 workspace tests,
  startup/CSS demo smoke (225 paint commands) and optimized workspace release build.

## 2026-10-06 - CSS resource source provenance

- MatchedDeclaration now retains the source style/link NodeId or inline-styled element;
  CollectedRule carries stylesheet provenance through normal and pseudo matching.
- External stylesheet discovery caches full LoadedStylesheet values and retains effective
  addresses after redirects per link NodeId beside text. PreparedDocument retains this map
  across reflow; Engine exposes active_stylesheet_address() for inspection/resource lookup.
- Added declaration-origin coverage across embedded/linked/inline and before/after buckets.
  A bounded loopback HTTP regression verifies redirected bases, duplicate-link reuse and
  stable resize reflow without new requests.
- Updated README, project plan, graph/slices and stylesheet/reflow wiki.
- Final verification passed rustfmt, warning-free workspace Clippy, all 204 workspace tests,
  startup/CSS demo smoke (225 paint commands) and optimized workspace release build.

## 2026-10-06 - Generated CSS URL image content

- Added ordered GeneratedContentItem Text/Image(url,style_node) lists beside pseudo textual
  inspection values. Quoted/unquoted URLs mix with strings, quotes, attrs and counters;
  invalid quoted URL grammar is excluded from cascade candidates.
- PageImages keeps DOM resources by NodeId and generated resources by host/pseudo/item index.
  One worker loader shares source policy, successful/failed cache and all candidate/request/
  encoded/pixel/time bounds. Computed display:none subtrees/pseudos do not load resources.
- CSS image bases use the consuming declaration's effective stylesheet address after
  redirects; embedded/inline sources use the document. var() URLs resolve at consumption.
- Generated images use intrinsic baseline/atomic wrapping/width fitting and block content,
  inherit anchor href for native clicks and preserve Arc pixels across reflow. Missing
  resources add no inline image while surrounding content remains.
- Added parser/computed ordering/provenance, layout geometry and source integration tests
  covering redirected CSS, local consumer bases, mixed DOM/generated cache/budgets, hidden/
  blocked/failed URLs, native image links and reflow after deleting source files.
- Added an original local CSS image fixture and updated plan, graph/slices, README and wiki.
  Full replaced sizing/CSS image sizing, gradients/modifiers and content alt syntax remain later.
- Final verification passed rustfmt, warning-free workspace Clippy, all 209 workspace tests,
  startup/CSS smoke (225 commands), generated-image smoke (20 commands), native generated
  image-only link navigation and optimized workspace release build.

## 2026-10-06 - Decorated DOM image inline boxes

- Image formatter items retain InlineStyle and their own DOM image InlineBoxStyle.
  Padding/solid border edges participate in atomic wrapping, text alignment and width fitting.
- Image border-box bottoms align to the text baseline; vertical edges expand safe line ascent.
  Precise background/border BoxDecorations paint before the raster with matching offsets.
- Image items now honor nowrap/pre soft-wrap suppression instead of always wrapping.
  Generated anonymous images retain intrinsic geometry without duplicating pseudo/ancestor boxes.
- Added exact edge/baseline/order/wrap/nowrap/fit regressions and an Engine paint/link/reflow
  test. Added a decorated image to the CSS fixture and updated plan, graph/slices, README/wiki.
- Native image click regions still cover raster pixels; generated replaced sizing/decorations,
  nested ancestor box stacks and CSS image dimensions remain next steps.
- Final verification passed rustfmt, warning-free workspace Clippy, all 212 workspace tests,
  startup smoke, decorated/generated image smoke (25 commands), native image-link navigation
  and optimized workspace release build.

## 2026-10-06 - Computed CSS DOM image sizing

- Moved bounded HTML image width/height attributes into computed pre-author hints. Author
  CSS, auto/global keywords and invalid computed var() winners override/reset those hints.
- DOM image content dimensions now resolve CSS percentage widths, font-relative lengths,
  definite heights and content-vs-border-box sizes before decorated inline placement.
- Added op_layout::replaced intrinsic/explicit/auto ratio sizing and min/max constraints.
  Compatible both-auto limits preserve ratio; conflicts/explicit sides can stretch. Minimum
  constraints win over smaller maxima. The existing viewport/4096-height fitting policy
  runs afterward and can shrink below CSS minima; percentage heights remain auto-like.
- Added source-cascade regressions, a 12-case actual-layout size table and pure geometry
  regressions for intrinsic/explicit/zero sizes and every min/max ratio conflict direction.
- Updated the Engine paint/link/reflow test to prove CSS border-box sizes override HTML hints.
  Updated the CSS image fixture, README, plan, module graph, slices and box/image/inline wiki.
- Final verification passed rustfmt, warning-free workspace Clippy, all 217 workspace tests,
  startup smoke, generated/decorated image smoke (25 commands), ordinary mixed-image smoke
  (19 commands), native generated image-link navigation and optimized workspace release build.

## 2026-10-06 - Sole-URL inline generated image replacement

- ComputedPseudoStyle.replaced_image marks exactly one parsed image URL before empty text/
  quote materialization. Empty-string/quote/image lists keep anonymous content-list semantics.
- Shared flow::resolve_image_size now resolves DOM images and inline image replacements:
  CSS sizes/min/max/box-sizing, intrinsic ratio constraints and viewport/draw-height fitting.
- Sole-image inline pseudos use their own host/pseudo box identity for padding/background/
  borders and atomic wrapping/baseline geometry without duplicating inherited decoration.
- Added replacement-classification, exact layout/decoration and Engine paint/link/reflow tests.
  Enlarged the image-only link fixture with CSS and updated README, plan, graph/slices/wiki.
- Block replaced geometry, unavailable replacement boxes and nested decorated stacks remain
  later work; mixed content images keep intrinsic sizing and native hit regions cover raster pixels.
- Final verification passed rustfmt, warning-free workspace Clippy, all 220 workspace tests,
  startup smoke, CSS-sized generated image smoke (30 commands), native image-only link
  navigation and optimized workspace release build.

## 2026-10-06 - Shared replaced block image geometry

- Added Context::block_image for DOM img and sole-URL generated block replacements.
  Auto width follows intrinsic raster size; CSS dimensions/min/max/box-sizing share sizing.
- Horizontal auto margins position the exact image border box. Percentage widths retain
  containing width as their basis while available-width fitting reserves specified margins.
- Padding/borders/background use exact block bounds; following flow advances by border-box
  height without anonymous text-line leading. Adjacent vertical margins use existing collapse.
- Added exact intrinsic/percentage/border-box/auto-margin/baseline-free block coordinates and
  generated block/link tests. Updated Engine paint/reflow coverage and the centered CSS fixture.
- Updated README, plan, graph/slices and image/inline wiki. Missing DOM block images retain
  alt-line fallback; unavailable replacement boxes and nested decorated stacks remain later.
- Final verification passed rustfmt, warning-free workspace Clippy, all 222 workspace tests,
  startup smoke, centered/decorated generated image smoke (30 commands), native image-only
  link navigation and optimized workspace release build.

## 2026-10-06 - Nested inline decoration stacks

- Replaced the single copied inline box style with Context-owned InlineBoxes parent-linked
  arena nodes. Characters carry one optional stack index; cumulative edges/depth are cached
  per owned box, avoiding copies of all ancestors per character.
- Text, images, empty DOM/pseudo boxes and generated text/image lists share iterative
  common-ancestor transitions. All ancestor edges contribute to wrapping, alignment and
  safe vertical extents; image own boxes remain atomic within continuous ancestor fragments.
- Image fitting reserves ancestor edges while CSS percentage sizes retain containing width
  as their basis. Outer fragment decorations are allocated before descendants and completed
  on close, preserving nested opaque background/border paint order.
- Added exact nested text/image/empty/pseudo geometry, repeated wrapped edges, percentage
  fitting, 128 decorated levels, Engine paint-order/link/reflow regressions and native demos.
- Updated README, plan, graph, slices and inline/box/image wiki. Fragment edges still clone;
  full CSS vertical positioning and sliced decoration behavior remain later work.
- Final verification passed rustfmt, warning-free workspace Clippy, all 229 workspace tests,
  startup smoke, nested/generated image smoke (56 commands), native image-only link navigation
  and optimized workspace release build.

## 2026-10-06 - Unavailable replacement and styled alt geometry

- Added private InlineImage used dimensions with optional Arc pixels. Missing sole-URL CSS
  replacements and DOM images with empty/absent alt use zero natural dimensions and shared
  inline/block box placement, without manufacturing raster buffers or image hit regions.
- Replaced sizing resolves axes independently when a natural dimension is zero. CSS size/
  min/max/box-sizing/padding/borders still apply; width/height fitting preserves zero axes.
  Atomic wrapping, nowrap, nested ancestor fragments, auto margins and exact block height
  work for transparent replacements through the same paths as decoded images.
- Nonempty DOM alt keeps its own inline box styling or uses BlockContent::ImageAlt normal
  block geometry. Mixed generated failures skip anonymous images while retaining text and
  empty pseudo decorations. Removed the fabricated [image] label for absent alt.
- Added zero-natural-axis sizing, exact missing inline/block/alt/empty/mixed coordinates,
  atomic wrap/nowrap, Engine no-raster paint/reflow regressions and failed/blocked HTTP cache
  assertions. Updated fixtures, README, plan, graph/slices and inline/box/image wiki.
- Consulted primary CSS Generated Content and HTML rendering rules. Full loading-state and
  quirks-mode HTML fallback semantics remain later work; native image hit regions require pixels.
- Final verification passed rustfmt, warning-free workspace Clippy, all 236 workspace tests,
  startup smoke, failed/styled/nested generated image smoke (79 commands), native image-link
  navigation and optimized workspace release build.

## 2026-10-06 - HWB computed colors and bounded hue conversion

- Added modern hwb() colors with hue units, percentage/number whiteness/blackness, optional
  alpha and none components resolved to zero for current used-color painting. W+B >= 100%
  normalizes to gray; components above 100% retain their relative contribution.
- Extracted unquantized HSL channels for shared conversion and rounds only final sRGB bytes.
  Hue unit normalization now uses wider intermediates and reduces turns before scaling,
  preventing overflow/NaN from large finite HSL/HWB angle values.
- Added primary CSS Color 4/WPT expected-color cases, malformed syntax/cascade recovery,
  custom-property substitution, text/background/border/pseudo display-list and retained
  reflow tests. Updated the CSS demo, README, plan, graph/slices and CSS syntax wiki.
- Missing-component interpolation/serialization, relative colors and color calc() remain
  later work. Current RGBA paint precision and white-page alpha composition are unchanged.
- Final verification passed rustfmt, warning-free workspace Clippy, all 239 workspace tests,
  native startup/CSS demo smoke (244 commands) and optimized workspace release build.

## 2026-10-06 - Complete compact CSS named-color table

- Replaced the basic named-color match with all 148 opaque CSS named colors and aliases,
  imported from the pinned 2026-09-30 W3C CSS Color 4 table. Import checks hexadecimal/
  decimal column agreement, 139 distinct RGB values, aliases and input SHA-256.
- Added op_css::named with packed names, six-byte offset/length/RGB records and allocation-
  free ASCII case-insensitive binary search. Static table storage is 2,210 bytes; unknown/
  oversized/non-ASCII names fail without lowercase-string allocation. Transparent and
  currentcolor remain special computed keywords outside the opaque table.
- Checked in named-colors.tsv and a Python standard-library offline generator/checker;
  normal Cargo builds require neither Python nor network access. Added generator check to CI.
- Exhaustive tests verify all names/case variants/source values and table storage. Added
  cascade/inheritance/var()/escaped-pseudo/currentcolor-border and Engine paint/reflow tests.
  Updated the CSS demo, README, plan, graph/slices and developer/CSS workflow documentation.
- Final verification passed rustfmt, offline generated-table check, warning-free workspace
  Clippy, all 243 workspace tests, native startup/CSS demo smoke (250 commands) and optimized
  workspace release build.

## 2026-10-06 - Predefined sRGB and linear-light color functions

- Added color(srgb ...) and color(srgb-linear ...) with number/percentage channels, slash
  alpha and used-value none components. Extracted shared modern component/alpha parsing
  from HWB; malformed argument counts, commas and unsupported spaces are rejected.
- Linear-light channels use the sRGB transfer curve with wider intermediates before final
  8-bit CssColor encoding. Initial channel clipping, white-page alpha painting and lack of
  retained color-space/missing-component metadata remain explicit limitations.
- Added independent reference/transfer-boundary colors, finite extreme/clipped channels,
  invalid-value cascade recovery, var() substitution and native text/background/border/
  generated paint plus retained reflow tests. Updated CSS demo, README, plan, graph/slices/wiki.
- Wide-gamut spaces, perceptual gamut mapping, higher-precision interpolation/serialization,
  calc() and relative colors remain later work; no runtime dependency or raster change added.
- Final verification passed rustfmt, warning-free workspace Clippy, all 246 workspace tests,
  named-color generator check, native startup/CSS demo smoke (256 commands) and optimized
  workspace release build.

## 2026-10-06 - Modern RGB/HSL components and strict legacy color grammar

- Modern RGB/HSL now share component/slash-alpha parsing with HWB/color(). Missing channels/
  alpha resolve to zero for current painting; modern RGB permits number/percentage mixing.
  Numeric HSL saturation/lightness uses the 0..100 percent reference scale and clamping.
- Legacy comma RGB now requires uniform numeric or percentage color channels. Comma HSL
  retains percentage-only saturation/lightness; legacy forms reject none and slash mixing.
- Added independent channel/alpha/number/clamping cases and cascade regressions proving
  invalid literal fallback versus invalid var() winner unset inheritance. Native display-list
  tests cover modern text/background/border/pseudo colors and retained reflow.
- Updated CSS demo, README, plan, graph/slices and syntax wiki. Missing-component metadata
  preservation and higher-precision interpolation/serialization remain later work.
- Final verification passed rustfmt, warning-free workspace Clippy, all 249 workspace tests,
  native startup/CSS demo smoke (262 commands) and optimized workspace release build.

## 2026-10-06 - Empty inline frames around invisible descendants

- Flow now synthesizes an own EmptyInline frame after collecting hidden/empty descendants
  or only collapsed whitespace. The old childless-only check dropped visible padding/borders
  around such hosts. Nested empty frames share arena ancestry without duplicate parent items.
- Added a block epoch guard so child/pseudo block boundaries, including zero-height blocks,
  do not create an extra synthetic inline line. Preformatted spaces remain real text.
- Added centered exact edge/height/position regressions, nested/no-duplicate geometry,
  block-boundary/preformatted cases and Engine background/hidden-text/reflow coverage.
- Updated plan, graph/slices, inline wiki and CSS demo. The added fixture exposed separately
  that HTML comments are currently rendered as literal text; declaration-token work follows.
- Final verification passed rustfmt, warning-free workspace Clippy, all 252 workspace tests,
  native startup/CSS demo smoke (268 commands) and optimized workspace release build.

## 2026-10-06 - Owned HTML comment states and invisible comment rendering

- Added an iterative private comment state machine and typed Comment tokens. Previously
  comments became literal visible text, including fixture descriptions. The tree builder
  now discards comment tokens without changing text buffering or open elements.
- Covered normal/abrupt/--!> closing, pending punctuation at EOF, nested markers, literal
  references/markup and NUL replacement; raw-text/RCDATA and attributes retain markers.
- Added DOM and Engine regressions comparing comment-free paint lists, CSS :empty geometry
  and retained reflow. Updated plan, graph/slices and HTML wiki. DOM comment storage,
  doctype/document modes and other declarations are explicit remaining parser work.
- Final verification passed rustfmt, warning-free workspace Clippy, all 257 workspace tests,
  native startup/CSS demo smoke (268 commands) and optimized workspace release build.

## 2026-10-06 - Typed doctype and bogus markup declarations

- Added an owned iterative doctype tokenizer carrying lowercase name, optional PUBLIC/
  SYSTEM identifiers and force_quirks. Missing/empty values remain distinct; malformed
  identifiers, trailing junk and EOF recover by the specified token-data states.
- Tree construction discards doctype tokens, removing previously visible doctype text.
  Unknown <! declarations/HTML CDATA-like declarations now produce invisible bogus
  comments with NUL replacement. Raw-text/RCDATA and attribute markers remain literal.
- Added token-value/recovery/context tests and Engine exact declaration-free rendering/
  retained-reflow comparisons. Updated plan, graph/slices and HTML wiki. Document-mode
  selection, doctype DOM nodes, processing instructions and foreign-content context remain
  later work; token force_quirks does not yet change layout.

## 2026-10-06 - DOM comment and DocumentType nodes

- Extended op_dom with Comment and DocumentType nodes plus DocumentTypeData retaining the
  tokenizer name, PUBLIC/SYSTEM identifiers and force_quirks value without coupling op_dom
  back to op_html.
- The tree builder now stores comments at their actual document position, flushing adjacent
  text around them without changing the open-element stack. It stores the first doctype
  before the document element and ignores later/in-element doctypes in this initial phase.
- CSS :empty and descendant-text extraction explicitly ignore comment/doctype payloads;
  layout emits no items for either node kind. Existing Engine tests still prove comments,
  doctypes and bogus declarations produce identical pixels and retained reflow.
- Added DOM/tree-builder coverage for node data, ordering, text splitting and late-doctype
  rejection. Updated project plan, code graph/slices and HTML wiki.
- Verification passed rustfmt, warning-free workspace Clippy, the complete workspace test
  suite, native startup smoke and optimized workspace release build. One pre-existing
  loopback generated-image test transiently hit WSAEWOULDBLOCK on the first full run, then
  passed standalone and again in the complete suite.

## 2026-10-06 - WHATWG document mode selection

- Added op_dom::DocumentMode with NoQuirks, LimitedQuirks and Quirks state stored directly
  on each parsed Document. Manually constructed documents retain a no-quirks default while
  HTML parsing explicitly selects the mode during its initial insertion phase.
- Added an owned doctype classifier covering the full WHATWG legacy compatibility matrix:
  exact public/system identifiers, all 55 quirks public prefixes, HTML 4.01 system-ID
  distinctions and XHTML 1.0 limited-quirks prefixes, all with ASCII-insensitive matching.
- The initial tree-building phase now ignores leading ASCII whitespace, preserves comments,
  accepts one correctly placed doctype, selects quirks when the doctype is missing/malformed
  or appears too late, and ignores subsequent doctypes. The internal start page now carries
  <!doctype html> so it remains explicitly in no-quirks mode.
- Added exhaustive classifier tests plus parser-level coverage for no-quirks, limited-quirks,
  missing/late doctypes, malformed declarations, case-insensitive legacy identifiers and
  initial whitespace/comment ordering. Updated plan, graph/slices and HTML wiki.
- Document mode is parser/DOM state in this iteration; mode-specific legacy CSS/layout
  behavior remains explicit future work rather than silently changing rendering now.
- Final verification passed rustfmt, warning-free workspace Clippy, the complete workspace
  test suite, native startup smoke and optimized workspace release build.

## 2026-10-06 - HTML structural insertion modes

- Replaced the previous generic tree-builder pass with explicit Initial, BeforeHtml,
  BeforeHead, InHead, AfterHead, Text and InBody modes, following the first structural
  portion of WHATWG tree construction instead of treating HTML like permissive XML.
- Missing html/head/body elements are now synthesized and explicit elements retain their
  attributes. Duplicate html/body start tags merge only attributes not already present,
  using a new mutable element accessor in op_dom.
- Head processing now keeps base/link/meta and raw/RCDATA title/style/script/noframes
  content in the head, including permitted head-only tokens encountered after </head>.
  The dedicated Text mode fixes non-whitespace raw text previously escaping into body.
- Ordinary non-void HTML elements no longer close just because their start tag carries a
  self-closing slash; void elements still remain non-pushing.
- Updated tree-builder tests for the normalized html/head/body structure and added coverage
  for omitted structural tags, insertion-mode comment placement, misplaced head metadata,
  duplicate attribute merging and non-void self-closing recovery.
- The remaining table/template/frameset/after-body insertion modes, active formatting
  elements/adoption agency, foster parenting, foreign content and script escape states
  remain explicit follow-up work.

## 2026-10-06 - Scope-aware in-body HTML recovery

- Expanded InBody from generic stack insertion/removal to WHATWG-style normal, list-item
  and button scope checks plus shared implied-end-tag generation.
- Block-level starts now close an open paragraph in button scope. Repeated p, li, dd and dt
  structures recover into sibling DOM nodes instead of nesting indefinitely; stray </p>
  synthesizes and immediately closes the required empty paragraph.
- Heading starts close paragraphs and replace a current heading; heading end tags close the
  heading in scope even when the literal h1-h6 name is mismatched, matching HTML recovery.
- Nested button starts close the button already in scope. Generic end tags now stop at
  special-element boundaries instead of tunneling through unrelated structural elements.
- Added in-body recovery for </br> -> <br>, the legacy <image> -> <img> alias and U+0000
  character suppression. Supported head-only tokens encountered after body parsing begins
  are attached to the stored head while preserving the existing body open-element stack.
- Expanded tree-builder coverage from 10 to 17 tests for paragraphs, lists, description
  lists, headings, buttons, generic end-tag boundaries, misplaced head tokens and legacy
  recovery cases.
- Verified the exact rule families against the current WHATWG tree-construction sections.
  Active formatting elements/adoption agency and table/foster-parenting remain separate
  follow-up milestones.
- Final verification passed rustfmt, warning-free workspace Clippy, the complete workspace
  test suite, native startup smoke and optimized workspace release build.

## 2026-10-06 - After-body HTML insertion modes

- Added explicit AfterBody and AfterAfterBody tree-construction modes so document endings no
  longer fall through the generic end-tag path.
- Corrected </body> and </html> handling to switch insertion mode without popping the body/html
  recovery stack; </html> is reprocessed through after-body as required by tree construction.
- After-body comments now attach to the html element and after-after-body comments attach to
  Document. ASCII whitespace and html start tags delegate through in-body rules, while
  unexpected trailing content switches back to in-body and is reprocessed for recovery.
- Added two focused tree-builder regressions covering comment placement, duplicate html
  attribute merging, whitespace after body/html and trailing-content recovery. The
  tree-builder integration suite now has 19 tests.
- Updated the project plan, code graph, code slices and HTML parsing wiki. Active formatting
  elements/adoption agency and table/template/frameset/foster-parenting remain separate work.
- Final verification passed rustfmt, warning-free workspace Clippy, all 287 workspace tests,
  native startup smoke and optimized release build. The release executable is 802,304 bytes.
  One pre-existing generated-image loopback test transiently hit WSAEWOULDBLOCK on the first
  full run; it passed standalone immediately afterward and the complete workspace retry passed.

## 2026-10-06 - Active formatting elements and adoption agency

- Added a separate active-formatting list retaining element NodeId, tag name and original
  attributes for a/b/big/code/em/font/i/nobr/s/small/strike/strong/tt/u.
- Formatting entries now reconstruct when they have fallen off the open-element stack.
  Matching entries are bounded by the Noah's Ark three-entry rule, including an internal
  regression for the cap.
- Formatting end tags now run a bounded eight-pass adoption-agency algorithm. It handles the
  simple no-furthest-block path plus the furthest-block path with cloned formatting nodes,
  stack/list replacement and real DOM reparenting.
- Repeated a/nobr starts use formatting recovery. applet/marquee/object create marker
  boundaries and matching end tags clear formatting back to the marker.
- Added four tree-builder regressions for crossed b/i formatting, furthest-block paragraph
  adoption, duplicate anchors and object marker isolation. Added an Engine regression proving
  recovered b/i structure reaches native display-list bold/italic runs.
- Updated project plan, code graph/slices and the HTML parsing wiki. Table/template/frameset
  modes, foster parenting, foreign content and mode-specific quirks remain separate work.
- Final verification passed rustfmt, warning-free workspace Clippy, all 293 workspace tests,
  native startup smoke and optimized release build. The release executable is 813,056 bytes.
  The pre-existing generated-image loopback test again transiently hit WSAEWOULDBLOCK on the
  first full run; it passed standalone and the complete workspace retry passed.

## 2026-10-06 - HTML table tree construction and foster parenting

- Added explicit InTable, InTableText, InCaption, InColumnGroup, InTableBody, InRow and
  InCell insertion modes with table-scope stack cleanup and insertion-mode reset.
- Added implicit tbody and tr recovery so rows/cells in malformed table markup are placed in
  the DOM structure required by HTML tree construction; conflicting table tokens close rows,
  cells and sections through the corresponding table-scope rules.
- Added pending table-character buffering: all-whitespace runs remain in table context while
  non-whitespace runs are reprocessed with foster parenting.
- Added foster-parent insertion before the last open table for misnested table text/elements.
  op_dom::Document now exposes insert_before, preserving sibling order while reparenting nodes.
- Cell entry creates an active-formatting marker and cell closure clears back to that marker,
  preventing formatting from leaking into following row/table content.
- Generalized head-token routing so style/script-like text tokens encountered from table mode
  return to their original table insertion mode rather than incorrectly falling back to InBody.
- Added seven table tree-builder regressions, one DOM sibling-insertion regression and one
  Engine regression proving fostered text/cell text reach the display-list text stream in DOM
  order. The tree-builder integration suite now has 30 tests.
- Updated project plan, code graph/slices and the HTML parsing wiki. A real CSS table
  formatting/layout context, template/frameset modes and foreign-content parsing remain later.
- Final verification passed rustfmt, warning-free workspace Clippy, all 302 workspace tests,
  native startup smoke and optimized release build. The release executable is 824,832 bytes.
  The full verification passed without the pre-existing generated-image socket flake.

## 2026-10-06 - Initial CSS table formatting context

- Expanded computed display values with table, table-caption, table-column-group/table-column,
  table-header-group/table-row-group/table-footer-group, table-row and table-cell roles.
  Native HTML table elements receive those UA roles; table uses border-box sizing, caption is
  centered, td/th receive 1px UA padding and th is bold. Author display table-role keywords use
  the ordinary cascade.
- Added an initial dedicated table formatter in op_layout instead of routing table structure
  through generic vertical block flow. Rows are collected across direct rows and
  thead/tbody/tfoot groups into a TableGrid.
- The grid tracks occupied columns across rowspan, supports colspan up to the bounded HTML
  range, treats rowspan=0 as the remaining rows, and counts col/colgroup span declarations when
  establishing the column grid.
- Table columns currently share the used table content width evenly with the 2px UA spacing.
  Captions flow above the grid; cells in the same row share a row top and cell contents reuse
  the existing block/inline formatter inside their own content rectangles.
- Table and cell backgrounds/borders/padding now emit ordinary BoxDecoration geometry.
  Colspan combines multiple column tracks and internal spacing; rowspan decoration extends over
  multiple row heights. Existing text/image paint therefore consumes table geometry without a
  special platform drawing path.
- Added one computed-style regression, two deterministic layout regressions covering real
  row/column placement plus colspan/rowspan geometry, and one Engine regression proving table
  coordinates and th boldness reach the display list.
- This is intentionally the initial table formatting context, not full CSS Tables conformance.
  Content-driven intrinsic/min/max column sizing, the border-spacing property, border-collapse,
  col/colgroup sizing hints, vertical-align, anonymous table boxes and inline-table remain later.
- Updated project plan, code graph/slices and CSS/rendering/HTML wiki documentation.
- Final verification passed rustfmt, warning-free workspace Clippy, all 306 workspace tests,
  native startup smoke and optimized release build. The release executable is 837,120 bytes.
  The full verification passed without the generated-image socket flake.

## 2026-10-06 - Disable remote GitHub Actions CI

- Removed .github/workflows/ci.yml so pushes and pull requests no longer create GitHub Actions
  runs. The remote account billing state prevented the Windows runner from starting, so the
  workflow produced failure notifications without executing any project checks.
- Kept the project completion gate local: rustfmt, warning-free Clippy and the complete workspace
  test suite remain mandatory, with targeted native smoke/release checks when the changed area
  requires them.
- Updated the development-workflow wiki to remove statements that CI runs generated-data and
  image/navigation smoke checks; those checks are now documented as local developer commands.
- Repository verification after removing the workflow passed rustfmt, warning-free workspace
  Clippy and all 306 workspace tests.

## 2026-10-06 - Intrinsic table tracks, spacing and collapsed cell borders

- Added inherited computed border-spacing and border-collapse properties. border-spacing accepts
  one or two nonnegative supported lengths; border-collapse accepts separate/collapse. Native
  tables retain the 2px/2px separate-border UA default while CSS initial remains zero spacing.
- Replaced equal table-column splitting with bounded content-driven min/max track preferences.
  Cell text uses the active TextMeasurer plus transform/letter/word spacing; loaded images,
  fallback image widths, padding/borders, width/min/max constraints and box-sizing contribute
  to cell intrinsic widths.
- Single-column cells establish direct track preferences. Colspan cells distribute unmet min/max
  requirements across their occupied tracks, and col/colgroup width hints participate before
  final available-width distribution.
- Authored horizontal/vertical border-spacing now controls real column and row gaps. In collapsed
  mode spacing becomes zero.
- Added initial collapsed cell-border conflict resolution on per-grid-boundary segments. Adjacent
  cell sides compete by used width; one winning edge is assigned to one side of the shared
  boundary so ordinary BoxDecoration paint does not double internal borders. Table/row-group/
  row/column border precedence remains a later CSS Tables layer.
- Added CSS cascade/inheritance coverage, deterministic layout regressions for intrinsic track
  sizing, col hints, horizontal/vertical spacing and collapsed border conflicts, plus an Engine
  regression proving content-driven track widths and spacing reach FillRect display-list geometry.
- Updated project plan, code graph/slices and CSS/rendering/HTML/box-model wiki documentation.
- Final verification passed rustfmt, warning-free workspace Clippy, all 312 workspace tests,
  native startup smoke and optimized release build. The release executable is 858,624 bytes.

## 2026-10-06 - Table-cell vertical alignment

- Added computed vertical-align support for the table-relevant baseline/top/middle/bottom keywords.
  The property is non-inherited by default; explicit inherit still uses the parent value, while
  initial/unset return to baseline.
- Table cell layout now records the text/image/decoration ranges emitted by each cell. Once row and
  rowspan heights are known, top/middle/bottom alignment shifts the whole nested content together
  without moving the cell border box.
- Baseline-aligned cells measure their first laid-out text line (or first image/fallback content
  edge) and participate in per-row baseline geometry. Cells with different font sizes therefore
  share one row baseline instead of merely being pinned to the top.
- Vertical free space accounts for cell padding/borders and actual content height, including
  explicit cell height constraints and final row/span expansion.
- Added CSS cascade/non-inheritance coverage, deterministic layout tests for top/middle/bottom,
  nested-decoration movement and mixed-font baseline alignment, plus an Engine regression proving
  the resulting Y coordinates reach the display list.
- The current vertical-align slice intentionally does not implement inline sub/super/text-top/
  text-bottom or length/percentage offsets yet.
- Updated project plan, code graph/slices and CSS/rendering/HTML/box-model wiki documentation.
- Final verification passed rustfmt, warning-free workspace Clippy, all 316 workspace tests,
  native startup smoke and optimized release build. The release executable is 862,208 bytes.

## 2026-10-06 - Initial anonymous table child fixup

- Reworked the table grid input model so a cell source can be either a real DOM element or a
  layout-only anonymous cell containing one or more existing DOM nodes. Rows are likewise gathered
  as layout sources rather than requiring every row to correspond to a DOM tr/table-row node.
- Added the initial child-side CSS table fixup inside an existing table formatting context:
  consecutive improper table children become one anonymous row; non-row children of a row group
  become anonymous rows; consecutive non-cell children of a row become one anonymous cell.
- Whitespace-only HTML text, comments, doctypes and display:none children are ignored by fixup so
  normally indented HTML tables do not acquire phantom rows or cells.
- Anonymous cells inherit the containing table/row inline text presentation while non-inherited
  box properties start from their initial values. No synthetic nodes are inserted into op_dom.
- Intrinsic width measurement, images, block/inline content collection, row sizing, baseline/
  vertical alignment and final paint output now consume real and anonymous cell sources through
  the same table-grid path. Authored collapsed borders still come only from real cells.
- Added two layout regressions for missing row/cell wrappers and one Engine regression proving
  anonymous-cell inheritance plus repaired real-cell geometry reach the display list.
- Missing-parent fixup for orphan table-row/table-cell boxes outside a table formatting context,
  remaining anonymous table objects and true inline-table remain later work.
- Updated project plan, code graph/slices and CSS/rendering/box-model wiki documentation.
- Final verification passed rustfmt, warning-free workspace Clippy, all 319 workspace tests,
  native startup smoke and optimized release build. The release executable is 866,304 bytes.

## 2026-10-06 - Anonymous table missing-parent repair

- Generalized the table formatter so real display:table elements and layout-only anonymous table
  wrappers share one table_box path for outer geometry, captions, intrinsic tracks, row/column
  placement, spacing, collapsed borders and cell vertical alignment.
- Normal-flow child collection now recognizes consecutive orphan table-internal siblings and groups
  them under one anonymous block table instead of laying each table-row/table-cell as an unrelated
  ordinary block.
- Repair-transparent whitespace/comments/display:none nodes between orphan table-internal siblings
  do not split the run, while trailing whitespace before ordinary content remains available to the
  normal flow collector.
- Consecutive orphan table-cell siblings therefore receive one anonymous row, consecutive orphan
  table-row siblings become rows of one anonymous table, and ordinary children of an orphan row
  still receive the child-side anonymous-cell repair from the previous milestone.
- Anonymous wrappers preserve inherited text presentation and reuse the existing table grid without
  adding synthetic DOM nodes or creating a second reduced table-layout implementation.
- Added three layout regressions for orphan cells, orphan rows and nested anonymous-cell repair, plus
  an Engine regression proving the repaired sibling group reaches display-list geometry.
- The initial core row/cell anonymous-table fixup is now present in both directions. True
  inline-table and remaining anonymous column/caption edge cases remain later work.
- Updated project plan, code graph/slices and CSS/rendering/box-model wiki documentation.
- The first full verification exposed only a Clippy too-many-arguments warning in the new sibling
  collector; its containing geometry was folded into one pair and verification was rerun.
- Final verification passed rustfmt, warning-free workspace Clippy, all 323 workspace tests,
  native startup smoke and optimized release build. The release executable is 869,888 bytes.

## 2026-10-06 - Atomic inline-table formatting

- Added display:inline-table to computed CSS display parsing as a distinct inline table role rather
  than aliasing it to block display:table.
- Added inline::InlineAtomic, a reusable atomic inline formatting payload carrying width, height,
  baseline, nested decorations, text boxes, image boxes and retained paint order.
- Extended Lines so atomic objects participate in whitespace handling and wrapping as one unit,
  contribute ascent/descent through an explicit baseline, and translate/remap their nested output
  into the final parent line without flattening it into fake text or raster placeholders.
- Added flow::inline_table_atomic. It reuses the normal table_box/grid formatter in a local Context,
  preserving the existing captions, rows, cells, colspan/rowspan, intrinsic tracks, spacing,
  collapsed borders and cell vertical alignment inside the atomic object.
- Auto-width inline tables now use an initial shrink-to-fit calculation from table min/max intrinsic
  tracks instead of expanding to the full containing block. Authored padding, borders, box-sizing
  and margins contribute to the final atomic dimensions.
- table_box now reports its border-box size and first-row baseline. Inline tables use that first row
  baseline against surrounding text, falling back to the table bottom when no row baseline exists.
- Nested text/images and retained LayoutItem order survive the local table context. An outer anchor
  around an inline-table is propagated to nested text/images that do not already carry link identity.
- Extended anonymous table coverage beyond row/cell repair: orphan table-caption participates in the
  same repaired wrapper and orphan table-column width hints still influence repaired tracks.
- Added four inline-table layout regressions for inline placement/baseline, atomic wrapping/linkage,
  nested image transfer and shrink-to-fit auto width; added box-model/margin coverage, two orphan
  caption/column repair regressions, and one Engine display-list regression.
- Updated project plan, code graph/slices and CSS/rendering/box-model wiki documentation. Remaining
  work explicitly includes full CSS Tables percentage/fixed algorithms, non-cell collapsed-border
  precedence, deeper colgroup/caption-side repair and non-baseline atomic vertical-align behavior.
- The first full workspace run hit the known Windows loopback WSAEWOULDBLOCK (10035) transient in
  redirected_css_images_share_cache_and_skip_hidden_blocked_and_failed_sources. The exact test then
  passed alone and the full workspace suite passed on retry.
- Final verification passed rustfmt, warning-free workspace Clippy, all 331 workspace tests,
  native startup smoke and optimized release build. The release executable is 880,128 bytes.

## 2026-10-06 - Fixed/percentage table sizing and caption-side

- Added computed CSS support for table-layout:auto/fixed and inherited caption-side:top/bottom,
  including global-keyword behavior, public exports and invalid-value cascade coverage.
- Refactored table captions into wrapper flow: top captions are laid out before the table border box
  and bottom captions after it, so table backgrounds/borders no longer incorrectly contain caption
  geometry. Inline-table baseline metrics account for a preceding top caption.
- Extended auto table intrinsic tracks with percentage constraints sourced from col/colgroup and
  cell widths. Percentage tracks reserve their used-width share before remaining width is expanded
  into auto tracks, while measured content min/max constraints still participate.
- Added initial table-layout:fixed for explicit-width tables. Track widths are selected from
  col/colgroup hints first, then explicit first-row cell widths (including percentage widths and
  colspan distribution), then unresolved tracks share the remaining width. Later-row content does
  not renegotiate fixed track geometry.
- Kept fixed sizing bounded to the existing table content box and added deterministic overconstraint
  squeezing; full CSS Tables min-width/overflow/overconstraint edge behavior remains later work.
- Added four layout regressions for bottom captions, auto percentage columns, fixed col hints/late
  content independence and first-row percentage widths, plus one Engine display-list regression
  proving the new CSS properties survive cascade -> layout -> paint.
- Updated project plan, code graph/slices and CSS/rendering/box-model wiki documentation.
- Final verification passed rustfmt, warning-free workspace Clippy, all 337 workspace tests,
  native startup smoke and optimized release build. The release executable is 890,880 bytes.

## 2026-10-06 - Atomic inline-table vertical alignment

- Extended InlineAtomic plumbing so each atomic inline-table carries its computed VerticalAlign into
  line construction instead of being forced through baseline placement.
- Added baseline/top/middle/bottom atomic alignment in Lines. Baseline keeps the table first-row
  baseline behavior; top anchors the complete atom to the line top; bottom anchors it to the final
  line bottom; middle centers it around the parent text middle approximation.
- Top/bottom-aligned atomic boxes now participate in line-height growth. Tall inline tables expand
  the line rather than overflowing into neighboring line geometry while pretending they are zero
  ascent/descent objects.
- Atomic placement moves the retained table decorations, text boxes and image boxes together, so
  the internal table paint tree cannot separate from its aligned outer inline box.
- Added a layout regression comparing top/middle/bottom inline-table placement against surrounding
  large text. Existing baseline, wrapping, nested image/link and box-model inline-table regressions
  remain green.
- Updated project plan, code graph/slices and CSS/rendering/box-model wiki documentation. General
  inline vertical-align values sub/super/text-top/text-bottom/length/% remain later work.
- Final verification passed rustfmt, warning-free workspace Clippy, all 338 workspace tests,
  native startup smoke and optimized release build. The release executable is 891,392 bytes.


## 2026-10-06 - JavaScript, request filtering and browser lifecycle foundations

- Replaced the placeholder op_js crate with the first executable original ECMAScript slice:
  lexer, AST parser, bytecode compiler and stack interpreter. The current subset covers
  primitive literals, single let/const/var declarations, identifier assignment/load,
  unary/arithmetic/comparison/equality operators, string concatenation and persistent globals.
- Added typed JavaScript syntax/reference/type errors plus project-owned lexer/parser/compiler/
  runtime regressions. Page <script> discovery, DOM bindings, objects/functions, GC and the
  event loop remain future work.
- Added test262_probe, a deliberately parse-only external-suite probe that measures positive
  parse acceptance and negative parse-error expectations without presenting the result as
  runtime Test262 conformance.
- Added tools/compatibility.ps1 to run the project-owned HTML/CSS/layout/engine/JavaScript
  regression baseline and optionally invoke the Test262 parse probe. WPT automation remains
  NEXT and owned test counts are explicitly not reported as WPT percentages.
- Added op_net::RequestFilter and placed it before current document, stylesheet and image
  loads. The initial Adblock-style subset supports ||host^, wildcard patterns, @@ exceptions,
  resource-type options, per-site allowlisting and checked/allowed/blocked counters.
- Added an integration regression proving a blocked document is rejected before transport.
  Stylesheet/image blocks reuse the existing nonfatal subresource-failure path.
- Added op_browser_core with canonical TabId/TabManager state, active/background/throttled/
  frozen/discarded/restoring lifecycle states, protection flags, estimated-memory input,
  automatic discard-candidate selection and retained address/scroll restore metadata.
  Native tab UI, process memory-pressure signals and renderer termination/restoration remain
  integration work rather than being falsely marked complete.
- Accepted ADR-0002 for browser/renderer ownership and staged IPC/process isolation, with one
  renderer process per active tab as the first target. Accepted ADR-0003 allowing DirectWrite
  as Windows text-shaping/font infrastructure while OPBrowser continues to own CSS/layout/paint
  semantics.
- Reprioritized M2 around readable static sites: position/overflow/floats/flex/media/font-face/
  background images/radius/shadows now outrank additional edge-case value polish; grid follows
  the first usable flex slice.
- Updated README, project plan, code graph, code slices and wiki pages for JavaScript,
  request filtering, compatibility measurement, process ownership and text shaping.
- GitHub CI/workflows were intentionally not changed in this pass; the account billing/CI
  issue is being handled separately.
- Added the first positioned-layout value slice immediately before this checkpoint: computed
  `position: static|relative` and `top/right/bottom/left` insets now participate in cascade,
  global keywords and relative-unit/percentage parsing. Layout translation is not wired yet,
  so this is explicitly computed-style support rather than completed CSS positioning.
- Checkpoint verification: rustfmt passed, workspace Clippy passed with `-D warnings`, browser
  build and native `--smoke-test` passed, and the full workspace test suite passed on retry
  with 355 tests. One initial run hit the existing Windows localhost socket `10035 WouldBlock`
  flake in `http_linked_stylesheet_reaches_native_display_list`; the complete op_engine suite
  and then the complete workspace suite passed immediately afterward.

## 2026-10-06 - GitHub Actions CI restored

- Restored `.github/workflows/ci.yml` after the earlier billing-related disablement.
- CI now runs on pushes to `main`, pull requests targeting `main`, and manual dispatch, with
  read-only repository permissions, a 30-minute job timeout and concurrency cancellation for
  superseded runs.
- Restored the previous Windows verification coverage: rustfmt, generated HTML/CSS data checks,
  warning-free workspace Clippy, full workspace tests, native startup/navigation/link/image/resize
  smoke tests and the optimized release build.
- Added the current `tools/compatibility.ps1` project-owned regression baseline to CI so the HTML,
  CSS, layout, engine and JavaScript compatibility guardrail is visible in public runs. This is
  explicitly not a WPT or runtime Test262 percentage.
- Local verification passed rustfmt, warning-free workspace Clippy, all workspace tests, the
  compatibility baseline, native startup smoke and the optimized release build.
- GitHub Actions run `37527054337` then completed successfully on `windows-latest`; every restored
  CI step passed, confirming the previous account billing lock no longer blocks hosted runners.

## 2026-10-06 - Versioned WPT/Test262 compatibility metrics

- Added deterministic, committed compatibility manifests pinned to exact upstream revisions:
  2,000 Test262 language paths and 200 static WPT HTML/CSS reftests.
- Extended `test262_probe` with manifest input, robust module skipping, machine-readable JSON
  output and a versioned parser percentage. Initial executable result: 364/1983 (18.36%);
  17 module entries were skipped.
- Added `op_browser::wpt_probe` and `op_platform_win::render_display_list_to_bgra`. Reftests
  render through `Engine::render_source` and the same Win32 GDI `paint_command` path as the
  visible browser before exact BGR pixel comparison. Initial result: 86/200 (43.00%) with
  zero render/infrastructure errors.
- Added `tools/build_compat_manifests.py`, pinned revision metadata under `compat/`, JSON/
  Shields badge artifacts and a combined Markdown compatibility summary.
- Split hosted CI into the normal read-only Windows verification job, a read-only external
  compatibility job, and a write-enabled metrics publisher that runs only after successful
  main pushes. Public README badges consume the generated `metrics` branch.
- Preserved existing `cargo run -p op_browser` behavior by declaring the browser binary as
  Cargo's default run target after adding the WPT utility binary.
- Updated compatibility documentation, development workflow, project plan, code graph, code
  slices and wiki pages so future agents can optimize against measured failing suites instead
  of milestone percentages.
- Final local verification passed rustfmt, warning-free workspace Clippy, the complete workspace
  test suite, native startup smoke, optimized release build, deterministic manifest regeneration
  and both external subset runs; the WPT/Test262 scores repeated exactly.

## 2026-10-07 - CSS Color 4 measured WPT pass

- Used WPT Static v1 as the work queue instead of another milestone estimate; the dominant
  failing family was CSS Color 4 device-independent and predefined color spaces.
- Added `op_css::color` with D50/D65 adaptation, Lab/LCH and OKLab/OKLCH conversion plus
  sRGB, linear sRGB, Display P3/linear P3, A98 RGB, ProPhoto RGB, Rec.2020 and XYZ conversion
  into the existing 8-bit sRGB `CssColor` paint path.
- Extended computed color parsing with `lab()`, `lch()`, `oklab()`, `oklch()` and the broader
  `color()` predefined-space set, including percentage reference ranges and alpha/none handling.
- Added `currentColor` resolution for `color`, `background-color`/color-only `background` and
  the existing border color path, with computed-style regression tests based on WPT examples.
- Kept Rec.2020 on the current CSS Color 4 BT.1886 gamma-2.40 transfer rather than changing the
  engine to satisfy two pinned tests that still encode the older piecewise transfer expectation.
  Near-zero OKLab/OKLCH failures likewise remain queued for real gamut mapping instead of a
  threshold special case.
- WPT Static v1 improved on the unchanged 200-test manifest from 86/200 (43.00%) to 126/200
  (63.00%), with zero render/infrastructure errors.
- Final local verification passed rustfmt, warning-free workspace Clippy, the full workspace test
  suite, Win32 startup smoke, optimized release build, `git diff --check` and a repeated external
  compatibility run with the same 126/200 WPT result and unchanged 364/1983 Test262 parser result.

## 2026-10-07 - System colors and selector semantics WPT pass

- Added a deterministic browser-owned CSS system-color palette and the mandatory deprecated CSS2
  system-color aliases to their modern CSS Color 4 counterparts. Fixed values avoid leaking the
  host theme while making native/offscreen rendering deterministic.
- Added simple declaration-form `@supports (property: value)` evaluation. Supported conditions
  recurse into their nested rules; unsupported declarations skip the block without poisoning the
  following stylesheet. Boolean/composed supports conditions remain future work.
- Added Selectors 4 `:lang()` parsing and RFC 4647 extended filtering with inherited HTML language,
  ASCII case-insensitive matching, wildcard subtags, quoted/identifier ranges and lists.
- Added `:dir()` with inherited valid HTML `ltr`/`rtl` directionality, including invalid-value
  fallback to an ancestor. Full `dir=auto` bidi-content inference remains later work.
- Added initial `:required`, `:optional`, `:open` and `:visited` semantics. Link history is not
  exposed yet, so `:visited` is valid but intentionally never matches.
- Targeted WPT slices passed 14/14 for the pinned language/direction tests and 3/3 for the selected
  state pseudos. The unchanged WPT Static v1 manifest improved from 126/200 (63.00%) to 151/200
  (75.50%), with zero render/infrastructure errors.
- Final local verification passed rustfmt, warning-free workspace Clippy, the complete workspace
  test suite, Win32 startup smoke, optimized release build, `git diff --check` and the repeated
  external compatibility run. Test262 Parser v1 remained 364/1983 (18.36%).

## 2026-10-07 - Deferred color expressions and display contents WPT pass

- Added a private computed-color expression layer so non-`color` properties can retain a
  `currentColor` dependency through `inherit` and resolve the used RGBA against the receiving
  element instead of freezing the parent's pixels.
- Added the initial `color-mix()` implementation for two stops in sRGB or LCH, including percentage
  normalization, alpha reduction when the authored total is below 100%, deferred `currentColor`
  operands and the inverse sRGB -> D50 Lab path required for LCH interpolation.
- Added the pinned relative-color `from currentColor` slice for RGB/HSL/Lab/OKLab and predefined/XYZ
  identity channels plus literal HSL hue replacement. The safe no-history `:visited` policy remains
  unchanged, so the visited-specific color-mix WPT is intentionally still red.
- Added `Display::Contents`; the existing transparent child/generated-content collection path now
  suppresses the principal box for basic `display:contents` while preserving inherited content.
  Flex, table-internal and SVG-specific contents semantics remain future formatting-context work.
- Fixed Selectors 4 filtered nth grammar so the selector list may begin immediately after `of`;
  whitespace is required before `of`, not after it.
- Targeted color-expression WPT passed 10/11 (the remaining case requires visited history), the
  simple display-contents slice passed 5/5, and the no-space filtered-nth pair now passes.
- The unchanged WPT Static v1 manifest improved from 151/200 (75.50%) to 172/200 (86.00%), with
  zero render/infrastructure errors.
- Final local verification passed rustfmt, warning-free workspace Clippy, the full workspace test
  suite, Win32 startup smoke, optimized release build, `git diff --check` and the repeated external
  compatibility run. Test262 Parser v1 remained 364/1983 (18.36%).

## 2026-10-07 - Relational selectors and zero-geometry inline WPT pass

- Added Selectors 4 `:has()` parsing and matching with implicit descendant plus explicit child,
  adjacent-sibling and general-sibling relative selectors. Matching starts at the anchor and walks
  forward through the requested relations instead of reusing the ordinary right-to-left entry point.
- `:has()` specificity uses the most specific relative selector. Nested `:has()` and pseudo-elements
  inside it are rejected; regression tests cover descendant/child/sibling chains and `:not(:has())`.
- Added the empty-namespace attribute form `[|attr]` while preserving the syntax rule that whitespace
  is not allowed between the namespace separator and attribute name.
- Diagnosed the remaining filtered-nth WPT failures past computed style: selectors and cascade were
  already correct, but background-only empty inline elements were creating fake 24px lines and
  interrupting whitespace collapse. Empty inline DOM/pseudo fragments now require padding or border
  geometry before emitting `EmptyInline`; background-only zero-content boxes remain zero-area.
- The pinned `:has()` slice passes 7/7, the three selector diagnostic reftests pass 3/3, and the
  unchanged WPT Static v1 manifest improved from 172/200 (86.00%) to 179/200 (89.50%) with zero
  render/infrastructure errors.
- Final local verification passed rustfmt, warning-free workspace Clippy, the complete workspace
  suite after one transient Windows `WSAEWOULDBLOCK` in an existing local-socket fixture passed both
  focused retry and full rerun, Win32 startup smoke, optimized release build, `git diff --check` and
  repeated external compatibility. Test262 Parser v1 remained 364/1983 (18.36%).
- The current local release binary was rebuilt at `target/release/op_browser.exe`; no OPBrowser
  shortcuts were found in Desktop, Start Menu or pinned taskbar locations, so there is no separate
  installed copy to synchronize.
- Hosted run #69 reproduced WSAEWOULDBLOCK in a second local HTTP fixture. Three fixtures that used
  a non-blocking listener but left accepted Windows streams non-blocking now explicitly switch each
  accepted stream back to blocking mode before applying the finite read timeout, matching the
  already-stable image/navigation fixture pattern.

## 2026-10-07 - Structural display:contents table fixup pass

- Extended the existing anonymous table fixup so structural `display:contents` wrappers can expose
  table-internal descendants to the table/grid formatter without mutating the DOM. This covers
  contents wrappers around rows, row groups and cells, including the implicit `tbody` inserted by
  HTML tree construction.
- The first implementation recursively flattened every contents wrapper and reduced the complex
  table reftest from 6,548 differing pixels to 81, but diagnostics showed that direct text inside a
  non-structural contents node then lost the node's inherited text color. The final implementation
  expands a contents wrapper only when all exposed non-ignorable descendants are table-internal;
  ordinary text/inline contents remain transparent style carriers in the normal collection path.
- Added layout regressions for two `display:contents` row wrappers forming one anonymous row and for
  nested contents around a real row/cell structure.
- The two pinned table-contents reftests now pass 2/2. WPT Static v1 improved on the unchanged
  manifest from 179/200 (89.50%) to 181/200 (90.50%), with 19 failures and zero render errors.
- Diagnosed the remaining select/option display reftest: its 239 differing pixels are exactly raw
  text inside `<select>`. OPBrowser does not yet have a real form-control renderer, so the text was
  deliberately not hidden merely to increase the metric.
- `::first-letter` was selected as the next focused slice. The selector/style map can be extended,
  but computed pseudo handling is currently generated-content-specific and layout has no
  first-letter fragment machinery, so it remains a separate coherent implementation rather than a
  test-specific patch.
- Final verification passed rustfmt, generated HTML/CSS table checks, warning-free workspace Clippy,
  the complete workspace test suite, Win32 startup smoke, optimized release build, `git diff --check`
  and the repeated external compatibility run. Test262 Parser v1 remained 364/1983 (18.36%).
- The local release browser was rebuilt at `target/release/op_browser.exe`: 946,688 bytes, SHA-256
  `A0C9393AE2C0E6BF6FEF352866A7442B53E99BDEB2D5F853057D0EE75FC56A92`.

## 2026-10-07 - Grapheme-aware ::first-letter pass

- Added `::first-letter` as a terminal CSS pseudo-element with normal selector specificity and a
  distinct computed fragment-pseudo path, rather than treating it as generated `::before/::after`
  content.
- Added the focused `unicode-segmentation` dependency for UAX #29 extended grapheme boundaries.
  Layout styles the first non-whitespace grapheme cluster atomically, so Regional Indicator pairs
  such as the UK flag remain one first-letter unit instead of being split by Unicode scalar value.
- First-letter fragment styling overlays only properties actually authored on the pseudo. This
  preserves descendant and `display:contents` inheritance for untouched properties; an earlier
  whole-style replacement incorrectly turned a green contents descendant red and was discarded.
- When the pseudo changes `font-size`, its used normal line-height is recomputed from the pseudo
  font size. This keeps first-letter line geometry aligned with an equivalent explicitly styled
  inline element.
- Added parser/style/computed/layout regressions for first-letter specificity, source buckets,
  non-generated computed style, leading whitespace, Regional Indicator grapheme clustering and
  `display:contents` inheritance.
- The pinned first-letter WPT slice passes 2/2, including
  `first-letter-flag-001.html` and `display-contents-first-letter-002.html`.
- The unchanged WPT Static v1 manifest improved from 181/200 (90.50%) to 182/200 (91.00%), with
  18 remaining failures and zero render/infrastructure errors. The next layout-focused queue is
  BFC/block-in-inline behavior around `display:flow-root`, followed by flexbox.
- Final verification passed rustfmt, generated HTML/CSS table checks, warning-free workspace Clippy,
  the complete workspace test suite, Win32 startup smoke, optimized release build, repeated external
  compatibility and `git diff --check`. Test262 Parser v1 remained 364/1983 (18.36%).
- The local release browser was rebuilt at `target/release/op_browser.exe`: 983,040 bytes, SHA-256
  `6371FF14F00625C7E51A5DEA24F9CD9D005B87E2F37DC2CA4C04ACD281B2A57D`.

## 2026-10-07 - Self-collapsing block-in-inline margin pass

- Added conservative self-collapsing block analysis for zero-height block subtrees. Eligible blocks
  have no vertical border/padding or nonzero resolved height and no generated before/after content.
- The analysis can cross whitespace-only text in normal/nowrap whitespace modes, undecorated inline
  wrappers and `display:contents`. Visible inline boxes, replaced content, preserved whitespace,
  table/atomic contexts and generated content deliberately fall back to ordinary layout.
- Self-collapsing blocks no longer consume their top margin and then flush their bottom margin as
  separate vertical movement. Their complete adjoining-margin set is merged into `pending_margin`,
  so it can still collapse with the previous/next sibling and through an otherwise empty parent.
- Added a regression matching the CSS2 block-in-inline case: a zero-height child with 30px top and
  40px bottom margin nested inside an inline wrapper now creates a 40px sibling gap rather than
  70px, with no visible red background.
- The pinned `block-in-inline-self-collapsing-only-child.html` reftest now passes exactly. The
  unchanged WPT Static v1 manifest improved from 182/200 (91.00%) to 183/200 (91.50%), with
  17 remaining failures and zero render errors.
- The two pinned `display:flow-root` tests remain red because they intentionally combine BFC margin
  containment with real float containment/avoidance, inline splitting and list-item behavior. The
  next pass will implement that BFC/float foundation rather than aliasing flow-root to block.
- Final verification passed rustfmt, generated HTML/CSS table checks, warning-free workspace Clippy,
  the complete workspace test suite, Win32 startup smoke, optimized release build, repeated external
  compatibility and `git diff --check`. Test262 Parser v1 remained 364/1983 (18.36%).
- The local release browser was rebuilt at `target/release/op_browser.exe`: 986,112 bytes, SHA-256
  `BD77BA98537F7EDB20762825E02527D8CC14DBC65B07F1E56E0D869ED7505116`.

## 2026-10-07 - Initial flow-root, BFC and float pass

- Added computed display forms for `flow-root`, `list-item` and the two-keyword
  `flow-root list-item` form. Layout treats them as block-level while keeping BFC creation
  separate from ordinary block-level behavior, so yesterday's self-collapse optimization still
  applies only to normal blocks.
- Added non-inherited `float:none/left/right` and `clear:none/left/right/both`. The block layout
  context now tracks active float rectangles independently of the normal-flow cursor. Clear advances
  below matching floats; flow-root snapshots the outer float set, avoids overlapping outer floats,
  lays out against a local float set, grows to contain local float bottoms and restores the outer set.
- Floated tables retain the dedicated table formatter inside float placement. `display:contents`
  suppresses its principal box before float placement, so a float declaration on a contents-only
  element does not manufacture a float box.
- Added CSS2-compatible single-colon `:before`, `:after` and `:first-letter`. This was required
  by the pinned flow-root reference's traditional clearfix syntax and maps to the same terminal
  pseudo-element representation as the modern double-colon spellings.
- Added inherited `visibility:visible/hidden`. Hidden text/images remain in layout with their normal
  metrics but carry a non-painting flag into the display-list builder; box paint colors are suppressed
  without changing geometry. This avoids GDI antialias remnants that remained when hidden glyphs were
  merely sent to the painter with alpha zero.
- Added regressions for BFC margin isolation, outer-float avoidance, child-float containment,
  float/clear/visibility inheritance semantics, hidden-text layout retention and hidden paint
  suppression.
- The first float attempt intentionally failed the full metric: although the two target tests moved
  sharply toward their references, it activated regressions in `display-contents-float-001` and
  `display-contents-table-002`. The final path fixes both by skipping float box creation for
  `display:contents` and preserving table formatting for floated tables.
- The four-test BFC/regression slice now passes 4/4. Both pinned flow-root tests are green, the two
  pre-existing contents tests remain green, and the unchanged WPT Static v1 manifest improves from
  183/200 (91.50%) to 185/200 (92.50%), with 15 failures and zero render errors.
- Remaining layout-oriented failures are the two flex-related `display:contents` tests, one SVG
  contents test, one form-control display test and `::first-line`/bidi. The remaining ten are
  color/ICC/gamut/compositing families, so initial flex formatting is the next metric-driven slice.
- Final verification passed rustfmt, generated HTML/CSS table checks, warning-free workspace Clippy,
  the complete workspace test suite, Win32 startup smoke, optimized release build, repeated external
  compatibility and `git diff --check`. Test262 Parser v1 remained 364/1983 (18.36%).
- The local release browser was rebuilt at `target/release/op_browser.exe`: 998,912 bytes, SHA-256
  `E696273FC943E107001D3424E326B5DA0CF0226511D85868C76B93F48B16B479`.

## 2026-10-07 - Initial flex formatting context WPT pass

- Added computed `display:flex` and `display:inline-flex` values instead of letting those
  declarations fall through to ordinary flow.
- Added an initial default single-line row flex formatter. Direct element children become flex
  items, contiguous text becomes whitespace-aware anonymous flex items, and nested
  `display:contents` wrappers recursively expose their descendants without manufacturing a
  principal background/border box.
- Reused the existing block/inline layout outputs inside flex items, then rebased decorations,
  text, images and display order horizontally into the flex row. Block flex establishes an isolated
  formatting context; `inline-flex` uses the same row as a shrink-to-content atomic inline box.
- Added computed-style coverage for both display keywords and layout regressions comparing
  `display:contents` flex trees with equivalent flattened references for both block flex and
  inline-flex.
- Focused pinned reftests `display-contents-flex-002` and
  `display-contents-inline-flex-001` moved from 0/2 to 2/2 with exact pixel matches.
- The unchanged WPT Static v1 manifest improved from 185/200 (92.50%) to 187/200 (93.50%),
  leaving 13 failures and zero render errors.
- Updated the project plan, compatibility history, code graph, code slices and wiki to record the
  new formatting boundary and its deliberate limits. Full Flexbox properties such as direction,
  wrapping, flexible lengths, ordering, alignment distribution and gaps remain future work.
- Final verification passed rustfmt, both generated-table checks, warning-free workspace Clippy,
  the complete workspace test suite, all Win32 CI smoke variants, the repeated 200-test WPT Static
  v1 probe, optimized release build and `git diff --check`.
- The local release browser was rebuilt at `target/release/op_browser.exe`: 1,009,664 bytes,
  SHA-256 `1D948C02A1EA83745BD3655C39E79FB3B21F0A3A30711A051D9B42757B1B7322`.

## 2026-10-07 - JavaScript control-flow bytecode pass

- Expanded the original ECMAScript lexer/parser with blocks, `if/else`, `while`,
  `break`/`continue`, comma-separated variable declarators and short-circuit `&&`/`||`.
  Break/continue are rejected outside loops rather than accepted as fake syntax.
- Reworked bytecode compilation around patched jump targets. Logical operators preserve the
  original operand value on short-circuit; branch and loop conditions are explicitly removed from
  the operand stack so control-flow paths keep stack invariants.
- Reworked `JsRuntime` from a linear instruction iterator to an instruction-pointer VM with
  `Jump`, `JumpIfFalse`, `JumpIfTrue` and `Pop` execution.
- Added a bounded per-execution instruction budget (1,000,000 by default, configurable for tests)
  and a dedicated execution-limit error so runaway loops cannot monopolize the renderer thread.
- Added lexer, parser, compiler and runtime regressions for nested branches/loops, continue/break,
  multiple declarations, short-circuit side effects/value preservation and runaway-loop aborts.
  The op_js suite increased from 8 to 14 core tests, plus the existing probe tests.
- The unchanged Test262 Parser v1 manifest improved from 364/1983 (18.36%) to
  391/1983 (19.72%). This remains a parse-expectation score only; runtime Test262 conformance is
  not claimed.
- External compatibility refresh also reconfirmed WPT Static v1 at 187/200 (93.50%) with the same
  13 known failures and zero render errors.
- Full verification passed rustfmt, warning-free workspace Clippy, the complete workspace test
  suite, native startup smoke, external compatibility refresh, optimized release build and
  `git diff --check`.
- The local release browser was rebuilt at `target/release/op_browser.exe`: 1,009,664 bytes,
  SHA-256 `38C1C367533F5A0F083F0EA003A3B09F82B41042F4483C6229A7BD9441D29310`.
- Page `<script>` execution is still intentionally not connected. The next high-leverage JS
  work is objects/properties/prototypes, then functions/calls/lexical environments; those are the
  main blockers before DOM scripting can be wired honestly.

## 2026-10-07 - JavaScript object heap, members and prototype-chain pass

- Extended the lexer/parser with object and array literal punctuation, dot/computed member access,
  member assignment targets, shorthand object properties, numeric/string property names, sparse
  array elements and trailing commas.
- Added stable `ObjectId` reference values backed by a runtime-owned heap instead of embedding or
  copying property maps in `JsValue`. Strict equality now preserves object identity across globals
  and assignments.
- Added bytecode instructions for object/array allocation plus property reads/writes. Member
  assignment preserves JavaScript evaluation order and leaves the assigned value as the expression
  completion.
- Added own string-keyed properties, prototype-chain lookup, object-literal `__proto__` prototype
  setters, later `__proto__` mutation and cycle rejection. Duplicate literal prototype setters are
  rejected as syntax errors while shorthand `{__proto__}` remains an ordinary shadowing property.
- Added sparse arrays on the same heap model with numeric property keys, an own `length` property
  and automatic length growth when a higher array index is assigned. Primitive boxing remains
  partial; string `.length` is implemented using ECMAScript UTF-16 code-unit length.
- Kept allocation bounded with a 100,000-object runtime budget while garbage collection remains a
  later milestone.
- Expanded op_js core coverage from 14 to 22 tests. New regressions cover object identity,
  own/computed properties, assignment completion values, inherited lookup, prototype mutation and
  cycles, sparse arrays, dynamic length growth, UTF-16 string length, nullish property errors and
  the special-vs-shorthand `__proto__` grammar.
- The unchanged Test262 Parser v1 manifest improved from 391/1983 (19.72%) to
  408/1983 (20.57%). This is still a parse-expectation metric, not runtime Test262 conformance.
- External compatibility refresh reconfirmed WPT Static v1 at 187/200 (93.50%) with 13 known
  failures and zero render errors.
- Final verification passed rustfmt, warning-free workspace Clippy, the complete workspace test
  suite, native startup smoke, external compatibility refresh and optimized release build.
- The local release browser was rebuilt at `target/release/op_browser.exe`: 1,009,664 bytes,
  SHA-256 `7F3FA3241F783E8AB8FE3B8F5F24596411AB5603DA1BEB99F7FEA50CCA5766E3`.
- Page scripting is still not wired. The next JS milestone is functions/calls with lexical
  environments and closures, followed by exceptions and then honest `<script>` integration.

## 2026-10-07 - JavaScript functions, lexical environments and closures

- Added `function` declarations/expressions, positional parameter lists, call expressions and
  `return` statements to the owned ECMAScript parser. Calls can be chained with member/call syntax,
  named function expressions keep their own recursive name, and top-level `return` is rejected.
- Extended bytecode with function templates, `CreateFunction`, `Call`, `Return`,
  `EnterScope`/`ExitScope` and `UnwindScopes`. Break/continue now unwind nested lexical blocks
  before jumping so control-flow exits do not leave the VM in a stale environment.
- Replaced flat global-only binding lookup with a runtime-owned lexical-environment arena. Scripts,
  functions and blocks now have explicit Global/Function/Block environments; let/const bind in the
  current block while var targets the nearest function/global environment.
- Function objects capture their creation environment and retain it after scope exit, enabling real
  closures that both read and mutate captured bindings. Function calls create parameter bindings,
  support missing arguments as undefined, recursion, anonymous/named function expressions and
  initial callable `name`/`length` properties.
- Added a 256-frame call-depth budget and a 100,000-environment allocation budget alongside the
  existing instruction/object budgets, so runaway recursion and unbounded scope creation fail with
  execution-limit errors instead of relying on the native stack indefinitely.
- Expanded op_js core coverage from 22 to 31 tests. New regressions cover block shadowing,
  function-scoped var, break/continue scope unwind, parameters/returns, recursion, persistent
  closures, exited-block capture, private named-function recursion and non-callable/call-depth
  failures.
- The unchanged Test262 Parser v1 manifest improved from 408/1983 (20.57%) to
  504/1983 (25.42%). This is still a parse-expectation metric, not runtime Test262 conformance.
- External compatibility refresh reconfirmed WPT Static v1 at 187/200 (93.50%) with 13 known
  failures and zero render errors.
- Full verification passed rustfmt, warning-free workspace Clippy, the complete workspace test
  suite, native startup smoke, external compatibility refresh and optimized release build.
- The local release browser was rebuilt at `target/release/op_browser.exe`: 1,009,664 bytes,
  SHA-256 `635A7C9CD1B66636DA08E114DB73F712FA0CE946DFA584AE9F477DA2CD9D9F41`.
- Function declaration hoisting, `this`, `new`, `arguments`, arrow/default/rest/destructuring
  forms, exceptions and fuller ASI are still missing. Page `<script>` execution remains
  intentionally disconnected until those semantics and the DOM binding boundary are further along.

## 2026-10-07 - JavaScript exceptions and broader control-flow pass

- Added C-style `for`, `do/while`, `switch/case/default`, switch fallthrough and the correct
  distinction between breakable loop/switch contexts and continue-only loop targets. Nested lexical
  scopes unwind before local jumps and switch-contained continue can target an enclosing loop.
- Added prefix/postfix `++`/`--` for identifier and member assignment targets. Runtime updates use
  the existing numeric conversion path, preserve the old value for postfix and new value for prefix,
  and still reject writes to const bindings.
- Added explicit JavaScript abrupt completions for `throw`, `try/catch/finally`, optional catch
  bindings, return, break and continue. Thrown values cross user-function calls into catch; finally
  executes for normal and abrupt completion and an abrupt finalizer overrides the prior completion.
- Added patched external break/continue targets to try bytecode so control can cross a try/finally
  boundary, execute the finalizer first, unwind the required lexical scopes and resume the enclosing
  loop or switch rather than being converted into an engine error.
- Added initial function-declaration hoisting within compiled statement lists. Direct declarations
  are instantiated before the remaining statements in scripts, blocks and function bodies, while
  full declaration instantiation, TDZ/var hoisting and Annex B block-function semantics remain later.
- The current C-style `for(let ...)` uses one lexical loop environment rather than the spec's fresh
  per-iteration binding, so closure capture across iterations is explicitly not complete yet.
- Explicit `throw` values are catchable, but runtime-generated Reference/Type failures still use
  the engine error path instead of JavaScript Error objects; catchable Error objects are a later
  runtime milestone.
- Reduced the temporary native-recursive call-depth budget from 256 to 64 after the previous limit
  was observed to overflow the Windows native stack before the guard fired. Explicit heap/VM call
  frames remain the proper long-term fix.
- Expanded op_js core coverage from 31 to 39 tests. New regressions cover for/do/switch execution,
  fallthrough, continue-through-switch, let/var loop scope, prefix/postfix update values, function
  hoisting, thrown values crossing calls, catch/finally return precedence and finally before
  break/continue.
- The unchanged Test262 Parser v1 manifest improved from 504/1983 (25.42%) to
  508/1983 (25.62%). This remains a parse-expectation metric, not runtime Test262 conformance.
- External compatibility refresh reconfirmed WPT Static v1 at 187/200 (93.50%) with 13 known
  failures and zero render errors.
- Final verification passed rustfmt, warning-free workspace Clippy, the complete workspace test
  suite, native startup smoke, external compatibility refresh and optimized release build.
- The local release browser was rebuilt at `target/release/op_browser.exe`: 1,009,664 bytes,
  SHA-256 `7112D2E4DDBA728EE892C23D4EF7C8822941A1F18703637AE79A06106D0BB8B1`.
- Page scripting remains disconnected. The next JS priorities are `this`/`new`/`arguments`,
  JavaScript Error objects and catchable runtime failures, modern function/parameter forms,
  per-iteration loop bindings, labels/for-in/of and explicit VM call frames.

## 2026-10-07 - JavaScript this, constructors, arguments and Error objects

- Added reserved `this` and `new` syntax plus owned AST/bytecode support for constructor calls.
  Calls now distinguish ordinary invocation from member invocation so `obj.method()` preserves the
  base object as the receiver instead of discarding it during property lookup.
- Added a runtime global object used as non-strict `this` for bare calls and script-level `this`.
  User-function environments receive an immutable this binding plus an array-like arguments object
  with indexed entries and length.
- Every user function now owns an ordinary prototype object with a constructor backlink. `new`
  resolves constructor.prototype, allocates a receiver, calls the function with that receiver and
  implements the JavaScript constructor return rule: returned objects replace the receiver while
  primitive returns do not.
- Added initial built-in Error, TypeError and ReferenceError constructors/prototypes with name,
  message, prototype and constructor links. They work both as calls and with `new`.
- Runtime ReferenceError/TypeError failures raised inside a try region are now materialized as
  catchable JavaScript error objects. Explicit execution-limit failures remain engine-level guards
  and intentionally cannot be neutralized from script.
- Uncaught thrown Error-family objects now surface useful `Name: message` text instead of the old
  generic `[object Object]` rendering.
- Expanded op_js core coverage from 39 to 44 tests. New regressions cover parser/lexer this/new,
  method receivers, global-this bare calls, arguments indexing/length, constructor prototype links,
  constructor object-return semantics, built-in Error constructors and catchable runtime
  TypeError/ReferenceError objects.
- The unchanged Test262 Parser v1 manifest improved from 508/1983 (25.62%) to
  523/1983 (26.37%). This remains a parse-expectation metric, not runtime Test262 conformance.
- External compatibility refresh reconfirmed WPT Static v1 at 187/200 (93.50%) with the same
  13 known failures and zero render errors.
- Final verification passed rustfmt, warning-free workspace Clippy, the complete workspace test
  suite, native startup smoke, external compatibility refresh and optimized release build.
- The local release browser was rebuilt at `target/release/op_browser.exe`: 1,009,664 bytes,
  SHA-256 `A02D5E283F92F303CD0E7EB27CBF0B4D4A0006F8C192475186440624B71B179E`.
- Explicit VM call frames are still pending; user-function calls currently recurse through the
  native Rust stack behind the temporary depth-64 guard. The next broad project pass shifts back to
  static-web readiness: positioning/overflow/media/font/background/radius work and a wider WPT
  subset, while M4 DOM bindings can begin in parallel once a narrow host boundary is defined.

## 2026-10-07 - Initial absolute/fixed positioning and WPT positioning baseline

- Extended computed `position` support from static/relative to
  `static|relative|absolute|fixed` while retaining the existing four inset longhands and cascade
  behavior. Added regression coverage for absolute/fixed keywords plus mixed px/percentage insets.
- Added an initial positioned-layout path in `op_layout`. Relative blocks keep their normal-flow
  contribution and translate only the decorations/text/images emitted by their subtree. Absolute
  and fixed blocks leave normal flow; absolute boxes resolve against the nearest positioned
  ancestor tracked by the layout context, while fixed boxes resolve against the viewport.
- The positioned ancestor stack stores the initial padding-box x/y/width needed by descendant
  absolute boxes. The first slice supports left/right horizontal placement, percentage horizontal
  insets and px top offsets while preserving existing block/table/image layout paths.
- Bottom-based absolute/fixed placement, vertical percentage insets, inline containing blocks,
  complete CSS2 absolute auto/overconstraint equations, sticky positioning and stacking/z-index are
  intentionally still open rather than approximated with invented geometry.
- Added three deterministic layout regressions covering nearest-positioned-ancestor absolute
  geometry, viewport-relative fixed geometry and relative visual translation without moving the
  following normal-flow block.
- Added a second pinned WPT reftest metric, `WPT Positioning v1`, with 100 deterministic tests from
  CSS2 positioning/visual formatting/dimensions and CSS Positioned Layout at the same pinned WPT
  revision. The sample intentionally includes unsupported sticky, vertical-writing and multicol
  families so it exposes the real layout gap.
- Initial WPT Positioning v1 baseline: **18/100 (18.00%)**, 82 failures and zero render errors.
  Existing WPT Static v1 remains frozen at **187/200 (93.50%)** so its history stays comparable.
- Generalized `wpt_probe` JSON/console suite identification from a hard-coded static-suite name to
  the manifest filename, allowing multiple versioned WPT metrics without mislabeled artifacts.
- CI sparse checkout now fetches the positioning/visual-formatting directories, compatibility
  artifacts include the new JSON/badge, successful main pushes publish that badge to the metrics
  branch, and README exposes the new WPT Positioning v1 badge beside the existing metrics.
- Test262 Parser v1 remains **523/1983 (26.37%)** after the preceding this/new/Error pass.
- Final verification passed rustfmt, warning-free workspace Clippy, the complete workspace test
  suite, native startup smoke, WPT/Test262 external compatibility and optimized release build.
- The local release browser was rebuilt at `target/release/op_browser.exe`: 1,013,248 bytes,
  SHA-256 `096270BB253A940950DD70BC6AB6F611209D3E7CC0DF6E525D6F09FFCA0C52ED`.
- Next readable-static-web priorities are driven by the new positioning failures: sticky/abspos
  sizing and containing-block semantics, then overflow, media queries, font faces, background
  images and border radius. The old 13 WPT Static failures remain visible but no longer monopolize
  compatibility work.

## 2026-10-07 - Positioned geometry / definite-height pass

- Made the main engine-to-layout path viewport-height-aware while preserving compatibility wrappers
  for existing width-only layout callers. Fixed positioning now resolves against the actual viewport
  width and height instead of a width-only synthetic context.
- Extended positioned containing blocks with definite height and added a direct block-height basis
  stack for percentage-height resolution. Positioned ancestors expose their padding-box geometry;
  explicit heights are min/max-clamped before descendants use them as a percentage basis, while
  auto-height boxes are not incorrectly made definite by min-height alone.
- Completed the next coherent absolute/fixed geometry slice: vertical percentage top/bottom insets,
  bottom-only placement, opposing left/right auto-width stretching, opposing top/bottom auto-height
  stretching, right-side margin accounting, and percentage relative vertical offsets when the direct
  containing block height is definite.
- Fixed computed-style inset parsing so percentage values are accepted for `top` and `bottom`;
  previously horizontal percentage insets survived cascade while vertical ones were discarded before
  layout could use them.
- Prevented the self-collapsing zero-height fast path from swallowing absolute/fixed blocks and taught
  that analysis to use a definite containing-height basis when one exists.
- Added regressions for fixed bottom/percentage-right placement against an 800x600 viewport,
  four-inset absolute auto stretching inside a definite positioned ancestor, and percentage-height
  descendants of a max-height-clamped positioned parent.
- The unchanged WPT Positioning v1 manifest improved from **18/100 (18.00%)** to
  **21/100 (21.00%)**, with 79 failures and zero render errors. WPT Static v1 remains
  **187/200 (93.50%)**, and Test262 Parser v1 remains **523/1983 (26.37%)**.
- Final verification passed rustfmt, warning-free workspace Clippy, the complete workspace test
  suite, native Win32 startup smoke, the pinned WPT/Test262 compatibility refresh, and the optimized
  release build.
- The rebuilt local browser at `target/release/op_browser.exe` is **1,017,856 bytes** with
  SHA-256 `5B24F2B210E89CFFE32D859B05993360FA62027631273B285946C7AFBCA2B24D`.
- Remaining positioning priorities are inline containing blocks, static-position and negative-margin
  cases, the rest of the CSS2 absolute-position constraint/auto-margin equations, then sticky and
  overflow.

## 2026-10-07 - Inline static-position / inline-block / intrinsic SVG pass

- Added initial `display:inline-block` computed-style support and an atomic inline-block layout path.
  Auto-width inline blocks use shrink-to-fit sizing, run their contents through a local block/BFC
  context, and contribute one atomic box to the surrounding inline formatting context.
- Added horizontal margins to inline fragment advance, including negative margins, while keeping
  those margins outside the painted border/background geometry. This closes the negative-margin
  static-position case without introducing test-name-specific behavior.
- Absolute/fixed inline boxes now leave a zero-width marker in the inline sequence. Auto-inset
  static positions therefore use the actual inline cursor and line y-coordinate without consuming
  inline width or forcing the surrounding line to flush.
- Extended positioned `width:auto` handling with an initial shrink-to-fit path for cases that are
  not stretched by opposing left/right insets.
- Extended `op_image` with bounded intrinsic metadata and an initial SVG image slice. Replaced
  sizing can now consume intrinsic width, height and ratio independently, including partial and
  ratio-only SVG intrinsics, while retaining existing encoded-byte, dimension and pixel budgets.
- Added regressions for inline absolute static markers, atomic inline-block margin geometry and the
  expanded replaced-element intrinsic sizing rules.
- The unchanged WPT Positioning v1 manifest improved from **21/100 (21.00%)** to
  **25/100 (25.00%)**, with 75 failures and zero render errors. The newly passing slice includes the
  negative-margin absolute-position case and three replaced-element min/width cases. WPT Static v1
  remains **187/200 (93.50%)**, and Test262 Parser v1 remains **523/1983 (26.37%)**.
- Final verification passed rustfmt, warning-free workspace Clippy, the complete workspace test
  suite, native Win32 startup smoke, the pinned WPT/Test262 compatibility refresh, and the optimized
  release build.
- The rebuilt local browser at `target/release/op_browser.exe` is **1,039,360 bytes** with
  SHA-256 `E428E9A4DBFF3D4D632FE7EEBF4993EE73B25EAFB4F7DFBE15DB221D33F1018C`.
- Remaining positioning priorities are full split-inline/inline containing-block geometry,
  block-level static-position directionality, CSS2 absolute-position overconstraint/auto-margin
  equations, sticky positioning and overflow.

## 2026-10-07 - CSS2 split-inline continuation pass

- Reworked block-inside-inline handling around explicit continuation nodes in the parent-linked
  `InlineBoxes` arena. Later inline descendants resolve to the newest continuation instead of
  reopening the original fragment after an intervening block.
- Split fragments now suppress the correct logical horizontal edge: LTR ending fragments drop the
  right edge and continuations drop the left edge; RTL reverses those physical sides. Added initial
  inherited computed `direction:ltr|rtl` support specifically for this logical fragment behavior;
  this does not claim bidi text reordering or vertical-writing support.
- Added split-fragment history so empty intermediate continuations preserve a zero-width line box
  only when CSS2 requires it. This distinguishes whitespace-only block separators from cases where
  real inline content already appeared before adjacent block children.
- Relative-positioned inline ancestors now carry their visual offset onto block and float descendants
  produced by a split while leaving normal-flow positions and float exclusion geometry unchanged.
- Large finite CSS lengths are no longer rejected solely for exceeding an arbitrary one-million-unit
  parse cap. They remain computed values and are bounded when converted to used integer layout
  geometry, which keeps extreme off-screen insets safe without turning them into `auto`.
- Updated deterministic inline regressions to compare block-inside-inline output against explicit
  start/middle/end fragment references, including the required whitespace-between-blocks line.
  Added computed-style coverage for direction inheritance/override and very large finite insets.
- The unchanged WPT Positioning v1 manifest improved from **25/100 (25.00%)** to
  **36/100 (36.00%)**, with 64 failures and zero render errors. The pass closes the empty-span,
  split-inline/removal, whitespace-between-blocks, relative-inline float and large-negative-inset
  slices without changing the manifest.
- WPT Static v1 remains **187/200 (93.50%)** and Test262 Parser v1 remains
  **523/1983 (26.37%)**.
- Final verification passed rustfmt, warning-free workspace Clippy, the complete workspace test
  suite, native Win32 startup smoke, the pinned WPT/Test262 compatibility refresh, and the optimized
  release build.
- The rebuilt local browser at `target/release/op_browser.exe` is **1,049,088 bytes** with
  SHA-256 `70789BDDDE62FA59C01268250E1F820D97707EE5D9E40BC7D9DCCB7E20A8D9B1`.
- Remaining positioning priorities are real inline containing-block rectangles for absolute
  descendants, CSS2 content-height/line-height cases, remaining absolute-position equations,
  vertical writing/multicol, sticky positioning and overflow.

## 2026-10-08 - CSS2 absolute/fixed margin constraint pass

- Fixed the old ordinary block auto-margin resolver being reused for absolute/fixed width,
  which centered positioned boxes against the entire viewport even when explicit insets
  constrained the available space.
- Non-replaced absolute/fixed blocks now distribute horizontal auto margins across the free
  space between definite left/right insets. One-auto-margin, both-auto-margin, negative
  free-space LTR/RTL, right-only and left-only inset cases use CSS2-style placement.
- Fixed horizontal overconstraint anchoring: left inset wins in LTR, right wins in RTL.
  Solved margins are passed to the actual block renderer, not only to its x-coordinate.
- Added corresponding vertical auto-margin resolution when top/bottom and a definite used
  height exist, including top/bottom-only auto-to-zero and negative-space bottom assignment.
- Added four deterministic regression tests for horizontal/vertical auto margins, negative
  free space and direction-dependent overconstraints; all seven absolute-related unit tests pass.
- Rechecked the same pinned external manifests: WPT Static v1 **187/200 (93.50%)**,
  WPT Positioning v1 **36/100 (36.00%)**, Test262 Parser v1 **523/1983 (26.37%)**.
  This targeted correction has no change to those frozen subset scores; it closes real
  project-observed geometry gaps without inventing a metric gain.
- Remaining positioned gaps: inline containing blocks, replaced/complex constraints,
  sticky, stacking/z-index, vertical writing/multicol and overflow.
- Final local verification passed cargo fmt --check, warning-free workspace Clippy,
  all workspace tests, Win32 startup/paint smoke and optimized release build.
- Updated local Windows executable: `target/release/op_browser.exe`, **1,051,136 bytes**,
  SHA-256 `36E7CDBE96D3F1D1E9CE6AD60998F52262B0645D2A72BB02987AF37256F2E4EE`.

## 2026-10-08 - Initial relative-inline containing rectangles

- Preserve `position:relative` inline ancestors as arena box nodes even without visible
  backgrounds or borders; this gives absolute descendants a real nearest positioned inline
  containing-block identity rather than always falling back to block/viewport geometry.
- Add first/last fragment geometry emitted from the normal line formatter, with direction-aware
  LTR/RTL padding edge resolution and definite height for percentage-positioned descendants.
- Translate relative inline decorations, text, images and atomic content visually without
  consuming extra inline advance or normal-flow height. Nested relative offsets accumulate.
  Fixed-position descendants continue to resolve against the viewport.
- Route absolute/fixed descendants through zero-width inline markers when the containing inline
  path has a relative ancestor, while retaining block-level static-position fallback inside
  unrelated unpositioned inline elements.
- Avoid creating spurious empty continuation lines for undecorated relative inline nodes and
  prevent metadata-only boxes from blocking normal empty-block margin collapse.
- Added five deterministic inline integration tests for undecorated ancestors, nested/viewport
  precedence, wrapping, RTL fragment edge selection and preservation of legacy static positions.
- Pinned compatibility metrics unchanged after regression correction: WPT Static v1
  **187/200 (93.50%)**, WPT Positioning v1 **36/100 (36.00%)**, Test262 Parser v1
  **523/1983 (26.37%)**. Initial implementation temporarily dropped two positioning
  cases; selective marker routing and empty fragment fixes restored the baseline.
- Known limitation: split inline ancestors spanning distinct block/line formatter runs do
  not yet accumulate one complete containing rectangle; vertical writing, bidi reordering,
  multicol and stacking/z-index remain unimplemented.
- Final verification passed rustfmt --check, workspace Clippy with -D warnings, full
  workspace tests, Win32 startup/paint smoke, pinned compatibility suites and release build.
- Rebuilt local `target/release/op_browser.exe`: **1,056,256 bytes**, SHA-256
  `0DEE29CA9CB3A15AB2A4FDA4B4EF2EA5F880ACA04FBB2453EF6342EFE189036D`.

## 2026-10-08 - Cross-run inline containing geometry and RTL static offsets

- Persist positioned inline fragment rectangles at the flow-context level rather than losing
  them after each local line-layout emission. Position-absolute children with relative inline
  ancestors now wait until all continuations are known across intervening block descendants.
- Add a bounded, index-drained deferred positioned queue: later absolute subtrees can enqueue
  their own nested relative inline descendants, which are resolved in the same pass.
  Independently formatted table, inline-block, inline-flex and atomic block subtrees drain
  their own pending positions before being transferred to their parent output.
- Carry deferred coordinates, fallback containing geometry and fragments through outer relative
  block translations. The static position now includes hypothetical flow width, allowing
  right-to-left absolutely/fixed positioned blocks with unspecified horizontal insets to anchor
  to their hypothetical flow right edge instead of the whole viewport.
- Eight new deterministic inline-layout regressions: spanning block-split fragments, markers
  before the last fragment, relative translation, nested deferred descendants, RTL continuations,
  RTL static absolute/fixed blocks and background geometry parity.
- Pinned external metrics remain WPT Static **187/200 (93.50%)**, WPT Positioning
  **36/100 (36.00%)**, Test262 Parser **523/1983 (26.37%)** with no new render errors.
- Investigated an intrinsic inline font-height correction; although several failed WPT pixel
  distances improved, it newly failed a previously passing static link-background test by one
  pixel row. Rolled that experimental correction back rather than accepting the regression.
  The known content-height/line-height cases remain for a dedicated typography pass.
- Full workspace rustfmt, Clippy with -D warnings, cargo tests, Windows startup/paint smoke
  and optimized release build passed. Rebuilt `target/release/op_browser.exe`:
  **1,058,304 bytes**, SHA-256
  `848E3E20616377D09A2B453CE76FBEB9175D0F50E0516778587F5F0B8C9BB25A`.

## 2026-10-08 - WPT reftest pixel-diagnostic output

- Add explicit `wpt_probe --dump-failures DIRECTORY`. Failed rendered test/reference
  images are written as Windows-compatible top-down 32-bit BGRA BMP pairs named
  with their 1-based manifest case number; no new encoder dependency.
- Validate dimensions and buffer length and guard 32-bit BMP size overflow.
  Limit dumps to the first 12 rendered failures; normal CI and probe use save nothing
  unless the flag is explicitly supplied. Parser and score semantics are unchanged.
- Add automated coverage for BMP structure, native BGRA byte order, top-down pixel
  orientation and invalid buffer dimensions. Manually exercised the flag on five
  focused WPT cases. This diagnosed a single-row background difference that caused
  a temporary Static-suite regression, allowing the suspect typography change to
  be reverted before publication.
- Verified the bounded dump behavior with a 15-case failing manifest: exactly 24 BMP
  files (12 test/reference pairs) were written, with all 15 failures still counted.
- Final workspace rustfmt check, Clippy with -D warnings, all workspace tests and native
  Win32 startup/paint smoke passed. The release browser binary from the layout commit
  remains unchanged.

## 2026-10-08 - Font-based inline content boxes, final-line baselines and CSS2 paint phases

- Inline background/border boxes now use their own font ascent/descent plus padding/borders,
  independently from line-height, which continues to control the strut and half-leading.
  Previous nested box tests were updated to the font-based geometry.
- The line formatter exposes the last in-flow baseline to its enclosing layout context.
  Inline-blocks use this baseline for atomic vertical alignment, with a bottom-border
  fallback when no line box exists.
- Added a Block/Inline paint-layer marker to decorations. Backgrounds and borders of
  block boxes now paint before inline decoration backgrounds/borders; this avoids
  following block backgrounds erasing inline text-area ink that extends beyond its
  line box. This is a preliminary paint phase, not full stacking-context support.
- Added focused inline-block alignment and overlapping paint order tests. The frozen
  WPT Positioning v1 manifest improved from 36/100 to **38/100 (38%)**; WPT Static v1
  remains 187/200 (93.50%), Test262 Parser v1 remains 523/1983 (26.37%), with zero
  render errors.
- Final rustfmt, warning-free workspace Clippy, all Rust workspace tests, pinned WPT/Test262
  probes and Win32 startup/paint smoke passed.
- The active Windows browser process held `target/release/op_browser.exe` open, so the
  optimized release build was produced independently and copied to
  `target/release/op_browser_next.exe`, without terminating the user's browser session.
  New binary: **1,057,792 bytes**, SHA-256
  `282E9647DE05520A66F15C7E0C27266C8CC44C83F32C0BD6F5CDA150274C8014`.
  To update the regular executable, close the running browser and replace it with
  `op_browser_next.exe`; do not overwrite an active Windows executable.

## 2026-10-08 - First foreground paint phase for positioned elements

- Add PositionedBlock/PositionedInline decoration phases and PositionedText/PositionedImage
  layout items; all four variants retain index identity when line, atomic and block
  formatters merge their paint streams.
- Context records an output-order cursor and promotes absolute/fixed descendants to a
  foreground paint phase, so positioned backgrounds and glyphs/images paint after
  normal-flow text rather than beneath it. Standalone relative blocks are promoted so
  relatively shifted table captions can cover earlier absolute indicator boxes.
- Identified and corrected a WPT Positioning regression during implementation: blindly
  promoting all descendants of a relative parent drew its normal text over nested
  absolute children. A relative parent containing positioned outputs avoids whole-
  subtree promotion until the engine has a real nested stacking-context tree.
- Add end-to-end tests for absolute/fixed foreground paint, relative caption precedence
  and positioned image ordering after normal text. Frozen compatibility manifests hold
  WPT Static **187/200**, Positioning **38/100**, Test262 Parser **523/1983**.
- Known limitations: nested stacking contexts/source order, computed z-index, negative
  stacking levels and selective inline-relative context promotion remain future work.
- Final rustfmt --check, warning-free Clippy, all workspace tests, fixed WPT/Test262 suites,
  and native Win32 smoke passed. Optimized Windows release was built into the primary
  `target/release/op_browser.exe` path after the former process stopped. Superseded
  `op_browser_next.exe` was removed, avoiding confusion between generations.
  EXE: **1,058,816 bytes**; SHA-256
  `9D91731725C606C1FDD0C6B73A39BE27B38AF1ACDBE98060B096AB632A814304`.

## 2026-10-08 - Flat positioned z-index paint ordering

- Complete previously started computed `z-index`: signed integer levels, auto,
  inherit/initial/unset CSS keywords and rejection of fractional/dimension values.
- Carry the level into layout Style and tag positioned block/inline decorations,
  text and images with PaintKey, preserving nested independently positioned keys.
- Sort positioned paint groups by level and node creation order, then paint each
  group's decorations followed by text and images. Static outputs remain normal.
- CSS cascade, native paint order, reflow and same-level tie regressions added.
- Frozen WPT Static 187/200 (93.50%) and Positioning 38/100 (38.00%);
  zero render errors. Negative groups still paint over normal flow; nested atomic
  stacking contexts and real DOM-order painting remain future work.

## 2026-10-08 - Initial atomic block stacking contexts

- Create PaintGroup relationships from positioned ancestors with explicit computed
  z-index (or fixed positioning), using finalized DOM preorder rather than arena
  creation order. Derived metadata includes visually empty context ancestors.
- Promote explicit-z relative blocks as atomic groups even when they contain
  independently positioned descendants, while preserving nested output keys.
- Replace global flat paint ordering with an iterative parent/child traversal.
  Negative root groups now paint below in-flow block backgrounds; negative nested
  groups paint above their context background but below its inline foreground.
- Add regressions for high-z descendants inside low-z parents, empty context
  isolation, negative root-level layering, and HTML foster-parented DOM ties.
- Frozen WPT Static v1 remained 187/200 (93.50%) and Positioning v1 remained
  38/100 (38.00%), with no render errors. Auto-z edge cases, positioned inline
  contexts and full CSS painting phase interleaving remain unimplemented.

## 2026-10-08 - Relative inline z-index and atomic descendants

- Give relative inline boxes an optional PaintKey. Traverse InlineBoxes ancestry
  to tag fragment backgrounds, text, and images with the nearest positioned key.
  This retains layering across wraps and keeps nested explicit-z spans atomic.
- Auto-z relative inline groups do not trap explicitly stacked children; those
  children retain their own ancestor stacking-context relationships.
- Associate ordinary nested InlineAtomic decoration/text/image output with the
  enclosing inline key, preserving independently positioned nested records.
  Use PaintGroup.inline_owner to paint the relative inline background before
  nested inline-block backgrounds without reversing ordinary block paint order.
- Fix an index-out-of-bounds panic: local inline-block, inline-flex, and
  inline-table contexts now clear the outer inline box arena reference before
  formatting independent content.
- Add five engine regressions: explicit z-index layering and reflow, nested
  explicit-z containment, auto-z escape, wrapped fragment grouping, and atomic
  inline-block ordering. All Rust workspace tests and strict Clippy passed.
- Frozen WPT Static v1 remained 187/200 (93.50%); WPT Positioning v1 remained
  38/100 (38.00%); zero render errors on both. Full CSS paint ordering
  and all stacking-context triggers remain future work.

## 2026-10-08 - Relative auto-z paint order for blocks and inline-blocks

- Remove the previous shortcut that left relative blocks in the normal paint
  phase whenever they contained independently positioned children.
  Tag only the parent's own unpositioned decoration/text/image outputs with
  its PaintKey. Auto-z uses level zero and does not create an ancestor atomic
  context; explicit-z descendants retain their own keys.
- Verify CSS source-order ties with earlier z=0 absolute siblings for both
  relative blocks and relative inline-blocks. Both tests failed before the
  patch and pass after it. A positive child still escapes an auto-z parent's
  paint group; a negative child remains behind the parent's own background.
- Validate explicit-z inline-block atomic containment with higher-z descendants,
  including a reflow round trip.
- Add five focused engine regression tests. Entire Rust workspace test suite
  and strict Clippy pass. Frozen WPT Static v1 187/200 (93.50%), Positioning v1
  38/100 (38.00%), zero render errors. Remaining auto-z paint-phase interleaving
  and additional stacking context triggers still need work.

## 2026-10-08 - Relative table parts and intrinsic auto column widths

- Improve table auto column sizing: include explicit pixel-sized in-flow descendant
  blocks in cell intrinsic widths, then cap preferred auto column totals by
  available width instead of stretching all columns to the parent.
- Traverse each table cell's structural DOM ancestors to apply CSS relative
  offsets from table rows, row groups, headers, footers, and positioned cells.
  Translate decoration, text, image, and independently positioned outputs without
  changing normal grid flow placement. Positioned td backgrounds receive paint
  keys, and relative row/section backgrounds become separate painted rectangles.
- Diagnose three final single-pixel failures using rendered BMP comparisons:
  an empty row with only an absolute child was painting a 1px green section
  background. Suppress this visual artifact without removing the grid track.
- Temporarily establish a table-part containing block for absolute descendants
  during cell layout. The preceding tbody contributes the unshifted row origin,
  and its relative visual offset is applied after layout. This corrects tfoot
  children that previously ignored preceding row heights.
- Add integration regressions for relative tbody geometry, relative td background
  layering, tfoot containing-block coordinates, and no stray pixel for an
  absolute-only table row. Clippy is enforced without warnings.
- Frozen WPT Positioning v1 improves from 38/100 (38.00%) to 53/100 (53.00%):
  +15 passing tests. Frozen WPT Static v1 remains 187/200 (93.50%), no render
  errors on either suite. Further work remains for complex table spanning,
  row backgrounds, auto wrapper sizing, and exact CSS painting phases.

## 2026-10-08 - Auto table wrapper intrinsic-width alignment

- Fixed a real render inconsistency: auto table grid columns were already
  shrink-to-intrinsic, but the outer table background/borders and captions
  kept the parent's full available width. A 60px table painted as 300px.
- Resolve the wrapper's content width from the intrinsic column minimum and
  preferred widths before laying out its captions, border/background or cells.
  Include the table's horizontal border spacing, padding and border extras
  in the final geometry. Preserve authored explicit widths and min-width.
- Prevent table-layout:fixed with width:auto from accidentally switching to
  the fixed track algorithm after internal width resolution.
- Six engine regression tests cover one/two-column shrink-wrap, border-spacing
  and extras, authored table width, fixed-layout auto width and min-width.
- Frozen WPT Static v1 stays 187/200 (93.50%), Positioning v1 stays
  53/100 (53.00%), zero render errors. More table sizing and row background
  work remains.

## 2026-10-08 - Rowspan height reconciliation and row-origin correction

- Fix table layout treating a rowspan cell's entire natural height as the
  starting row's height, then adding later row heights again. Example:
  120px rowspan across two 20px rows produced a 140px table.
- Measure single-row cells and baselines first. Sort spanning cells by their
  final covered row; calculate the covered track height including internal
  border-spacing, and add shortages to the last covered track. Recalculate
  final row origins and shift subsequent cells' decoration, text and images
  without changing the measured cell content.
- Set rowspan decoration heights and vertical alignment from the corrected
  interval. Keep colspan intrinsic-width distribution unchanged.
- Add three end-to-end table regressions: two-row rowspan height, overlapping
  multi-row spans followed by another row, and vertical border-spacing and
  following-row placement. Verify the existing colspan regression as control.
- Frozen WPT Static v1 remains 187/200 (93.50%), WPT Positioning v1 remains
  53/100 (53.00%), both with zero render errors. Full CSS row-height
  distribution, row-group backgrounds and baseline corner cases remain future
  work.

## 2026-10-08 - WPT Static near-black colors, select text, and SVG text references

- Re-run pinned Static v1 baseline (187/200) and inspect all 13 failures.
- Diagnose OKLab/OKLCH with 0.0001% lightness: direct sRGB component clipping
  made visible colored pixels rather than black. Add dark-end chroma reduction
  only where achromatic output rounds to black in an 8-bit surface. Preserve
  bright out-of-gamut color handling for two already passing Display P3 green
  references; the initial unrestricted chroma-mapping prototype regressed
  those cases, so the final logic remains intentionally limited.
- Diagnose the 239-pixel select reftest difference to a directly rendered text
  node inside select without option; suppress that raw text in collection while
  preserving visible sibling content and hidden option handling.
- Diagnose the SVG reftest: defs and display:contents SVG text were being
  painted as regular text, while use did not expand the referenced text.
  Add bounded SVG ancestry and id lookup with basic use-to-text expansion,
  and limit rendering for SVG display:contents where required.
- Add focused engine regressions for select text and SVG definitions/use
  behavior, plus a color regression checking black at near-zero OKLab/OKLCH
  and preserved bright wide-gamut results. Pinned Static v1 now passes
  191/200 (95.50%), up from 187/200 (93.50%). Nine static tests remain.
- The remaining features include ICC color handling, tagged PNG backgrounds,
  full opacity/filter compositing, CSS :visited state, first-line effects and
  bidi. No WPT fixtures, thresholds or reference images were altered.
  Current CSS Color 4 Rec.2020 transfer is gamma 2.4, which differs from
  the expected colors in two pinned references. Further full-spec work is
  required for legitimate 200/200.

## 2026-10-08 - First nested group-opacity and filter compositing

- Investigate the nine remaining WPT Static failures. Confirm the contemporary
  CSS Color 4 Rec.2020 reference mandates a display-referred gamma of 2.4;
  preserve standards compliance rather than modifying historic WPT references.
- Add non-inherited computed CSS opacity and a single invert() filter with
  global keywords, percentage/number support, clamping, and a conservative
  parser that rejects unimplemented filter chains.
- Preserve effect ownership in layout paint groups, including static block
  owners and positioned child contexts. The paint builder wraps affected
  contexts in BeginLayer/EndLayer markers, retaining the stacking traversal.
- In the shared Win32 GDI painter, render nested groups to paired black/white
  offscreen DIBs, reconstruct alpha coverage, apply inversion to the
  premultiplied channels and composite once at the owning group opacity.
  Bounded layers use a 4M-pixel/4096px cap and a depth limit of 32. On
  failure, preserve document content with an unfiltered fallback.
- Add GDI tests proving overlapping rectangles retain uniform half-opacity,
  nested invert under parent opacity yields uniform light-blue, and CSS
  computed style tests for cascade/non-inheritance/invalid values.
- The previously failing WPT Static
  css/css-color/composited-filters-under-opacity.html now passes. Pinned WPT
  Static improves 191/200 -> 192/200 (96%), Positioning stays 53/100
  (53%), both with no render errors. ICC images, shadows, color gamut
  mapping, more filter functions and full SVG remain incomplete.

## 2026-10-08 - Self-link visited semantics, tiled CSS background URLs, ICC PNG conversion

- Start from commit eacdcc9, WPT Static 192/200 and Positioning 53/100.
- Treat an empty href as a known visited self-navigation to the current
  document while retaining unvisited matching for other URLs until history
  storage and history-leak-resistant styling exist. This closes the pinned
  color-mix-currentcolor-visited reftest. Selector tests cover empty href,
  external URLs, fragments and links without href.
- Add first CSS background-image URL support to the existing computed
  cascade. Track declaration stylesheet origin; handle !important, shorthand
  reset and explicit inherit. Feed URLs into the worker's budgeted page
  image loader without introducing a new fetch or rendering engine.
- Store decoded background image Arcs with ordinary block/table/cell
  decorations and emit a separate BackgroundImage paint command. Repeat
  natural image tiles with a saved GDI clip and a finite tile budget;
  retain the CSS background color and border paint phases. Tests verify
  cascade and visible pixel colors at tile and clip boundaries.
- Fix WIC ICC PNG handling: GetColorContexts first reports the number of
  contexts, but the caller must create IWICColorContext objects for the
  actual array before requesting profile contents. Transform embedded
  profiles to the sRGB destination using IWICColorTransform prior to
  PBGRA conversion. The pinned swapped-red-green PNG now decodes as
  green #009900, matching the tagged sRGB PNG. Conditional test checks
  the pinned WPT fixtures if present; normal codec tests remain independent.
- Frozen Static v1 improves 192/200 -> 194/200 (97.00%), with 0 render
  errors and no fixture, threshold or exact-comparison changes. Positioning
  remains 53/100. Remaining Static failures: CSS @color-profile,
  currentcolor-003 effects, XYZ (author-specified fuzzy not used by our
  exact comparator), bidi first-line, and two Rec.2020 reference cases.
- Limitations: only a single default-repeat image layer with no background
  position/size/multi-layer controls, no ICC @color-profile rule at this
  commit, and no private arbitrary visited URL history state.

## 2026-10-08 - Bounded CSS Color 5 named ICC profiles

- Start from 05919f9 with Static WPT 194/200 and Positioning 53/100.
- Implement a token-aware @color-profile --name { src:url(...) }
  discovery pass for inline/linked author styles, supporting quoted or
  unquoted URLs and ignoring unrelated CSS strings and invalid prelude
  names. Stop emitting an incorrect unsupported-at-rule diagnostic.
- Load ICC binary resources relative to each effective stylesheet
  address through the existing filtered network loader. Bound traversal
  (20k DOM nodes), profile requests (8), payload (1 MiB aggregate),
  and number of extracted declarations (16/sheet). Unknown or invalid
  profiles do not paint made-up colors.
- Extend op_image's WIC bridge to convert standalone 3-channel RGB
  source values to output sRGB through a 1x1 color-managed bitmap.
  Validate with pinned swapped.icc: [153,0,0] becomes [0,153,0].
- Before computed-style cascade, scan matched style declarations,
  including pseudo-elements, for valid color(--profile R G B) tokens.
  Resolve numeric and percentage channels to bytes; replace only the
  recognized function token span, leaving other values and CSS strings
  intact. Memoize conversions per prepared document.
- Add parser extraction, token-safe CSS rewriting and WIC conversion
  regressions. Frozen CSS color at-color-profile-001.html now passes:
  Static 194/200 -> 195/200 (97.50%) with no fixture edits. Remaining
  exact-pixel Static cases are currentcolor-003, predefined-012,
  rec2020-001, xyz-003, and first-line-bidi-002.
- Limitations: RGB-only custom profiles, no alpha/device-CMYK syntax,
  incomplete general CSS Color 5 handling; unchanged exact-pixel
  WPT comparator does not apply WPT fuzzy metadata.

## 2026-10-08 - First-line pseudo fragment painting and WPT bidi match

- Start from commit 6e5fddc with frozen Static WPT 195/200.
- Inspect the remaining five cases. Confirm actual XYZ blue = 244 vs
  the explicit reference = 245, while WPT's own maxDifference=0-1
  metadata permits such quantization; retain the strict unchanged
  pixel-exact checker. Confirm Rec.2020 gamma 2.4 remains specified by
  CSS Color 4 despite two earlier test reference values expecting brighter
  greens; do not introduce test-specific conversion constants.
- Extend CSS selector grammar with ::first-line and legacy :first-line,
  collect pseudo declarations in the author cascade, and resolve their
  fragment pseudo styles without creating generated DOM content.
- Pass an optional first-line text/background style into inline Lines
  only for the host block's final collected inline sequence. Apply the
  color to inheriting text runs only on the actual first flush. Draw the
  background as inline ink behind the first-line text, not subsequent
  lines. After comparing actual/reference paint screenshots, align the
  first-line rectangle to the same GDI text metrics as ordinary inline
  spans rather than using line-height leading.
- Add engine tests for a forced break (first-line green, second-line red)
  and reference-equivalent background height. The previously failing
  selectors/first-line-bidi-002.html now passes pixel-for-pixel.
- Pinned WPT Static improves 195/200 -> 196/200 (98.00%), no render
  errors, unchanged tests and thresholds. Four failures remain:
  currentcolor-003, predefined-012, rec2020-001 and xyz-003. The first
  still needs first-line currentcolor inherited decorations, gradients,
  outline/box/text/drop shadows; the others need precision/spec/reftest
  reconciliation without falsifying comparison scores.

## 2026-10-08 - Dynamic currentcolor in first-line inline decorations

- Start from commit 78dc150 and frozen Static WPT 196/200.
- Capture actual/reference currentcolor-003 renders: first and third
  lines had correct green text but red descendant inline backgrounds and
  borders, while the second line should remain red.
- Preserve currentcolor dependence in computed background-color and
  per-side border-color cascade results. Propagate these flags to inline
  box styles rather than treating matched explicit red as currentcolor.
- At the first actual line flush, re-resolve dependent background and
  border decoration colors only when the child inherits the host color
  changed by ::first-line. Leave explicit red paint, separately blue
  child text/backgrounds, and later-line ink unchanged.
- Add engine regression for dependent vs explicit colors, including
  forced line breaks and a separately colored child. The unchanged
  currentcolor-003 WPT test now matches pixel-for-pixel.
- Static v1 improves from 196/200 to 197/200 (98.50%). Three exact
  failures remain: predefined-012 and rec2020-001 (legacy green refs
  versus current CSS Color 4 gamma 2.4), and xyz-003 (one channel byte
  within its own WPT fuzzy metadata). Test fixtures, thresholds and
  manifest are unchanged.

## 2026-10-08 - Standard UA medium font and opt-in WPT fuzzy metadata report

- Start from bdbdb58, frozen strict Static 197/200 and Positioning 53/100.
- Verify the 2026-10-07 CSS Color 4 Candidate Recommendation explicitly
  specifies Rec.2020 BT.1886 gamma 2.4 (rather than camera OETF);
  keep legitimate color conversion instead of changing constants only
  to match two stale green-reference samples.
- Inspect xyz-003 original meta name=fuzzy:
  maxDifference=0-1;totalPixels=0-18432. Exact rendering differs
  only by one blue channel byte, but the 18px default made the
  box 23328 pixels, too large to qualify under the authored allowance.
- Normalize initial UA font-size 18px -> 16px, aligning with typical
  browser medium defaults and 12em x 6em = 192x96 = 18432 pixels.
  Update exact expected CSS rem, block, inline, line and typography
  test dimensions and add an explicit initial-font/rem regression.
  Frozen WPT Positioning remains 53/100; Static exact still 197/200.
- Add --report-wpt-fuzzy to the existing wpt_probe without changing
  default behavior. Use op_html::Tokenizer on the original WPT file,
  parse optional global/reference-specific fuzzy meta values (including
  inclusive ranges), and measure observed largest channel difference
  together with total differing pixels. Both authored bounds must pass.
  Unmarked tests receive no additional tolerance. Add tests for
  malformed bounds, overrides, inclusive ranges and independent limits.
- On the SAME pinned manifest and render output, report strict Static
  197/200 (98.5%), WPT metadata-aware 198/200 (99%), Positioning
  53/100. rec2020-001 and predefined-012 remain failures in both
  modes. No edits to frozen fixtures, manifest, output-pixel comparison,
  or original CI/JSON metric.

## 2026-10-08 - Documentation audit, Wiki refresh, connected Code Graph and Code Slicer

- Audit README, AGENTS.md, docs, 17 existing local Wiki pages, CI workflow
  and GitHub Wiki availability. Correct outdated project overview and
  compatibility descriptions, add current status, roadmap, code-intelligence
  and known-divergence Wiki navigation pages. Local Wiki now remains the
  version-controlled documentation source until the separately hosted
  OPBrowser.wiki.git repository is initialized.
- Verify the current W3C CSS Color 4 CRD (7 October 2026) defines Rec.2020
  as BT.1886 display-referred gamma 2.4. Classify rec2020-001 and
  predefined-012 as known non-blocking legacy-reference divergences
  without deleting/skipping their WPT entries, making them pass in a
  reporting branch, or changing RGB conversion for test-specific output.
  Review this determination on any spec, WPT revision or numeric
  conversion change.
- Reiterate exact Static 197/200, separate WPT-authored fuzzy Static
  198/200, Positioning 53/100 and Test262 Parser 523/1983. These are
  narrow pinned suites, not browser completion estimates.
- Add Python-standard-library-only tools/code_intelligence.py: derive
  Cargo crate dependency graph and a Mermaid report; validate feature
  slice paths, Rust source anchors, unique slice IDs and multiple-crate
  coverage from tools/code_slices.json. Generate committed Markdown under
  docs/GENERATED_CODE_GRAPH.md and docs/GENERATED_CODE_SLICES.md.
- Wire --check and unit tests into the Windows CI job, validating
  generated report freshness, source anchors and internal Wiki links.
  Preserve manually maintained CODE_GRAPH.md and CODE_SLICES.md
  for design intent and complex architectural ownership not inferable
  from Cargo manifests.
- Document why GitHub Wiki cannot yet be populated: its separate git
  remote is not initialized/available. Local Markdown pages are ready
  to publish once that repository is created.

## 2026-10-08 - Publish OPBrowser Wiki to its own Git repository

- User enabled the GitHub Wiki feature and initialized the first Home page.
  The newly available OPBrowser.wiki.git remote cloned successfully with
  the initial Home-only commit (501525f).
- Added a version-controlled _Sidebar.md to the local Wiki source, grouping
  project overview/status, architecture, implementation, and testing
  documentation. The published Wiki has 22 ordinary pages plus the sidebar.
- Added tools/publish_wiki.py, a bounded source-to-Wiki synchronizer that
  rewrites relative Markdown page links to GitHub Wiki URLs and repository
  docs links to the main branch, without deleting unrelated Wiki pages.
  The tool requires a checkout of the separate Wiki Git repository and
  supports --write/--check; it never performs Git commits automatically.
- Synced 23 Markdown files into the Wiki checkout and pushed Wiki commit
  40b3655 to master. Verified the remote HEAD and clean Wiki checkout.
- Updated the main-repo Home, workflow documentation, README and project plan
  to reflect that the Wiki is now published, rather than pending.
- Added publisher unit tests for page links, docs links, unchanged external
  links, missing references, and complete source rendering. The main repo
  remains canonical; Wiki updates must be committed/pushed independently.

## 2026-10-08 - First inline JS to real DOM and native paint (M4.1)

- Introduced native host binding to op_js::JsRuntime:
  document.getElementById returns an owned element view, and writing
  element.textContent produces detached, bounded mutation records.
  No ready-made JS engine, borrowed DOM pointer or unsafe bridge.
- op_dom::Document::set_text_content detaches an element's old child
  subtree and inserts an ordinary DOM Text node. op_engine::scripts
  discovers classic inline scripts, skips src/module/non-JS cases,
  shares one runtime between scripts and applies mutations between
  them before author CSS/computed styles, image loading and layout.
- Limits include 16 scripts, 128 KiB/script, 50,000 VM instructions
  per script, 4096 DOM snapshot nodes, 256 mutations/script,
  and 64 KiB per mutation; runtime heap/call depth limits still apply.
  Errors are non-fatal and reported as ScriptReport counts.
- Tests prove DOM mutation affects display-list text for an actual
  data:text/html navigation and persists after retained reflow;
  VM tests cover live text reads, missing ids and oversized text.
  Added standalone examples/js/dom-text.html for native inspection.
- Strict Static WPT unchanged at 197/200 (98.50%), author-fuzzy
  198/200 (99.00%), Positioning 53/100 with zero render errors.
  HTML5test still cannot calculate its score: the site's external
  JavaScript, event lifecycle, and numerous DOM/Web APIs are absent.
- Caveat: all classic inline scripts currently run after the full DOM
  has been parsed rather than parser-blocking; no async scripts, event
  handlers, script.src, DOM creation or security-origin model yet.

## 2026-10-08 - M4.2 external classic JS in native resource pipeline

- Build a dedicated Script resource path in op_net, with explicit
  `ResourceType::Script` filtering and WinHTTP JavaScript Accept headers,
  bounded file reads, encoding/MIME verification and an independent
  ScriptTooLarge error. Reject cross-origin requested sources and
  cross-origin final redirected URLs; HTTPS downgrade remains blocked.
- Preserve DOM order of classic inline/external/inline scripts in one
  shared bounded op_js::JsRuntime; apply DOM text mutations after
  each script and continue on missing/failed external resources.
- Bound requests to 8, external text to 512 KiB total and 128 KiB
  per script, 16 total selected scripts and a 10-second request phase.
  Async/defer/integrity scripts are skipped pending correct scheduling
  and subresource integrity enforcement.
- Add WinHTTP loopback end-to-end test, filesystem source order/reflow,
  non-fatal 404/missing source behavior, same-origin rejection and
  request filter blocking test. Add examples/js/external.html and .js.
- Frozen WPT Static remains 197/200 strict (198/200 metadata-aware);
  WPT Positioning remains 53/100, zero render errors. Code Slicer S8
  and Wiki updated.
- Remaining gaps: no parser-blocking lifecycle, native event loop,
  addEventListener, full Web API set, CSP/CORS/SRI or HTML5test score.
  Redirects may be fetched before cross-origin final-URL rejection.

## 2026-10-08 - M4.3 native click events to retained JavaScript VM

- Keep a per-page JsRuntime alive beyond initial script loading, with
  document IDs, closures and global state. Introduce DOM click handler
  storage for `element.addEventListener("click", fn)` and `onclick`
  property replacement/removal; cap listener registrations.
- `JsRuntime::dispatch_dom_click` calls retained functions under
  instruction/call-depth budgets, exposes basic event type/target and
  `this` binding, and queues bounded DOM text mutations.
- Record id-bearing block hit regions in op_layout, taking relative
  positioning into account. Win32 WM_LBUTTONUP, when not a hyperlink,
  converts the mouse point to document pixel coordinates including
  toolbar and wheel scroll. Worker `Engine::click_at` dispatches
  callbacks, mutates the retained DOM, recomputes styles, and reflows
  without changing URL/history or resetting scroll.
- Added repeated-click and onclick replacement VM regressions, engine
  click target/reflow regression and external-JS click sample
  `examples/js/click.html` / `click.js`.
- Strict Static WPT 197/200 and metadata-aware Static 198/200 remain
  unchanged; Positioning WPT remains 53/100 with no render errors.
- Scope is intentionally a first event slice, not DOM Events
  conformance: no capture/bubble, preventDefault, keyboard activation,
  full element hit-testing, class/style mutation or HTML5test score.

## 2026-10-08 - Fix Windows GitHub Actions Code Slicer validator failure

- Investigated five failed main-branch Actions runs (74f48ec through
  bbb6ce1). All Windows/Rust jobs stopped at Python unit test
  `test_rejects_stale_symbol_and_single_crate_slice`, before Rust checks.
  The parallel compatibility job succeeded; no Rust failure was reported.
- Cause: temporary fixture's source root was patched globally and compared
  without normalizing path identity on a clean Windows runner. The fixture
  path was rejected as outside the declared root. The user saw repeated
  GitHub failure notifications after every push.
- Fix: give `slices_report` an explicit optional source root and resolve
  it canonically. Tests pass that root directly, avoiding global state
  patching and dependency on Windows temporary directory spelling.
  Production default still uses the repository root; generated outputs
  remain unchanged.
- Verify Python suite and `code_intelligence.py --check` locally,
  including invocation from a different working directory. Await fresh
  hosted Windows CI confirmation after push.

## 2026-10-08 - M4.4 bubbling click dispatch and listener removal

- Extended native hit-testing to select the smallest id-bearing block under a click even when only an ancestor has a listener.
- Engine walks the attached DOM parent chain (bounded to 64 elements); the retained original JS VM dispatches click callbacks at target and along ancestors.
- Added event.bubbles, event.eventPhase (2 target, 3 bubbling) and changing currentTarget/this with stable target.
- Implemented removeEventListener("click", callback) by function identity without changing the existing callback deduplication and budgets.
- Added VM regressions for bubbling/removal and an Engine integration regression for child-to-parent dispatch through DOM mutation, layout and paint.
- Added a native interactive example and updated plan, Code Graph/Slicer, README and Wiki.
- Remaining limits: no capture, cancellation, keyboard, listener options, default actions, full hit-testing or complete script scheduling.

## 2026-10-08 - M4.5 capture and cancellation event slice

- Added boolean capture listener registration/removal and three-phase
  native click dispatch: capture, target and bubble.
- Added stopPropagation and preventDefault event methods plus cancelable
  and defaultPrevented state.
- Added VM regressions for exact phase order and cancellation semantics.
- Native default actions are not yet connected to preventDefault; listener
  option objects, once/passive, stopImmediatePropagation and keyboard
  dispatch remain future work.

## 2026-10-08 - M4.6 parser-blocking classic script integration

- Exposed an op_html tree-builder callback after closing script elements,
  so op_engine executes classic inline and filtered same-origin external
  sources before parsing subsequent elements into DOM.
- Kept the page's own JS runtime across parser pauses and refreshed
  reachable DOM snapshots before scripts and at final parsing completion.
  Retained click listeners can now resolve elements inserted after the
  registering script.
- Added parser/engine integration regressions for invisible future nodes,
  flushed script source, shared globals, partial textContent, and retained
  callbacks referencing late elements.
- The tokenizer is still eager; document.write, reentrant parsing, async,
  defer, integrity verification and full HTML loading events are absent.

## 2026-10-08 - M4.7 bounded external defer/async script scheduler

- Added scoped parallel source fetching for classic defer and async scripts
  through the existing filtered/same-origin network loader.
- Deferred scripts run after DOM completion in source order; async scripts
  run in download completion order at parser/end-of-load polling points.
- JS execution and DOM mutations stay on one engine thread; in-flight
  fetches reserve a share of the aggregate external byte budget.
- Added local-file regressions and WinHTTP slow/fast async ordering test.
- Browser event loop, readyState/DOMContentLoaded/load, document.write,
  modules, SRI and interactive async dispatch are still future work.

## 2026-10-08 - M4.8 document lifecycle

- Added document.readyState loading/interactive/complete phases.
- Wired readystatechange, DOMContentLoaded and load dispatch to retained VM.
- Added lifecycle listener APIs and callback mutation synchronization.
- Added engine tests for event ordering, state, listener removal, and
  deferred-script registration. The browser event loop remains future work.

## 2026-10-08 - M4.8 document lifecycle events

- Added host-owned document.readyState updates (loading, interactive,
  complete), lifecycle registration/removal, and onreadystatechange.
- Added window.addEventListener/removeEventListener for load and onload,
  using non-bubbling lifecycle event objects with correct receiver binding.
- Engine dispatch now follows parser completion, deferred scripts,
  DOMContentLoaded, pending async completion, complete and window load.
  Listener DOM mutations apply before initial page presentation.
- Added regressions for state/event order, receiver, listener removal,
  immutable readyState and defer listeners registered before DOMContentLoaded.
- Full event loop, timers, modules, document.write, post-presentation async
  work and script-resource events remain future work.
