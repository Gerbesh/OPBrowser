# Inline Layout

DOM image content sizes come from computed CSS width/height/min/max/box-sizing around the
intrinsic raster ratio. HTML size attributes are hints before author cascade; CSS auto resets
them. Percent widths/font-relative units and definite heights are supported. Percentage
height propagation remains later work. Existing viewport
fitting still runs after CSS constraints and can shrink below minima.

DOM img elements now carry their own padding/background/solid-border box. The outer width
participates in atomic wrapping/alignment; raster fitting reserves horizontal edge space.
The border-box bottom aligns to the text baseline and vertical edges expand safe line ascent.
Decorations use exact image bounds. nowrap suppresses soft wrapping of image items. Nested
ancestor box stacks remain around image boxes; native image
click regions still cover the painted raster rectangle.

Inline before/after with exactly one parsed URL now uses the same CSS dimensions and own
decorated atomic image box as DOM img. Mixed content lists retain intrinsic anonymous images,
including lists whose text resolves empty. Computed replacement identity survives retained reflow.

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
- `white-space: normal` collapses ASCII HTML whitespace as before. `nowrap` keeps the same
  collapse behavior but disables soft text wrapping. `pre` preserves spaces/tabs/newlines
  without soft wrapping; `pre-wrap` preserves them and wraps; `pre-line` preserves explicit
  newlines while collapsing other whitespace. Tabs currently expand to four spaces.
- `br` forces a line; repeated breaks produce empty lines with normal line height.
- Words normally move whole to the next line. Oversized words use an emergency
  Unicode-scalar split. Exponential probing and binary search measure prefixes
  close to the available width instead of every remaining long suffix.
- UTF-8 href byte ranges are reconstructed for each wrapped text fragment; image
  rectangles retain inherited hrefs and use existing scroll-aware hit testing.
- Computed display:none/block/inline participates in flow. Heading/paragraph/list spacing
  is now represented as temporary UA computed margins in the block box-model path.
- Computed font-size, font-weight, font-style, line-height, text decoration, spacing,
  text-transform and text color can change inside one inline line. Italic/oblique use italic
  GDI font realization for both measurement and paint. Underline/line-through are carried
  through the display list and painted over measured segment widths. `text-align` offsets
  each completed line. letter/word spacing participates in wrapping and native advances.
- Non-replaced inline elements produce real background/padding/solid-border fragments.
  Their horizontal extras participate in fitting and alignment; vertical extras enlarge the
  safe line box, and wrapped fragments become independent BoxDecoration records. An ordinary
  nested `<b>/<em>/<a>` without its own box continues the outer decorated fragment. Decorated
  descendants add their own frames inside the existing ancestor frames. A parent-linked arena
  stores each box once; characters carry one index, independent of decorated nesting depth.
  Text, images, empty boxes and generated pseudos share iterative ancestor transitions.
  All ancestor edges affect wrap/alignment and vertical extents. Inline background and border
  height uses the element's own font metrics with vertical padding/border, independently
  from line-height. Line-height still controls the parent line strut and half-leading;
  inline-block atomic alignment uses the last in-flow line's baseline rather than its box bottom.
  Outer decorations are allocated before inner decorations so opaque outer backgrounds paint
  first. The painter also draws block backgrounds before inline backgrounds, preserving inline
  ink that extends past a following block's top.
- Block children inside an inline now split the active decorated path into continuation nodes.
  Ending and continuing fragments suppress the appropriate logical edge for computed `direction`,
  so LTR and RTL preserve different physical start/end borders and padding. Later descendants
  resolve to the newest continuation rather than reopening the original fragment. Empty continuation
  lines are materialized only for the CSS2 cases that require line-height geometry between blocks.
  Relative inline ancestors also carry their visual offset onto split block/float descendants while
  their normal-flow and float exclusion geometry remains unchanged.
- An undecorated `position:relative` inline now retains identity in the parent-linked arena
  even without padding, border or background. Text, nested images and inline decorations receive
  cumulative relative visual offsets but consume their original line widths and flow heights.
  Each line emits measured inline fragment rectangles. The flow context accumulates these
  records across line formatter invocations, even when intervening blocks split the inline.
  Positioned absolute children are queued until all fragments of their relative inline
  ancestor are available. First/last fragment padding edges (LTR/RTL) supply the containing
  rectangle; nested deferred positioned children are drained after their parents, and
  relative-block visual translations move fragment and static-position records too.
  Independently formatted inline-block/table/flex contexts drain their own queues.
  Fixed children with insets still use viewport dimensions, while right-to-left block static
  positions with unspecified horizontal insets use their hypothetical flow width rather
  than incorrectly anchoring to viewport width. A block inside an unrelated non-positioned
  inline keeps the previous static-position path rather than changing the line sequence.
  Bidi line reordering and vertical writing are not implemented yet.
- Generated `::before`/`::after` text from strings, `attr()`, CSS counters and quote commands is converted
  to ordinary InlineChar items at the host's child boundaries. Counter state is resolved before
  layout, so the formatter only sees final generated Unicode text. Pseudos inherit host typography,
  can carry their own text/box
  styles, and use `(NodeId, PseudoElement)` identity so a pseudo fragment cannot merge into an
  identically styled host fragment. Generated content inside a link inherits the href and the
  normal hit-testing path.

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

This is still not a bidi-capable inline formatter. Computed `direction:ltr|rtl` currently affects
logical start/end handling for split inline fragments, but text ordering, bidi embedding and vertical
writing remain future work. Mixed computed inline typography and initial inline
padding/background/solid-border fragments are supported. Block-level box-model support, including
adjacent sibling margin collapse, is described in [CSS Block Box Model](CSS-Box-Model.md).
Parent/child margin collapse, font families, decoration color/style/thickness, `tab-size`, advanced
shaping/font fallback, bidi and grapheme-aware/full Unicode line breaking remain future work. `text-transform: capitalize`
currently uses whitespace word starts rather than full locale/context-sensitive CSS rules;
word-spacing targets processed ASCII spaces, and spaced native painting advances per Unicode
scalar while layout width stays anchored to whole-run GDI measurement plus CSS spacing.
Inline replaced image boxes honor nowrap and keep their own padding/borders inside ancestor
fragments; generated pseudos on DOM replaced elements remain unsupported. Generated `display:block` uses the ordinary block box model with
dimensions/min/max, margins, padding, borders and background. Empty generated block strings
still materialize decorations. Empty generated strings and ordinary visually empty inline elements
with their own box carry EmptyInline items: edge width affects wrapping/nowrap/alignment and
font/vertical-edge metrics affect line geometry. They emit BoxDecoration without TextBox glyphs.
Hidden/empty descendants and only collapsible whitespace preserve an own empty host frame when
their edges reserve geometry. Block-inside-inline boundaries use continuation history to distinguish
a required zero-width intermediate line from a fragment that should disappear after earlier visible
inline content. Nested empty frames share all ancestor geometry; preformatted spaces retain their
existing text behavior.
Generated `url()` images share ordered inline text/image lists, baselines, atomic wrapping,
intrinsic sizes and anchor click identity. Missing sole-URL replacements preserve CSS geometry
around zero natural dimensions without pixels; mixed failures skip images and preserve text/
empty pseudo boxes. Nonempty DOM alt receives its own inline styling or ordinary block geometry.
`::first-letter` now has an initial fragment-pseudo path. The first non-whitespace text cluster is
selected with Unicode grapheme segmentation (UAX #29, via the focused `unicode-segmentation`
dependency), so a Regional Indicator flag is styled as one cluster. The pseudo overlays only the
inline properties it explicitly authored, preserving descendant and `display:contents` inheritance
for untouched properties; an explicit font-size also recomputes the pseudo's used normal line-height.
Leading/trailing punctuation rules, first-letter float/box geometry and the broader CSS Pseudo
property surface remain future work.

Language-aware automatic quote selection, custom counter styles and fully spec-complete counter
scope edge cases are not implemented. Fragment edges currently clone
on each wrapped line rather than implementing `box-decoration-break: slice`. Floats/tables/
flex/grid also remain future work. Hyperlinks use computed text color/decoration, with UA
blue/underline defaults overridden by author CSS. Their byte spans and measured native hit
regions remain independent of presentation. These tests do not claim complete CSS conformance.

[Page Reflow](Page-Reflow.md) now rebuilds these lines on window resize using the
retained DOM and shared image pixels. Engine and painter synchronize Windows font
operations with `op_paint::GDI_TEXT_LOCK`; concurrent regression tests verify stable
extents. Layout and network work remain outside the gate.

Primary references: [Microsoft GDI text extents](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-gettextextentpoint32w),
[CSS2 inline formatting](https://www.w3.org/TR/CSS2/visuren.html#inline-formatting) and
[CSS Text whitespace processing](https://www.w3.org/TR/css-text-3/#white-space-processing).
