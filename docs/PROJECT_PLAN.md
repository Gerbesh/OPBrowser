# OPBrowser Project Plan

Last updated: 2026-10-06

## North star

Build an independent, lightweight, privacy-oriented Windows 11 browser with an
original web engine and original ECMAScript engine, while progressively reaching
modern web-platform conformance.

Primary conformance targets:

- HTML5test feature coverage: full target, never faked.
- Web Platform Tests: primary browser-platform conformance signal.
- TC39 Test262: primary ECMAScript conformance signal.

## Status legend

- DONE: implemented and verified at the current milestone level.
- IN PROGRESS: active engineering work.
- NEXT: queued immediately after active work.
- BLOCKED: configured or implemented, but prevented by an external dependency/state.
- LATER: planned but intentionally deferred.

## M0 - Foundation

Status: DONE at initial level.

- DONE Rust workspace under C:\OPBrowser.
- DONE Native Win32 top-level browser window.
- DONE Separate browser, engine, DOM, HTML, CSS, layout, paint, JS, network, and Windows crates.
- DONE Git repository.
- DONE rustfmt / Clippy / test workflow.
- DONE debug and release builds.
- DONE non-interactive Win32 startup/paint smoke test.
- DONE documented dependency policy.
- DONE project documentation workflow, code graph, code slices, and local wiki.
- DONE public GitHub repository: https://github.com/Gerbesh/OPBrowser
- DONE continuous Git commit/push workflow to public main.
- IN PROGRESS GitHub Actions Windows CI is restored for main pushes, pull requests and manual
  dispatch. Superseded runs are cancelled automatically; live runner/account verification is
  pending the first post-restore push.

## M1 - First static document pipeline

Goal: own bytes -> own HTML parser -> own DOM -> own layout -> own paint -> pixels.

- DONE initial HTML tokenizer state machine.
- DONE owned comment tokenizer states, malformed closing/EOF recovery and invisible
  comment handling through tree construction, CSS :empty and retained painting.
- DONE typed doctype tokens with name/PUBLIC/SYSTEM/force-quirks recovery and bogus
  HTML declarations as invisible comments, preserving raw-text/RCDATA and reflow.
- DONE DOM comment and DocumentType nodes with preserved token data; comments split DOM
  text at their real position while CSS/layout keep them non-rendering and :empty-neutral.
- DONE WHATWG doctype-driven DocumentMode selection for no-quirks, limited-quirks and
  quirks, including the complete legacy public/system identifier compatibility matrix.
- DONE first real tree-construction insertion-mode slice: initial/before-html/before-head/
  in-head/after-head/text/in-body, implicit html/head/body creation, head-token routing,
  duplicate html/body attribute merging and HTML non-void self-closing recovery.
- DONE in-body structural recovery slice: normal/list-item/button scope checks, implied
  end tags, paragraph/block autoclosing, li/dd/dt and heading recovery, nested-button
  recovery, generic end-tag special-boundary handling, </br> and legacy <image> recovery.
- DONE after-body/after-after-body insertion modes: </body>/</html> switch parser state
  without destroying the recovery stack, comments land on html/document as specified,
  whitespace/html tokens delegate to in-body and stray trailing content re-enters in-body.
- DONE active formatting elements/adoption-agency slice: reconstruction for the HTML formatting
  tag set, Noah's Ark three-entry cap, repeated-anchor/nobr recovery, applet/marquee/object
  marker boundaries and furthest-block DOM reparenting for misnested formatting.
- DONE table tree-construction slice: in-table/text/caption/column-group/table-body/row/cell
  insertion modes, implicit tbody/tr recovery, table-scope cleanup, cell formatting markers,
  pending table-text buffering and foster parenting before the last table.
- DONE CSS table sizing/alignment follow-up: table display roles/UA defaults, captions, row groups,
  2D row/cell placement, colspan/rowspan occupancy, content-driven min/max track sizing from
  measured text/images/cell constraints, col/colgroup width hints, inherited border-spacing,
  initial collapsed cell-edge conflict resolution and table-cell baseline/top/middle/bottom
  vertical alignment, all through native paint.
- DONE initial anonymous-table fixup: child-side repair creates missing rows/cells inside table
  roots/row groups, while normal-flow sibling collection groups consecutive orphan table-internal
  boxes under one layout-only anonymous table wrapper. Orphan captions join the repaired wrapper
  and orphan columns contribute track width hints. No synthetic DOM nodes are created and
  whitespace/display:none separators do not split a repaired sibling run.
- DONE initial true inline-table formatting: display:inline-table creates one atomic inline object
  backed by the normal table_box/grid formatter, uses first-row baseline alignment, shrink-to-fit
  auto width, authored box model/margins and transfers nested text/images/link identity into paint.
- DONE atomic inline-table vertical alignment for baseline/top/middle/bottom; line ascent/descent
  expands for tall top/bottom-aligned atoms and final nested paint output follows the aligned box.
- DONE initial advanced table width/caption pass: auto layout retains percentage constraints from
  col/colgroup/cell widths, table-layout:fixed uses column hints then first-row widths before sharing
  remaining space, and caption-side:top/bottom places captions outside the table border/background.
- LATER mode-specific legacy CSS/layout quirks, processing instructions, foreign content/CDATA,
  template/frameset insertion modes, complete script-data escape states and remaining advanced
  table layout: complete CSS Tables overconstraint/min-width/percentage edge algorithms,
  table/row-group/row/column collapsed-border conflict precedence and deeper anonymous colgroup
  repair. General inline vertical-align still needs sub/super/text-top/text-bottom/length/% support.
- DONE initial DOM arena with stable NodeId values, attributes, and parent/child relationships.
- DONE initial HTML tree builder from tokenizer output into op_dom::Document.
- DONE initial document-to-layout pipeline with basic text flow and wrapping.
- DONE initial paint/display-list primitives.
- DONE Win32 GDI backend consuming OPBrowser display-list commands.
- DONE first in-memory HTML page rendered by the complete OPBrowser pipeline.
- DONE smoke test verifies WM_PAINT actually ran.
- DONE local filesystem document loading.
- DONE file: URL loading with percent decoding.
- DONE data:text/html URL loading with percent and base64 decoding.
- DONE startup source argument wired through op_net -> engine -> renderer.
- DONE navigation history core with navigate/back/forward/reload.
- DONE failed navigations leave history unchanged.
- DONE new navigation after Back discards the old forward branch.
- DONE post-startup native address input, Go/Back/Forward/Reload, display replacement.
- DONE initial owned HTTP(S) URL parser (ASCII hosts, ports, IPv6, UTF-8 paths/query).
- DONE bounded HTTP(S) HTML fetching using WinHTTP transport and system TLS/proxy.
- DONE worker-thread loading with visible loading/errors and history rollback.
- DONE mouse-wheel scrolling and block defaults inside structural HTML containers.
- DONE external https://example.com navigation + native repaint verified.
- DONE clickable text hyperlinks with measured hit regions and scroll-aware input.
- DONE relative HTTP(S)/local-file link resolution using the loaded document base.
- DONE link input -> worker -> load -> history -> pixels smoke coverage.
- DONE owned UTF-8/UTF-16/Windows-1251/Windows-1252 document decoding.
- DONE BOM -> transport -> first-1024-byte meta charset selection and label aliases.
- DONE common named and numeric references in HTML text/attributes, including hrefs.
- DONE initial raw-text/RCDATA tokenizer handling to preserve script/style text.
- DONE Windows-1251 source -> Cyrillic pixels -> decoded hyperlink smoke coverage.
- DONE full HTML named-reference table, two-scalar results and longest-match lookup.
- DONE exhaustive named-reference text/attribute/RCDATA tests and native link smoke.
- NEXT broader encoding/URL conformance.
- DONE basic PNG/JPEG/GIF/BMP image subresources from HTTP(S), local files and data URLs.
- DONE bounded worker loading, per-page reuse, alt fallback, dimensions and alpha painting.
- DONE clickable image links with scroll-aware native hit regions and image smoke tests.
- DONE initial mixed text/image lines, measured font extents, baseline alignment and wrapping.
- DONE inline sibling grouping around blocks, HTML whitespace/NBSP and explicit br breaks.
- NEXT progressive image loading, animation and additional formats.
- DONE resize reflow from retained DOM/images without refetch or history mutation.
- DONE debounced resize input, stale-width result suppression, scroll/link-region updates.
- NEXT richer CSS block/inline layout and shaping.

Exit condition: OPBrowser renders a non-trivial local HTML document using only its
own HTML/DOM/layout/paint pipeline. Achieved at the initial M1 level. The current
iteration also opens external HTML sites, including declared Windows-1251 pages,
through the address bar and renders raster images beside measured text; full CSS,
image animation, scripting and modern-site compatibility remain later milestones.

## M2 - CSS foundation

Status: IN PROGRESS.

- DONE initial owned CSS tokenizer with spans, comments, strings/escapes, identifiers,
  hashes, numbers, percentages, dimensions, functions and structural tokens.
- DONE stylesheet and declaration-list parsers with bounded error recovery, !important
  extraction and custom-property name preservation.
- DONE initial selector AST/parser for type, universal, class and ID simple selectors,
  selector lists, descendant/child combinators and specificity calculation.
- DONE collect CSS from embedded style elements and inline style attributes.
- DONE match the supported selector subset against op_dom and retain per-node StyleMap
  candidates with specificity, source order, stylesheet/inline source and parse errors.
- DONE Engine retains the author StyleMap beside DOM/images across resize reflow.
- DONE initial author cascade over !important, inline-vs-stylesheet source, specificity
  and source order; invalid literal supported-property values are ignored before winner choice.
- DONE initial inheritance/global keywords plus ComputedStyleMap for display, color,
  font-size and font-weight, with temporary UA defaults matching the M1 layout baseline.
- DONE Engine retains computed styles across resize reflow.
- DONE feed computed display/font-size/font-weight/color into layout and paint, including
  mixed inline style runs sharing one line/baseline and display:none/block/inline flow.
- DONE render CSS text color through the platform-neutral display list; alpha colors are
  composited against the current white page background.
- DONE load bounded external `<link rel="stylesheet">` resources from local/file/data/HTTP(S)
  through op_net, preserve their DOM source order with embedded styles, and retain the result
  across reflow; failed stylesheet subresources are nonfatal.
- DONE initial block-level box model: `margin`/`padding`, `background-color` and solid/none
  borders reach computed style, layout geometry, display-list fills and native paint.
- DONE expanded block sizing/value layer: margin/padding side longhands, border side and
  width/style/color shorthands/longhands, `width`/`height` + min/max, `box-sizing`, auto
  horizontal margins, negative margins, percentages, em/rem and CSS absolute length units.
- DONE adjacent sibling vertical margin collapsing (positive/negative combinations) and
  block-boundary whitespace suppression; parent/child/empty-block collapsing remains later.
- DONE move the previous temporary heading/paragraph/list spacing into computed UA margins
  so normal flow consumes one box-model spacing path instead of separate semantic offsets.
- DONE expand selector syntax/matching with attribute selectors (`[a]`, =, ~=, |=, ^=,
  $=, *=, i/s flags), adjacent/general sibling combinators and structural pseudo-classes
  :root/:first-child/:last-child/:only-child/:empty/:link.
- DONE expand color values with legacy/modern `rgb()`/`rgba()` and `hsl()`/`hsla()`,
  percentage/alpha channels, hue angle units, clamping, CSS basic named colors plus
  `rebeccapurple`, and functional colors in border shorthand/longhands.
- DONE modern `hwb()` percentage/number whiteness and blackness, gray normalization,
  hue units, alpha and used-value none components through text/background/border/pseudo paint.
- DONE all 148 opaque CSS named colors and aliases from pinned W3C data, allocation-free
  case-insensitive lookup, 2,210-byte static table and reproducible offline generation/CI check.
- DONE `color(srgb ...)` and `color(srgb-linear ...)` with percentage/number channels,
  optional alpha/none, linear-to-encoded transfer and initial 8-bit channel clipping.
- DONE modern RGB/HSL none components and numeric HSL saturation/lightness on the percent
  reference scale; strict legacy RGB uniform channel units and comma-HSL percentage grammar.
- DONE richer initial typography: inherited `text-align` start/end/left/right/center,
  `line-height` normal/number/percent/length with real line-box geometry, and numeric
  `font-weight` 1-1000 plus bolder/lighter mapped onto the current normal/bold backend.
- DONE `font-style` normal/italic/oblique through GDI font realization/measurement, initial
  `text-decoration`/`text-decoration-line` underline + line-through native painting, and
  `white-space` normal/nowrap/pre/pre-wrap/pre-line behavior in the owned line formatter.
- DONE inherited `letter-spacing`/`word-spacing` lengths plus `text-transform`
  none/uppercase/lowercase/capitalize; transformed Unicode text is measured before layout,
  spacing affects wrapping/alignment/link bounds, and Win32 paints matching visual advances.
- DONE initial inline box fragments for non-replaced inline elements: background-color,
  padding and solid per-side borders participate in width/wrapping/alignment, expand safe
  line geometry, survive ordinary nested text styling and emit per-line BoxDecoration paint.
- DONE functional pseudo-classes `:is()`/`:where()`/`:not()` with nested selector lists and
  correct specificity rules, plus `:nth-child(An+B)` over element siblings and a color-only
  `background` shorthand sharing cascade priority with `background-color`.
- DONE terminal `::before`/`::after` pseudo-elements with type specificity, separate author
  cascade buckets and computed pseudo styles; quoted-string `content` enters the normal inline
  formatter with inherited typography plus its own color/background/padding/solid borders.
- DONE inherited case-sensitive CSS custom properties with author cascade/`!important`,
  computed per-element token values, bounded `var(--name, fallback)` substitution, directed cycle
  invalidation and pseudo-element inheritance/overrides before normal property value parsing.
- DONE generated `content` functions for `attr(name)`, `counter()` and `counters()` plus
  initial `counter-reset`/`counter-set`/`counter-increment`, nested counter scopes and decimal,
  decimal-leading-zero, alpha/latin and roman formatting.
- DONE inherited `quotes` auto/none/string pairs, generated open/close/no-open/no-close quote
  commands and UA `<q>` pseudos; nesting follows emitted document-order content, repeats the
  deepest pair and ignores hidden/absent pseudos and subtrees for quote/counter mutation.
- DONE generated `display:block` shares ordinary block sizing/margins/padding/borders,
  including percentage/min/max sizes, auto margins, wrapping and empty decorated block boxes.
- DONE definite block height controls flow/decorations even when text overflows.
- DONE iterative custom-property dependency graph/SCC resolution including unused fallback
  edges and self-cycles, valid empty custom values and bounded expansion/storage/depth.
- DONE invalid-at-computed-value-time var() winners resolve to unset for supported properties,
  including shorthand components/pseudo content/quotes/counters; malformed var() is rejected
  before cascade. Literal invalid values remain parse-time exclusions.
- DONE forgiving is()/where() selector-list recovery, strict not()/of lists, filtered
  nth-child/nth-last-child with maximum-filter specificity and token-aware An+B grammar;
  functional selector nesting is bounded to 64 levels.
- DONE empty generated/DOM inline decoration boxes reserve edges, wrap/align and paint
  without fabricated text commands; nowrap and line extents use the existing formatter.
- DONE empty inline frames with hidden/empty descendants or only collapsed whitespace;
  block boundaries suppress duplicate empty-fragment synthesis and preformatted spaces remain text.
- DONE first/last/only-of-type and nth-of-type/nth-last-of-type share sibling indexing with
  same-tag filtering, token-aware An+B grammar and normal pseudo-class specificity.
- DONE generated `url()` images in ordered before/after text/image lists, stylesheet-relative
  bases including redirects/var() consumers, shared DOM/generated resource budgets/cache,
  intrinsic inline/block image flow, generated image links and retained reflow.
- DONE DOM image CSS width/height/min/max/box-sizing, inherited font-relative and percentage
  width resolution, HTML size hints before author cascade and intrinsic ratio constraints.
- DONE sole-URL inline generated image replacement with CSS width/height/min/max/box-sizing,
  own padding/background/borders, intrinsic ratio constraints and retained image link identity.
- DONE shared block DOM/sole-URL replaced-image geometry with intrinsic/CSS width, auto
  margins, padding/borders, precise box height and adjacent vertical margin collapsing.
- DONE nested decorated-inline stacks with parent-linked arena indices, cumulative edge
  geometry, shared text/image/empty/pseudo fragments and outer-before-inner background paint.
- DONE unavailable sole-URL/empty-or-absent-alt image geometry with zero natural sizes,
  independent CSS axes, transparent atomic/block boxes and no fake raster allocation.
- DONE styled nonempty alt fallback through ordinary inline fragments or block sizing;
  mixed generated failures skip only images while retaining empty pseudo decorations.
- NEXT sliced inline decoration edges and broader computed values.
- DONE DOM image padding/background/solid borders as atomic inline boxes, edge-aware
  width fitting, border-box baseline extents, text alignment and nowrap behavior.
- DONE declaration source-node provenance and retained effective external stylesheet
  addresses across redirects, duplicate link reuse and resize reflow.
- DONE CSS Url/BadUrl tokenization with escapes, punctuation-preserving addresses/data URLs,
  quoted function distinction, bounded bad-URL recovery and declaration rejection.
- DONE computed UA link color/underline defaults and author overrides through native GDI,
  including nested/generated text and retained reflow with unchanged link hit identity.
- LATER language-aware `quotes:auto` (currently deterministic English Unicode pairs).
- LATER broader custom-property grammar/registration/animation-taint behavior,
  relational selectors, sliced inline decoration edges and fuller
  parent/child margin collapsing / definite percentage-height propagation.
- IN PROGRESS readable-static-web priority: computed `position: static|relative` plus
  `top/right/bottom/left` insets now exist with cascade/global-keyword tests; applying relative
  offsets in layout is still NEXT, followed by absolute/fixed/sticky, `overflow`, floats,
  flexbox, media queries, font faces, background images, border radius and shadows.
  Grid follows the first usable flex slice; obscure value-space polish must not displace these.
- LATER broader computed values outside the readable-static-web priority.
- LATER fuller normal flow and CSS inline formatting plus Unicode line breaking.
- NEXT migrate the text backend toward DirectWrite shaping/fallback behind TextMeasurer per ADR-0003.
- NEXT wire a static HTML/CSS WPT subset; progressively expand CSS WPT coverage from measured results.

## M3 - Original JavaScript engine

Status: IN PROGRESS.

- DONE initial owned ECMAScript lexer for scalar literals, identifiers/keywords, comments and
  arithmetic/comparison/assignment punctuation.
- DONE initial parser/AST for single `let`/`const`/`var` declarations, assignment, unary,
  arithmetic, comparison/equality and scalar literals.
- DONE initial bytecode format/compiler for the implemented syntax.
- DONE initial stack interpreter with persistent globals, mutable/const bindings, scalar
  coercion, arithmetic, string concatenation, truthiness and loose/strict equality.
- NEXT values/objects/prototypes beyond the current primitive JsValue set.
- LATER garbage collector.
- NEXT functions/closures and lexical environments.
- NEXT exceptions and control flow.
- LATER promises/microtasks.
- LATER modules.
- LATER standard built-ins.
- DONE initial Test262 parse-expectation probe and combined compatibility command; this is
  explicitly not runtime conformance yet.
- NEXT build the real Test262 harness progressively as language/runtime semantics land.
- LATER JIT only if profiling justifies it after correctness.

## M4 - DOM scripting and Web APIs

- LATER Web IDL binding layer.
- LATER DOM mutation/events.
- LATER timers and event loop.
- LATER Fetch.
- LATER URL/Encoding/Streams.
- LATER forms/editing.
- LATER storage.
- LATER workers/service workers.
- LATER WebSocket.
- LATER Canvas/SVG/MathML.

## M5 - Browser product architecture

Status: IN PROGRESS (foundations only; current UI still exposes one renderer/tab).

- DECIDED browser/renderer ownership and staged process split in ADR-0002; current navigation
  worker remains a temporary in-process renderer boundary.
- NEXT implement browser/renderer IPC and one renderer process per active tab.
- LATER renderer sandbox and process/site isolation policy after IPC is stable.
- DONE UI-independent `op_browser_core` tab model foundation.
- DONE lifecycle states: active/background/throttled/frozen/discarded/restoring.
- DONE initial discard protection and memory-pressure candidate policy with retained restore state;
  OS memory-pressure triggering and renderer teardown/restoration remain later.
- NEXT connect multiple tab-owned renderers to the native UI and session persistence.
- LATER built-in task manager using browser/renderer memory/CPU counters.
- LATER downloads/history/bookmarks/settings/permissions.
- DONE initial native request-filter layer on document/stylesheet/image loads with host/wildcard
  rules, exceptions, resource types, site allowlisting and counters.
- NEXT scalable filter indexing, list subscriptions/updates, third-party/domain options and UI;
  cosmetic filtering depends on DOM/style integration.

## M6 - Advanced platform

- LATER WebAssembly.
- LATER WebGL.
- LATER WebGPU.
- LATER media pipeline.
- LATER accessibility tree.
- LATER advanced security/isolation.
- LATER performance and power optimization.

## Continuous work

Every milestone continuously tracks security, WPT/Test262 regressions, startup time,
memory use, binary size, background CPU, and dependency growth. `tools/compatibility.ps1`
provides the project-owned baseline plus an optional Test262 parse probe; owned unit-test
counts must not be mislabeled as external conformance percentages.
