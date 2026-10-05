# CSS Syntax Foundation

OPBrowser now owns the first M2 CSS syntax layer in the op_css crate. This layer
parses CSS text into structured data but deliberately does not affect layout or paint
yet.

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

## Initial selector subset

The current selector AST supports:

- type selectors such as p and article;
- the universal selector *;
- class selectors such as .card;
- ID selectors such as #hero;
- compound selectors such as article.card.feature;
- comma-separated selector lists;
- descendant combinators;
- child combinators using >;
- specificity counts for IDs, classes and types.

Pseudo-classes, pseudo-elements, attribute selectors, sibling combinators and at-rules
are not supported yet. They are reported as explicit parser errors instead of being
silently accepted.

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

The initial computed subset is:

- display: inline, block, none;
- color: black/white/red/green/blue/transparent and #RGB(A)/#RRGGBB(AA);
- font-size: bounded px values;
- font-weight: normal, bold, 400, 700;
- global keywords: inherit, initial, unset.

color, font-size and font-weight inherit through the element tree. display uses its
initial value unless explicitly inherited. Temporary UA defaults mirror the existing M1
block/hidden tag rules and heading font sizes/weights, avoiding an unrelated visual
regression when layout begins consuming computed styles. The CSS hash tokenizer also
accepts digit-leading hash values required by hexadecimal colors.

PreparedDocument retains both the author StyleCollection and ComputedStyleMap. Resize
reflow therefore does not reparse, rematch or recascade CSS.

## Rendering integration

ComputedStyleMap is now consumed by op_layout. display:none removes the subtree from
layout, display:block creates a flow boundary, and display:inline stays in the current
inline flow. font-size, font-weight and color are carried on inline character runs, so a
span can change typography/color without forcing a new line. Runs with different font
metrics share the same baseline.

Text color reaches op_paint and the Win32 painter through TextBox/paint commands. Because
background-color is not implemented yet, alpha text colors are currently composited over
the white page background before GDI drawing. The existing hyperlink glyph/underline
path still paints native link blue, so author color on links is an explicit temporary
limitation rather than silently changing hit-testing representation.

External `<link rel="stylesheet">` resources now join embedded rules at their actual DOM
positions before selector matching, so stylesheet source order crosses file boundaries.
The loader supports bounded local/file/data/HTTP(S) CSS and retains the resulting author
and computed styles across resize reflow. See [Stylesheet Loading](Stylesheet-Loading.md)
for activation rules, decoding, security policy and budgets.

The local demonstration page is `examples/css/index.html`; it now links a real
`examples/css/theme.css` and uses a later embedded override to expose source order.
The built-in start page also describes the supported subset in a normal release launch.

The next S3 work is the first box-model properties (background/border, margin and padding).
Broader selectors, values, at-rules, media queries and full CSS conformance remain later.
