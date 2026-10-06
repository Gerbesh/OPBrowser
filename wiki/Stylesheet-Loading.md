# Stylesheet Loading

OPBrowser now loads initial external author stylesheets from `<link rel="stylesheet">`
without delegating CSS parsing or cascade behavior to another browser engine.

## Supported sources

Active stylesheet links may load from:

- relative or absolute local paths when the document itself is local;
- `file:` URLs under the same local-source policy;
- `http:` and `https:` URLs through the existing WinHTTP transport;
- `data:text/css` URLs.

A network document cannot use a stylesheet link to escape into local files. An HTTPS
document cannot downgrade a stylesheet request to HTTP.

## Link activation subset

A `<link>` participates when:

- `rel` contains the ASCII-case-insensitive `stylesheet` token;
- `rel` does not contain `alternate`;
- `disabled` is absent;
- `type` is absent, empty, or `text/css`;
- `media` is absent, empty, `all`, or `screen`;
- `href` exists and resolves under the source policy.

Full media-query evaluation is not implemented yet. `print` and other media values are
currently skipped instead of guessed.

## Loading and budgets

External CSS is discovered during page preparation on the same navigation worker that
loads the document and image subresources.

Current bounds per prepared document are:

- at most 32 stylesheet link nodes are considered;
- at most 8 distinct stylesheet requests are attempted;
- one stylesheet is capped at 1 MiB;
- total decoded stylesheet text is capped at 2 MiB;
- stylesheet discovery/loading has a 10 second overall guard;
- HTTP stylesheet requests use a 2 second WinHTTP timeout and 5 second read deadline.

Duplicate resolved stylesheet URLs share one fetched text payload but each link node keeps
its own position in document source order.
The cache also preserves the final stylesheet address after redirects; each link NodeId
retains that effective base in PreparedDocument. Engine exposes active_stylesheet_address()
for resource-base inspection. Reflow reuses this metadata without fetching again.

A stylesheet load failure is nonfatal to the HTML document. The failed link contributes
no rules and preparation continues.

## HTTP and decoding

HTTP requests send an Accept preference for `text/css`. An explicit response MIME type
other than `text/css` is rejected; a missing Content-Type is temporarily accepted.

CSS text decoding uses this initial priority:

1. Unicode BOM;
2. HTTP/Data URL charset parameter;
3. an initial exact `@charset "...";` rule;
4. UTF-8 default.

The currently owned decoder supports UTF-8, UTF-16, Windows-1251 and Windows-1252.
The consumed initial `@charset` rule is removed before the CSS parser sees the text.

## Cascade order

Loaded external stylesheets are attached to their DOM link NodeId. `op_css` traverses the
document and parses embedded `<style>` and loaded `<link rel="stylesheet">` rules at their
actual DOM positions. This preserves stylesheet source order across external and embedded
rules before inline `style=""` declarations enter the author cascade.
MatchedDeclaration.style_node identifies the originating style/link node or inline-styled
element. Computed var() copies preserve that provenance for later CSS resource resolution.

A successfully prepared document retains its author StyleCollection and ComputedStyleMap.
Resize reflow therefore does not refetch external CSS or rerun stylesheet discovery.

## Deliberate current limits

This milestone does not implement:

- `@import` resource loading;
- general media queries;
- `<base>`-element URL changes for stylesheet resolution;
- CSS `url(...)` subresources such as fonts/background images;
- preload/module/style-set behavior;
- CORS/CSP/referrer/integrity processing;
- browser cache persistence across document preparations.

The parser currently reports unsupported at-rules rather than pretending they worked.

## Verification

The local demonstration uses a real external file:

    target\release\op_browser.exe examples\css\index.html

`examples/css/index.html` links `examples/css/theme.css` and also contains a later
embedded rule to verify source-order override behavior.

Tests cover local external CSS, a missing nonfatal stylesheet, retained CSS after the
source files are deleted and reflowed, loopback HTTP CSS requests, CSS MIME/charset
handling, byte limits and HTTPS/local source policy.
