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
- BLOCKED GitHub Actions Windows CI execution: the workflow is configured, but
  GitHub currently refuses to start jobs because the account is locked due to a billing issue.

## M1 - First static document pipeline

Goal: own bytes -> own HTML parser -> own DOM -> own layout -> own paint -> pixels.

- DONE initial HTML tokenizer state machine.
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
- DONE first/last/only-of-type and nth-of-type/nth-last-of-type share sibling indexing with
  same-tag filtering, token-aware An+B grammar and normal pseudo-class specificity.
- NEXT generated `url()` content and fuller replaced-content/nested-inline geometry.
- LATER language-aware `quotes:auto` (currently deterministic English Unicode pairs).
- LATER broader custom-property grammar/registration/animation-taint behavior,
  relational selectors, nested decorated-inline
  stacks/replaced inline decorations and fuller
  parent/child margin collapsing / definite percentage-height propagation.
- LATER broader computed values.
- LATER normal flow block layout.
- LATER full CSS inline formatting and Unicode line breaking (initial M1 subset exists).
- LATER fonts/text shaping integration.
- LATER progressively expand CSS WPT coverage.

## M3 - Original JavaScript engine

- LATER ECMAScript lexer.
- LATER parser/AST.
- LATER bytecode format/compiler.
- LATER bytecode interpreter.
- LATER values/objects/prototypes.
- LATER garbage collector.
- LATER functions/closures.
- LATER exceptions.
- LATER promises/microtasks.
- LATER modules.
- LATER standard built-ins.
- LATER Test262 harness and progressive conformance.
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

- LATER browser/renderer/network process separation.
- LATER renderer sandbox.
- LATER process/site isolation policy.
- LATER tab model and session restore.
- LATER lifecycle states: active/background/throttled/frozen/discarded/restoring.
- LATER intelligent memory-pressure tab discarding.
- LATER built-in task manager.
- LATER downloads/history/bookmarks/settings/permissions.
- LATER native request-filter layer and ad-block list support.

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
memory use, binary size, background CPU, and dependency growth.
