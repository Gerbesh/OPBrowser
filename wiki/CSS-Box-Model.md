# CSS Block Box Model

OPBrowser now has an initial owned CSS box-model path for ordinary block-level elements.
The geometry is computed in `op_layout`; the Windows backend only draws platform-neutral
display-list rectangles and text.

## Supported properties

The current computed subset adds:

- `background-color` using the existing named/hex color subset;
- `margin` shorthand with 1 to 4 values;
- `padding` shorthand with 1 to 4 values;
- `border: none`;
- `border: <length> solid <color>` with the three components accepted in any order.

`margin` and `padding` accept nonnegative `px` lengths or unitless zero. Values are bounded
to 4096 px. These box properties are non-inherited by default, but `inherit`, `initial`
and `unset` follow the current global-keyword rules.

## Geometry

For an ordinary block element, layout now computes, in order:

    containing block
      -> margin box offset
      -> border box
      -> padding box
      -> content box / available inline width

Child inline lines and nested blocks use that content-box width and x position. This also
means images inside a padded block are fitted against the reduced available content width.

The older heading/paragraph/list vertical spacing is no longer a separate semantic-layout
table on the normal computed path. It is represented as temporary UA computed margins, so
author `margin` can override the same geometry path.

## Painting

`op_layout` emits `BoxDecoration` records containing border-box bounds, background color
and the current uniform solid-border width/color. `op_paint` expands one decoration into
a background `FillRect` plus up to four border-side `FillRect` commands. The Win32 painter
therefore does not implement CSS itself.

RGBA box colors currently composite over the white page background, matching the existing
text-color behavior. Proper stacking/background propagation and alpha composition are
later rendering work.

## Deliberate current limits

This first slice only applies CSS box geometry/decorations to ordinary non-replaced block
boxes. It does not yet implement:

- margin collapsing;
- `margin-*`, `padding-*`, `border-*` side longhands;
- `width`, `height`, min/max sizing or `box-sizing`;
- `auto`, percentages, em/rem or negative margins;
- separate border widths/colors/styles per side;
- inline box fragments, inline padding/background/borders;
- CSS box decorations on block images/replaced elements;
- border radius, outlines, shadows, background images or multiple backgrounds;
- stacking contexts and full background propagation.

Unsupported/invalid values are ignored before cascade winner selection, so a lower-priority
valid declaration can still win under the existing computed-style rules.

## Verification

The checked-in demo exercises nested block boxes:

    target\release\op_browser.exe examples\css\index.html

`examples/css/theme.css` gives the outer and nested blocks different margin, padding,
background and solid borders. Deterministic CSS/layout/paint/engine tests separately verify
computed shorthand expansion, UA-margin override, content geometry, BoxDecoration output,
four-side border painting and the final display-list coordinates.
