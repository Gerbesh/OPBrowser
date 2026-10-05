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

- At most 32 visible-subtree img nodes receive resource processing per page.
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
Old/new display lists can coexist while the worker prepares a replacement, so the
page budget is not a total-process memory limit. GDI temporarily copies one source
raster into a DIB for each visible paint; codec/OS working memory is separate from
the owned buffer limits. No image timers or animation CPU run in the background.

## Layout, fallback and native drawing

Images currently occupy separate lines inside normal vertical flow. Natural sizes
are used unless supported integer width/height attributes override them. One
dimension preserves aspect ratio; two dimensions may stretch. Oversized attribute
values above 4096 are ignored; zero dimensions suppress drawing/fallback. Boxes
shrink proportionally to fit content width and cap displayed height at 4096.
Surrounding text retains order; unavailable/blocked/over-budget images use `alt`
(or `[image]` when alt is absent). An explicitly empty alt remains empty.
An image failure does not fail document navigation or roll back history.

ImageBox and Image paint commands carry Arc pixels and inherited anchor hrefs.
The native backend uses a transient selected DIB/DC and [GDI AlphaBlend](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-alphablend)
with per-pixel alpha, then restores/releases GDI objects. Fully clipped images are
skipped. Image rectangles use the existing toolbar/scroll-aware click/cursor hit
testing and clear when the display list is replaced.

Full inline replaced-element layout, progressive loading, animation, srcset/picture,
CSS image sizing, SVG/WebP/AVIF, EXIF orientation and color management remain future work.

## Verification

    cargo run -p op_browser -- examples/images/index.html
    cargo run -p op_browser -- --image-smoke-test
    cargo run -p op_browser -- --link-smoke-test examples/images/index.html

The fixtures contain original generated color pixels, not external assets. Native
smokes require actual raster painting and a click on the image link. Unit/integration
tests cover four codecs, exact PNG premultiplication, oversized/truncated input,
data and redirected HTTP images, duplicate/failure caching, byte/pixel/node/attempt
budgets, dimensions, alt/history behavior and real GDI colors/alpha/hit regions.
