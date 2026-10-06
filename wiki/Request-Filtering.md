# Request Filtering

`op_net::RequestFilter` is the browser's native request-policy layer. It runs before the
current document, stylesheet and image loading paths.

The initial syntax implements a deliberately small deterministic subset of common network
filter rules:

- `||example.com^` host/subdomain blocking;
- `*` wildcard text patterns;
- `@@` exception rules;
- resource options `$document`, `$stylesheet`/`$css`, `$image`, `$script`;
- per-site allowlisting.

The filter tracks checked/allowed/blocked request counters. Blocked loads return
`LoadError::BlockedRequest`. Stylesheet and image failures remain nonfatal at the engine
layer, so a blocked subresource behaves like another unavailable subresource rather than
destroying the loaded document.

This is only the network-filter foundation. EasyList-scale parsing/indexing, domain options,
third-party semantics, subscriptions, cosmetic filtering and a settings UI are not
implemented yet. Rule matching must be profiled before large public lists are enabled; a
linear scan is acceptable for the initial correctness slice but not the final design.
