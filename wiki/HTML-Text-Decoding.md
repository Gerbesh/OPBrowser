# HTML Text Decoding

The byte loader and tokenizer perform two separate conversions: document encodings
turn source bytes into Unicode; HTML character references turn escaped source text
into text/attribute characters. Neither operation uses an existing browser engine.

## Document encodings

op_net::encoding is a shared, dependency-free module used by HTTP, files and HTML
data URLs. It currently supports:

- UTF-8 and its supported Encoding Standard aliases;
- UTF-16LE/BE, including BOM-selected Unicode;
- Windows-1251 (`windows-1251`, `cp1251`, `x-cp1251`);
- Windows-1252 and the standard ASCII/Latin1 label aliases.

The compact single-byte tables follow the [WHATWG Encoding indexes](https://encoding.spec.whatwg.org/#indexes),
with source links and index dates in the code. In particular, Windows-1251 Cyrillic,
Ё/ё and № load correctly instead of producing UnsupportedCharset.

Selection order is BOM, transport charset (HTTP Content-Type or data URL parameter),
then early HTML meta. The prescan examines at most 1024 bytes, skips comments and
quoted attributes in other tags, accepts charset declarations or a Content-Type
http-equiv pragma, ignores duplicate attributes after the first, and skips unknown
meta labels. UTF-16 meta declarations select UTF-8 as specified by the
[HTML prescan rules](https://html.spec.whatwg.org/multipage/parsing.html#prescan-a-byte-stream-to-determine-its-encoding).
The implementation is an initial subset, not complete encoding sniffing.

No recognized declaration defaults to strict UTF-8 to preserve the initial source
loader behavior. Unknown transport labels still fail; malformed UTF-8/UTF-16 produce
typed errors. Locale/heuristic detection, XML declaration detection, parser restart,
replacement-mode Unicode decoding and other legacy encodings remain future work.

The HTTP limit applies to decompressed input bytes (2 MiB); converting legacy/UTF-16
input to the engine's UTF-8 String can expand the text size.

## HTML character references

op_html::references implements numeric decimal/hexadecimal references, optional
numeric semicolons, invalid-scalar recovery and HTML's legacy C1 numeric mapping.
Named-reference support is a deliberate common subset: amp/lt/gt/quot/apos/nbsp,
copy/reg/euro/trade, ndash/mdash/hellip, laquo/raquo/times/divide/bull and selected
standard uppercase variants. Unknown names are preserved. The full named table
and multi-character named results remain future work.

The original tokenizer consumes references in text and all attribute-value states.
It respects legacy attribute ambiguity for names without semicolons and does not
recursively decode output. `&lt;b&gt;` stays literal text; it never becomes a b element.
Href `?a=1&amp;b=2` becomes the actual link `?a=1&b=2` before layout/hit testing.
These rules follow the [HTML character-reference states](https://html.spec.whatwg.org/multipage/parsing.html#character-reference-state).

Initial raw-text handling preserves script/style source, including literal markup
and ampersands. Title/textarea RCDATA treats internal markup as text while consuming
references. Full script escaped/double-escaped states and complete HTML tokenizer
conformance are not yet implemented.

## Verification and example

Loopback HTTP tests cover Windows-1251 headers, meta-only selection and BOM override.
Unit tests cover aliases, precedence, comments/quoted meta lookalikes, prescan bounds,
Unicode errors, numeric references, escaped markup and href query separators.

examples/encoding/windows-1251.html intentionally contains Windows-1251 bytes and a
meta declaration. Engine tests assert exact Cyrillic/decoded paint text and link
metadata. CI renders it through the native address/worker/repaint smoke path:

    cargo run -p op_browser -- --navigation-smoke-test examples/encoding/windows-1251.html

To verify its link as well:

    cargo run -p op_browser -- --link-smoke-test examples/encoding/windows-1251.html
