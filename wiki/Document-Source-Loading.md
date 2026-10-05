# Document Source Loading

Document loading lives in op_net so the engine does not care whether bytes came from
a local file, a data URL, HTTP, cache, or eventually another source.

## Supported now

```text
https://example.com
http://example.com/path?query=value
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

## HTTP and HTTPS

op_net::http parses an initial HTTP URL subset and uses the system WinHTTP API for
GET transport, proxy settings, certificate-validated TLS, HTTP framing and gzip /
deflate decompression. WinHTTP never parses HTML, lays out documents or executes
scripts. The existing windows-sys package supplies bindings; the engine is original.

- Explicit http:// or https:// scheme is required; ASCII DNS names, IPv4, bracketed
  IPv6 and explicit nonzero ports are accepted. UTF-8 path/query bytes are percent
  encoded, existing escapes retained, and fragments omitted from the GET target.
- Credentials, whitespace/control characters, backslashes, malformed escapes and
  non-ASCII hostnames are rejected. Full WHATWG URL/IDNA and relative URLs are later.
- At most five redirects are followed; HTTPS-to-HTTP redirects are disallowed. The
  final response URL is returned as LoadedDocument.address and shown in the UI.
- Non-2xx statuses become typed errors. Only text/html is accepted (missing
  Content-Type defaults to text/html at this milestone).
- UTF-8 HTML, optionally with a UTF-8 BOM, is supported. UTF-8/us-ascii charset
  declarations are accepted; other declared charsets and invalid UTF-8 fail.
  Meta-tag charset sniffing and legacy encodings are not implemented yet.
- Decoded body limit: 2 MiB. Resolve/connect/send/receive operation timeouts: 10 s.
  A 30 s elapsed deadline is checked between body reads; it is not a strict total
  wall-clock deadline, because synchronous OS operations can finish after it.
- Automatic cookies/authentication are disabled. No persistent cache or cookie
  store exists, and no request starts without an explicit document source.
- HTTP transport currently requires Windows. Local and data loaders are portable.

Requests execute on the browser's navigation worker, preserving UI responsiveness.
Navigation history is committed only after loading and own-engine rendering succeed.
Loopback tests cover redirects, compression, framing and failure limits without
depending on internet availability. External HTTPS is verified separately with:

    cargo run -p op_browser -- --navigation-smoke-test https://example.com
