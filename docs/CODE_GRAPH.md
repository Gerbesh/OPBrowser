# OPBrowser Code Graph

## 20 October 2026: original WM_CHAR to editable source DOM + rasterized caret

Windows page HWND `WM_CHAR` (UTF-16 single code unit or surrogate pair) -> ordered `NavigationEvent::TextInput` -> `op_browser` worker -> `op_engine::Engine::text_input` -> `forms::TextEditor` bounded Unicode scalar-based value/caret/anchor transitions -> original JS `beforeinput` (cancelable) -> real DOM `value` attribute and VM snapshot synchronization -> original bubbling `input` listeners / DOM mutations -> `op_layout::flow` inline glyph/hit-region generation from edited input or textarea value, with inline placeholder, password masking and bounded synthetic width -> `PreparedDocument::render` original op_paint text, native field outline and caret. `keyboard_event` uses the same form editor after cancelable keydown for Backspace/Delete/arrows/Home/End/Ctrl+A and textarea Enter; canceled keydown prevents the following WM_CHAR. The separate Windows address EDIT control is unaffected. The deep-span stress regression runs on an explicit 16MiB test thread, not a modified production stack.

## 20 October 2026: script-owned focus crosses VM, DOM, and CSS paint boundary

`op_js::BuiltinFunction::{DomFocus,DomBlur}` are installed per real DOM element alongside click/listeners. The VM checks current focusable/attached HTML metadata, updates the same `document.activeElement` used by native Tab/mouse, dispatches existing `focus/blur/focusin/focusout` DOM listener paths synchronously, and marks `dom_focus_dirty` with a bounded reentrancy depth. `JsRuntime::take_programmatic_focus_change` is an owned one-shot handoff consumed by `op_engine::PreparedDocument::render`: it resolves raw DOM node indexes to live generation-safe NodeId, synchronizes `StyleInteractionState.focused` and recomputes original CSS cascade before layout/paint. Host-originated focus updates keep using `sync_native_active_element` and do not enqueue duplicate focus events. Native click handlers may change focus without triggering event delivery twice.

## 20 October 2026: synchronized activeElement and cancelable native KeyboardEvent

`op_js::JsRuntime::sync_native_active_element` writes the original DOM document's read-only `activeElement` reference, with body as the initial/no-focus fallback. Original `op_engine::scripts::dispatch_focus_change` updates this before target/ancestor focus/blur listener delivery. `Engine::click_at` now dispatches focus events and DOM mutations before JS click when the native mouse target changes focus. `op_js::NativeKeyboardInput` defines a compact cross-layer keyboard envelope; `JsRuntime::dispatch_dom_keyboard_path` shares the existing trusted/phase-correct original DOM dispatcher with native click/focus and adds standard key/code/modifier/repeat fields. `op_engine::scripts::dispatch_keyboard_event` runs original JS listener delivery, commits DOM ops, then returns cancellation to `Engine::keyboard_event`, which allows or vetoes Tab/Enter defaults. The `op_engine` public re-export prevents direct op_browser->op_js crate linkage. `op_platform_win` translates page-only WM_KEYDOWN/WM_KEYUP into NavigationEvent::KeyDown/KeyUp, retaining address-field input, Ctrl+L and F5. `op_browser` queues events in order and propagates viewport repaint, navigation and one-shot focus scroll bounds as before.

## 20 October 2026: original focus geometry, nested CSS scrolling and JS event dispatch

`op_layout::flow` clones the layout-owned, translated `ClickRegion` geometry into `LayoutTree.focus_regions` **before** applying overflow clip scopes. Existing clipped `click_regions` still govern mouse hit testing. `op_engine::PreparedDocument::{layout_for_focus,reveal_nested_focus}` resolves ancestor `ScrollContainer` scroll positions through original CSS layout and JS/host scroll offsets, then transfers remaining top/bottom document bounds through a single-consumer `Engine.pending_focus_bounds` slot. `op_browser::LoadResult.focus_scroll_bounds` propagates this only on native FocusNext responses; the Win32 window adjusts SCROLL_Y after presenting the fresh repaint, preserving keyboard URL navigation. `op_js::JsRuntime::dispatch_dom_focus_path` shares the original synchronous event capture/target/bubble dispatch implementation with click but configures focus/blur as nonbubbling, all four events as noncancelable. `op_engine::scripts::dispatch_focus_change` walks bounded connected generation-safe paths, dispatches old blur/focusout and new focus/focusin, applies DOM mutations and GC checkpoint before style/layout refresh.

## 20 October 2026: Win32 keys through original DOM focus and native pixels

`op_platform_win::navigation_event` emits `FocusNext {backwards}` and `ActivateFocused` only for WM_KEYDOWN addressed to the page HWND, while preserving ADDRESS control Enter and Ctrl+L behavior. The original `op_browser::main` queues ordered Tab/Enter commands during worker paint, executes `op_engine::Engine::focus_next/activate_focused`, and differentiates Enter navigation vs repaint by comparing navigation history before and after the action. `PreparedDocument::keyboard_focus_order` traverses connected DOM with CSS visibility and tabindex prioritization; `StyleInteractionState.focused/focus_visible` triggers author style matching and a fallback 2px native ring in the existing `op_paint::DisplayList`. `activate_focused` calls original JS click dispatch directly on the focused generation-safe node, applies DOM mutations, and honors cancellation before link navigation. No ready-made browser, JS engine, or OS WebView dependency.

## 20 October 2026: JS click cancellation reaches original link default navigation

`op_js::JsRuntime::dispatch_dom_click_path_with_result` emits `DomClickDispatch` carrying both handler-delivery state and `Event.defaultPrevented`; existing bool-only public dispatch API remains compatible. The original event VM checks canceled state after `finish_event_dispatch`, including stopped capture/bubble branches. `op_engine::scripts::dispatch_click` applies DOM operations and records the cancellation flag for each native click. `Engine::click_at` resets that flag per click, preserves CSS focus and event delivery, while `activate_link_at` invokes the click before calling `follow_link` only if no cancellation occurred. `op_js::JsRuntime::call_isolated_event_property_handler` applies the HTML property return-false rule without changing `addEventListener` callbacks. Native Win32 still supplies href and document coordinates from painted links, filtered through the canonical DOM hit map.

## 20 October 2026: CSS pointer-events and DOM-authorized Win32 link activation

`op_css::ComputedStyle.pointer_events: PointerEvents` is inherited and resolved by the original declaration cascade. `op_engine::PreparedDocument.hit_target` and the fresh native click-layout selector skip pointer-ineligible targets, preserving the shared stacking-context hit order. `op_layout::InlineImage.node: Option<NodeId>` carries DOM identity for ordinary loaded `<img>` elements and emits a `ClickRegion` with the same coordinates and `PaintKey` as the raster. Win32 emits `NavigationEvent::FollowLinkAt {href,x,y}` using document coordinates instead of unaudited `FollowLink(href)`. The browser worker hands this to `Engine::activate_link_at`: it resolves the topmost DOM target, walks ancestors to confirm an exact matching `a[href]`, then navigates or routes the click to the covering element. The Win32 link painter iterates its stored regions in reverse display-list order. The JavaScript click handler remains independent on successful navigation; script preventDefault is not yet integrated.

## 20 October 2026: original stacking-context paint order shared with DOM hit resolution

`op_layout::ClickRegion.paint_key: Option<PaintKey>` now captures positioning metadata. `flow::Context::mark_positioned_outputs_since` annotates hit rectangles together with decorations/foreground commands; `inline::InlineFragment` and glyph-run `InlineHitSpan` inherit inline paint-group keys. `op_engine::order_native_hit_regions(layout, document)` combines the authoritative `layout.paint_groups` parent graph with any unregistered hit-only keys, orders negative and nonnegative children by the same key as `op_paint::build_display_list`, and uses connected DOM preorder for same-group ties. `PreparedDocument.hit_regions` stores this order once per render; `hit_target` reverse-scans it. `Engine::click_at` uses the same ranked order with fresh layout to preserve JS target dispatch and focus without changing original scrollbar precedence.

## 19 October 2026: block and glyph-run DOM hit regions to pointer-active CSS

`op_layout::flow` emits `ClickRegion` for every native block and from `inline::Lines.hit_regions`; an `InlineChar.origin` carries the owning inline DOM NodeId, and `PreparedText.origin_spans` measures UTF-8 text-prefix glyph widths without changing text wrapping or painted `TextBox` boundaries. `OutputStart.clicks` makes translated hit rectangles follow positioned and scrolled output; original overflow clipping applies to the same regions. `op_engine::PreparedDocument.hit_target` shares cached hit testing for hover and press; `Engine::press_at/release_press` drives `StyleInteractionState.active`, and click dispatch clears active before handling scrollbar and DOM targets. `op_platform_win` handles capture and off-window cancellation through `PointerDown/PointerCancel`; `op_browser` queues ordered press/release events and coalesces hover. Remaining painted stacking/inline atomic coverage is incomplete.

## 19 October 2026: UI event to original CSS interaction state

`op_platform_win::NavigationEvent::Hover` is sourced from Win32 mouse movement and leave tracking. `op_browser::main` coalesces pending pointer moves and delivers them to `op_engine::Engine::hover_at`. `PreparedDocument.hit_regions` caches the generation-safe `op_layout::ClickRegion` subset during native render, so a no-op cursor movement avoids layout/style recalculation. `PreparedDocument.interaction: op_css::StyleInteractionState` records hovered, focused and reserved active/focus-visible state, prunes detached targets at GC checkpoints, and passes input state to `collect_author_styles_with_linked_at_viewport_with_state`. Recursive selector matching and `compute_styles` update the authoritative CSS cascade before native paint. Click dispatch updates mouse focus on supported focusable blocks. Dynamic JS selector API and complete hit testing remain future work.


## 19 October 2026: viewport-aware original CSS media cascade and state pseudo-classes

`op_css::parser::parse_stylesheet_at_viewport` binds a screen viewport to nested `@media` parsing; `media_query_matches` conservatively evaluates simple width/height/orientation queries and propagates unknown conditions as errors instead of applying them. `op_css::style::collect_author_styles_with_linked_at_viewport` preserves original author declaration order for inline and linked CSS; `op_engine::PreparedDocument.style_viewport` triggers re-cascade only when its viewport changes. `PseudoClass` and `pseudo_class_matches` now recognize static form attributes and initial false interaction states; input-driven hover/focus still require event integration. `live_site_probe::style_error_category` produces fixed privacy-safe group labels instead of returning arbitrary selector names from CSS parser errors.


## 19 October 2026: read-only real-site DOM content diagnostics

`op_engine::DomContentDiagnostics` is assembled on demand by `document_content_diagnostics`, walking generation-checked, **connected** `op_dom::Document` nodes with a 100,000-node safety bound and `op_css::ComputedStyleMap`. It separates script/head/template/noscript text from CSS-suppressed and potentially visible text and counts form/input/iframe/main/script tag categories. `Engine::active_dom_content_diagnostics` exposes only numerical counters; `op_browser::live_site_probe` adds them to bounded per-site output. No JS, CSS or HTML sources, attribute values or user content are retained. Native regression tests cover CSS-hidden text, parser-consumed scripts, detached-node exclusion and navigation reset.


## 19 October 2026: guarded async if, template, throw, multi-declarator lowerer and diagnostics

`op_js::parser` keeps scoped `await_offsets` to report offending operator bytes and statement categories. `op_js::await_lowering::split_statement` handles if-tests and throw, `split` preserves eager template substitutions by coercing earlier fields before a Promise checkpoint, and recursive guarded `lower_terminating_branch` lowers if arms only when they unconditionally return/throw. `expression_await_count` and `statement_await_count` validate the supported syntax budget and fail closed for loops/try. Multi-binding variable declarations with await split into ordered single bindings before the original Promise CPS machinery. Captured cells survive original JS GC, and `op_engine` verifies real DOM text reaches native paint.


## 19 October 2026: branch-aware async return lowering

`op_js::await_lowering::lower_return_branch` maps direct one-sided ternary await and returning logical `&&`, `||`, `??` RHS awaits into an original `Statement::If` with independently lowered return arms. A NUL-prefixed local capture evaluates a logical left operand once, with equality-to-null deciding `??`; bypassed branches return directly without a Promise microtask. `supports_awaits` admits only one direct source await for this path; tests protect timing, GC preservation and the `op_engine` native-paint path. It is not a generic suspended interpreter frame and does not handle `try/finally` or awaits in loops.


## 19 October 2026: object fields and two-arm awaited conditional

`op_js::await_lowering::split` captures preceding ordinary object property values and defers `Instruction::CreateObject` until resumption. For a ternary with one direct await in each arm, it selects the correct arm before awaiting and `supports_awaits` counts two source awaits as one runtime suspension. VM GC tests and original `op_engine` native paint integration cover the path; general branching/loop/try/finally continuations remain outside this bounded transform.


## 19 October 2026: expression-level async continuations and call receiver

New `crates/op_js/src/await_lowering.rs` is an original bounded continuation-passing transform imported by `op_js::parser`. It extracts the first await from eager AST expressions, emits ordered `const` captures for operands evaluated before the pause, and resumes the remaining expression through the existing self-hosted Promise and lexical-arrow microtask machinery. `Expression::CallWithReceiver` is compiler-only and generates original `Instruction::Call { has_receiver: true }`; it preserves method identity and `this` even when a function's writable `.call` property is replaced. The parser validates the number of extractable awaits; unsupported branch/try/loop placement rejects before bytecode compilation. Captured JS objects/closures are traced by the original GC. The native engine tests verify a post-await DOM text mutation reaches Win32 paint. This is not a general interpreter-frame suspension mechanism.

Last updated: 2026-10-16

This document is the maintained human-readable code/dependency graph. It is updated
whenever crates, important types, or ownership boundaries change.

**Connected Code Graph:** [GENERATED_CODE_GRAPH.md](GENERATED_CODE_GRAPH.md)
is rebuilt directly from Cargo manifests by
`python tools/code_intelligence.py --write`. The `--check` variant is
enforced in Windows CI and fails when manifest-derived edges or Wiki
references change without committed updates. This maintained document
includes platform/native flow and ownership details that Cargo cannot infer;
the generated graph describes **crate-level edges only**, not a call graph.
See [Code Slicer](GENERATED_CODE_SLICES.md) for curated source-backed
functional paths. The 8 October baseline contains 12 crates and 22
local dependency edges.

## Original JS regex objects and sequence assignment (16 October 2026)

`op_js::lexer` selects `TokenKind::RegExp` after expression-leading
tokens and keeps division after values. `parser::Expression::RegExp`
-> `bytecode::Instruction::CreateRegExp` -> `JsRuntime` allocates a
GC-owned `ObjectKind::RegExp`; `RegExpTest`/`RegExpExec` and String
`replace/match/split` call bounded Rust `regex::RegexBuilder`.
This is an infrastructure regex matcher, not a JS engine; unsupported
ECMAScript patterns reject. `parser::Expression::CompoundAssignment`
-> existing original Binary and Assign/SetProperty bytecode, with
`Instruction::Dup2` preserving object/key evaluation once.
`parser::Expression::Sequence` -> compiler discards intermediate
operands but preserves ordered side effects. Parser uses it only where
comma is an expression operator rather than an argument/declaration
separator. JS and native DOM regression tests cover the complete path.

## Object accessors and method shorthand (16 October 2026)

`parser::ObjectPropertyKind` distinguishes data/getter/setter expressions.
The ordinary function compiler builds accessor/method bodies; VM
`CreateObject` stores GC-traced internal `ObjectKind::AccessorDescriptor`
cells. `get_property`/`set_property` call inherited and own accessors
on the original receiver; GC retains closures through property edges.
Thrown-value identity through accessors and complete property descriptor
APIs remain unsupported.

## Original JavaScript labeled control flow and try/finally (16 October 2026)

`op_js::parser::Statement::Labeled` records contextual identifier-colon
labels; the parser verifies unique active labels and valid iteration
targets for labeled `continue`, resetting labels at function boundaries.
`op_js::bytecode::Compiler` attaches loop labels to `ControlContext`,
patches named `break`/`continue` jumps and preserves scope unwinding.
The original `Instruction::Try` carries explicitly patched
`LabeledJump` destinations. Nested try/catch/finally code passes named
`RunOutcome::Break/Continue` signals to the owning outer context,
executing every intervening finally block. No external JS engine.
Original-VM tests and an HTML -> DOM mutation -> paint regression
exercise the entire path. General ECMAScript label/ASI coverage
remains incomplete.

## Original Promise-returning async functions without await (19 October 2026)

`op_js::parser` marks contextual `async function` declarations and expressions via `is_async`, rejecting `await` until real suspension is available. `op_js::bytecode::FunctionTemplate` propagates the marker through original closure allocation. `op_js::runtime::call_user_function` executes a no-await body in the native VM and uses the existing self-hosted `Promise.resolve`/`Promise.reject` for value adoption and rejection. Standard Error objects represent runtime failures; execution-budget errors remain hard stops. Async functions have no own `prototype`, and `new` rejects. This is not a complete async scheduler.

## Sequential await and async arrows (19 October 2026)

`op_js::parser::Parser` recognizes contextual async arrows and `Expression::Await` only inside async functions. Per-function/arrow validation rejects unsupported nested expressions, loops, try and non-direct awaits (maximum 32). `lower_async_await` rewrites straight-line awaits into `Promise.resolve(...).then(lexicalArrow)` continuations, recursively preserving following statements; NUL-prefixed internal callback variables are not spellable by page scripts. `op_js::bytecode::FunctionTemplate` carries separate is_arrow/is_async flags and original GC traces all retained closure environments. The original FIFO Promise microtasks resume pending fetch replies after GC. A native engine test verifies HTML -> authoritative DOM mutations -> paint text after two awaits. This is not a generally resumable JS VM frame.

## Namespace-aware DOM node creation and error diagnostics (17 October 2026)

`op_dom::ElementData::namespace_uri: Option<String>` now survives
authoritative DOM allocation; ordinary HTML element creation defaults to
the XHTML namespace. The original JS VM implements
`document.createElementNS(namespace, qualifiedName)` with bounded
ASCII-qualified-name parsing, XML prefix/namespace validation, and a
distinct staged `DomOperation::CreateElementNS`. The host resolves it to
`Document::create_element_with_namespace` and binds synthetic to
generation-checked native IDs, preserving the original JS element identity.
`dom_namespaces` tracks per-element namespace and is synchronized for
parser-owned nodes, transferred on binding, and removed on subtree cleanup
or document reset. `namespaceURI`, `localName`, `prefix`, `nodeName`,
and `tagName` respect HTML versus foreign namespace case behavior.

`OPBROWSER_JS_ERROR_CONTEXT=1` additionally reports a sanitized,
max-48-character member name on null/undefined property reads, and remains
disabled by default. The real YouTube createElementNS TypeError disappears;
four other TypeErrors remain (`set`, `responseStart`, `prototype` twice).
This does not implement SVG graphics rendering, foreign-content HTML parser
switching, Unicode-complete XML qualified names, or multimedia playback.

## Original Object.defineProperty with tracked descriptors (17 October 2026)

`JsObject::attributes` lazily stores non-default
`PropertyAttributes { writable, enumerable, configurable }`
indexed by string or symbol `PropertyKey`. The original VM
`Object.defineProperty` now parses a descriptor object, owns
data or accessor cells, and enforces essential non-configurable
and non-writable restrictions. `Object.getOwnPropertyDescriptor`
reads recorded attributes; ordinary assignment, delete,
`for...in`, and `JSON.stringify` respect them. Hidden
attributes are GC-independent metadata of the owning object,
and the existing GC traces all stored values. Deterministic
VM, GC and HTML-to-native-paint regressions pass.

Not yet full ES descriptors: inherited non-writable property
assignment, WebIDL/host attributes, array length redefinition,
symbol accessors and enumerable getter invocation during JSON
serialization remain incomplete or explicitly rejected.

## Native callsite diagnostics and queue/reflection builtins (17 October 2026)

`op_js::runtime::run_code` has opt-in, bounded and sanitized
non-callable-member diagnostics gated by
`OPBROWSER_JS_ERROR_CONTEXT=1`. A 16-entry recent-member trace
correlates the operand stack's receiver/callee with property
reads, revealing the missing member name without retaining
site scripts. The default VM path does not enable this trace.

`JsRuntime::install_standard_primitives` now installs
`Array.prototype.shift/unshift`, `Object.create`, and
`Object.getOwnPropertyDescriptor`. Array operations preserve
sparse holes and bound resulting length at 4096; Object.create
supports null or object prototypes but explicitly rejects a
nonempty second property-descriptor argument. Reflection
covers existing own data/accessor property cells, not the
complete ECMAScript attribute model for host/builtin fields.
Focused VM and native HTML-to-paint tests pass.

## Original Symbol primitive and GC-traced symbol properties (17 October 2026)

`op_js::value::JsValue::Symbol(JsSymbol)` represents an original
non-object, identity-bearing primitive with bounded unique IDs.
`JsRuntime::install_standard_primitives` installs `Symbol()`,
`Symbol.for`, `Symbol.keyFor`, and selected well-known symbols
including `iterator`, `toStringTag` and `dispose`. VM
`symbol_registry` holds the global per-runtime intern table.
`JsObject::symbol_properties: Option<Box<HashMap<u64,JsValue>>>` lazily keeps
symbol-keyed properties structurally separate from ordinary
string keys. `Instruction::GetProperty`, `SetProperty`,
`DeleteProperty`, `UpdateProperty` and `BinaryOp::In`
route symbols through dedicated operations and prototype lookup;
for-in and JSON ignore symbol keys. The original GC traces values
in symbol-keyed properties and resets them on recycled object
slots. Numeric symbol coercion fails explicitly.

Original VM and HTML-to-native-paint tests pass. Descriptor
reflection, Object.getOwnPropertySymbols, Reflect.ownKeys,
Symbol.toPrimitive, custom iterator protocol, complete UTF-16
and computed symbol keys inside object literals remain future
work. The original ECMAScript engine is not replaced.

## Original navigator and UTF-16 String search (17 October 2026)

`JsRuntime::install_dom_snapshot` creates a GC-traced `navigator`
per document and binds it both on the global environment and
`window`. This conservative object identifies OPBrowser rather
than claiming Chromium/WebKit compatibility, sets `cookieEnabled`
false (the native network loader discards cookies), and leaves
unsupported APIs absent for feature detection. The language is
currently an explicit `en-US` fallback, not an OS-locale promise;
available processor parallelism is clamped to 1..64.

`BuiltinFunction::StringIndexOf` and `StringIncludes` resolve
search offsets as UTF-16 code units; search inputs are bounded.
`Function.prototype.call` preserves null/undefined receiver for
native builtins, while ordinary non-strict user functions continue
using the global object. VM and HTML-to-native-paint regressions
verify both. Full Navigator/WebIDL interface and strict functions
remain future work.

## Original JavaScript Math and RegExp globals (16 October 2026)

`JsRuntime::install_standard_primitives` creates GC-traced
`Math` and `RegExp` globals in the original object arena.
`BuiltinFunction::Math(MathBuiltin)` provides 17 numeric methods,
standard numeric constants and non-cryptographic bounded random.
`BuiltinFunction::RegExpConstructor` and
`Instruction::CreateRegExp` share `allocate_regexp_object`,
validated through the existing bounded native regex implementation.
The original prototype chain supports `instanceof RegExp`.
VM tests, invalid-input tests and HTML-to-native-paint regressions
pass. Full ECMAScript number/protocol behavior remains incomplete.

## Modern arithmetic/shift operators and real-site for-in grammar (16 October 2026)

`op_js::lexer::TokenKind` recognizes `*=`, `/=`, `%=`,
`<<`, `>>`, `>>>` and their compound-assignment forms;
`parser::BinaryOp` now carries signed and unsigned shifts at the
correct precedence between addition and comparison. Original
`bytecode::Compiler` already evaluates assignment targets exactly
once, so new compound operations reuse `Dup2/GetProperty/Binary/
SetProperty` without new heap state. The Rust VM handles JS ToInt32,
five-bit shift counts, signed extension and unsigned 32-bit output.
`Parser::for_statement` separately uses `sequence()` for for-in
right-hand expressions but preserves the assignment-expression-only
for-of grammar. Focused AST, VM and native DOM-to-paint tests pass.
Neither a complete ECMAScript number model nor real-site readiness
is claimed.

## Opt-in real-script diagnostics and bounded for-in (16 October 2026)

`ParserScriptRunner::evaluate_code` receives `JsError.offset` from
original lexer/parser; only `OPBROWSER_JS_ERROR_CONTEXT=1` emits a
limited printable source excerpt into existing capped local reports.
No automatic raw-script storage or telemetry. Original
`parser::Statement::ForIn` -> `bytecode::Compiler::for_of_statement`
(reused private-index lowering with in-mode) ->
`Instruction::ToForInKeys` -> original `JsRuntime` snapshots a bounded
object's own keys into an original array, then executes the loop body
through existing scoping/control flow and DOM operations. Prototype
enumeration and proper property ordering remain pending.

## Original JS nullish and synchronous for-of integration (16 October 2026)

`op_js::lexer::TokenKind::QuestionQuestion` -> `parser::LogicalOp::Nullish`
-> `bytecode::Instruction::JumpIfNullish` -> `JsRuntime` branch checks
only null/undefined, preserving short-circuit value and evaluation order.
Parser distinguishes required grouping with boolean logic.
`parser::Statement::ForOf` and `Expression::ForOfValues` lower to
scoped ordinary-for bytecode with unspellable private iterator/index
names. `Instruction::ToForOfValues` snapshots bounded arrays, strings
or selected DOM collections in the original heap; per-iteration block
scopes protect lexical closure captures. This is not Symbol.iterator,
full iteration or dynamic array-length semantics. Existing JS/native GC
identity and parser-DOM integration pathways are reused.

## Loaded page URL to browser-owned Location object (16 October 2026)

`op_engine::scripts::parse_and_execute` forwards its verified loaded
`page_base` to `op_js::JsRuntime::set_page_location` at parser-script
boundaries. The per-document VM owns one `ObjectKind::Location` that
`window.location` and `document.location` reference; document URL
fields use the same source. It is a read-only HTTP(S) subset with
explicitly unsupported JS location navigation setters, not a second
navigation engine or new crate dependency. Tests cover loaded URL ->
script -> DOM text and URL component reads.

## Browser-owned Image constructor and host attribute reflection (16 October 2026)

`op_js::JsRuntime::install_dom_snapshot` registers a native `Image`
constructor on the global/window object. Its built-in reuses the
`document.createElement('img')` path, stages width/height attributes
and produces the same identity and DOM operations as authored nodes.
Image `src`/`alt`/`width`/`height` properties reflect the VM's staged
attributes and feed `op_engine::scripts::apply_dom_operations`.
The native image loader and display-list painter consume attached
`img` elements without a new dependency. Off-document network load
events and absolute `src` URL reflection remain unimplemented.

## Delete syntax from original JS lexer to VM (16 October 2026)

`op_js::lexer` reserves `delete` as a unary keyword. Parser emits
`UnaryOp::Delete`; bytecode selects `DeleteBinding`, `DeleteProperty`
or a side-effect-evaluating non-reference path. VM removes ordinary
object/array own properties but keeps array length intact; DOM/host
backing state is protected. No new crate dependency is introduced.

## Arrow functions, lexical host roots and call semantics (15 October 2026)

`op_js::lexer` tokenizes `=>` separately from assignment and
comparison. `op_js::parser::Parser::arrow_parameters` uses
non-mutating lookahead to distinguish plain parameter lists from
grouped expressions. `Expression::Arrow` lowers to an original
`FunctionTemplate` with an `is_arrow` marker. VM allocation keeps
arrow closure environment ownership but omits an own prototype;
invocation inherits `this` and `arguments` from the closure instead
of adding call-time bindings. `construct_value` rejects arrow
constructors. Normal closure reachability/GC infrastructure is reused
without new dependencies or alternative JS engines.

## Untagged template literals through the original JS stack (15 October 2026)

`op_js::lexer` tracks template interpolation frames and nested
brace depth, emitting head/middle/tail boundaries without tokenizing
raw template text as JavaScript. `op_js::parser` creates
`Expression::Template` with cooked quasis and ordered substitution
expressions. `op_js::bytecode` lowers each part to existing
string-seeded `BinaryOp::Add` instructions, retaining the VM's
ToPrimitive conversion and execution order. This introduces no
external JS engine or crate dependency; tagged templates remain
unsupported and are not silently treated as valid expressions.

## Browser-owned page performance timing (15 October 2026)

`op_js::JsRuntime` captures an `Instant` and `SystemTime` origin at VM
initialization. `install_dom_snapshot` installs `window.performance`
and its `timeOrigin` and `now()` members. The
`BuiltinFunction::PerformanceNow` dispatch validates
`ObjectKind::Performance` and reports monotonic elapsed milliseconds.
This per-document Web API is not the interactive phase profiler; no
crate boundary or dependency changed.

## Real-site JS diagnostic feedback loop (15 October 2026)

`ParserScriptRunner` keeps up to 24 bounded notes describing why initial
page scripts failed or were skipped; `PreparedDocument` owns the notes
per navigation and `Engine::active_script_diagnostics` exposes a read-only
view. `live_site_probe` exports counts and note categories to the
isolated live-web runner without retaining source HTML. Its Markdown
report groups failure/skip categories for prioritization. First
language blocker removed: `op_js` now parses and executes ECMAScript
bitwise `&`, `|`, `^` and `~` via correctly ordered expression
precedence and ToInt32 conversion. No crate-level dependency changed.
Real sites remain far from fully supported.

## Real live-web regression harness (15 October 2026)

`tools/live_web_suite.py` consumes an explicit 20-site HTTPS manifest,
spawns one isolated `op_browser::live_site_probe` per URL with a strict
wall-time limit and captures diagnostics and first-viewport images.
The probe invokes `Engine::navigate` and `Engine::tick_timers` using
OPBrowser-owned HTML/DOM/CSS/JS processing, then rasterizes through
`op_platform_win::render_display_list_to_bgra`. No browser engine
substitution, virtual HTML fixture or new crate dependency is involved.
The manifest, result JSON and PNG/BMP snapshots are separate from the
synthetic GC and WPT test suites. The 20-site first baseline proves zero
fully functional modern pages: missing JS support, blank SPA output and
broken CSS, with HTTP 403/429 cases explicitly separated from engine bugs.
See [real-web baseline](REAL_WEB_BASELINE.md).

## G3 process-private commit and real HTTP navigation regression (15 October 2026)

`gc_stress_probe` calls Windows `K32GetProcessMemoryInfo` with the extended
`PROCESS_MEMORY_COUNTERS_EX` layout. The CSV now stores `PrivateUsage`
(private committed bytes) alongside `WorkingSetSize` (resident working set)
without exposing unrelated user telemetry. The optional metric is zero on
platforms where it is not available, not a measured zero allocation. The
standalone `tools/analyze_gc_probe.py` validates per-second counters and
reports first/last, medians and ordinary least-squares byte trends, without
automatically equating Windows trimming with garbage collection.

A loopback HTTP integration test creates an old-page pending fetch, navigates
to a second page with a reused request ID, and delivers the delayed old
HTTP body before the current response. The engine generation gate prevents
the old completion from mutating the new page; the correct new response still
resolves its Promise. This verifies real `op_net` transport, not only
injected `NetworkCompletion` instances. No engine dependency changed.

## G3 host-side GC checkpoint wall time and navigation generation (15 October 2026)

`scripts::collect_at_idle` returns phase timings for an actual successful
page-owned idle collection (JS VM, native root tracing/sweep, reclaimed-node
VM metadata). The `PreparedDocument::finish_gc_checkpoint` boundary includes
weak page image/stylesheet/scroll-cache pruning, then records complete
checkpoint wall time, breakdown of its single slowest pass and counts
above 16/50 ms. `Engine::active_host_gc_checkpoint_telemetry` and
`gc_stress_probe` expose these bounded per-page counters. Parser-internal
collections remain separate. A test injects a stale network completion with
a request ID reused after navigation: the generation guard rejects the old
response and the current page callback receives only its own response.
No crate/dependency changes.

## G3 pause attribution and composite-fetch roots (15 October 2026)

The original `op_js::JsRuntime::collect_garbage_with_roots` times validated mark,
host-side weak metadata pruning and generational arena sweep independently.
`JsGcTelemetry` exposes `JsGcPhaseTimings` for the single worst automatic
checkpoint, plus threshold counts at 16 and 50 milliseconds. The browser worker
bridges these counters through `Engine::active_script_gc_telemetry`;
`gc_stress_probe` records five additional CSV/summary columns. These are
wall-clock diagnostic phases, not CPU sampling or total page-host checkpoint
time. A pending fetch rooted by an `AbortSignal.any` composite remains
alive through repeated idle sweeps and aborts once without delivering a stale
reply. No crate dependencies changed.

## G3 partial: generational native DOM arena and host coordination

`op_dom::NodeId` encodes a slot and generation; index() is an opaque handle, never an arena subscript. Document owns optional NodeSlots, a free list and live/reserved/GC counters; retired generations never recycle. Native GC validates all roots and keeps connected nodes and complete native components of JS wrappers/views/queries/callbacks. `scripts::collect_at_idle` runs JS validation followed by native sweep outside parser ownership, then prunes VM metadata. Text replacement synchronizes old child detachment. PreparedDocument removes dead image/stylesheet/scroll cache keys; style snapshots refresh on actual DOM mutations. Engine and probe export DomGcTelemetry, including native-only pause statistics. No new crate/dependency.

## G3 partial: native binding connectivity and recoverable staging quota

`bind_dom_node` preserves the virtual node's connection state, moves it to the physical ID and canonicalizes both ends of parent/child edges. Binding a detached node no longer creates a false GC root, and detaching a previously bound node cannot leave a connected virtual alias. `allocate_virtual_dom_node` shares a 256-outstanding-unbound-node/operation guard between element and text creation; monotonic IDs never recycle or enter the physical ID range. Committed or collected nodes stop consuming that staging quota. The authoritative 20,000-node host arena limit remains.

## G3 partial: complete pause distribution and native DOM arena visibility

JsRuntime owns a 64-counter `JsGcPauseHistogram` (512 bytes of buckets plus sample count). Successful automatic idle sweeps record one pause; rejected passes and explicit collections do not contribute. `JsGcTelemetry` exposes the histogram through `Engine::active_script_gc_telemetry`. `Engine::active_document_node_slots` reads authoritative `Document::len`, independently of JS wrapper occupancy. `gc_stress_probe` selects timer or detached-DOM/listener workloads and exports both counters and bounded percentile estimates. No new crate or dependency.

## G3 partial: heap arena headroom at browser idle checkpoints

JsRuntime::collect_if_needed_at_idle compares object/environment reserved
arena slots and recyclable free slot counts against each budget's 1/8
headroom margin, in addition to next_gc_live_trigger. A pressure-triggered
checkpoint still delegates to generation-safe collect_garbage_with_roots,
and retains GC pause/error telemetry. No new module or dependency.

## G3 partial: unwrapped native DOM descendants and network callbacks

Successful GC sweeps extend discarded detached wrappers down native
dom_pending_parents/dom_children edges while excluding attached nodes,
live wrappers and nodes referenced by reachable query/tag collections.
Reachable query NodeLists trace native ancestry to a materialized wrapper.
Pending fetch callbacks/signals are still host roots until completion or
abort removes them; new regressions verify both paths. No new crate.

## G3 partial: detached DOM ownership

JsRuntime::trace_heap_marks roots wrappers of connected nodes and follows
node-indexed dom_edges for descendants, ancestors, listeners, signal options,
and native DOM view caches. Live collections trace backing nodes.
collect_garbage_with_roots prunes dead disconnected wrapper/listener entries
and snapshot metadata before generation-safe slot reuse. No crate changes.

## G3 partial: weak timeout queue ownership

JsRuntime::trace_heap_marks treats abort_deadlines as conditional roots:
only entries with a live abort listener or onabort registration directly
keep their signal alive. Other roots (global JS bindings, pending fetch
text_request_signals and AbortSignal.any reverse follower traversal) also
preserve timeout signals. A successful collect_garbage_with_roots pass prunes
deadline records whose signal was not marked before slot generations change.
This prevents orphan timeouts from exhausting the fixed 64-entry deadline
queue across repeated idle checkpoints. No new crate edges are introduced.

## G3 partial: EventTarget-owned listener GC edges

JsRuntime::trace_heap_marks indexes lifecycle listeners and options by
receiver. Only a reachable EventTarget marks its callback objects, captured
environments and option AbortSignals. Window/document remain ordinary
global roots. collect_garbage_with_roots prunes dead receiver listener lists,
options and weak lifecycle_property_registration keys before recycling slots.
No new crate or third-party JavaScript runtime is involved.

## G3 partial: reverse AbortSignal retention and browser GC telemetry

AbortSignal.any composite -> source edges are traced through reverse lookup during marking; the source-to-follower host map itself does not act as a strong root. Successful sweeps purge links to dead composites; aborted composites detach from other sources. Engine::active_script_gc_telemetry bridges JS VM GC diagnostics to the native page worker. The `gc_stress_probe` binary drives real timer checkpoints and samples operating-system process working set. No new engine crates or external JavaScript runtime are involved.


## G2c partial: observable VM garbage collection

Original JsRuntime now retains failure count, last successful sweep report, and last/maximum pause Duration for browser-owned automatic checkpoints. The report is preserved on rejected traces; a pending host-owned network callback is a strong root until removed. No new crate dependencies or automatic in-flight collection were introduced.


## G2b partial: automatic browser-owned idle GC

op_js::JsRuntime::collect_if_needed_at_idle monitors live heap and lexical
environment slots with a 512-entry initial threshold and adaptive trigger
based on survivors. Only op_engine's browser page boundary invokes it
after script and DOM operations, native click callbacks, timer callbacks,
and network completions; ordinary embedders still use explicit root lists.
JS execution-frame barriers continue to prevent in-flight collection.
Host side maps for Headers, query NodeLists, tag collections and boxed
primitives are now weak-owned. Live wrapper objects mark their boxed values;
dead owner metadata is removed before generation-safe slot recycling.
Tests cover these ownership edges and live browser parser checkpoints.

## G2a partial: generation-safe heap and lexical environment recycling

op_js::value::ObjectId encodes slot index plus generation in its usize handle.
op_js::runtime::EnvironmentId uses the same scheme. The original JsRuntime
retains Vec<JsObject>/Vec<Environment> but adds alive/generation metadata
and free lists. Recycled slots bump generation; stale object() / object_mut()
and environment lookups fail closed. Existing host-held map keys keep their
generations, preserving identity across safe sweeps.
JsRuntime::trace_heap_marks returns validated object+environment bitsets to
both the existing mark-only diagnostic and the explicit idle
collect_garbage_with_roots API. A validated pass frees owned property
maps, function closures and lexical bindings before returning slots to
the allocator. JsGcReport exposes freed/live/reserved slot counts.
No automatic GC or host-wide root lifetime registration exists yet.

## G1.1 partial: native JS execution barrier and host roots

op_js::JsRuntime now owns active_execution_depth: Arc<AtomicUsize> and
ActiveExecutionGuard RAII tracks nested run_code and call_value execution.
trace_heap_reachability rejects active Rust interpreter, DOM event, and
programmatic-click phases; trace_heap_reachability_with_roots accepts explicit
external JsValue references for one idle snapshot. The Arc preserves Send
for the existing op_browser worker and Engine ownership.
No slots are released or reused. Generation-safe object and environment
ownership must precede actual GC.

## R1.2e partial: horizontal scroll ownership and paint

op_css::ComputedStyle computes overflow_x/y longhands with the existing
shorthand cascade. op_layout::flow::Context accepts separate vertical and
horizontal offset maps and shifts descendant ink, hit regions, nested
scrollports and clips. ScrollContainer owns max_scroll_left/scroll_left and
native horizontal track/thumb geometry. op_paint::HorizontalScrollbar
reuses the original Win32 GDI paint path. op_engine::PreparedDocument owns
horizontal offsets; Engine::scroll_horizontal_at handles bounded wheel
input and engine click_at handles horizontal track input. The browser
worker receives Win32 horizontal-wheel / Shift+wheel navigation events.

The JS VM still provides only scrollTop, not scrollLeft; advanced CSS
overflow and full real-site behavior are not claimed.

## R1.2d partial: nested scrollbar clip ownership

op_layout::flow::OverflowScope now keeps descendant ScrollContainer ranges
and intersects ancestor clips into each container after layout. Vertical
scrollbar geometry is not altered. op_paint converts the clip into the
existing PaintCommand::Clipped wrapper around the scrollbar overlay;
op_platform_win's native painter and recursive scrollbar_hit both honor it.
op_layout::ScrollContainer::visible_at gates engine wheel, track-click and
thumb capture behavior in hidden regions. Engine and original GDI bitmap
tests verify the end-to-end path.

This does not implement horizontal scrolling, gutter reservations,
all positioned clipping cases or broad WPT Overflow conformance.

## R2c: flex-basis, weighted shrink and shorthand to real pixels

op_css::ComputedStyle adds non-inherited flex_shrink (initial 1)
and flex_basis (auto or bounded pixel/percentage lengths).
op_css::computed::parse_flex_shorthand resolves common CSS flex
keywords and one/two/three-component shorthand forms; each of
its longhand values competes against explicit flex-grow/shrink/basis
by ordinary declaration cascade priority and source order.

op_layout::flow::FlexItem keeps its intrinsic/base width plus shrink
factor. The first item measurement uses flex-basis when specified,
including percentages of the flex container. If a line exceeds its
available content width, a bounded redistribution loop reduces
items in proportion to shrink factor times initial outer width,
freezing min-width-constrained items and offering leftover deficit
to others. A shared reflow_flex_item_to_outer_width path recalculates
text, backgrounds and hit geometry for both growth and shrink.

Original GDI bitmap tests cover weighted shrink, min-width freezing,
flex-basis override, percent basis, flex:1 and zero-shrink overflow.
Cascade tests verify shorthand precedence, keywords and invalid input.
Fixture: examples/css/flex-sizing.html.

LIMITS: horizontal row direction only, approximate CSS sizing based
on outer-width weights; full automatic min-content constraints,
flex-basis:content, align-items, column direction, ordering and
CSS Grid are still missing. This is not full Flexbox conformance.
Static/Positioning frozen WPT stay 194/200 and 54/100.

## R2b: proportional flex growth and distributed row positions

op_css::ComputedStyle::justify_content and flex_grow enter through
the original noninherited CSS cascade (unsupported or invalid values
are rejected). op_layout::flow::FlexItem retains the source NodeId,
style and link so a row item can be measured at its natural width,
grouped into a flex line, and then reflowed at a proportional assigned
width instead of merely stretching a pre-painted rectangle.

After growth, justify-content allocates the unused line width to
leading/trailing space or between/around/evenly slots. Each line is
handled independently; default flex-start retains original placement.
Render lists and native GDI pixels contain the final item widths and
text positions. Tests cover CSS keywords, proportional widths,
all supported justify modes and true native RGB sample pixels.

LIMITS: positive grow only, row direction only; no negative shrink,
basis, shorthand flex, flex-direction, cross-axis alignment, item
order or full min-content/bidi behavior. WPT Static and Positioning
frozen sets remain 194/200 and 54/100; no broad Flexbox compliance.

## R2a: computed flex-wrap and gap to multi-line native pixels

The original op_css::ComputedStyle parses flex-wrap:wrap and the gap
shorthand as non-inherited flex properties. Length parsing currently
accepts non-negative absolute/font-relative pixel values, not
percentage gaps. flex-wrap defaults to nowrap; unsupported wrap-reverse
and broader flex-flow shorthand remain unimplemented.

op_layout::flow::Style carries these values through to flex_row_atomic:
row-direction items keep their intrinsic widths, wrap when the next
item plus column gap would exceed the container, and advance by the
tallest line item plus row gap. No-wrap preserves its old behavior.
The resulting paint/hit geometry follows the normal original display
list and Win32 GDI path, not a separate layout renderer.

Layout regressions prove row breaks, horizontal/vertical gap, next
block placement and wider-container non-wrapping. The original GDI
pixel regression confirms painted RGB rectangles on distinct rows.
A manual offline example is examples/css/flex-wrap-cards.html.

Limitations: no grow/shrink, justify-content/align-items, ordering,
column-direction flex, min-content algorithm or CSS Grid. This is
a bounded row flex implementation, not general Flexbox conformance.


## R1.2c: native thumb capture to the original layout scroll offset

Win32 maps mouse-down on a painted scrollbar thumb into capture,
followed by moves and a release/capture-loss event. The browser
worker serializes start/end and coalesces consecutive moves while
busy without discarding capture boundaries. op_engine stores a
transient node/grab-offset, uses the original op_layout geometry to
derive clamped scrollTop under out-of-track mouse movement, and
repaints. Script-driven and no-JS page offsets use the same path.
Navigation cancels drag state; native input and engine tests cover
capture, nested thumbs, release and event coalescing.

No horizontal CSS overflow, reserved gutter or broad positioned
clipping compatibility is implied.


## R1.2b: native vertical scrollbar paint and input

Original op_layout::ScrollContainer computes a proportional 10px
vertical track and thumb from real layout max_scroll_top/scroll_top.
auto paints tracks only for overflowing elements, scroll always
paints a track, hidden never does. op_paint emits original scrollbar
commands after regular page ink; op_platform_win draws the track
and thumb as native GDI pixels and prioritizes their hit areas over
links beneath the overlay. Engine::click_at intercepts track clicks
before DOM dispatch and repaints at the clicked bounded offset.
Tests cover native BGRA pixels and script/no-script behavior;
examples/css/overflow-scrollbars.html is the offline fixture.
No thumb drag, horizontal overflow, reserved gutter or complex
positioned clipping is implemented.


## R1.2a: layout-derived scrollTop range reconciliation

Original op_layout::ScrollContainer exposes max_scroll_top calculated from
the actual block content height and viewport. PreparedDocument::render now
supplies these maxima to JsRuntime::sync_dom_scroll_limits after each
render and reflow (including timer and DOM style changes or resize).
The VM stores bounded per-node maxima, clamps pending and subsequent
scrollTop assignments, and returns the actual position to JS after layout.
No-script HTML pages also reconcile retained native scroll offsets.

Reads from an initial parser script can still see an unclamped request
before first layout. Only normal-flow vertical containers are supported;
there are no visible scrollbars, horizontal axes or full CSSOM View APIs.
Unit and original HTML/page-engine tests cover changed bounds and readback.


## G1.0: original non-destructive VM root tracing to page diagnostics

op_js::JsRuntime::trace_heap_reachability traverses global
prototypes/environment, own JS properties, function captured
EnvironmentId chains, and conservatively retained DOM/Events/
AbortSignal/timer/microtask/fetch side-map roots using an
explicit bounded iterative object/environment worklist.
Cycles are marked once. JsHeapReachability exposes allocated,
reachable and unreachable slot counts without freeing memory.

op_engine::Engine::active_script_heap_reachability exposes this
read-only snapshot for an idle script-enabled page, returning
None for plain HTML without a VM. VM and engine tests validate
retained closures, orphan cyclic objects, DOM/timer roots and
no changes to allocation counts. The implementation does NOT
register active native execution frames, so automatic GC is
forbidden until G1.1 root coverage and G2 slot reclamation
are proven. No third-party JavaScript engine is used.

## R1.1: JS scrollTop and nested Win32 wheel into original pixels

The page-owning original JsRuntime stores per-element scrollTop
requests (bounded by node count). For documents without JavaScript,
PreparedDocument holds native scroll offsets independently. A shared
layout_document_with_scrolling accepts the current scroll map:
flow::Context computes content scroll ranges from natural vs
specified height, translates descendant decoration/text/image
outputs, click hit rectangles, nested clip scopes and nested
container bounds while retaining the original block flow height.
Auto/scroll boxes now clip through R1.0's original display-list/GDI
pipeline; hidden boxes permit programmatic but not wheel scrolling.

op_engine::Engine::scroll_at selects the innermost scrollable
box that can move and repaints, so wheel input chains to a parent
at the inner boundary or falls back to document scrolling. Win32
NavigationEvent::Scroll routes screen-to-document wheel positions
through the existing renderer worker; the UI retains document
scroll if no nested element accepted the wheel. Engine::tick_timers
repaints scrollTop-only JS mutations even when text/DOM remains
unchanged. Native GDI bitmap tests and original hit-target tests
cover this cross-crate path.

This is bounded vertical scrolling. Visible scrollbars, overflow
axis longhands, full JS max-range clamping and advanced positioned
fragment behavior are not implemented. Native page-scrolling
coordinates still use the existing window SCROLL_Y.

## R1.0: CSS overflow -> native GDI clipping and hit testing

Original op_css::Overflow is a noninherited computed property.
op_layout::flow::Context records overflow scopes as descendant
primitive index ranges plus padding-box rectangles. At layout
completion nested ranges intersect into LayoutTree::clips, while
ClickRegion bounds are clamped to visible areas. Source-index
clipping persists across paint stacking-context reordering.

op_paint::clip_commands_since wraps individual leaves in
PaintCommand::Clipped. Original op_platform_win::paint_command
uses GDI SaveDC/IntersectClipRect/RestoreDC; native layer bounds
and hyperlink hit rectangles respect clipping. Engine::click_at
consumes the clipped regions. CSS cascade, two actual Windows
bitmap regressions and one real clipped mouse-hit test cover
the cross-crate path. Frozen WPT Static v1 197/200 and
Positioning v1 53/100 did not regress.

**No scrolling yet.** overflow:auto/scroll syntax is parsed
but intentionally remains visually unclipped until R1.1
implements offsets/wheel/scrollTop. Axis longhands and full
positioned/inline-atomic overflow remain unsupported.

## M4.35: parser documentElement and allocation accounting

op_engine::scripts::sync_dom_tree identifies the real parser html
child, calling JsRuntime::set_dom_document_element_node to expose
document.documentElement with original DOM identity. Event constructor
phase constants and element/global dispatch preserve any stop flag
set before dispatch; finish_event_dispatch clears them afterward.
The unchanged original upstream WPT v8 source smoke validates
21/21 attempted files with 3 explicit SKIP.

JsRuntime::heap_usage returns allocated object/environment slots
against budgets. This includes unreachable allocations, and is
**not** a collector or live-object count. GC implementation phases
and roots are specified in docs/GC_IMPLEMENTATION.md. The main
development priority has shifted to scroll/overflow rendering and
real garbage collection.

## M4.34: pinned upstream WPT to native rendering

op_engine::wpt_dom_probe checks the real upstream git revision, reads original source and instruments only WPT harness references. Its synchronous test context uses op_js::JsRuntime::FunctionCall via call_value and the shared function prototype. The same original VM exposes Event.srcElement and resets stopPropagation flags in DomInitEvent and finish_event_dispatch. Native op_engine/op_dom/op_layout/op_paint renders a sticky PASS/FAIL verdict. No foreign engine or new crate dependency.

## M4.33a-b: standalone EventTarget and handleEvent object listeners

JsRuntime::default creates a dedicated EventTarget prototype with
shared addEventListener/removeEventListener/dispatchEvent methods,
a globally installed constructor, and original ObjectKind::EventTarget
instances allocated only through new. This prototype is also inherited
by the original global Window, Document, DOM Elements and AbortSignal,
which retain their existing native-specific own method dispatch.

is_global_event_target recognizes standalone targets alongside the
existing document/window/signal receivers; dispatch_global_custom_event
builds one-object paths for standalone emitters. Original
lifecycle_listeners and lifecycle_listener_options remain the single
owned listener registry with stable registration IDs, once/passive,
AbortSignal removal and callback error isolation.

call_isolated_registered_listener dispatches either original callable
functions (receiver=currentTarget) or objects with dynamic handleEvent
(receiver=listener object). Both lifecycle and Element delivery use
that function; identity/dedup is the object itself. Null/undefined
listeners do not add records. Native repaint stays routed through
op_engine -> op_dom/op_layout -> op_paint, without external engines.

## M4.32b: native hit-test trust and original Event fields

Original JsRuntime::dispatch_dom_click_path calls the common
dispatch_dom_click_path_with_trust(path,true) for op_engine native
hit-tested clicks. The JavaScript Element.click builtin calls it
with false, preserving one capture/target/bubble implementation and
distinguishing isTrusted without bypassing JavaScript delivery.
Host-generated clicks are composed in the original light DOM path;
new Event() now parses EventInit.composed. Host lifecycle and abort
events explicitly advertise isTrusted=true/composed=false.

JsRuntime::set_property ignores ordinary assignment to readonly
Event data fields (type, target, currentTarget, eventPhase,
bubbles, cancelable, composed, defaultPrevented, isTrusted).
The original runtime still changes trusted host event properties
internally; no DOM engine replacement, crate dependency or UI/layout
ownership transfer. End-to-end tests use op_engine::Engine::click_at
and original op_paint output.

## M4.32a: active event paths and legacy stopPropagation alias

Original JsRuntime stores per-event ObjectId -> Vec<ObjectId>
event_paths only for active dispatch, rather than retaining stale
paths after return. install_element_event_path resolves the owned
Element ancestry and includes Document/Window for connected nodes;
install_global_event_path records paths for document, window and
AbortSignal. ensure_event_composed_path_method puts the owned
EventComposedPath builtin on new, legacy and host-created events.
finish_event_dispatch clears the active path before microtask
checkpoints and resets currentTarget/eventPhase. The method returns
a new JS Array snapshot during dispatch and [] afterward.

Event.cancelBubble is computed from the same internal stopPropagation
flag used by original dispatch_custom_event_inner and
dispatch_dom_click_path. The setter can set the stop flag but not
clear a prior stop. Legacy returnValue also reaches native click's
event shape. DOM-driven callbacks continue updating op_dom through
op_engine and native op_paint without any new crate dependencies.

## M4.31: stable EventTarget listener IDs and property-slot chronology

The owned op_js::JsRuntime now numbers registrations through
next_listener_registration_id and version_listener_options.
Element dom_listener_options and document/window/AbortSignal
lifecycle_listener_options retain a registration ID alongside
once/passive/signal. deliver_element_listeners and
deliver_lifecycle_listeners snapshot both the callback and the ID,
then recheck the live option map before dispatch: removing and
re-adding the same function cannot activate an obsolete snapshot.

dom_onclick_registration and lifecycle_property_registration
track property handlers separately. Target-phase callbacks now use
one registration-ordered stream for onclick plus Element callbacks
or onreadystatechange/onload/onabort plus global callbacks.
stopImmediatePropagation and callback exception isolation are shared
with the existing native EventTarget delivery. The click property's
registration ID remaps with synthetic-to-native op_dom node identity.
Engine mutation replay and original op_layout/op_paint remain unchanged.
No dependencies or external JS engines added.

## M4.30b-d: DOMException -> AbortSignal composition -> fetch completion

op_js::JsRuntime::new_dom_exception creates branded original
DOMException objects on their dedicated prototype (name/message,
legacy code and toString). AbortController default reasons and
AbortSignal.timeout deadlines use AbortError and TimeoutError.

JsRuntime's PendingAbortDeadline records a signal ObjectId and
Instant; next_timer_wait and run_due_timers integrate deadlines
with existing original TimerReport and microtask checkpoints.
abort_followers tracks the directed AbortSignal.any cascade,
bounded by source/input and follower counts. abort_signal marks
state before delivering the event, clears deadline state, removes
EventTarget registrations, revokes queued request callbacks and
propagates reason identity to dependent signals.

The self-hosted crates/op_js/src/async_fetch.js Request/Fetch facade
stores Request.signal and passes a native ObjectId to opFetchText.
The original runtime's text_request_signals associates fetch task
IDs with signals, while the existing network manager
op_engine::Engine::dispatch_text_requests still owns same-origin
worker requests. Upon abort the Promise rejects from the signal's
abort event and outstanding callbacks/queued requests are removed;
in-flight WinHTTP calls may still finish but late results are ignored
by complete_text_response_request. No crate/foreign JS dependency
was added and the rest of the DOM -> paint path is unchanged.

## M4.30a: bounded original AbortController/AbortSignal flow

op_js owns new ObjectKind::AbortController and AbortSignal and
BuiltinFunction handlers for their constructors, abort operation,
static signal factory and throwIfAborted(). JsRuntime::new_abort_signal
builds the native signal event target. DomListenerOptions now carries
an optional AbortSignal ObjectId in both Element and Document/Window
listener maps. AbortController.abort marks signal state, synchronously
calls remove_aborted_signal_listeners across both maps, then uses
dispatch_abort_signal_event and the existing lifecycle EventTarget
delivery and exception isolation. Existing native click/path dispatch
checks live registrations before invoking each callback, so abort
during dispatch suppresses later queued callbacks. op_engine/DOM/
layout/op_paint flow remains unchanged. No third-party engine or new
crate dependency.

## M4.29b: connected Element event path and native hit testing

op_engine::scripts::sync_dom_tree publishes the real document-root
NodeId to op_js::JsRuntime before parser scripts and later resyncs.
The original JS runtime's dom_document_root is reset with navigation,
and element_event_path stops at the non-Element parser root.
path_reaches_document gates delivery to attached Document/Window.
dispatch_custom_event_inner and dispatch_dom_click_path now use
deliver_element_global_capture/bubble around existing Element
capture/target/bubble. The native op_engine::scripts::dispatch_click
prefilter calls has_dom_click_path_listener to detect global-only
handlers. Synthetic click, page event dispatch, host native clicks,
op_dom mutation replay, layout and op_paint remain owned components.
Detached/removal behavior is covered by VM tests. No new dependencies.

## M4.29a: Document/Window EventTarget and lifecycle dispatch

The original op_js now stores lifecycle_listener_options per
(receiver ObjectId, event type, capture, callback ObjectId).
deliver_lifecycle_listeners uses once removal before callback invocation,
passive-state management and the existing call_isolated_event_handler
exception boundary. deliver_lifecycle_target preserves target
capture/normal ordering, property callbacks and immediate-stop checks.
dispatch_lifecycle_event retains host-owned document.readyState and
the existing document-targeted load event compatibility behavior.
The new dispatch_global_custom_event routes document-targeted
custom Events through window capture, document target and optional
window bubble, with bounded recursion and event cleanup.
op_engine still replays resulting DOM changes through op_dom,
layout and op_paint to native pixels; no new crate dependencies.

## M4.28b: listener metadata, exception boundary and native paint

op_js owns listener metadata keyed by DOM node/type/capture/function.
The original VM reads once/passive/capture dictionaries and keeps its
existing per-event callback maps. deliver_element_listeners merges
native click and custom DOM Event delivery, removes once registrations
before nested calls, suppresses passive cancelation, and isolates
ordinary handler exceptions into bounded runtime diagnostics. Synthetic
node IDs remap listener metadata to physical op_dom IDs after replay.
op_engine exposes Engine.active_event_listener_errors and preserves
DOM mutation -> layout -> op_paint rendering on successful handlers.
No new dependencies or foreign JS engines were added.

## M4.28a: immediate-stop event control

In op_js, EventStopImmediatePropagation marks the event's immediate-stop and path-stop flags; dispatch_dom_click_path and dispatch_custom_event_inner check after each callback. The custom event path resets its immediate-stop state for reuse. Engine script mutation replay through op_dom, layout and op_paint remains unchanged. VM and cross-crate native paint tests verify the flow. No new crate dependencies.

## M4.27: selector chains and original typed DOM events to native pixels

Original op_js now matches bounded compound selector tokens
through descendant/child ancestor chains against the live
parent/child/attribute snapshot. Document/Element querySelector,
querySelectorAll, Element.matches and closest share this
matcher; static NodeLists keep their snapshot and identity.

Original typed DOM event listener maps now carry event type
and capture mode, alongside the existing original click tables.
Event constructor, Element.dispatchEvent and legacy Event
initialization execute callback chains through capture/target/
bubble. The return boolean reflects cancelation, with guarded
reentry/depth. Event-driven DOM mutations are replayed by
op_engine::scripts into authoritative op_dom then laid out/
painted by original CSS/layout/op_paint. Seven end-to-end
tests assert updated native text output.

Pinned original WPT DOM/Events v5 includes the untouched
EventTarget-dispatchEvent-returnvalue.html (two tests).
Twelve manually chosen original HTML source files pass;
three files explicitly skipped. No representative WPT
compatibility can be inferred from this sample.

## M4.26: JS in, scoped query selectors, programmatic click and WPT DOM v4

op_js lexer and parser now recognize relational in. The original
VM performs bounded HasProperty checks, including live DOM
accessor names, and forwards querySelector/querySelectorAll
requests through its authoritative staged parent/child/attribute
tree. A new static query-result NodeList differs from retained
live HTMLCollection/tag lists; saved handles survive binding
virtual DOM IDs to physical native op_dom NodeIds.

element.click calls the existing native DOM event propagation
path (capture/target/bubble, bounded nested dispatch), then
op_engine replays resulting DOM mutations and rebuilds
CSS/layout/op_paint output. Five cross-crate tests verify
real text pixels and mutation behavior.

Pinned original-source WPT DOM smoke v4 adds a fixture using
JS in, keeping the existing synchronous testharness shim and
fail-sticky callback count. Fourteen files manually selected:
11 executed/passing, 3 skips. Not broad WPT conformance.

## M4.25: live tag-name search plus hardened WPT multi-case runner

Original op_js now implements Document/Element.getElementsByTagName,
Element.hasAttribute and hasAttributes. Live tag collections query
the current bounded element tree with descendant-only traversal;
parser snapshots and later DOM operations/timers feed the same
authoritative state. Script DOM changes recascade and paint
through op_engine and op_paint as before.

op_engine wpt_dom_probe reads the pinned upstream original source
and executes each original synchronous test body. The shim
counts callbacks, stores failures persistently, and verifies the
manifest's exact per-file test count and native PASS marker.
Ten original WPT DOM files pass, three explicitly unsupported
files remain SKIP. Two additional harness unit tests demonstrate
sticky-failure behavior. No official WPT conformance claim.

## M4.24: live Element children and pinned WPT DOM multi-fixture probe

The original JS VM computes childElementCount, firstElementChild,
lastElementChild and previous/nextElementSibling from live child
linkage and DOM element tags. Element.children retains the same
HTMLCollection object across mutation and synthetic-to-physical
binding, with basic item and namedItem resolution.

The pinned WPT DOM probe now reads seven upstream original
source files, runs their unchanged synchronous test bodies
with a narrow testharness substitute, and checks a native
paint-visible PASS marker. Three selected fixtures remain SKIP.
The sample is not representative WPT conformance.

## M4.23: Node connectivity and original WPT source to native pixels

op_js::JsRuntime derives Node siblings, isConnected,
ownerDocument, nodeName/tagName and Element.contains from the
bounded live parent/children model synchronized by op_engine.
classList mutations are validated before any write and support
multiple tokens. The original style declaration scanner keeps
semicolon-containing CSS values intact inside quotes/parentheses.

A new op_engine wpt_dom_probe reads an original file from a
pinned external WPT Git revision; replaces only harness imports
with minimal synchronous assert_equals/test support, executes
unchanged assertions in the original JS VM and checks the
native display-list result. A separate frozen manifest lists
explicitly unsupported DOM fixtures. No broad WPT pass rate
is asserted. See docs/COMPATIBILITY.md for limitations.

## M4.22: live JS NodeList/DOMTokenList/style to physical tree and pixels

The original JS runtime builds stable per-node live childNodes,
classList and style objects plus parentNode and first/last child
property access; NodeList length, item and index read the current
staged tree rather than a frozen snapshot. Element.replaceChild
and remove are queued as ordered op_js DomOperation records.

op_engine::scripts syncs child arrays, physical Text nodes and
attributes from the op_dom authoritative tree. Native
op_dom::replace_child validates ownership/ancestor constraints.
When virtual JS handles become physical NodeIds, cached
childNodes/classList/style objects and JS element identity survive.
Style/class changes flow through original CSS matching, computed
layout and op_paint. Nine integration tests verify resulting pixels
even after native timer callbacks. No external DOM or JS engine.

## M4.21: reparenting and Text creation -> host DOM -> CSS -> pixels

Original JS creates real Text nodes and stages removeChild,
insertBefore, setAttribute and removeAttribute in a bounded
DomOperation queue. It tracks existing parser-owned node attributes
and parent relationships and updates nested ID lookup immediately.
op_engine::scripts::apply_dom_operations validates and commits on
op_dom::Document; the engine refreshes author style matching and
computed styles using cached linked CSS/color profiles, then
native layout and paint. Nine integration cases verify pixel changes.

## M4.20: DOM VM op queue -> real DOM nodes -> recomputed native pixels

op_js::JsRuntime now owns bounded synthetic DOM node handles,
persistent JS object identity, detached parent relationships and
ordered DomOperation records. document.createElement constructs a
detached node; appendChild, id and textContent produce real host
operations with synthetic-to-physical node binding. document.body is
exposed when the HTML tree-builder has instantiated it.

op_engine::scripts::apply_dom_operations is the sole host replay
function: it allocates op_dom::Document nodes, updates attributes and
text, appends/reparents attached nodes and tracks actual mutations.
Parser script runner, retained click dispatch and Engine::tick_timers
all call the same ordered operation replay. Engine recomputes styles,
layout and native display list after changes. op_dom::Document
refuses ancestor cycles before changing parentage.
Integration tests verify dynamic identity, nested DOM, timer, click,
pixel output and cycle safety. General DOM methods remain absent.

## M4.19: native String/Array methods -> JS script -> DOM repaint

JsRuntime::install_standard_primitives owns Array.push/pop and the
String.charAt method, and a dedicated SyntaxError constructor/prototype
used by JSON.parse. The op_engine integration test confirms results
update existing textContent and render to native pixels. No foreign JS
engine or fake dynamic DOM nodes are used.

## M4.18: Test262 broader fixture -> own syntax VM -> native pixels

The pinned runtime v2 manifest is generated deterministically from
25 named upstream language/builtin feature groups via the source-only
build_test262_runtime_v2.py selector. The existing Test262 runtime
probe evaluates each case on a fresh original JS runtime, now using
correct multiline include/flag metadata and version-specific JSON.
tools/compatibility.ps1 reports runtime v1 and runtime v2 separately.

The lexer now tokenizes typeof and ?, the parser builds conditional
expressions, and bytecode emits lazy branching instructions or special
TypeofBinding for missing-name semantics. The runtime implements
typeof_value and native standard Array/Number/Object helper methods.
An op_engine integration test verifies these features mutate DOM and
paint native text pixels. It never embeds another browser/JS runtime.

## M4.17: object primitives -> runtime conversion -> Test262 verdict

A new boxed_values VM map holds primitive payloads separately from
ordinary JS properties. new Boolean/Number/String and Object(primitive)
create real object references tied to their constructor prototypes.
The bytecode binary instruction resolves ToPrimitive via valueOf then
toString, respecting own/inherited callable methods, user-thrown values,
the existing VM call-depth budget and object prototype walks.
instanceof instead searches the receiver's prototype chain with a
callable constructor and a validated constructor.prototype. Unary
void evaluates for side effects; non-strict undeclared assignments
create mutable global var bindings. Test262 Runtime v1 now passes
82/91 previously pinned cases versus 59/91 at M4.16.

## M4.16: pinned Test262 runtime probe -> original VM -> standard globals

The separate op_js bin/test262_runtime_probe evaluates each pinned
Test262 fixture against a fresh original JsRuntime after injecting a
small original-JS assertion/Test262Error bootstrap. Metadata-aware
skip reasons and expected runtime-negative exception types produce a
per-case JSON report plus an independent, scoped runtime percentage.
tools/compatibility.ps1 runs it separately from test262_probe and WPT.

JsRuntime::install_standard_primitives exposes Object, Array,
Boolean, Number, String, isNaN, isFinite, Number constants and
Infinity/NaN. JsValue::to_number parses 0x/0o/0b numeric strings.
The 91-case locked arithmetic/equality suite measures these changes
as 18->59 passes without hiding the remaining 32 failures. No
strict-mode/modern-ES coverage or complete boxed primitive semantics.

## M4.15: native JSON → original VM object → Promise → DOM repaint

op_js::json implements strict bounded parsing, independent of JS eval.
The runtime exposes native JsonParse and JsonStringify builtins, creates
owned arrays/objects with json_to_value and serializes with cycle
detection through json_from_value. The JSON global exists without DOM.

The self-hosted async_promise.js implements all/race/allSettled/any
over array-like input; Response.json uses Promise.resolve(body).then
to call native JSON.parse at a normal microtask checkpoint. Existing
Engine::dispatch_text_requests, Engine::tick_timers and
JsRuntime::complete_text_response_request preserve the safe network
boundary, followed by DOM changes and native pixels. No foreign engine.

## M4.14: Request/Headers → vetted GET → pre-connect redirect policy

The browser-context VM now installs a native Headers constructor with
owned HashMap-backed, budgeted get/has/set/append/delete and initializer
handling. A self-hosted Request wraps URL, GET method, Headers, same-origin
mode, credentials:omit and follow/error redirect policy. fetch(Request,
init) feeds these detached request options into opFetchText and
JsRuntime::take_text_requests, which preserves page generation identity.

Engine::dispatch_text_requests calls the new
NetworkContext::load_text_response_for_page_with_options. Both op_js and
op_net validate the allowlisted request header set. The WinHTTP loader
adds vetted fields via WinHttpAddRequestHeaders; only bounded,
same-origin GET network work runs off the page thread.

For ResourceKind::Text, WinHTTP auto-follow is disabled.
op_net::http::load_text_with_options resolves each Location and checks
same_origin before the next network connection, at most five hops;
redirect:error rejects the first redirect. Non-fetch loaders continue
with their prior redirect behavior. Response metadata and body then
return to the VM through the existing Promise/microtask/repaint path.

## M4.13: WinHTTP status and headers through owned Response and native pixels

The op_net HTTP text loader now retains status, reason phrase, final URL,
redirect flag and bounded headers in LoadedTextResponse; non-text loaders
still reject non-2xx. NetworkContext::load_text_response_for_page filters
and checks same-origin at the request and final-response boundaries.
Engine::dispatch_text_requests and Engine::tick_timers carry detached
metadata through the generation-tagged page completion channel.
JsRuntime::complete_text_response_request builds a TextResponse host
object with an ObjectKind::Headers value and native HeadersGet/HeadersHas
functions. Header names are normalized and Set-Cookie is excluded.
The self-hosted Response calculates ok from HTTP status, then schedules
Promise reactions using the already-owned microtask queue; DOM changes
flow to native repaint. Legacy opFetchText retains status-error callbacks.

## M4.12: self-hosted Promise → filtered fetch → native repaint

The original `op_js` VM bootstraps `async_promise.js`, implementing
Promise state/settlement and reaction dispatch in its own ECMAScript subset.
Reactions call the original `queueMicrotask` builtin and drain at retained
task checkpoints in `JsRuntime::drain_microtasks`. No foreign JS runtime
or second microtask queue is involved.

For page contexts, `JsRuntime::install_dom_snapshot` also loads
`async_fetch.js`: `fetch()` wraps `opFetchText` in a Promise and
`Response.text()` returns a second Promise. The existing
`JsRuntime::take_text_requests` → `Engine::dispatch_text_requests` →
`NetworkContext::load_text_for_page` → `Engine::tick_timers` →
`JsRuntime::complete_text_request` pipeline delivers a completion back
to the owning VM; Promise reactions then record `DomTextMutation`
and reuse CSS/layout/native paint. Page generations reject stale results.
This is not yet full HTTP Response semantics: synthetic 200/OK on
successful filtered text loads; failed HTTP status currently rejects.

## First live JS → DOM → repaint boundary (M4.1)

The `op_engine::scripts` preparation step traverses parsed HTML, collects
at most 16 bounded classic inline scripts and supplies
`op_js::JsRuntime::install_dom_snapshot` with detached element IDs and
textContent values. The VM's own builtin getElementById returns an element
object. Its textContent setter queues bounded `DomTextMutation` records;
it never borrows or holds a raw pointer to the live DOM. Between scripts,
`op_engine` applies queued records with `op_dom::Document::set_text_content`,
then continues normal author CSS collection/computation, layout and paint.
A `PreparedDocument` retains the mutated DOM so resize reflow remains
consistent. The runtime reports executed/failed/skipped/mutations rather
than aborting navigation for unsupported JavaScript.

This is not DOM scripting conformance or a complete script lifecycle:
external scripts, event loop, DOM mutation observers, document.write and
real parser-blocking timing remain absent.

## M4.2: filtered external JavaScript resource path

`op_engine::scripts::execute_for_page` now accepts the filtered
`NetworkContext` and document base. Its document-ordered script
sequence may contain inline source or an external URL. For external
sources, `op_net::resolve_script_source` rejects unsafe/cross-origin
references, then `NetworkContext::load_script_for_page` invokes
`ResourceType::Script` filtering. The resource decoder uses explicit
JavaScript MIME types, UTF-8/BOM decoding, bounded local file reads and
WinHTTP transport. Following a redirect, its final origin is checked
against the page before code execution.

The loaded code executes in the same bounded `JsRuntime` as adjacent
inline scripts; detached `DomTextMutation` records update retained
DOM, then CSS/layout/paint work exactly as in M4.1. Failed requests
do not abort subsequent scripts. Unsupported async/defer/integrity
external scripts are skipped; scheduling remains post-parse and
full browser document lifecycle is still absent.

## M4.3: click event → persistent VM → reflow

The per-page `PreparedDocument` retains an `Option<JsRuntime>`,
including closures registered by `addEventListener("click", fn)` or
the `onclick` property. `op_layout::flow` records block
`ClickRegion` rectangles for `id`-bearing elements, with relative
position offsets. `op_platform_win::NavigationEvent::Click` converts
client coordinates to document coordinates using toolbar height and
scroll offset; hyperlink actions retain priority. The browser worker
calls `op_engine::Engine::click_at`, which hit-tests the current layout,
dispatches to the registered JS callback, applies bounded detached
DOM text mutations, recomputes CSS when needed, and sends a non-navigation
reflow page back to the Win32 painter.

M4.4 extends this path: hit-testing selects the smallest id-bearing
block region even if only an ancestor registered a listener. The engine
follows `op_dom::Node::parent` and calls
`JsRuntime::dispatch_dom_click_path` for target-then-ancestor callbacks.
The retained VM exposes `event.target`, `event.currentTarget`,
`eventPhase`, and `event.bubbles`; `removeEventListener` removes
callbacks by function identity. Detached mutations still cross the
owned DOM and computed-style/reflow boundary. Capture, cancellation,
keyboard dispatch and a full hit-test tree are not yet implemented.

## Crate dependency graph

```mermaid
graph TD
    B[op_browser<br/>browser process bootstrap]
    BC[op_browser_core<br/>tab + lifecycle policy]
    E[op_engine<br/>renderer orchestration]
    W[op_platform_win<br/>Win32 platform]
    D[op_dom<br/>DOM storage]
    H[op_html<br/>HTML tokenizer/tree builder]
    C[op_css<br/>CSS/style]
    L[op_layout<br/>layout]
    P[op_paint<br/>display list]
    J[op_js<br/>ECMAScript VM]
    N[op_net<br/>source/network stack]
    T[Windows WinHTTP<br/>HTTP framing + TLS + proxy]
    I[op_image<br/>bounded raster buffers + codec adapter]
    K[Windows WIC<br/>Microsoft raster codecs only]
    G[Windows GDI<br/>font extents + pixel output]

    B --> BC
    B --> E
    B --> P
    B --> W
    E --> D
    E --> H
    E --> C
    E --> L
    E --> P
    E --> J
    E --> N
    E --> I
    E --> G
    N --> T
    H --> D
    L --> D
    L --> I
    P --> L
    P --> I
    I --> K
    W --> P
    W --> G
```

No browser engine or ready-made JavaScript engine is below this graph.

## Current key types

```mermaid
classDiagram
    class Engine {
        -EngineState state
        -NetworkContext network
        -NavigationState navigation
        -Option~String~ document_address
        -Option~PreparedDocument~ active_document
        +new()
        +start()
        +state()
        +navigation()
        +render_html()
        +set_html_page()
        +reflow()
        +render_source()
        +navigate()
        +follow_link()
        +go_back()
        +go_forward()
        +reload()
    }

    class NavigationState {
        -Vec~NavigationEntry~ entries
        -Option~usize~ current_index
        +entries()
        +current_index()
        +current()
        +can_go_back()
        +can_go_forward()
    }

    class PreparedDocument {
        address
        mime_type
        document
        images
        PageImages elements / generated
        stylesheet_addresses
        +render(width, height)
    }

    class NavigationEntry {
        +String request
        +String address
        +String mime_type
    }

    class RenderedPage {
        +String address
        +String mime_type
        +DisplayList display_list
    }

    class NetworkContext {
        -RequestFilter request_filter
        +request_filter()
        +request_filter_mut()
        +load_document(source) Result~LoadedDocument, LoadError~
        +load_stylesheet_for_page(source, top_level, limit)
        +load_image_for_page(source, top_level, limit)
    }

    class RequestFilter {
        +import_adblock_rules(text) FilterImportReport
        +check(url, resource_type, top_level) RequestDecision
        +allow_site(host)
        +stats() FilterStats
    }

    class TabManager {
        +open(address, activate) TabId
        +activate(id)
        +close(id)
        +automatic_discard_candidate() TabId
        +discard(id, reason, scroll_y)
        +finish_restore(id)
    }

    class JsRuntime {
        -heap JsObject[]
        -environments Environment[]
        -global_env EnvironmentId
        -object_prototype ObjectId
        -array_prototype ObjectId
        -error_prototype ObjectId
        -type_error_prototype ObjectId
        -reference_error_prototype ObjectId
        -global_object ObjectId
        -instruction_budget
        -object_budget
        -environment_budget
        -call_depth_budget
        +new()
        +with_instruction_budget(limit)
        +eval_script(source) JsValue
        +execute(compiled) JsValue
        +global(name) JsValue
        +get_property(target, key) JsValue
    }
    class Environment {
        parent EnvironmentId?
        kind Global / Function / Block
        bindings
    }
    class FunctionObject {
        implementation User / Builtin
    }
    class FunctionImplementation {
        User FunctionTemplate + EnvironmentId
        Builtin Error / TypeError / ReferenceError
    }
    class JsObject {
        properties
        prototype ObjectId?
        kind Ordinary / Array / Function
        function FunctionObject?
    }
    class ObjectId {
        opaque heap index
    }
    class CompiledScript {
        code Instruction[]
    }
    class FunctionTemplate {
        name
        params
        code Instruction[]
    }
    class TryTemplate {
        try_code Instruction[]
        catch_param
        catch_code Instruction[]?
        finally_code Instruction[]?
    }
    class RunOutcome {
        Complete / Returned / Thrown
        Break / Continue
    }
    class Instruction {
        Push / Load / Declare / Assign
        UpdateBinding / UpdateProperty
        CreateObject / CreateArray / CreateFunction
        GetProperty / SetProperty / Call / Construct
        Unary / Binary / Dup / Pop
        EnterScope / ExitScope / UnwindScopes
        Jump / JumpIfFalse / JumpIfTrue
        Try / Throw / BreakSignal / ContinueSignal
        Return / SetCompletion / Halt
    }

    class Encoding {
        Utf8
        Utf16Le
        Utf16Be
        Windows1251
        Windows1252
    }

    class LoadedDocument {
        +String address
        +String mime_type
        +String text
        +SourceKind source_kind
    }

    class NativeBrowserWindow {
        -HWND hwnd
        +create(title, display_list)
        +hwnd()
        +painted_once()
        +viewport_size()
        +submit_address()
        +present()
        +link_at_client_point()
        +click_first_link()
        +set_navigation_state()
        +set_status()
        +run_message_loop(on_event)
    }

    class NavigationEvent {
        Navigate(source)
        FollowLink(href)
        Back
        Forward
        Reload
        Poll
    }

    class HttpUrl {
        secure
        host
        port
        target
    }

    class Document
    class Tokenizer
    class Characters {
        first
        second
        +iter()
    }
    class NamedEntry {
        name_offset
        value_offset
        name_length
        value_length
        legacy
    }
    class CssToken {
        kind
        Url(value) / BadUrl
        start
        end
    }
    class Stylesheet {
        rules
    }
    class StyleRule {
        selectors
        declarations
    }
    class Selector {
        compounds
        combinators
        pseudo_element
        specificity
    }
    class AttributeSelector {
        name
        matcher
        value
        case_insensitive
    }
    class PseudoClass {
        root / first-child / last-child / only-child / empty / link
        first-of-type / last-of-type / only-of-type
    }
    class FunctionalSelector {
        is / where / not -> Selector[]
        nth-child / nth-last-child -> NthSelector
    }
    class NthSelector {
        expression(a,b)
        of Selector[]
        from_end
        same_type
    }
    class PseudoElement {
        Before
        After
    }
    class Declaration {
        name
        value
        important
    }
    class Specificity {
        ids
        classes
        types
    }
    class StyleMap {
        NodeId -> MatchedDeclaration[]
        (NodeId, PseudoElement) -> MatchedDeclaration[]
        +declarations_for(node)
        +declarations_for_pseudo(node,pseudo)
    }
    class MatchedDeclaration {
        style_node
        declaration
        specificity
        source_order
        source
        value_from_var
    }
    class StyleCollection {
        styles
        errors
    }
    class ComputedStyleMap {
        NodeId -> ComputedStyle
        (NodeId, PseudoElement) -> ComputedPseudoStyle
        NodeId -> CustomPropertyMap
        (NodeId, PseudoElement) -> CustomPropertyMap
        +style_for(node)
        +pseudo_style_for(node,pseudo)
        +custom_properties_for(node)
        +pseudo_custom_properties_for(node,pseudo)
    }
    class CustomPropertyMap {
        case_sensitive_name -> resolved TokenKind[]
    }
    class CustomDependencyGraph {
        sorted names
        directed edges including fallback references
        iterative finish order / reverse SCC traversal
        cyclic node mask
    }
    class ValueBudget {
        token count
        token storage bytes
        fallback depth
    }
    class ComputedPseudoStyle {
        style
        content
        items Text / Image(url,style_node)
        replaced_image sole parsed URL
        quotes
    }
    class ComputedQuotes {
        Auto / None / Pairs(open,close)
    }
    class GeneratedContext {
        counters
        quote_depth
        suppressed
    }
    class CounterContext {
        counter_name -> value_stack
        +reset(name,value)
        +set(name,value)
        +increment(name,amount)
        +current(name)
        +values(name)
    }
    class CounterOperation {
        name
        value
    }
    class Display {
        Inline / Block / FlowRoot
        Flex / InlineFlex
        Table roles / Contents / None
    }
    class ComputedStyle {
        display
        color
        font_size_px
        font_weight
        font_style
        line_height
        text_align
        white_space
        text_decoration_line
        letter_spacing_px
        word_spacing_px
        text_transform
        background_color
        margin_edges
        padding_edges
        border_edges
        width / min_width / max_width
        height / min_height / max_height
        box_sizing
    }
    class InlineBoxStyle {
        node_id
        pseudo_identity
        relative_position_marker
        local_visual_offset
        padding_edges
        background
        border_edges
    }
    class NamedColorEntry {
        u16 name offset
        u8 name length
        three RGB bytes
        six-byte record
    }
    class InlineBoxes {
        parent-linked arena nodes
        continuation links / split-fragment history
        cached cumulative edges and depth
        iterative stack transitions
        nearest positioned ancestor / visual offsets
    }
    class InlineFragment {
        node_id / direction
        border box / padding edge bounds
        persistent cross-run relative containing geometry
    }
    class DeferredInlinePositioned {
        element identity / style / display
        static x-y / hypothetical flow width
        inline ancestor identity
        fallback positioning context
    }
    class InlineStyle {
        typography
        optional box stack index
    }
    class EmptyInline {
        InlineStyle
        no glyph payload
    }
    class InlineAtomic {
        width / height / baseline
        nested decorations / text / images / order
    }
    class InlineImage {
        used width and height
        optional Arc RasterImage
        href
        InlineStyle ancestor stack index
        optional own InlineBoxStyle
        atomic wrapping / nowrap
    }
    class BlockContent {
        Element(NodeId)
        Generated(NodeId,PseudoElement)
        ImageAlt(NodeId)
    }
    class FlowContext {
        viewport_width
        viewport_height
        current y / floats
        positioning_stack PositioningContext[]
        flow_height_stack Option<int>[]
        inline_fragments InlineFragment[]
        deferred_inline DeferredInlinePositioned[]
        output vectors
    }
    class PositioningContext {
        x
        y
        width
        definite height
    }
    class BoxDecoration {
        DecorationPaintLayer Block|Inline|PositionedBlock|PositionedInline
        bounds
        background
        border_top/right/bottom/left
    }
    class Lines {
        last_baseline Option<int>
        font content-box metrics / line strut
    }
    class LayoutTree {
        box_decorations
        text_boxes
        image_boxes
        order
    }
    class LayoutItem {
        Text_index
        Image_index
    }
    class TextMeasurer {
        +measure(text, size, weight, style) TextMetrics
    }
    class TextMetrics {
        width
        ascent
        descent
    }
    class LinkSpan {
        start_byte
        end_byte
        href
    }
    class LinkRegion {
        measured_bounds
        href
    }
    class DisplayList
    class RasterImage {
        width
        height
        pixels
        +width()
        +height()
        +pixels()
    }
    class ImageBox {
        bounds
        Arc~RasterImage~ image
        href
    }

    Engine --> NavigationState
    JsRuntime --> CompiledScript : compile / execute
    JsRuntime --> Environment : owns lexical environment arena
    JsRuntime --> JsObject : owns bounded heap
    JsObject --> ObjectId : prototype reference
    JsObject --> FunctionObject : optional callable payload
    FunctionObject --> FunctionImplementation : user bytecode or builtin
    FunctionImplementation --> FunctionTemplate : owned user bytecode template
    FunctionImplementation --> Environment : captured user closure
    CompiledScript --> Instruction : ordered bytecode with patched jump/control targets
    Instruction --> TryTemplate : nested try/catch/finally bytecode
    JsRuntime --> RunOutcome : normal and abrupt completion propagation
    NavigationState --> NavigationEntry
    Engine --> NetworkContext
    NetworkContext --> LoadedDocument
    NetworkContext --> HttpUrl
    NetworkContext --> Encoding : decode_html / BOM-header-meta selection
    Engine --> RenderedPage
    LoadedDocument --> Engine : render
    Document --> EngineStyles : discover active stylesheet links
    EngineStyles --> NetworkContext : bounded stylesheet requests
    NetworkContext --> LoadedStylesheet
    LoadedStylesheet --> Encoding : decode_css / BOM-header-charset selection
    EngineStyles --> StyleCollection : linked CSS keyed by link NodeId
    Tokenizer --> Characters : consume references
    Characters --> NamedEntry : bounded prefix lookup
    Tokenizer --> Document : tree builder
    CssToken --> Stylesheet : parse_stylesheet
    Stylesheet --> StyleRule
    StyleRule --> Selector
    StyleRule --> Declaration
    Selector --> Specificity
    Selector --> FunctionalSelector : recursive functional pseudo arguments
    FunctionalSelector --> NthSelector : filtered or same-type sibling order / maximum filter specificity
    NthSelector --> Selector : strict of filters using ordinary complex selector matcher
    Selector --> PseudoElement : terminal generated target
    Document --> StyleMap : DOM-order linked/embedded collection / selector matching
    StyleMap --> MatchedDeclaration
    MatchedDeclaration --> Declaration
    MatchedDeclaration --> Specificity
    StyleCollection --> StyleMap
    StyleMap --> CustomPropertyMap : custom-property cascade / inheritance
    CustomPropertyMap --> CustomDependencyGraph : op_css::custom dependency discovery
    CustomDependencyGraph --> CustomPropertyMap : exact cycle invalidation / dependency-order resolution
    ValueBudget --> CustomPropertyMap : bounded expansion and per-target retained storage
    CustomPropertyMap --> ComputedStyleMap : retained host + pseudo snapshots
    StyleMap --> ComputedStyleMap : host + pseudo cascade / substituted value parsing
    MatchedDeclaration --> ComputedStyleMap : invalid computed var() candidate keeps priority and resolves unset
    StyleMap --> CounterOperation : counter-reset/set/increment winner parsing
    CounterOperation --> CounterContext : document-order scoped counter mutation
    Document --> CounterContext : sibling-aware nested scope traversal
    CounterContext --> ComputedPseudoStyle : counter()/counters() generated text
    GeneratedContext --> CounterContext : document-order counter ownership
    GeneratedContext --> ComputedPseudoStyle : emitted quote depth / hidden subtree exclusion
    StyleMap --> ComputedQuotes : inherited quotes winner / var substitution
    ComputedStyleMap --> ComputedQuotes : per-host retained pairs / quotes_for
    ComputedPseudoStyle --> ComputedQuotes : inherited host or pseudo-local pairs
    Document --> ComputedPseudoStyle : attr() reads originating element attributes
    ComputedStyleMap --> ComputedStyle
    ComputedStyle --> Display : resolved formatting role
    ComputedStyleMap --> ComputedPseudoStyle
    ComputedPseudoStyle --> PseudoElement : keyed generated target
    Engine --> StyleCollection : retained author style candidates/errors
    Engine --> ComputedStyleMap : retained resolved initial CSS properties
    Engine --> PreparedDocument : one retained successful page
    PreparedDocument --> Document : DOM snapshot
    PreparedDocument --> RasterImage : shared Arc image resources
    Document --> LayoutTree : flow grouping / inline lines
    Display --> LayoutTree : block / table / flex / contents formatting dispatch
    ComputedStyleMap --> LayoutTree : display/text style + line-height/alignment/white-space/decorations/spacing/transform + block/inline box geometry
    ComputedStyle --> InlineBoxStyle : resolved inline padding/background/solid borders
    ComputedPseudoStyle --> LayoutTree : generated before/after inline items
    ComputedPseudoStyle --> BlockContent : display block retained text
    Document --> BlockContent : ordinary element child traversal
    BlockContent --> BoxDecoration : shared normal-flow block geometry / empty boxes
    FlowContext --> PositioningContext : nearest positioned ancestor / viewport fallback
    InlineBoxes --> InlineFragment : relative inline measured line fragments
    InlineFragment --> PositioningContext : first/last fragment padding bounds within a line formatting run
    PositioningContext --> BoxDecoration : abs/fixed insets, axis margin equations and containing geometry
    InlineAtomic --> LayoutTree : inline-table and inline-flex atomic placement
    InlineAtomic --> BoxDecoration : nested flex/table decorations rebased into parent flow
    InlineImage --> ImageBox : only available raster payloads produce paint items
    InlineImage --> BoxDecoration : transparent failed replacements preserve CSS geometry
    ComputedPseudoStyle --> InlineBoxStyle : pseudo decoration identity + box style
    InlineBoxes --> InlineBoxStyle : one style per owned node / parent index
    NamedColorEntry --> CssColor : binary search in packed names / opaque RGB result
    InlineStyle --> InlineBoxes : one stack index per character or image
    InlineBoxes --> BoxDecoration : per-line nested fragments / outer-before-inner allocation
    ComputedPseudoStyle --> EmptyInline : decorated empty generated strings
    Document --> EmptyInline : visually empty inline with its own box decoration
    EmptyInline --> BoxDecoration : edge width / line metrics / alignment without TextBox
    LayoutTree --> BoxDecoration : block + inline backgrounds / solid borders
    BoxDecoration --> DisplayList : background + four border FillRects
    LayoutTree --> DisplayList : styled text / image paint commands
    Engine --> TextMeasurer : worker-local GDI adapter
    TextMeasurer --> TextMetrics : whole href-run extents
    LayoutTree --> TextMeasurer : injected metric interface
    LayoutTree --> LayoutItem : ordered text / image indexes
    Engine --> RasterImage : visible img resources / bounded worker decode
    LayoutTree --> ImageBox : atomic inline box / shared baseline
    ImageBox --> RasterImage : shared Arc pixels
    DisplayList --> RasterImage : Image paint commands
    NativeBrowserWindow --> RasterImage : transient DIB / AlphaBlend
    LayoutTree --> LinkSpan : TextBox links
    DisplayList --> LinkSpan : Text paint command links
    NativeBrowserWindow --> LinkRegion : GDI measurement / hit testing
    RenderedPage --> DisplayList
    NativeBrowserWindow --> DisplayList
    NativeBrowserWindow --> NavigationEvent
```

## Navigation invariants

- A history entry is committed only after source loading and rendering succeed.
- Back/forward reload the historical request but do not create duplicate entries.
- Reload does not mutate the history list or current index.
- Navigating from the middle of history truncates the old forward branch.
- No-target back/forward operations are no-ops.
- Engine.document_address follows the last successful loaded page, including
  redirects on back/forward/reload. It is separate from immutable history requests.
- Engine::follow_link resolves the href against that effective document address
  before entering the same navigate/commit-after-success path.
- Reflow borrows only the active PreparedDocument and never loads sources or changes
  history/effective address. Failed navigation leaves that snapshot unchanged.

## Current ownership boundaries

- op_browser owns the current bootstrap, UI command/result channels and one temporary
  worker-owned renderer. Only the UI thread touches HWNDs. Results carry viewport
  dimensions; stale-width pages trigger a latest-size reflow and are not presented.
  ADR-0002 fixes the target boundary as browser-process-owned product state supervising
  renderer processes, so the current worker is intentionally a migration stage.
- op_browser_core owns UI-independent tab identity, lifecycle, protection flags, retained
  restore metadata and the initial memory-pressure discard-candidate policy. The native UI
  is not connected to multiple tabs yet.
- op_platform_win owns Windows-specific window/input/surface/process glue and consumes
  platform-neutral display lists.
- op_engine currently owns per-page navigation state plus orchestration between loading
  and web-engine subsystems. This state will later become per-tab.
- Engine.active_document retains one PreparedDocument with parsed DOM, address,
  MIME type, shared image resources, author StyleCollection and ComputedStyleMap after
  successful navigation/back/forward/reload. Stateless render_source remains uncached;
  set_html_page initializes the start page.
- op_net owns document-source interpretation, initial link-reference resolution,
  an initial HTTP URL parser, owned document byte decoding, bounded HTTP(S) loading,
  response validation and errors. RequestFilter now runs before document, stylesheet and
  image loads and supports the initial host/wildcard Adblock-style subset, exceptions,
  resource types, per-site allowlisting and counters. Its private http::windows module
  uses RAII WinHTTP handles for transport/TLS/proxy/framing/decompression. Cache and
  cookies remain future work.
- op_net::encoding owns charset label resolution, Unicode/single-byte decoding
  tables and a bounded initial HTML meta prescan. HTTP/file/data loaders share it;
  unsupported labels and malformed Unicode remain typed errors.
- op_html owns HTML tokenization and tree construction rules. Its private
  comments module implements iterative comment start/body/less-than/end states and
  emits Token::Comment(String). The tree builder now creates ordered DOM Comment nodes,
  flushing adjacent text at comment boundaries without changing the open-element stack.
  Comment detection occurs only in normal tag-open context, preserving raw-text/RCDATA.
  The private declarations module emits Token::Doctype(Doctype), preserving missing/empty
  name/public/system identifiers and force_quirks with iterative recovery. Unknown <!...
  declarations use bogus comment tokens. The tree builder maps the first pre-element
  doctype to a DOM DocumentType node and ignores later/in-element doctypes. Its private
  document_mode module applies the WHATWG compatibility matrix and stores NoQuirks,
  LimitedQuirks or Quirks on op_dom::Document; missing/late doctypes select Quirks.
  TreeBuilder now owns initial, before-html, before-head, in-head, after-head, text,
  in-body, after-body and after-after-body insertion modes, automatically creates missing
  html/head/body elements, routes metadata/text-only head tokens back to the head pointer,
  merges duplicate html/body attributes and ignores the self-closing flag for ordinary
  non-void HTML elements. InBody owns normal/list-item/button scope checks, implied-end-tag
  generation, p/block/list/description/heading/button recovery and special-element boundaries
  for generic end tags; head-only tokens encountered in body are routed back through the
  stored head pointer. Body/html end tags now switch insertion modes without popping the
  recovery stack; after-body comments attach to html, after-after-body comments attach to
  Document, and delegated/trailing tokens follow the specified in-body recovery path.
  TreeBuilder also owns an ActiveFormattingEntry list with marker boundaries and retained
  start-tag attributes. Formatting starts use the Noah's Ark three-entry cap; reconstruction
  recreates stale formatting entries on the open-element stack; formatting end tags use the
  bounded adoption-agency algorithm, including furthest-block DOM reparenting and cloned
  formatting nodes. Repeated anchors/nobr recover through the same path, while applet/marquee/
  object add and clear formatting markers. Table construction adds InTable/InTableText/
  InCaption/InColumnGroup/InTableBody/InRow/InCell, table-scope cleanup, implicit tbody/tr
  insertion, cell markers and pending table-character buffering. Foster parenting uses the
  last open table to insert misnested nodes before that table; op_dom::Document::insert_before
  supplies the required sibling insertion/reparent primitive. Foreign-content/CDATA,
  template/frameset modes and CSS table layout remain later work.
  Its private
  references module consumes the full named-reference table and numeric references
  before text/attribute tokens enter the DOM. Characters carries one or two Unicode
  scalars. references::named contains a generated sorted table of eight-byte
  NamedEntry records plus packed names/deduplicated UTF-8 values (35,378 static bytes).
  Prefix range searches require no allocation and examine at most 31 input characters.
  tools/generate_html_entities.py regenerates/verifies it offline from the pinned
  WHATWG data/entities.tsv; no new crate or runtime/build dependency is involved.
  Initial raw-text/RCDATA context keeps
  references and markup from being incorrectly parsed inside script/style/title.
- op_dom owns document/node storage, mutable element attributes used by tree-construction
  merge rules, Comment nodes, DocumentTypeData (name/public/system/force-quirks),
  DocumentMode (NoQuirks/LimitedQuirks/Quirks), DOM parent/child invariants, append_child
  reparenting and insert_before for parser-required sibling placement such as foster parenting.
- op_layout owns text-flow, block-box used-value geometry, the initial table formatting context,
  structural-container traversal and UTF-8 LinkSpan ranges preserved across whitespace
  normalization and line wrapping. It resolves percent/auto/min/max/content-vs-border-box
  widths, independent border sides, block height minima/maxima and adjacent-sibling vertical
  margin collapse. Table layout consumes computed Table/TableCaption/TableColumnGroup/
  TableColumn/TableHeaderGroup/TableRowGroup/TableFooterGroup/TableRow/TableCell roles,
  collects rows across row groups, builds an occupancy grid for colspan/rowspan and computes
  per-column min/max preferences from measured cell text, images, width/min/max constraints and
  col/colgroup hints. Available width is distributed between those preferences instead of being
  split equally. Inherited border-spacing supplies separate horizontal/vertical gaps; collapse
  mode suppresses spacing and resolves cell-cell border conflicts per grid segment by choosing
  one winning edge, assigning each internal boundary to one adjacent cell so paint does not
  double it. Table cells also retain output ranges for their nested decorations/text/images so
  baseline/top/middle/bottom vertical alignment can reposition the whole cell content after final
  row/span heights are known. Baseline cells compare first-line baselines across the row. Before
  grid placement, child-side table fixup normalizes each table root into layout-only row/cell
  sources: consecutive improper table children form anonymous rows, row-group children that are
  not rows form anonymous rows, and consecutive non-cell row children form anonymous cells.
  Anonymous cells inherit the parent table/row text properties but keep initial non-inherited box
  properties; no synthetic DOM nodes are created. Normal flow also groups consecutive orphan
  table-internal siblings, ignoring only repair-transparent whitespace/comments/display:none
  separators between them, and sends that run through an anonymous block table using the exact
  same table_box/grid path as a real display:table. Captions participate in that repaired wrapper
  and orphan table-column boxes still feed declared-column width hints. Table auto sizing carries
  percentage constraints alongside content min/max preferences so percentage tracks reserve their
  share before remaining width is expanded into auto tracks. Explicit-width table-layout:fixed uses
  col/colgroup hints first, then explicit first-row cell widths, then divides remaining track space;
  later-row content does not renegotiate those fixed tracks. caption-side is inherited and the
  table wrapper now lays top captions before the table border box and bottom captions after it, so
  the table background/border no longer incorrectly contains caption geometry.
  display:inline-table uses flow::inline_table_atomic: it computes an initial shrink-to-fit width,
  runs the same table_box in a local layout Context, then packages its nested decorations/text/
  images/order into inline::InlineAtomic. Lines treats that object as one wrapping unit and carries
  the element VerticalAlign with it. Baseline uses the table first-row baseline; top/bottom anchor
  the whole atomic box to the final line box, middle centers it around the parent text middle
  approximation, and tall top/bottom atoms enlarge line descent so they are not clipped. Placement
  then offsets retained nested output into the final aligned box and remaps LayoutItem indices
  without flattening the table into fake text or pixels. Outer anchor identity is inherited by
  nested text/images when they do not already carry a link.
  Caption flow, real cell backgrounds/borders/padding and span geometry reach ordinary
  BoxDecoration/text/image output. Whitespace-only text between block siblings is suppressed before
  it can create anonymous line geometry.
  op_image::RasterImage carries IntrinsicSize separately from raster canvas dimensions. Ordinary
  raster decoders expose their natural width/height/ratio; the initial bounded SVG slice derives
  optional root width/height plus viewBox ratio and rasterizes simple rect content into the same BGRA
  resource. op_layout::replaced consumes optional intrinsic width/height/ratio for explicit/auto
  replaced sizing, CSS default object dimensions and min/max ratio conflicts. flow converts CSS
  percentage-width/font-relative/content-vs-border-box sizes and applies the existing
  available-width/4096-height fitting policy after CSS used sizes.
  flow::resolve_image_size shares this path between DOM img and sole-URL inline pseudos;
  mixed generated lists retain anonymous intrinsic image items. Pseudo image box identity
  preserves host/pseudo separation and does not duplicate inherited decorated ancestors.
  Context::block_image shares DOM/sole-URL block used sizes, separate percentage basis/fit
  width, auto margins, exact decoration bounds and collapsed vertical-margin flow without
  introducing anonymous text-line leading around a replaced block.
- op_paint owns platform-neutral paint commands/display lists, CSS color conversion and
  BoxDecoration -> FillRect expansion for backgrounds/four border sides. Computed UA link
  color/underline defaults and author overrides use ordinary text commands; LinkSpan carries
  click identity without overriding presentation. Alpha colors composite over the white page.
  TEXT_FONT_FAMILY and Windows GDI_TEXT_LOCK are shared by engine metric adapter and
  native painter. Font realization, measurement/drawing and cleanup are synchronized
  per operation; networking and original layout do not hold this gate.
  The current GDI backend measures painted glyph ranges for native hit testing;
  network addresses are resolved only by the worker/engine, not by the painter.
- op_css owns CSS tokenization/parsing, author-style matching and the initial cascade.
  It traverses op_dom, interleaves loaded link stylesheets with style elements at their
  actual DOM positions, collects inline style attributes, and matches the
  selector AST right-to-left and builds per-NodeId MatchedDeclaration candidates. Selector
  matching now includes attribute operators, adjacent/general element siblings and the
  initial structural/link pseudo-class set. compute_styles resolves supported values using
  !important, inline source,
  specificity and source order, then applies inheritance/global keywords into a
  ComputedStyleMap. Display now distinguishes inline/block/none plus table, caption, column,
  row-group, row and cell roles; the HTML UA defaults assign native table elements those roles,
  center captions, bold th cells, give td/th 1px padding, make table sizing border-box and give
  tables the 2px separate-border spacing default. Computed properties include inherited
  border-spacing and border-collapse plus non-inherited vertical-align
  (baseline/top/middle/bottom) alongside color, font-size/font-weight, background-color,
  margin/padding edges, independent border edges, width/height min/max and box-sizing.
  Box shorthand/longhand
  candidates are compared by normal cascade priority;
  length parsing covers percent, em/rem and CSS absolute units. One CssColor parser now
  handles hex/all 148 opaque named colors plus RGB/HSL/HWB for text/background/borders.
  Unquantized HSL channels feed HWB white/black mixing before final CssColor byte conversion.
  Hue units normalize in wider arithmetic before scaling, avoiding overflow for large angles.
  color(srgb)/color(srgb-linear) share modern three-channel/alpha parsing with HWB. Linear
  channels use the sRGB transfer curve before final 8-bit CssColor encoding; initial channel
  clipping/used-value none resolution do not preserve color-space metadata for interpolation.
  All modern RGB/HSL/HWB/color() functions share three-component/slash-alpha parsing. Modern
  HSL accepts numeric/percentage saturation/lightness and missing components. Legacy RGB
  requires uniform number or percentage channels; legacy HSL keeps percentage-only S/L.
  op_css::named owns allocation-free ASCII case-insensitive binary search through packed
  names and six-byte records (2,210 static bytes). Its generated named::data comes from pinned
  crates/op_css/data/named-colors.tsv; tools/generate_css_named_colors.py regenerates/checks
  it offline. Transparent/currentcolor stay special computed keywords outside the opaque table.
  UA defaults mirror M1 block/hidden tags and heading typography; heading/paragraph/list spacing is represented
  as computed margins instead of a separate layout spacing table.
- op_engine::styles walks link nodes during page preparation, applies the initial
  stylesheet-link activation subset, resolves against the effective document address,
  and owns per-document request/text budgets plus duplicate-source reuse. Load failures
  are nonfatal and successful CSS is keyed by the link NodeId for source-order collection.
- op_net::stylesheets resolves and loads bounded local/file/data/HTTP(S) CSS, blocks
  network-to-file access and HTTPS-to-HTTP downgrade, validates HTTP CSS MIME, and decodes
  BOM/transport-charset/@charset/UTF-8 before handing source text to op_css.
- op_engine::images walks computed-visible DOM and before/after content in document order.
  PageImages stores elements by NodeId and generated resources by (NodeId,pseudo,item index).
  URL sources use effective document or consuming stylesheet bases, including redirected CSS.
  Both share one worker loader/cache and candidate/request/encoded/pixel/time budgets.
  Image failure does not fail document history. Arc pixels are reused across both maps/reflow.
- op_net::images loads bounded binary HTTP/file/data image bytes; HTTP shares the
  WinHTTP transport, with image-specific Accept/byte/time limits. Source policy
  rejects network-page file access and HTTPS-to-HTTP image downgrades.
- op_image owns validated top-down premultiplied BGRA RasterImage buffers and size
  checks before pixel copying. Targeted windows bindings call explicit Microsoft
  WIC PNG/JPEG/GIF/BMP decoders, never HTML/DOM/layout/painting or a browser engine.
- op_layout places ImageBox records in normal vertical order with intrinsic or
  HTML width/height sizes, viewport fitting, inherited href and alt fallback.
- op_platform_win::raster owns transient DIB/DC lifetimes and alpha drawing; image
  rectangles enter the existing scroll-aware hit testing and clear on replacement.
- op_js owns the first executable original ECMAScript slice: lexer -> AST parser -> bytecode
  compiler -> stack VM with primitive values/global bindings plus a parse-expectation
  Test262 probe. It is not connected to <script>, DOM bindings or the page event loop yet.

## Temporary architectural constraints

- The initial Windows renderer uses GDI as an OS drawing backend.
- Display-list, scroll and measured link-region storage are currently process-global
  because M1 has one window. Display replacement clears old link regions.
- The visible product is still single-tab, but canonical tab/lifecycle/discard state now
  exists in op_browser_core so renderer/UI migration no longer needs to invent that model.
- A single in-flight navigation disables navigation buttons; the window continues
  processing paint/input/close messages. A 30 ms Win32 timer polls worker results
  only while a worker command is active and is removed on completion (no idle timer).
- WM_SIZE uses a separate 120 ms one-shot debounce timer; minimized events do not
  schedule it. Reflow presentation preserves address edits and clamps pixel scroll
  to the new page height, clears old hit regions and immediately repaints new ones.
- Win32 events are queued before calling application code, so the window procedure
  never performs networking or mutates engine history.
- HTTP URL parsing is a documented subset, not full WHATWG URL conformance.
- ADR-0002 requires browser/renderer process separation, one renderer per active tab as the
  first implementation, explicit IPC, then sandboxing and measured process sharing.

## Active graph changes

Connected: Win32 navigation events -> op_browser command channel -> worker-owned
Engine -> NetworkContext -> RequestFilter -> allow/block -> op_net/WinHTTP-or-local source
pipeline -> result channel -> UI-thread NativeBrowserWindow::present -> WM_PAINT. Also connected: painted LinkSpan -> measured
LinkRegion -> scroll-aware mouse click -> FollowLink -> resolve_link -> same worker.
Engine preparation now connects parsed DOM -> bounded external stylesheet loading ->
DOM-order linked/embedded CSS collection -> selector matching -> cascade/inheritance ->
retained ComputedStyleMap -> CSS-aware layout -> display-list text styling -> Win32 pixels.
Reflow reuses author candidates and computed values without refetching/reparsing CSS.
CSS color parsing now feeds op_css::color for Lab/LCH/OKLab/OKLCH and predefined RGB/XYZ
space conversion -> D50/D65 adaptation where required -> encoded sRGB CssColor -> existing
layout/paint commands. Background colors also retain a private ComputedColorValue expression
when they depend on `currentColor`; inheritance copies that expression and resolves the used
CssColor against the receiving element. Initial `color-mix()` supports sRGB/LCH interpolation,
and the current relative-color slice preserves or overrides channels from `currentColor` before
feeding the same 8-bit paint path.
System/deprecated color identifiers feed the same CssColor path through a deterministic
browser-owned palette. The stylesheet parser can conditionally recurse into simple declaration
`@supports` blocks. Selector matching now also resolves inherited HTML language/direction
for `:lang()`/`:dir()`, plus initial open/required/optional/link-history state pseudos.
Separately, op_js now has source -> tokenize -> AST -> bytecode -> VM as an executable
standalone language slice, and op_browser_core has tab -> lifecycle/protection -> discard
candidate -> restore-state flow ready for later UI/renderer integration.
The block-box path includes used width/min/max/auto-margin geometry, per-side borders and
adjacent sibling margin collapse before BoxDecoration/background-border FillRects. Conservative
zero-height self-collapsing subtrees now merge their entire adjoining-margin set into the pending
block margin without advancing y; the analysis may pass through whitespace-only normal text,
undecorated inline wrappers and display:contents, which lets block-in-inline collapse through an
otherwise empty parent while preserving the ordinary path for visible/boxed content. `FlowRoot` and
`FlowRootListItem` establish an initial block formatting context: the block path snapshots the outer
float set, avoids floats overlapping its start position, lays out with a local float set, extends its
natural height to contained float bottoms, then restores the outer set. Float placement keeps the
normal-flow y unchanged, while `clear` advances to the bottom of matching active floats. Floated
tables preserve their dedicated table formatter rather than degrading into generic blocks.
`Display::Contents` uses the existing child/generated-content collection path without creating a
principal box; float on a contents-only element therefore does not create a float box. Table
formatting additionally pre-expands contents wrappers only when their exposed non-ignorable
descendants are table-internal, so anonymous row/cell fixup sees the correct structure while ordinary
text/inline contents nodes remain present to carry inherited style. Flex and SVG-specific contents
behavior remains separate work. Selector matching now also recognizes `::first-letter` as a
terminal pseudo-element. Its declarations are computed as a fragment pseudo rather than generated
content; layout overlays only explicitly authored inline properties onto the first non-whitespace
Unicode grapheme cluster, using UAX #29 segmentation so Regional Indicator pairs stay atomic. This
preserves descendant/`display:contents` inherited styles for properties the pseudo did not author.
Selector matching also adds attributes, +/~ and structural pseudos before the same cascade, including filtered nth selectors
whose `of` list may begin immediately after the `of` token. `:has()` parses a strict relative-selector
list and matches forward from its anchor through descendant/child/following-sibling relations while
reusing compound matching and normal specificity. Empty-namespace `[|attr]` uses the same HTML
attribute matcher without accepting whitespace between `|` and the name.
Nested inline text/image/empty/pseudo items retain parent-linked decoration stacks. Empty
inline elements/pseudos create an `EmptyInline` item only when edges or a required split continuation
reserve fragment geometry; a background alone on zero content does not fabricate a line. Flow owns
the InlineBoxes arena; each character stores one optional index without copying ancestors per
character. A block child can split the active arena path into continuation nodes, preserving fragment
history while suppressing the logical ending/continuing edge for LTR or RTL. Cached cumulative edges
are recomputed after a split and iterative common-ancestor transitions participate in width fitting,
wrap and alignment. Lines allocates outer decorations when opening fragments, then fills bounds when
closing, so nested opaque backgrounds paint in containment order. Image own boxes remain atomic inside ancestor fragments; fitting reserves
ancestor edges while percentage dimensions retain the containing block width as their basis.
InlineImage separates used dimensions from optional pixels. Missing sole-URL and DOM empty/
absent-alt images use zero natural dimensions, independent CSS size axes and shared atomic/
block decorations without allocating pixels or native image hit regions. Nonempty alt uses
the styled inline formatter or BlockContent::ImageAlt normal block path. Mixed generated
failures omit anonymous images while preserving empty pseudo decorations.
Next: sliced inline decoration edges and broader computed values.

Positioned paint metadata flows from `op_css::ComputedStyle::z_index` into
`op_layout::flow::Style`, then `Context::mark_positioned_outputs_since`
produces `BoxDecoration::paint_key` and positioned `LayoutItem` variants.
`flow::layout` walks the finalized DOM to assign preorder source indices.
`collect_paint_groups` derives `PaintGroup { key, parent }` metadata from
positioned ancestors with explicit z-index (and fixed-position ancestors),
including contexts without their own visible output. `op_paint` reconstructs
the child groups by parent key and traverses them iteratively with CSS-like
negative/background/foreground/positive phases. InlineBoxStyle now carries an
optional PaintKey for positioned relative inline elements; InlineBoxes resolves
the innermost ancestor key and Lines tags inline decorations, text/images and
ordinary atomic outputs. LayoutItem.with_optional_paint_key leaves in-flow
items untouched when no inline key exists. PaintGroup.inline_owner identifies
contexts where the inline background must precede nested atomic block
decorations. Independently formatted inline-block/table/flex boxes clear outer
inline arena indices before constructing local Context instances.
Context::block now always calls mark_positioned_outputs_since for relative
boxes after geometry translation; this tags only previously unpositioned output
even when descendants already carry their own paint keys. Auto-z blocks and
inline-blocks enter the zero-level sibling order without creating an atomic
ancestor in collect_paint_groups. The result is an incremental CSS stacking
slice, not the full CSS painting algorithm.

Table layout adds table_cell_explicit_descendant_width to the existing intrinsic
column measurement. For width:auto, table_box feeds column widths a shrink-to-
preferred width basis instead of always using the containing block width.
table_part_ancestors resolves structural table ancestry for each placed cell;
table_part_offset and translate_table_cell move the cell's paint records without
changing its normal flow slot. paint_positioned_table_part_backgrounds emits
source-ordered relative row/section backgrounds and skips background on empty
rows whose only contents are absolute. layout_table_cell temporarily adds the
nearest relative table section/row/cell to the positioning stack for absolute
children. The fixed manifest moves Positioning WPT 38/100 to 53/100.
The later table wrapper consistency slice moves the grid's intrinsic
min/preferred width computation ahead of wrapper caption/background geometry
in table_box. For width:auto, it resolves the shared content width including
horizontal table spacing and box extras, then passes that same content width to
column layout. The original auto-width flag preserves auto layout when
table-layout:fixed is requested without a definite author width. No additional
engine module or dependency is introduced. Rowspan reconciliation in table_box
separates initial per-row height/baseline measurement from multi-row spanning
constraints. The latter sort by end row, add any deficit to the last covered
track, calculate row-origin deltas, and translate TableCellLayout paint ranges
before applying per-cell alignment and relative-table offsets. Thus later
rows advance correctly without measuring all cell content twice.

Additional static compatibility paths: computed::parse_oklab_function and
parse_oklch_function call color::oklab_to_srgb_gamut_mapped, which only
reduces chroma when the corresponding neutral lightness would round to
black on an 8-bit SDR surface. Brighter values keep the previous clipping
path. flow::collect_children suppresses direct select text nodes without
suppressing option descendants. flow::collect distinguishes SVG ancestry
when processing defs, text with display:contents, outermost SVG unboxing,
and limited use -> referenced text children; recursive traversal is bounded
during id lookup. These slices improve WPT Static 187/200 to 191/200, with
no additional external engine dependency.

ComputedStyle now holds independently cascaded opacity and a supported
single invert() filter value, with CSS-global keyword handling. flow::Style
carries them into block painting; collect_paint_groups adds opacity/filter
owners as atomic ancestors of positioned descendants and records effect
amounts. op_paint::build_display_list emits BeginLayer/EndLayer for each
affected subtree without reordering negative or positive z-index phases.
op_platform_win::paint_commands_inner matches nested layer boundaries and
composite_layer paints each group onto paired GDI white/black DIBs,
recovers an approximate premultiplied BGRA image, applies inversion, and
AlphaBlends the result once onto the parent target. This compositor is
shared between the real Win32 window and headless WPT rendering. Bounded
layer allocation/depth falls back to unfiltered visible contents rather
than silently dropping them.

The next Static compatibility path adds ComputedStyleMap::background_image_for,
populated by winning_background_image (background or background-image
declarations with stylesheet origin tracking). Engine::images::load retrieves
the URL through Loader using the corresponding linked-style address and
retains background RasterImage Arcs. Engine::PreparedDocument::render passes
those resources through layout_document_with_backgrounds_and_resources;
flow::Context places images into block/table/cell BoxDecoration. Paint emits
BackgroundImage after fill color and before borders, while native
raster::paint_background repeats intrinsic tiles inside an HDC clip.
op_image::wic now resolves PNG ICC source contexts and transforms frames
to the display sRGB target before PBGRA conversion. :visited matching
recognizes only known-current-document empty href self-links and does
not expose arbitrary user history. This improves frozen Static 192/200
to 194/200 with no manifest or pixel-tolerance changes.

CSS Color 5 custom profile path: op_css::parse_color_profiles scans
stylesheet token streams for @color-profile declarations; op_engine::styles
loads bounded ICC bytes from the corresponding stylesheet-relative base
using the filtered resource loader. Before op_css::compute_styles, the
matched StyleMap resolves recognized color(--profile R G B) CSS function
tokens into output sRGB hex, preserving declaration source order,
importance, unrelated tokens and unknown color spaces. The resolver uses
op_image::convert_icc_rgb with native IWICColorTransform on a 1x1 bitmap,
caching repeated profile/color pairs in the engine. This is a profile-
aware computed color path, not a new renderer or external browser engine.
The frozen Static score rises 194 -> 195/200 on the same exact comparator.

CSS ::first-line text path: op_css::parser and op_css::style now collect
FirstLine selectors; op_css::computed creates a fragment-only pseudo style
per host. flow::Context::emit_with_first_line optionally passes the host's
text/background changes as FirstLinePaint into inline::Lines. During only
the first Lines::flush, inherited-color text runs are recolored and
optional background BoxDecoration ink is generated behind their text;
the paint rectangle reuses GDI text glyph metrics, matching the existing
inline-span decoration geometry. The text-line traversal still owns
wrapping and explicit breaks. Strict Static v1 reaches 196/200 while
Positioning remains measured independently.

Late currentcolor path: ComputedStyle::background_depends_on_currentcolor
reads the computed relative color expression; per-side border dependencies
are tracked in apply_border_declarations. flow::collect passes dependency
flags into InlineBoxStyle, and inline::Lines::flush re-resolves dependent
first-line fragment background/border ink to the host's pseudo text color.
Children with different explicit colors and later lines retain their original
colors. The unchanged static currentcolor-003 image now passes exactly,
raising strict Static WPT from 196/200 to 197/200.

The WPT compliance reporting pipeline now runs two independent checks
against the same rendered BGRA surfaces: the original strict byte-for-byte
comparison, and an optional source-authored fuzzy report. In
op_browser::bin::wpt_probe, --report-wpt-fuzzy reads the original test
document through op_html::Tokenizer; parses WPT meta[name=fuzzy] bounds
plus reference-specific overrides; computes actual maximum RGB channel
difference and count of nonidentical pixels; then requires BOTH fields
to fall within their inclusive ranges. Only extra console metrics are
emitted, leaving the pre-existing JSON schema, CI badge, pinned manifests,
and raw strict pass/fail score intact. The default ComputedStyle font-size
is normalized to customary 16px, fixing em/rem sizing for WPT tests.
Strict Static remains 197/200; optional WPT-authored metadata yields
198/200. Neither metric reports the two Rec.2020 cases as passing.

Flow synthesizes own EmptyInline items after collecting hidden/empty/collapsible-space
descendants if no visible item was produced. A block epoch counter guards against emitting
an extra empty inline fragment after a block child/pseudo flushed the collection. Nested
empty frames share arena ancestry rather than adding duplicate parent items.

## Compatibility measurement graph

The external conformance path is now explicit and versioned:

```text
compat/test262-parser-v1.txt
  -> op_js::bin::test262_probe
  -> op_js::parse_script
  -> JSON + badge metric

compat/wpt-static-v1.tsv
  -> op_browser::bin::wpt_probe
  -> Engine::render_source(test + reference)
  -> DisplayList
  -> op_platform_win::render_display_list_to_bgra
  -> shared paint_command/GDI path
  -> exact BGR pixel comparison
  -> optional first 12 failed reference/test BGRA BMP pairs
  -> JSON + badge metric

.github/workflows/ci.yml
  -> pinned upstream checkouts from compat/upstream.env
  -> tools/compatibility.ps1
  -> workflow artifact
  -> metrics branch on successful main push
```

`render_display_list_to_bgra` owns only an offscreen top-down DIB surface; it does not
replace layout or paint semantics. Both visible WM_PAINT and compatibility reftests call the
same `paint_command` implementation. The Test262 probe remains parser-only and does not
pretend that the current VM implements the Test262 runtime harness.

## M4.6: parser pause during HTML tree construction (current path)

op_html::parse_document_with_script_hook invokes the engine at a closing
script tag after flushing its text, before inserting later HTML tokens.
op_engine::scripts::parse_and_execute retains one ParserScriptRunner
across those callbacks. Bounded classic inline and same-origin external
scripts use the shared JsRuntime. Detached DOM text mutations are applied
to op_dom::Document before tree building resumes. The JS-visible DOM
snapshot is refreshed between scripts and once after completion, without
discarding globals, closures or click listeners registered earlier.

Unlike the M4.1/M4.2 historical post-parse implementation above, the
current engine uses this parser-pause path. Tokenization remains eager,
and document.write, async/defer, script modules and DOM lifecycle events
are still unsupported.

## M4.7: bounded defer/async resource scheduling (current)

The op_engine::scripts::parse_and_execute parser callback now schedules
same-origin external classic defer/async source fetches on scoped workers
using the page's immutable NetworkContext. Each fetch reserves a bounded
byte allowance, shares request/deadline accounting, and sends a completion
record over a channel. Only the engine thread can run JsRuntime::eval_script
or apply detached DomTextMutation updates.

ParserScriptRunner::drain_ready polls async completions at script closing
tags; deferred completions wait in a keyed table and are executed after
tree construction in original document order. Initial load drains remaining
async work before paint. Scoped workers cannot outlive page preparation.
An independent browser event loop remains unimplemented.

## M4.8: retained JS document lifecycle

ParserScriptRunner advances document state once tree building finishes
and again after the initial async/defer load phase. JsRuntime holds
document/window event listeners and dispatches lifecycle callbacks on
the engine thread, applying queued text mutations before render.

## M4.8: page lifecycle state and events (current)

op_js::JsRuntime retains a document object with host-owned readyState,
exposes lifecycle registration on document/window, and dispatches
readystatechange, DOMContentLoaded and load callbacks in the original
bounded JS VM. Event objects expose target/currentTarget/type/eventPhase.
Callbacks may mutate the DOM through detached textContent mutations.

op_engine::scripts::ParserScriptRunner::advance_state moves the page to
interactive after parsing and complete after pending async work. The
engine fires DOMContentLoaded only after deferred scripts execute and
window load after async completion, applying mutations before layout.
There is no independent event loop or post-presentation async dispatch.

## M4.9: timer-driven repaint after page presentation

The retained JsRuntime keeps a Vec<PendingTimer> with monotonic Instant
deadlines and stable numeric handles, exposing setTimeout and clearTimeout
through window/global bindings. JsRuntime::next_timer_wait returns the
earliest timer deadline; run_due_timers executes a bounded number of
callback closures on the engine thread, without an OS thread for each
timer. Timer callback failures are counted but do not stop later tasks.

The browser worker in op_browser::main calls recv_timeout to process
commands or wake for a timer, and Engine::tick_timers applies detached
DomTextMutation records, recalculates computed CSS, creates a paint
display list and sends a non-navigation reflow to Win32. Replacing
PreparedDocument with a new navigation drops the entire prior timer
queue. This is a small macrotask scheduler, not a browser event loop
with microtasks, timers from network services or setInterval.

## M4.10: repeating timers and microtask checkpoints

The retained JsRuntime now stores PendingTimer entries with an optional
repeat Duration and a separate VecDeque of callback-only microtasks.
setInterval shares handles and cancellation with setTimeout. Repeating
tasks are reinserted before their callbacks, allowing cancellation from
inside the active callback, and are scheduled from the execution time
with a minimum 4ms interval.

queueMicrotask enqueues a callable in FIFO order. JsRuntime::execute,
finish_event_dispatch and run_due_timers drain bounded microtask
checkpoints before the next macrotask. If the per-checkpoint limit is
reached, next_timer_wait keeps the page worker awake to finish bounded
pending work. Total enqueues are capped per page to stop self-enqueue
loops. Promise resolution/reaction jobs are not implemented. Initial
external-script workers still finalize before presentation, and no
network-completion events enter this page task loop yet.

## M4.11: network completion tasks after initial paint

The JS runtime now owns a bounded queue of detached TextRequest records
and associated retained callable callbacks. opFetchText is intentionally
not named fetch because Promise/Response semantics are not available.
The engine worker consumes URL-only records, resolves them relative to
the page using the existing same-origin classic source subset, and
starts background IO with a cloned request-filter snapshot and a global
worker cap. NetworkContext::load_text_for_page checks same-origin and
resource filters before loading, accepts a limited set of text MIME
types, caps each response at 64 KiB and rejects cross-origin redirect
results. JS closures and live Document never enter network threads.

Completed requests return through mpsc tagged with the current page's
generation counter. Engine::tick_timers now also drains bounded network
completions, runs callbacks and microtask checkpoints in the retained
single-threaded JS VM, then applies textContent mutations and rerenders.
Engine::next_timer_wait polls 20ms only for in-flight IO and otherwise
allows indefinite worker sleep. Navigation invalidates old generations.
The cloned filter enforces current-at-start policy, but its per-clone
statistics are not yet merged into the originating filter.
