# CSS Syntax Foundation

OPBrowser owns CSS tokenization, selector/declaration parsing, author matching, cascade and
the current computed-style subset in `op_css`. Supported values now feed layout and paint;
this page records the syntax/matching boundary rather than a parser-only prototype.

## Current flow

```text
CSS text
  -> tokenize()
  -> Token { kind, start, end }
  -> parse_stylesheet() / parse_declaration_list()
  -> Stylesheet
  -> StyleRule
  -> Selector + Specificity
  -> Declaration
```

The tokenizer recognizes whitespace, comments, identifiers, hashes, strings and
escapes, numbers, percentages, dimensions, functions and CSS structural punctuation.
Malformed comments and strings produce recoverable CssError values with byte offsets.
Unquoted `url(...)` is one Url token, preserving punctuation in paths and data URLs and
decoding escapes. Quoted url() remains a function containing a String token. Internal
unescaped whitespace, quotes, parentheses, control characters and newline escapes produce
BadUrl; recovery consumes through an unescaped closing parenthesis. EOF retains the URL
value with a diagnostic. BadUrl/BadString invalidate whole declarations, including custom
properties and unused var() fallbacks; URL payloads count toward expansion storage limits.
Generated before/after content now loads URL images through the bounded image worker;
CSS background images remain outside the supported subset.
Reference: [CSS URL tokenization](https://www.w3.org/TR/css-syntax-3/#consume-a-url-token).

The stylesheet parser keeps valid rules after malformed declarations or unsupported
rules where recovery is possible. Declaration parsing preserves custom-property name
case, normalizes ordinary property names to ASCII lowercase and extracts trailing
!important.

## Current selector subset

The current selector AST/matcher supports:

- type, universal, class and ID selectors;
- compound and comma-separated selector lists;
- descendant and child (`>`) combinators;
- adjacent (`+`) and general (`~`) element-sibling combinators;
- attribute existence plus `=`, `~=`, `|=`, `^=`, `$=`, `*=` matchers;
- explicit ASCII `i`/`s` attribute-value flags;
- `:root`, `:first-child`, `:last-child`, `:only-child`, `:empty`, `:link`;
- specificity counts with attributes/pseudo-classes in the class column.

Matching runs right-to-left. Sibling combinators operate on element siblings and therefore
ignore intervening text nodes, matching CSS tree semantics. `:empty` still treats any text,
including whitespace text, as content. The functional/pseudo-element expansion is described
below; namespace selectors and at-rules remain explicit unsupported syntax.

## Author style collection and matching

The engine now collects CSS from embedded style elements and from style attributes.
The supported selectors are matched right-to-left against op_dom. Each matching element
gets StyleMap candidates rather than a prematurely resolved winner. Functional
`:is()`/`:where()`/`:not()` recursively reuse the same Selector matcher, while
`:nth-child()`/`:nth-last-child()` index element siblings matching optional `of` filters.
Selectors 4 `:lang()` uses case-insensitive RFC 4647-style extended filtering over the nearest
inherited HTML `lang`, including wildcard subtags and comma-separated ranges. `:dir()` currently
resolves inherited valid `ltr`/`rtl` HTML direction values; full `dir=auto` bidi inference is
later work. Initial `:required`, `:optional`, `:open` and `:visited` parsing/matching is present;
visited history is deliberately not exposed yet and therefore never matches.
Terminal `::before`/`::after` are represented
as a selector target rather than fake DOM nodes and contribute type-level specificity.

```text
DOM
  -> collect <style> + style=""
  -> parse author CSS
  -> match supported selectors
  -> MatchedDeclaration { declaration, specificity, source_order, source }
  -> StyleMap host bucket keyed by NodeId
     + pseudo buckets keyed by (NodeId, Before|After)
  -> retained StyleCollection in PreparedDocument
```

Stylesheet and inline declarations remain distinguishable so the next cascade stage can
apply the correct precedence instead of encoding an approximation now. Selector-list
rules are stored once per declaration using the highest specificity among selectors that
matched that element. CSS parser errors are retained with the NodeId that supplied them.
A style element with a non-CSS type is ignored in this initial collection layer.

Resize reflow reuses the retained StyleCollection together with the DOM and image
resources, so CSS is not reparsed merely because the window changed size.

## Cascade, inheritance and computed values

Author candidates now resolve into a ComputedStyleMap. For supported declarations the
cascade key is, from strongest dimension to weakest: !important, inline-style source,
specificity, then source order. Invalid values for a supported property are filtered
before choosing the winner, allowing a lower-priority valid declaration to apply.

The current computed subset includes display, text color/font sizing/weight, block
backgrounds, margin/padding/border edges, width/height min/max and box-sizing. A color-only
`background` shorthand shares cascade winner selection with `background-color`. Color accepts
#RGB(A)/#RRGGBB(AA), all 148 opaque CSS named colors (including aliases/rebeccapurple), legacy comma and modern
space/slash rgb()/rgba(), hsl()/hsla() and modern hwb() with hue angle units and alpha. HWB
whiteness/blackness accept percentages or numbers on the 0..100 reference scale; nonnegative
values above 100 normalize to gray rather than clamp individually. none components resolve
to zero for current used-color painting. Comma HWB syntax is invalid. The same color
parser feeds text, background and borders. Font-size accepts percentages, px/em/rem,
absolute CSS length units and the current keyword set. Global inherit/initial/unset handling
is shared by the supported properties.

color, font-size, font-weight, line-height and text-align inherit through the element tree.
display uses its initial value unless explicitly inherited. Temporary UA defaults mirror the existing M1
block/hidden tag rules and heading font sizes/weights, avoiding an unrelated visual
regression when layout begins consuming computed styles. The CSS hash tokenizer also
accepts digit-leading hash values required by hexadecimal colors.

CSS custom properties now have their own computed token environment. Case-sensitive `--name`
declarations use the same author importance/specificity/source-order cascade, inherit by
default, and resolve nested `var(--name, fallback)` references on the element where the custom
property is computed. The resolved map is then used to substitute `var()` into ordinary
property declarations before the existing value parsers run. Missing/cyclic references can use
nested fallbacks; all directed cycle participants become unavailable, including cycles in
unused fallback branches. Noncyclic consumers can still recover with fallback. `initial` removes
a custom property while `inherit`/`unset` reuse the parent's computed value.

Pseudo buckets cascade independently from their host. A generated pseudo starts from the
host's inherited text and custom properties, applies its own declarations, and becomes a
`ComputedPseudoStyle { style, content }` only when a supported `content` value generates text.
The current `content` subset concatenates quoted strings with `attr(name)`, `counter()` and
`counters()`. `attr()` reads the originating element's DOM attribute and returns an empty string
when that attribute is absent. `counter-reset`, `counter-set` and `counter-increment` mutate a
scoped document-order CounterContext before generated content is resolved; nested reset stacks
feed `counters()` while sibling increments remain visible inside the same scope. Decimal,
decimal-leading-zero, lower/upper alpha/latin and lower/upper roman formats are supported.
`none` and `normal` suppress creation. Because `var()` substitution happens first, generated
content and counter declarations may come from inherited or pseudo-local custom properties. PreparedDocument retains both the author StyleCollection and
ComputedStyleMap, including host/pseudo CustomPropertyMap snapshots.
Resize reflow therefore does not reparse, rematch or recascade CSS.

## Rendering integration

ComputedStyleMap is now consumed by op_layout. display:none removes the subtree from
layout, display:block creates a flow boundary, display:inline stays in the current inline
flow, and inline-table/table/table-caption/table-column-group/table-column/table-header-group/
table-row-group/table-footer-group/table-row/table-cell feed the table formatting machinery.
display:inline-table is parsed separately from display:table and enters inline flow as one atomic
object rather than forcing a block break. border-spacing accepts one/two nonnegative lengths and
inherits; border-collapse accepts separate/collapse and inherits. table-layout accepts auto/fixed
and is non-inherited; caption-side accepts top/bottom and inherits. Explicit-width fixed tables use
column hints and first-row widths without late-row intrinsic resizing, while auto tables retain
percentage constraints alongside content preferences. Captions are placed above or below the table
border box according to caption-side. vertical-align currently recognizes baseline/top/middle/
bottom as a non-inherited property; table cells use those values internally and atomic inline-table
boxes now carry the same values into line placement. Table UA style starts separate at 2px/2px.
These table properties reach the grid formatter, including zero spacing, shared-cell border
resolution and post-row vertical content placement.
The formatter performs anonymous row/cell child fixup inside display:table and normal-flow
collection groups consecutive orphan table-internal siblings under one anonymous block table,
including initial caption and column-hint repair, so CSS-generated table structures can recover
missing wrappers without altering the DOM. font-size, font-weight/style, line-height,
decoration, spacing, transform and
color are carried on inline character runs. text-align offsets completed lines, while
white-space controls collapse/newline preservation/soft wrapping. text-transform is applied
before measurement and link-range reconstruction; spacing changes wrapping and native paint.
Italic styling participates in both measurement and GDI font creation.

Text color reaches op_paint and the Win32 painter through TextBox/paint commands. Block
background, margin/padding, sizing/box-sizing and independent solid border sides also flow
through computed style into layout BoxDecoration geometry and platform-neutral FillRects.
Non-replaced inline elements now resolve background-color/padding/solid borders into an
InlineBoxStyle, contribute those extras to line fitting, and emit per-line BoxDecoration
fragments before text painting. Generated `::before` text is inserted before real children and
`::after` after them; both use the same line formatter, text transforms/spacing, and fragment
paint path. Inline box identity includes the pseudo target so generated and host decorations
stay distinct.
Alpha text/box colors are currently composited over the white page background. The existing
hyperlink glyph/underline path uses computed color and decoration. Anchors with href have
blue/underlined UA defaults before author cascade; nested/generated text can override them.
Anchors without href retain ordinary inherited presentation.

External `<link rel="stylesheet">` resources now join embedded rules at their actual DOM
positions before selector matching, so stylesheet source order crosses file boundaries.
The loader supports bounded local/file/data/HTTP(S) CSS and retains the resulting author
and computed styles across resize reflow. See [Stylesheet Loading](Stylesheet-Loading.md)
for activation rules, decoding, security policy and budgets.

The local demonstration page is `examples/css/index.html`; it now links a real
`examples/css/theme.css` and uses a later embedded override to expose source order.
The built-in start page also describes the supported subset in a normal release launch.

The expanded block-level box model is documented in [CSS Block Box Model](CSS-Box-Model.md).
Initial text alignment, line-height, font-style, underline/line-through, white-space,
letter/word spacing, text-transform and inline background/padding/solid-border fragments now
reach layout/native paint. Functional `:is()`/`:where()`/`:not()` and `:nth-child(An+B)` now
participate in selector matching with their initial specificity rules. Terminal
`::before`/`::after`, generated strings/`attr()`/counters, inherited custom properties and
`var()` fallbacks now reach native layout/paint too. Counter traversal computes `::before`
before children and `::after` after completed child counter work. Computed var() failures retain
their cascade priority and become unset for supported properties. Language-aware automatic quotes,
custom counter styles and complete counter scoping remain later. Next S3 work moves into generated
replaced content and nested-inline geometry, then relational selectors,
advanced color spaces, at-rules, media queries and full CSS
conformance remain later.

### Quotation marks

`quotes` inherits and accepts `auto`, `none` or one or more opening/closing string pairs,
plus initial/inherit/unset and `var()` substitution. Pseudos can override their host pairs.
The content keywords open-quote/close-quote emit the appropriate pair and change nesting;
no-open-quote/no-close-quote change nesting without glyphs. `quotes:none` suppresses glyphs
while preserving those nesting effects. Depth is shared in emitted document order, the last
pair repeats at deeper levels and closing at depth zero does nothing. Losing/invalid content
candidates, display:none subtrees/pseudos and content:none/normal never change that state.
The same hidden/absent-box exclusion applies to counter mutations. `<q>` gets default
open/close pseudos, overridable by author content. `auto` currently chooses deterministic
English Unicode pairs; language-specific selection remains future work.

Behavior reference: [CSS quotation marks](https://www.w3.org/TR/CSS2/generate.html#quotes).

Generated `display:block` now routes retained pseudo text through the ordinary block geometry
path instead of approximating a block with line breaks. It honors widths/heights/min/max,
percentages, box-sizing, auto/negative margins, padding, solid borders and backgrounds.
An empty string still creates the block box, allowing CSS-only rules and bars without glyphs.
Definite heights control normal flow and border/background extents even when text overflows.
Parent/child and empty-block margin collapse and full replaced/nested-inline geometry remain later.

### Custom-property dependency and resource limits

`op_css::custom` constructs a directed dependency graph per element/pseudo, including every
var() reference in fallback branches even if that fallback would not be selected. Iterative
Kosaraju SCC traversal invalidates exactly cycle participants; dependent values can recover
using their own fallback. Computed inherited values contain no var() references, preserving
their original meaning when descendants override other names. Valid empty custom values
substitute an empty token stream instead of selecting a fallback; bare `--` names are rejected.

Substitution permits at most 16,384 tokens and 256 KiB token storage (enum payload plus string
bytes) per value, 2 MiB retained resolved values per element/pseudo and 64 nested fallback
levels. Expansion checks happen before cloning tokens. Oversized values become invalid and
ordinary consumers can select fallback values. Graph traversal and dependency resolution
do not recurse on the native stack; tests exercise a 10,001-variable chain and exponential
expansion. Broader custom-property grammar, registration and animation-taint handling remain later.
Reference: [CSS variable cycles and length limits](https://www.w3.org/TR/css-variables-1/#cycles).

Computed declaration copies carry value_from_var so failed/empty substitutions and values
that fail the supported property's grammar resolve as unset while retaining importance,
specificity and source order. Shorthands compete separately for each longhand, preserving
later overrides. Literal parse-time invalid values still allow lower valid declarations.
Invalid content winners suppress pseudos and their counter mutations; invalid quotes inherit
and invalid counter operations reset to their initial empty list. var() syntax is validated
before cascade, including references inside unused fallback branches. A table-driven regression
compares missing/wrong-type/empty var() results with explicit unset across all 52 supported
ordinary style property names; Engine tests verify inherited/initial paint and retained reflow.

### Forgiving selectors and filtered sibling indexing

`:is()`/`:where()` discard invalid/unsupported/pseudo-element branches individually. Empty
or all-invalid argument lists are valid and match nothing; discarded branches contribute no
specificity. `:not()`, nth `of` filters and ordinary top-level lists remain strict. Nested
functional selectors are bounded to 64 levels before recursive parsing/matching.

`:nth-child(An+B of selector-list)` and `:nth-last-child(...)` filter inclusive element
siblings using the ordinary complex-selector matcher, count each element once even if several
branches match, and index from the front/back. Their specificity adds one pseudo-class plus
the maximum filter specificity, independent of the matching branch. An+B parsing uses integer,
n-dimension and n-ident token grammar, preserving sign/whitespace rules rather than joining
arbitrary tokens. Coefficients are bounded to i32 and arithmetic uses i64.
Reference: [Selectors 4](https://www.w3.org/TR/selectors-4/#the-nth-child-pseudo) and
[CSS An+B syntax](https://www.w3.org/TR/css-syntax-3/#anb-microsyntax).

Empty generated inline strings and empty DOM inline elements with their own padding/border/
background now produce real decoration geometry through an EmptyInline formatter item.
They reserve horizontal edges, participate in wrapping/nowrap/alignment and expand safe line
extents using font/vertical-edge metrics, while emitting no fabricated text or link spans.

Typed first/last/only-of-type pseudo-classes filter inclusive element siblings by the
candidate's HTML tag name. nth-of-type/nth-last-of-type reuse the same token-aware An+B
parser and indexing arithmetic, with same_type metadata instead of an authored of-list.
Other element types and text do not affect positions. All five contribute one class-level
specificity component; typed nth functions do not accept `of` arguments. Namespace-aware
typed selector behavior remains outside the current HTML-only selector subset.

HWB uses shared unquantized HSL channels before final 8-bit sRGB conversion. Hue angles
normalize before unit scaling, so large finite turns cannot overflow the color conversion.
Missing-component preservation for interpolation/serialization, relative colors and calc()
inside colors remain later work. Reference: [CSS Color 4 HWB](https://www.w3.org/TR/css-color-4/#the-hwb-notation).

Named colors use allocation-free ASCII case-insensitive binary search in op_css::named.
Packed names plus six-byte RGB records total 2,210 static bytes. The full source is pinned
in crates/op_css/data/named-colors.tsv; the offline generator verifies 148 names/139 distinct
RGB values, aliases and import-time hexadecimal/decimal agreement. Normal builds use the
checked-in Rust data. Transparent/currentcolor remain separate special keywords.
Reference: [pinned CSS Color named colors](https://www.w3.org/TR/2026/CRD-css-color-4-20260930/#named-colors).

CSS Color 4 device-independent and predefined spaces now participate in ordinary computed
color: lab(), lch(), oklab(), oklch(), color(srgb ...), color(srgb-linear ...), display-p3,
display-p3-linear, a98-rgb, prophoto-rgb, rec2020 and xyz/xyz-d50/xyz-d65. Percentage
reference ranges, D50/D65 chromatic adaptation, RGB transfer curves and matrix conversions
live in op_css::color; final used colors are clipped/rounded to the current 8-bit sRGB paint
target. currentColor resolves from the same element's computed color for background/borders,
and on the color property behaves as inherited color. Shared modern argument parsing still
rejects commas and malformed counts. Perceptual gamut mapping, missing-component preservation,
interpolation/serialization precision, calc(), relative colors and color-mix remain later work.
The same values feed text/background/border/pseudo cascade, var() substitution, painting and
retained reflow. CSS system colors use a deterministic browser-owned palette, with deprecated
CSS2 system names mapped to their CSS Color 4 modern equivalents; fixed system values are used
instead of exposing host theme details. Simple declaration-form `@supports (property: value)`
can now gate nested rules for the supported color declarations; boolean/composed supports
conditions remain later work. Reference: [CSS Color predefined spaces](https://www.w3.org/TR/css-color-4/#predefined).

Modern RGB/HSL also accept none channels/alpha through the shared modern argument parser.
RGB may mix numeric and percentage channels in modern syntax; comma syntax requires all
three channels to be numbers or all percentages. Modern HSL saturation/lightness accept
numbers on the 0..100 percentage reference scale and clamp before conversion; comma HSL
retains percentage-only saturation/lightness. Legacy forms reject none and mixed slash/
comma grammar. Invalid literals allow lower-priority valid declarations to win; invalid
var() winners resolve as unset. Reference: [CSS Color RGB/HSL syntax](https://www.w3.org/TR/css-color-4/#color-syntax).
