# CSS Block Box Model

DOM img sizes now use computed width/height/min/max and box-sizing around intrinsic raster
dimensions. HTML bounded size attributes supply hints before author cascade; CSS auto and
global keywords override them. Percent widths use containing content width; font-relative
units use computed font size. Definite heights and min/max ratio constraints are supported.
Both-auto compatible bounds preserve ratio; conflicts and two explicit dimensions may
stretch. Percentage heights await containing-height propagation. The existing viewport/
draw-height fitting policy still applies after CSS sizes and may reduce minimum sizes.
Reference: [CSS replaced size constraints](https://www.w3.org/TR/CSS2/visudet.html#min-max-widths).

Generated `::before`/`::after` with `display:block` uses this same geometry path, including
empty-string decorated blocks. Definite heights set the box/flow height while content may
overflow; background/border geometry does not grow merely to contain that overflowing text.

OPBrowser owns the current block box-model pipeline from specified CSS through used layout
geometry. Windows remains only a drawing backend for platform-neutral display-list commands.

## Supported properties and values

The current block subset includes:

- `background-color` using the current color parser;
- `margin` plus `margin-top/right/bottom/left`;
- `padding` plus `padding-top/right/bottom/left`;
- `width`, `min-width`, `max-width`;
- `height`, `min-height`, `max-height`;
- `box-sizing: content-box | border-box`;
- `border` and `border-top/right/bottom/left`;
- `border-width`, `border-style`, `border-color` and all side longhands.

Margin accepts `auto`, negative values and percentages. Padding and box sizes reject negative
used lengths. Supported dimensions include px, em, rem, in, cm, mm, Q, pt and pc; percentage
horizontal sizing resolves from the containing-block width. Unitless zero is accepted.
`font-size` additionally accepts percentages and the current absolute/relative size keywords.

Border styles currently implement only `none` and `solid`. Border widths accept lengths and
`thin`/`medium`/`thick`; border color accepts the normal color subset plus `currentColor`.
Shorthand and longhand declarations do not run in a hard-coded order: each affected side or
subproperty chooses its winner using the normal importance/source/specificity/source-order
cascade key.

## Used block geometry

For an ordinary non-replaced block, `op_layout` resolves:

    containing block
      -> percentage / auto / negative margins
      -> width + min/max constraints
      -> content-box or border-box interpretation
      -> independent border edges
      -> percentage / absolute padding
      -> content box and available inline width

A specified width with both horizontal margins `auto` is centered. Auto width fills the
remaining containing width. Nested blocks, inline lines and images receive the resulting
content-box x/width. `height` and min/max height work for definite absolute values; a
percentage height is parsed but currently behaves auto-like when no definite containing
height is available, avoiding invented geometry.

The current rem conversion uses OPBrowser's initial root font-size baseline. Recomputing rem
from an author-modified root font size remains later computed-value work.

## Margin collapsing

Adjacent sibling block margins collapse. Two positive margins use the larger; two negative
margins use the more negative; mixed signs combine the largest positive with the most
negative. Whitespace-only DOM text between block siblings is discarded at the block-flow
boundary so indentation/newlines do not create a fake anonymous line and break collapse.

Self-collapsing zero-height blocks now merge their top/bottom margins into the shared pending
margin set instead of advancing block-flow y. The detector can cross whitespace-only normal-flow
text, undecorated inline wrappers and `display:contents`, allowing an empty block nested inside an
inline-only wrapper and empty margin-transparent parent to collapse with surrounding sibling
margins. It is deliberately conservative: borders/padding/nonzero resolved height, preserved
whitespace, replaced content, generated before/after content or decorated inline wrappers fall
back to ordinary layout. General non-empty parent/first-child and parent/last-child margin collapse
still remains later work.

## Initial floats and block formatting contexts

`float:left/right` now creates an out-of-flow float record without advancing the normal block-flow
cursor. `clear:left/right/both` advances a following block to the bottom of matching active floats.
`display:flow-root` and `display:flow-root list-item` establish an initial BFC: they avoid active
outer floats at their start edge, isolate descendant floats, include those floats in their natural
height and prevent child margins from collapsing through the BFC boundary. Floated tables keep the
table formatting algorithm inside float placement, while `display:contents` suppresses the
principal box and therefore ignores float placement on the contents element itself.

This is deliberately not complete CSS2 float layout yet. Text wrapping around floats, floating
replaced images, complex left/right float packing, list markers and the full clearance/margin rules
remain future work.

## Painting

`op_layout` emits `BoxDecoration` records with border-box bounds, background color and four
independent border width/color pairs. `op_paint` expands them into one background `FillRect`
and up to four side-specific border `FillRect` commands. This keeps CSS knowledge out of the
Win32 painter.

RGBA box colors currently composite over the white page background. Proper stacking,
background propagation and general alpha composition remain later rendering work.

## Deliberate current limits

- inline fragments clone edges on each wrapped line; sliced edge behavior remains later;
- nested text/image/empty/pseudo boxes reserve all ancestor edges and paint outer frames first;
- unavailable replacements preserve CSS boxes around zero natural sizes; nonempty alt uses text;
- self-collapsing empty-block propagation is implemented for conservative zero-geometry subtrees,
  but general parent/first-child and parent/last-child collapse remains incomplete;
- no `border-radius`, outlines, shadows, background images or multiple backgrounds;
- percentage height needs a definite-height containing-block propagation pass;
- table layout is implemented as a dedicated initial formatter with intrinsic content tracks,
  colspan/rowspan, border-spacing, cell-edge collapse and table-cell baseline/top/middle/bottom
  alignment; core anonymous-table fixup handles both missing child row/cell wrappers and orphan
  table-internal sibling runs in normal flow without mutating the DOM, including initial orphan
  caption and column-hint behavior. Structural `display:contents` wrappers are transparent to this
  fixup when they expose table-internal descendants; wrappers around ordinary text/inline content
  remain style-transparent containers so inherited text properties are preserved. Auto tracks retain percentage column/cell constraints and
  table-layout:fixed on an explicit-width table uses column hints, first-row widths and remaining
  space without letting later-row content resize tracks. caption-side:top/bottom participates in
  wrapper flow outside the table border/background. display:inline-table is an atomic inline
  formatting context using the same table formatter, with shrink-to-fit auto width, margins,
  padding and borders. Atomic baseline/top/middle/bottom vertical-align moves the entire retained
  table box and contributes to line height. Complete CSS Tables overconstraint/min-width/percentage
  edge algorithms, non-cell collapsed-border precedence and deeper colgroup repair remain later;
  general inline sub/super/text-top/text-bottom/length/% alignment is not implemented yet;
- no floats, positioning, flexbox or CSS Grid yet;
- no complete stacking-context/background-propagation model.

Unsupported or invalid values are ignored before cascade winner selection, so a valid lower
priority declaration can still win for the currently recognized properties.

## Verification

The checked-in demo now exercises centered percentage-sized boxes, auto margins, min/max
width, negative margin, em/percentage padding, border-box sizing, independent border sides
and sibling margin collapsing:

    target\release\op_browser.exe examples\css\index.html

Deterministic CSS/layout/paint/engine tests separately verify shorthand/longhand cascade,
unit conversion, used coordinates, per-side paint commands, fixed border-box height and
positive/negative sibling margin-collapse arithmetic.
