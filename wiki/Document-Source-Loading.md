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

Unicode BOMs select and strip the encoding. Invalid Unicode and malformed percent/base64 payloads
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
  non-ASCII hostnames are rejected. Full WHATWG URL/IDNA processing is later.
- At most five redirects are followed; HTTPS-to-HTTP redirects are disallowed. The
  final response URL is returned as LoadedDocument.address and shown in the UI.
- Non-2xx statuses become typed errors. Only text/html is accepted (missing
  Content-Type defaults to text/html at this milestone).
- UTF-8, UTF-16LE/BE, Windows-1251 and Windows-1252 are supported with documented
  label aliases. ASCII/Latin1 labels use Windows-1252 mappings. Selection checks
  BOM first, then transport charset, then meta declarations in the first 1024 bytes.
  No declaration defaults to strict UTF-8; unsupported transport labels and malformed
  Unicode fail. See [HTML Text Decoding](HTML-Text-Decoding.md) for exact limits.
- Body limit after HTTP decompression and before charset decoding: 2 MiB. The final
  Unicode string can grow when converting single-byte or UTF-16 input to UTF-8.
  Resolve/connect/send/receive operation timeouts: 10 s.
  A 30 s elapsed deadline is checked between body reads; it is not a strict total
  wall-clock deadline, because synchronous OS operations can finish after it.
- Automatic cookies/authentication are disabled. No persistent cache or cookie
  store exists, and no request starts without an explicit document source.
- HTTP transport currently requires Windows. Local and data loaders are portable.
- After an explicit document load, visible img nodes may start bounded binary
  HTTP(S)/local/data image subrequests on the worker. See [Image Loading](Image-Loading.md)
  for their byte/time budgets and source restrictions. WIC raster decoding requires Windows.

Requests execute on the browser's navigation worker, preserving UI responsiveness.
Navigation history is committed only after loading and own-engine rendering succeed.
Loopback tests cover redirects, compression, framing and failure limits without
depending on internet availability. External HTTPS is verified separately with:

    cargo run -p op_browser -- --navigation-smoke-test https://example.com

## Hyperlink reference resolution

resolve_link handles absolute HTTP(S) links plus relative paths, root paths,
network-path references, query/fragment references and dot segments. Percent-encoded
dot segments are recognized without decoding escaped slashes. The base is the last
successfully loaded effective address, not the original pre-redirect request.

Local documents resolve relative paths beside the loaded file, with UTF-8 percent
decoding. Relative links from data URLs have no base. Other absolute schemes
(including file:, data:, javascript:, mailto: and ftp:) are rejected for page-link
navigation; address-bar source loading retains its existing file/data support.
Fragment references load the document again; anchor scrolling and HTML base-element
handling are not implemented yet. URL whitespace must be percent-encoded.
