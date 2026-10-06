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

Parent/child collapse and empty-block self-collapse are not implemented yet. A final child
margin is currently consumed before the parent's padding/border boundary instead of escaping
through a margin-transparent parent.

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
- no parent/child or empty-block margin collapse;
- no `border-radius`, outlines, shadows, background images or multiple backgrounds;
- percentage height needs a definite-height containing-block propagation pass;
- no floats, positioning, tables, flexbox or grid yet;
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
