# Image Loading

OPBrowser supports initial `<img src>` loading for PNG, JPEG, GIF and BMP. Files,
HTTP(S) resources and `data:image/...` URLs feed the same owned resource pipeline.
Only the first frame is decoded; GIF animation is not implemented.

## Ownership and source policy

The HTML parser already decodes references in src/href/alt attributes. After a
document loads, op_engine::images walks image nodes outside hidden head/title/style/
script/template subtrees. Relative sources use the effective loaded document
address, including redirects. Local image paths resolve beside the loaded file;
local pages can also use explicit file URLs/drive paths. Data documents have no
relative base, but can contain data or absolute HTTP(S) image sources.

Network pages cannot request local files. HTTPS pages reject HTTP image downgrades;
WinHTTP also rejects HTTPS-to-HTTP redirects. Cookies/automatic authentication stay
disabled. Missing Content-Type or application/octet-stream permits supported raster
signature detection; other HTTP types must be PNG/JPEG/GIF/BMP. HTML responses are
rejected. Image data URLs accept percent encoding or base64; unsupported parameters
or types fail. These policies are distinct from page-link navigation.

op_net owns bounded binary transport. op_image owns immutable, validated top-down
premultiplied BGRA RasterImage buffers. Targeted windows 0.62.2 bindings call explicit
Microsoft WIC decoders for the supported container signatures. WIC only handles
the raster codec, not HTML, DOM, CSS, layout, painting or Web APIs. See Microsoft's
[WIC overview](https://learn.microsoft.com/en-us/windows/win32/wic/-wic-about-windows-imaging-codec).
COM interfaces remain local to decoding and are released before balancing COM
initialization. Only owned pixels cross the worker/UI channel.

## Bounds and lifecycle

- At most 32 visible DOM/generated image candidates receive resource processing per page,
  sharing one budget. Computed display:none subtrees and suppressed pseudos are skipped.
- At most eight distinct resolved sources are attempted, including failed loads.
- Each encoded image is limited to 4 MiB; the page accepts at most 8 MiB of
  successfully returned encoded resources. Partial failed responses are bounded by
  the per-request limit and attempt count, not added to that accepted-byte budget.
- Decoded images allow at most 4096 pixels per dimension and 4,194,304 total pixels
  (16 MiB BGRA). Width/height/budget checks occur before pixel allocation/copy.
- Unique raster buffers together are limited to 32 MiB per page. Repeated successful
  sources share Arc pixels; failed source results are cached within the page too.
- New source attempts stop after ten elapsed seconds; image HTTP operations use
  two-second timeouts and check a five-second elapsed deadline between body reads.
  These checks are not a hard total wall-clock cap on synchronous OS operations or
  codec work. Images load serially before the worker publishes the page.

Navigation/reload builds a fresh per-page cache; there is no persistent image cache.
The active page retains its successful per-node Arc resources for
[Page Reflow](Page-Reflow.md); resizing performs no image requests or decoding.
Old/new display lists can coexist while the worker prepares a replacement, so the
page budget is not a total-process memory limit. GDI temporarily copies one source
raster into a DIB for each visible paint; codec/OS working memory is separate from
the owned buffer limits. No image timers or animation CPU run in the background.

## Layout, fallback and native drawing

Images share measured lines with text, align their bottom edge to the text baseline
and wrap as atomic boxes. See [Inline Layout](Inline-Layout.md). Natural sizes
are used unless supported integer width/height attributes override them. One
dimension preserves aspect ratio; two dimensions may stretch. Oversized attribute
values above 4096 are ignored; zero dimensions suppress drawing/fallback. Boxes
shrink proportionally to fit content width and cap displayed height at 4096.
DOM img padding/background/solid borders form atomic inline boxes around those content
dimensions. Width fitting reserves edge space, the border-box bottom aligns to baseline,
and nowrap keeps image boxes on the current line. Image hit regions cover the raster content.
Surrounding text retains order; unavailable/blocked/over-budget images use `alt`
(or `[image]` when alt is absent). An explicitly empty alt remains empty.
An image failure does not fail document navigation or roll back history.

ImageBox and Image paint commands carry Arc pixels and inherited anchor hrefs.
The native backend uses a transient selected DIB/DC and [GDI AlphaBlend](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-alphablend)
with per-pixel alpha, then restores/releases GDI objects. Fully clipped images are
skipped. Image rectangles use the existing toolbar/scroll-aware click/cursor hit
testing and clear when the display list is replaced.

Full CSS replaced-element layout, progressive loading, animation, srcset/picture,
CSS image sizing, SVG/WebP/AVIF, EXIF orientation and color management remain future work.

## CSS generated image content

Before/after content accepts quoted/unquoted `url()` mixed with strings, attr/counter values
and quotes. ComputedPseudoStyle.items retains text/image order and the consuming style_node.
External URLs resolve relative to the effective stylesheet address after redirects; embedded
and inline source bases use the document. Custom URLs stay unresolved until consumed by a
normal content declaration, so var() uses the consuming stylesheet's base.
References: [generated image values](https://www.w3.org/TR/css-content-3/#content-image),
[relative URLs in custom properties](https://www.w3.org/TR/css-variables-1/#defining-variables).

PageImages retains generated Arc resources by (host NodeId, pseudo, item index) beside DOM
images. Both share the cache and all bounds above; failed URLs remain cached. Generated
images use intrinsic sizes, shrink to available content width, share normal text baselines,
wrap atomically and can enter generated block flow. They inherit anchor href for native clicks.
Unavailable generated images add no inline image or alt label. Their surrounding content
still renders. Sole-image CSS replaced sizing, image modifiers/gradients and alternative-text
content syntax remain future work. In-memory render_html/set_html_page still perform no
subresource loading; use source navigation for images.

    cargo run -p op_browser -- --image-smoke-test examples/css/generated-images.html
    cargo run -p op_browser -- --link-smoke-test examples/css/generated-images.html

## Verification

    cargo run -p op_browser -- examples/images/index.html
    cargo run -p op_browser -- --image-smoke-test
    cargo run -p op_browser -- --link-smoke-test examples/images/index.html
    cargo run -p op_browser -- --image-smoke-test examples/images/inline.html
    cargo run -p op_browser -- --link-smoke-test examples/images/inline.html

The fixtures contain original generated color pixels, not external assets. Native
smokes require actual raster painting and a click on the image link. Unit/integration
tests cover four codecs, exact PNG premultiplication, oversized/truncated input,
data and redirected HTTP images, duplicate/failure caching, byte/pixel/node/attempt
budgets, dimensions, alt/history behavior and real GDI colors/alpha/hit regions.
