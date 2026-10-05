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

## What is not connected yet

Cascade, inheritance, computed values and the box model are not implemented yet.
Therefore pages still render with the M1 HTML/layout defaults even though their author
style candidates are now parsed, matched and retained.

The next S3 slice is:

```text
StyleMap candidates
  -> cascade / inheritance
  -> initial computed values
  -> CSS-aware layout
```

Linked stylesheets follow after the local author-style/cascade path is working and tested.
