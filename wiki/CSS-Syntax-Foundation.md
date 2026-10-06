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
including whitespace text, as content. Functional pseudo-classes, pseudo-elements, namespace
selectors and at-rules remain explicit unsupported syntax rather than silently succeeding.

## Author style collection and matching

The engine now collects CSS from embedded style elements and from style attributes.
The supported selectors are matched right-to-left against op_dom. Each matching element
gets StyleMap candidates rather than a prematurely resolved winner.

```text
DOM
  -> collect <style> + style=""
  -> parse author CSS
  -> match supported selectors
  -> MatchedDeclaration { declaration, specificity, source_order, source }
  -> StyleMap keyed by NodeId
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
backgrounds, margin/padding/border edges, width/height min/max and box-sizing. Color accepts
#RGB(A)/#RRGGBB(AA), CSS basic named colors plus rebeccapurple, legacy comma and modern
space/slash rgb()/rgba(), and hsl()/hsla() with hue angle units and alpha. The same color
parser feeds text, background and borders. Font-size accepts percentages, px/em/rem,
absolute CSS length units and the current keyword set. Global inherit/initial/unset handling
is shared by the supported properties.

color, font-size, font-weight, line-height and text-align inherit through the element tree.
display uses its initial value unless explicitly inherited. Temporary UA defaults mirror the existing M1
block/hidden tag rules and heading font sizes/weights, avoiding an unrelated visual
regression when layout begins consuming computed styles. The CSS hash tokenizer also
accepts digit-leading hash values required by hexadecimal colors.

PreparedDocument retains both the author StyleCollection and ComputedStyleMap. Resize
reflow therefore does not reparse, rematch or recascade CSS.

## Rendering integration

ComputedStyleMap is now consumed by op_layout. display:none removes the subtree from
layout, display:block creates a flow boundary, and display:inline stays in the current
inline flow. font-size, font-weight, line-height and color are carried on inline character
runs, so a span can change typography without forcing a new line. text-align is inherited
into the block formatter and offsets completed lines inside the content box. Runs with
different metrics still share the same baseline.

Text color reaches op_paint and the Win32 painter through TextBox/paint commands. Block
background, margin/padding, sizing/box-sizing and independent solid border sides also flow
through computed style into layout BoxDecoration geometry and platform-neutral FillRects.
Alpha text/box colors are currently composited over the white page background. The existing
hyperlink glyph/underline path still paints native link blue, so author color on links
remains an explicit temporary limitation.

External `<link rel="stylesheet">` resources now join embedded rules at their actual DOM
positions before selector matching, so stylesheet source order crosses file boundaries.
The loader supports bounded local/file/data/HTTP(S) CSS and retains the resulting author
and computed styles across resize reflow. See [Stylesheet Loading](Stylesheet-Loading.md)
for activation rules, decoding, security policy and budgets.

The local demonstration page is `examples/css/index.html`; it now links a real
`examples/css/theme.css` and uses a later embedded override to expose source order.
The built-in start page also describes the supported subset in a normal release launch.

The expanded block-level box model is documented in [CSS Block Box Model](CSS-Box-Model.md).
Initial text alignment and line-height now reach layout geometry. Next S3 work moves into
inline box fragments/decorations, font-style/text-decoration and white-space controls.
Functional pseudos, pseudo-elements, advanced color spaces, at-rules, media queries and full
CSS conformance remain later.
