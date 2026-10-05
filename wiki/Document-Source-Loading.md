# Document Source Loading

Document loading lives in op_net so the engine does not care whether bytes came from
a local file, a data URL, HTTP, cache, or eventually another source.

## Supported now

```text
C:\pages\test.html
.\examples\hello.html
file:///C:/pages/test.html
data:text/html,%3Ch1%3EHello%3C%2Fh1%3E
data:text/html;base64,PGgxPkhlbGxvPC9oMT4=
```

NetworkContext::load_document returns a LoadedDocument containing the normalized
address, MIME type, UTF-8 text and SourceKind.

Local UTF-8 BOMs are stripped. Invalid UTF-8 and malformed percent/base64 payloads
produce typed LoadError values rather than silently replacing bytes.

## Not supported yet

HTTP and HTTPS intentionally return UnsupportedScheme. Network transport will be
added behind the same NetworkContext boundary, so HTML/DOM/layout code will not need
to know how the resource was obtained.

Navigation history is the next layer above this loader.
