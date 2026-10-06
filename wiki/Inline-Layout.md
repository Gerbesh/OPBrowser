# Inline Layout

The initial M1 layout places text and raster images on the same measured lines.
It remains an original OPBrowser algorithm; Windows GDI supplies font metrics only.

## Ownership and metrics

`op_layout::TextMeasurer` returns width, ascent and descent for a text run and font
size/weight. `op_engine::text` implements it using GetTextExtentPoint32W and
GetTextMetricsW on Windows. Each render caches selected Segoe UI fonts and scratch
DCs by size/weight, then restores/releases them before returning the display list.
Font creation matches the native painter and both use `op_paint::TEXT_FONT_FAMILY`.
Measurements split when href or computed inline style changes because the painter may
draw those runs separately. Whole-run extents preserve spacing that summing individual
characters would lose. Mixed font-size/weight runs are measured independently, then
aligned to one line baseline. Portable helpers and failed GDI calls use approximate metrics.

GDI does not select line breaks or HTML flow. `op_layout::flow` chooses the body,
filters hidden subtrees and groups consecutive inline siblings around block
children. `op_layout::inline` constructs lines and decides all box positions.
`LayoutTree::order` indexes TextBox/ImageBox storage in placement order; op_paint
emits this sequence instead of painting all text before all images.

## Supported behavior

- Text, nested inline labels, images and failed-image alt labels share lines.
- Images wrap as atomic boxes and align their bottom edge to the common baseline.
  Tall images expand the line ascent so subsequent lines do not overlap them.
- Font ascent/descent plus computed `line-height` establish each run's line strut.
  `normal`, unitless multipliers, percentages and lengths reach real ascent/descent geometry;
  larger styled runs increase the shared line box while smaller runs share its baseline.
- ASCII HTML spaces, tabs, LF, CR and form feed collapse to one space; leading and
  trailing collapsed spaces disappear. NBSP and other Unicode spaces stay intact.
- `br` forces a line; repeated breaks produce empty lines with normal line height.
- Words normally move whole to the next line. Oversized words use an emergency
  Unicode-scalar split. Exponential probing and binary search measure prefixes
  close to the available width instead of every remaining long suffix.
- UTF-8 href byte ranges are reconstructed for each wrapped text fragment; image
  rectangles retain inherited hrefs and use existing scroll-aware hit testing.
- Computed display:none/block/inline participates in flow. Heading/paragraph/list spacing
  is now represented as temporary UA computed margins in the block box-model path.
- Computed font-size, font-weight, line-height and text color can change inside one inline
  line. `text-align: start/end/left/right/center` offsets each completed line inside the
  content box after wrapping. Consecutive inline nodes around blocks form anonymous groups.

Image source policy, dimensions, viewport fitting and pixel budgets are described
in [Image Loading](Image-Loading.md).

## Verification and limits

Open `examples/images/inline.html` to inspect mixed lines, multiple images, breaks,
NBSP and inline fallback. Native CI checks:

    cargo run -p op_browser -- --image-smoke-test examples/images/inline.html
    cargo run -p op_browser -- --link-smoke-test examples/images/inline.html

Deterministic layout tests cover exact baseline/box geometry, atomic wrapping,
sibling/block boundaries, blank lines, whitespace, zero-sized images, alt links,
Unicode byte offsets and a long-word measurement-work bound. Windows engine tests
verify variable glyph widths, Unicode, font-cache reuse, exact GDI-based positions
and Text/Image/Text display-list order. Native smokes verify raster painting and
a click through to the linked destination.

This is still an initial left-to-right subset. Mixed computed inline size/weight/color
is supported, but inline padding/background/border fragments are not. Block-level box-model
support, including adjacent sibling margin collapse, is described in
[CSS Block Box Model](CSS-Box-Model.md). Parent/child margin collapse, font-style/families,
text decoration, white-space modes, advanced
shaping/font fallback, bidi, grapheme-aware/full Unicode line breaking, preformatted
whitespace modes, floats/tables/flex/grid remain future work. Hyperlink glyph color is
still the native default blue. These tests do not claim complete CSS conformance.

[Page Reflow](Page-Reflow.md) now rebuilds these lines on window resize using the
retained DOM and shared image pixels. Engine and painter synchronize Windows font
operations with `op_paint::GDI_TEXT_LOCK`; concurrent regression tests verify stable
extents. Layout and network work remain outside the gate.

Primary references: [Microsoft GDI text extents](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-gettextextentpoint32w),
[CSS2 inline formatting](https://www.w3.org/TR/CSS2/visuren.html#inline-formatting) and
[CSS Text whitespace processing](https://www.w3.org/TR/css-text-3/#white-space-processing).
