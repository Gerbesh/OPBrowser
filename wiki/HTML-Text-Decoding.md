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
Named-reference support includes the complete [WHATWG table](https://html.spec.whatwg.org/multipage/named-characters.html#named-character-references):
2125 case-sensitive names and 106 legacy spellings without a semicolon. Results
contain one or two Unicode scalars, including `&NotEqualTilde;` (U+2242 U+0338),
`&fjlig;` (f j) and `&ThickSpace;` (U+205F U+200A). The longest matching spelling
wins. `&notin;` becomes ∉, while `&notin` in text uses the legacy `not` prefix and
becomes ¬in. In attributes that prefix stays literal when followed by an ASCII
letter, digit or `=`. When no standard spelling/prefix matches, input stays literal.

The runtime uses OPBrowser-owned prefix-range lookup over a sorted compact table:
eight-byte entries, packed ASCII names and deduplicated UTF-8 results total 35,378
static bytes. It allocates no lookup strings and examines at most 31 input characters.
The pinned source spellings/codepoints are in crates/op_html/data/entities.tsv,
normalized from [entities.json](https://html.spec.whatwg.org/entities.json) with its
source SHA-256 retained. tools/generate_html_entities.py uses Python's standard
library to regenerate/check references/named.rs offline. Normal Cargo builds need
no Python or network. This is standard data incorporated under BSD-3-Clause, with
attribution/license retained in third_party/WHATWG-HTML-LICENSE.txt.

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

Exhaustive tokenizer tests verify all 2231 exact source spellings at EOF, before
punctuation, in single/double/unquoted attribute values and title/textarea RCDATA.
An independent longest-prefix oracle over the source snapshot checks suffix recovery
and attribute ambiguity. Additional tests cover two-scalar results, unknown/case
variants, bounded lookup and raw-text/nonrecursive decoding.

examples/encoding/named-references.html covers Latin/Greek/Cyrillic names, arrows,
math and legacy fallback. Engine tests assert exact paint text and UTF-8 link spans,
then follow its decoded Unicode query link. CI also runs the native click smoke:

    cargo run -p op_browser -- --link-smoke-test examples/encoding/named-references.html

The GDI text backend's existing font/shaping limits still apply; scalar preservation
does not imply complete typography or complete HTML parser conformance.

HTML comments beginning with <!-- in normal HTML are tokenized separately and stored
as ordered DOM Comment nodes. Tags, references and NULs inside them cannot create visible
text or elements; token data keeps literal references and replaces NUL with U+FFFD.
The owned iterative state machine handles abrupt empty closing, --!>, pending dashes,
nested markers and EOF recovery according to the
[WHATWG comment states](https://html.spec.whatwg.org/multipage/parsing.html#comment-start-state).
A comment splits adjacent DOM text nodes at its real position but does not affect CSS
:empty, element sibling matching, layout, open elements or retained reflow. Markers in
script/style/title/textarea and quoted attributes stay literal; RCDATA still decodes
references. The full script-data escape state machine remains future parser work.

Doctype declarations are separate tokens preserving lowercase names, PUBLIC and SYSTEM
identifiers (including empty versus missing) and the force-quirks flag. Their owned state
machine handles malformed quotes, missing identifiers, bogus trailing data and EOF using
the [WHATWG doctype states](https://html.spec.whatwg.org/multipage/parsing.html#doctype-state).
The first doctype in the initial insertion phase becomes a DOM DocumentType node; later
or in-element doctypes are ignored. The parser now stores DocumentMode as no-quirks,
limited-quirks or quirks using the WHATWG legacy public/system identifier matrix with
ASCII case-insensitive matching. Missing doctypes, force-quirks tokens, wrong names and
legacy quirks identifiers select quirks; XHTML 1.0 transitional/frameset and HTML 4.01
transitional/frameset with a non-empty system identifier select limited-quirks. Leading
ASCII whitespace in the initial insertion phase is ignored while comments remain DOM
nodes.

Tree construction now uses explicit initial, before-html, before-head, in-head, after-head,
text, in-body, after-body and after-after-body insertion modes. Missing html/head/body
elements are synthesized in their standard locations; comments are inserted according to
the active mode; title/style/script/noframes content uses a dedicated text mode and returns
to its previous insertion mode; permitted metadata tokens encountered after head are
inserted back under the stored head element. Repeated html/body start tags merge only
previously missing attributes, and the self-closing slash is ignored for ordinary non-void
HTML elements rather than incorrectly closing them.

The in-body mode now implements the first scope-sensitive recovery layer from WHATWG:
normal, list-item and button scope checks; implied end-tag generation; automatic paragraph
closure before block starts; li/dd/dt predecessor closure; heading recovery; nested-button
recovery; special-element boundaries for generic end tags; </br> recovery; and the legacy
<image> alias to img. Supported head-only tokens found after body parsing has started are
processed against the stored head element without discarding the current body stack.
Body/html end tags now move through after-body/after-after-body states without popping that
stack. Comments after body attach to html, comments after html attach to Document, specified
whitespace/html tokens delegate to in-body, and unexpected trailing content re-enters
in-body for recovery.

Active formatting elements are now tracked separately from the open-element stack for
a/b/big/code/em/font/i/nobr/s/small/strike/strong/tt/u. Stale entries are reconstructed
before relevant in-body insertion, identical entries are bounded by the Noah's Ark rule,
and formatting end tags run the bounded adoption agency algorithm instead of generic stack
popping. That path handles the no-furthest-block case and the furthest-block case that
clones/reparents DOM nodes. Repeated a/nobr starts use formatting recovery, while
applet/marquee/object create marker boundaries that are cleared on their matching end tags.

Table tree construction now includes in-table, in-table-text, in-caption, in-column-group,
in-table-body, in-row and in-cell modes. Missing tbody/tr wrappers are synthesized where the
tree-construction rules require them, cells close on conflicting table tokens, and each cell
uses an active-formatting marker boundary. Pending table character tokens keep all-whitespace
runs in the table, while non-whitespace runs and other misnested table content are foster
parented before the last open table. The DOM layer exposes insert_before so foster parenting
can place nodes at the required sibling position. Head text tokens such as style/script routed
from table mode return to that table mode after text parsing. The resulting table DOM now feeds
the initial table formatting context: row groups/rows/cells form a two-dimensional grid,
colspan/rowspan affect occupied tracks, captions flow above the grid, and table/cell
backgrounds, borders, padding and text reach paint. Columns now use measured cell content and
CSS sizing hints instead of equal shares, author border-spacing controls grid gaps, and collapse
mode resolves shared cell borders to a single winning edge. Template/frameset modes,
foreign-content parsing and remaining advanced table layout such as full percentage/intrinsic
rules, non-cell collapsed-border precedence, vertical-align and anonymous table boxes remain
future work.

Unknown <! declarations and CDATA-like declarations in the current HTML-only context become
Comment nodes and remain non-rendering. Raw-text/RCDATA and attributes still keep declaration
markers literal. Processing instructions, foreign-content/CDATA and the CSS/layout behavior
differences between document modes remain later work.
