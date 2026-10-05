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

## What is not connected yet

HTML style elements and style attributes are not collected by the engine yet. Selector
matching, cascade, inheritance, computed values and the box model are also not
implemented. Therefore current pages still render with the M1 HTML/layout defaults even
when they contain CSS.

The next S3 slice is:

```text
DOM
  -> collect <style> + style=""
  -> parse author CSS
  -> match supported selectors
  -> style map / styled tree
  -> layout-facing values
```

Linked stylesheets follow after the local author-style path is working and tested.
