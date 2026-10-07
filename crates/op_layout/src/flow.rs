use super::inline::{
    InlineAtomic, InlineBoxStyle, InlineBoxes, InlineChar, InlineImage, InlineStyle, Item, Lines,
};
use super::*;
use op_css::{
    BorderCollapse, BorderEdges, BorderSpacing, BorderStyle, BoxSizing, CaptionSide,
    ComputedFontWeight, ComputedLineHeight, ComputedStyle, ComputedStyleMap, Display,
    FontStyle as CssFontStyle, LengthPercentage, MarginEdges, MarginValue, PaddingEdges,
    PseudoElement, TableLayout, TextAlign, TextTransform, VerticalAlign, WhiteSpace,
};
use unicode_segmentation::UnicodeSegmentation;

pub(super) fn layout(
    document: &Document,
    viewport_width: i32,
    images: &ImageResources,
    generated_images: &GeneratedImageResources,
    computed_styles: &ComputedStyleMap,
    measurer: &mut dyn TextMeasurer,
) -> LayoutTree {
    let viewport_width = viewport_width.max(240);
    let mut stack = vec![document.root()];
    let mut root = document.root();
    while let Some(id) = stack.pop() {
        if document.element(id).is_some_and(|e| e.tag_name == "body") {
            root = id;
            break;
        }
        stack.extend(document.children(id).iter().rev());
    }

    let mut context = Context {
        document,
        images,
        generated_images,
        computed_styles,
        measurer,
        inline_boxes: InlineBoxes::default(),
        width: (viewport_width - 64).max(160),
        y: 28,
        pending_margin: None,
        block_epoch: 0,
        decorations: Vec::new(),
        text: Vec::new(),
        images_out: Vec::new(),
        order: Vec::new(),
    };

    let root_style = document
        .element(root)
        .map_or_else(default_style, |element| {
            context.element_style(root, element.tag_name.as_str(), default_style())
        });
    if document.element(root).is_none_or(|element| {
        context.element_display(root, element.tag_name.as_str()) != Display::None
    }) {
        context.block(
            BlockContent::Element(root),
            None,
            root_style,
            32,
            context.width,
        );
        context.flush_pending_margin();
    }

    LayoutTree {
        viewport_width,
        content_height: context.y + 24,
        box_decorations: context.decorations,
        text_boxes: context.text,
        image_boxes: context.images_out,
        order: context.order,
    }
}

struct Context<'a, 'm> {
    document: &'a Document,
    images: &'a ImageResources,
    generated_images: &'a GeneratedImageResources,
    computed_styles: &'a ComputedStyleMap,
    measurer: &'m mut dyn TextMeasurer,
    inline_boxes: InlineBoxes,
    width: i32,
    y: i32,
    pending_margin: Option<i32>,
    block_epoch: usize,
    decorations: Vec<BoxDecoration>,
    text: Vec<TextBox>,
    images_out: Vec<ImageBox>,
    order: Vec<LayoutItem>,
}

/// Generated blocks share ordinary block sizing without adding synthetic DOM nodes.
#[derive(Clone, Copy)]
enum BlockContent {
    Element(NodeId),
    Generated(NodeId, PseudoElement),
    ImageAlt(NodeId),
}

#[derive(Clone, Copy)]
struct Edges {
    top: i32,
    right: i32,
    bottom: i32,
    left: i32,
}

#[derive(Debug, Clone, Copy)]
struct UsedBorderSide {
    width: i32,
    color: TextColor,
}

#[derive(Clone, Copy)]
struct UsedBorderEdges {
    top: UsedBorderSide,
    right: UsedBorderSide,
    bottom: UsedBorderSide,
    left: UsedBorderSide,
}

#[derive(Clone, Copy)]
struct UsedBlockHorizontal {
    margin_left: i32,
    padding: Edges,
    border: UsedBorderEdges,
    content_width: i32,
    border_width: i32,
}

#[derive(Clone, Copy)]
struct Style {
    inline: InlineStyle,
    text_align: TextAlign,
    vertical_align: VerticalAlign,
    margin: MarginEdges,
    padding: PaddingEdges,
    background: TextColor,
    border: BorderEdges,
    width: Option<LengthPercentage>,
    min_width: LengthPercentage,
    max_width: Option<LengthPercentage>,
    height: Option<LengthPercentage>,
    min_height: LengthPercentage,
    max_height: Option<LengthPercentage>,
    box_sizing: BoxSizing,
    border_collapse: BorderCollapse,
    border_spacing: BorderSpacing,
    table_layout: TableLayout,
    caption_side: CaptionSide,
}

#[derive(Debug, Clone)]
enum TableCellSource {
    Element(NodeId),
    Anonymous {
        nodes: Vec<NodeId>,
        inherited_from: NodeId,
    },
}

#[derive(Debug, Clone)]
struct TableCellPlacement {
    source: TableCellSource,
    row: usize,
    column: usize,
    colspan: usize,
    rowspan: usize,
}

#[derive(Debug)]
struct TableRowSource {
    cells: Vec<TableCellSource>,
}

#[derive(Debug, Default)]
struct TableGrid {
    row_count: usize,
    cells: Vec<TableCellPlacement>,
    columns: usize,
}

#[derive(Debug, Clone, Copy)]
struct TableBoxMetrics {
    width: i32,
    height: i32,
    baseline: i32,
}

#[derive(Debug, Clone, Copy)]
struct TableCellLayout {
    decoration: Option<usize>,
    row: usize,
    rowspan: usize,
    natural_height: i32,
    content_height: i32,
    vertical_extras: i32,
    vertical_align: VerticalAlign,
    baseline_from_top: Option<i32>,
    decoration_start: usize,
    decoration_end: usize,
    text_start: usize,
    text_end: usize,
    image_start: usize,
    image_end: usize,
}

#[derive(Debug, Clone, Copy)]
struct TableColumnIntrinsic {
    min: i32,
    max: i32,
    percent: f32,
}

impl Default for TableColumnIntrinsic {
    fn default() -> Self {
        Self {
            min: 1,
            max: 1,
            percent: 0.0,
        }
    }
}

#[derive(Debug)]
struct TableCollapsedBorders {
    rows: usize,
    columns: usize,
    vertical: Vec<UsedBorderSide>,
    horizontal: Vec<UsedBorderSide>,
}

impl TableCollapsedBorders {
    fn new(rows: usize, columns: usize) -> Self {
        Self {
            rows,
            columns,
            vertical: vec![
                empty_used_border_side();
                rows.saturating_mul(columns.saturating_add(1))
            ],
            horizontal: vec![
                empty_used_border_side();
                rows.saturating_add(1).saturating_mul(columns)
            ],
        }
    }

    fn vertical_mut(&mut self, row: usize, boundary: usize) -> Option<&mut UsedBorderSide> {
        if row >= self.rows || boundary > self.columns {
            return None;
        }
        let stride = self.columns.saturating_add(1);
        self.vertical
            .get_mut(row.saturating_mul(stride).saturating_add(boundary))
    }

    fn horizontal_mut(&mut self, boundary: usize, column: usize) -> Option<&mut UsedBorderSide> {
        if boundary > self.rows || column >= self.columns {
            return None;
        }
        self.horizontal
            .get_mut(boundary.saturating_mul(self.columns).saturating_add(column))
    }

    fn vertical_winner(&self, row_start: usize, row_end: usize, boundary: usize) -> UsedBorderSide {
        let mut winner = empty_used_border_side();
        if boundary > self.columns {
            return winner;
        }
        let stride = self.columns.saturating_add(1);
        for row in row_start.min(self.rows)..row_end.min(self.rows) {
            if let Some(candidate) = self
                .vertical
                .get(row.saturating_mul(stride).saturating_add(boundary))
                .copied()
            {
                choose_collapsed_border(&mut winner, candidate);
            }
        }
        winner
    }

    fn horizontal_winner(
        &self,
        boundary: usize,
        column_start: usize,
        column_end: usize,
    ) -> UsedBorderSide {
        let mut winner = empty_used_border_side();
        if boundary > self.rows {
            return winner;
        }
        for column in column_start.min(self.columns)..column_end.min(self.columns) {
            if let Some(candidate) = self
                .horizontal
                .get(boundary.saturating_mul(self.columns).saturating_add(column))
                .copied()
            {
                choose_collapsed_border(&mut winner, candidate);
            }
        }
        winner
    }
}

impl<'a> Context<'a, '_> {
    fn emit(&mut self, items: &mut Vec<Item<'a>>, style: Style, x: i32, width: i32) {
        if items.iter().all(|item| {
            matches!(
                item,
                Item::Char(ch)
                    if matches!(ch.ch, ' ' | '\t' | '\n' | '\r' | '\u{c}')
                        && matches!(ch.style.white_space, WhiteSpace::Normal | WhiteSpace::NoWrap)
            )
        }) {
            items.clear();
            return;
        }
        self.flush_pending_margin();
        let lines = Lines::new(
            self.measurer,
            &self.inline_boxes,
            style.inline,
            style.text_align,
            x,
            self.y,
            width.max(1),
        )
        .layout(std::mem::take(items));
        self.y = lines.bottom();
        self.order
            .extend(lines.order.into_iter().map(|item| match item {
                LayoutItem::Text(index) => LayoutItem::Text(index + self.text.len()),
                LayoutItem::Image(index) => LayoutItem::Image(index + self.images_out.len()),
            }));
        self.decorations.extend(lines.decorations);
        self.text.extend(lines.text_boxes);
        self.images_out.extend(lines.image_boxes);
    }

    fn block(
        &mut self,
        content: BlockContent,
        href: Option<&'a str>,
        style: Style,
        containing_x: i32,
        containing_width: i32,
    ) {
        self.block_epoch = self.block_epoch.saturating_add(1);
        if let BlockContent::Element(id) = content
            && let Some(margin) = self.self_collapsing_block_margin(id, style, containing_width)
        {
            self.merge_pending_margin(margin);
            return;
        }

        let used = resolve_block_horizontal(style, containing_width);
        let margin_top = resolve_vertical_margin(style.margin.top, containing_width);
        let margin_bottom = resolve_vertical_margin(style.margin.bottom, containing_width);
        let padding_top = used.padding.top;
        let padding_bottom = used.padding.bottom;

        self.apply_collapsed_margin(margin_top);
        let border_x = containing_x.saturating_add(used.margin_left);
        let border_y = self.y;
        let content_x = border_x
            .saturating_add(used.border.left.width)
            .saturating_add(used.padding.left);
        let content_width = used.content_width.max(1);

        let has_border = used.border.top.width > 0
            || used.border.right.width > 0
            || used.border.bottom.width > 0
            || used.border.left.width > 0;
        let decoration = if style.background.alpha > 0 || has_border {
            let index = self.decorations.len();
            self.decorations.push(BoxDecoration {
                x: border_x,
                y: border_y,
                width: used.border_width,
                height: 0,
                background: style.background,
                border_top: DecorationBorder {
                    width: used.border.top.width,
                    color: used.border.top.color,
                },
                border_right: DecorationBorder {
                    width: used.border.right.width,
                    color: used.border.right.color,
                },
                border_bottom: DecorationBorder {
                    width: used.border.bottom.width,
                    color: used.border.bottom.color,
                },
                border_left: DecorationBorder {
                    width: used.border.left.width,
                    color: used.border.left.color,
                },
            });
            Some(index)
        } else {
            None
        };

        self.y = self
            .y
            .saturating_add(used.border.top.width)
            .saturating_add(padding_top);
        let content_top = self.y;

        let first_letter_host = match content {
            BlockContent::Element(id) => Some(id),
            BlockContent::Generated(_, _) | BlockContent::ImageAlt(_) => None,
        };

        let mut items = Vec::new();
        match content {
            BlockContent::Element(id) => {
                self.collect_generated(
                    id,
                    PseudoElement::Before,
                    href,
                    style,
                    (content_x, content_width),
                    &mut items,
                );
                self.collect_children(
                    id,
                    self.document.children(id),
                    href,
                    style,
                    (content_x, content_width),
                    &mut items,
                );
                self.collect_generated(
                    id,
                    PseudoElement::After,
                    href,
                    style,
                    (content_x, content_width),
                    &mut items,
                );
            }
            BlockContent::Generated(id, pseudo) => {
                self.collect_generated_items((id, pseudo), href, style, content_width, &mut items)
            }
            BlockContent::ImageAlt(id) => {
                if let Some(element) = self.document.element(id) {
                    self.collect_image(id, element, href, style, content_width, &mut items);
                }
            }
        }
        if let Some(host) = first_letter_host {
            self.apply_first_letter_style(host, &mut items);
        }
        self.emit(&mut items, style, content_x, content_width);
        // Parent/child margin collapse is intentionally deferred; consume the final
        // child margin before this block's padding/border boundary.
        self.flush_pending_margin();

        let natural_content_height = self.y.saturating_sub(content_top).max(0);
        let target_content_height = resolve_block_content_height(
            style,
            natural_content_height,
            padding_top,
            padding_bottom,
            used.border,
        );
        // Definite height/min/max controls the box even when its text overflows.
        self.y = content_top.saturating_add(target_content_height);

        self.y = self
            .y
            .saturating_add(padding_bottom)
            .saturating_add(used.border.bottom.width);

        if let Some(index) = decoration {
            self.decorations[index].height = self.y.saturating_sub(border_y).max(0);
        }
        self.pending_margin = Some(margin_bottom);
    }

    fn table(&mut self, id: NodeId, style: Style, containing_x: i32, containing_width: i32) {
        let children = self.document.children(id).to_vec();
        self.table_box(&children, id, style, containing_x, containing_width);
    }

    fn inline_table_atomic(
        &mut self,
        id: NodeId,
        mut style: Style,
        containing_width: i32,
    ) -> InlineAtomic {
        let children = self.document.children(id).to_vec();
        let margin_left = resolve_margin(style.margin.left, containing_width).unwrap_or(0);
        let margin_right = resolve_margin(style.margin.right, containing_width).unwrap_or(0);
        let margin_top = resolve_vertical_margin(style.margin.top, containing_width);
        let margin_bottom = resolve_vertical_margin(style.margin.bottom, containing_width);
        let available = containing_width
            .saturating_sub(margin_left)
            .saturating_sub(margin_right)
            .max(1);

        if style.width.is_none() {
            let grid = self.build_table_grid(&children, id);
            if grid.columns > 0 {
                let (horizontal_spacing, _) = used_table_spacing(style);
                let intrinsic = self.table_intrinsic_columns(
                    &children,
                    &grid,
                    style,
                    available,
                    horizontal_spacing,
                );
                let spacing =
                    horizontal_spacing.saturating_mul(intrinsic.len().saturating_add(1) as i32);
                let preferred_min = intrinsic
                    .iter()
                    .map(|column| column.min.max(1))
                    .fold(spacing, i32::saturating_add);
                let preferred = intrinsic
                    .iter()
                    .map(|column| column.max.max(column.min).max(1))
                    .fold(spacing, i32::saturating_add);
                let extras = table_horizontal_extras(style, available);
                let available_content = available.saturating_sub(extras).max(1);
                let content_width = preferred_min.max(available_content.min(preferred));
                let specified = match style.box_sizing {
                    BoxSizing::ContentBox => content_width,
                    BoxSizing::BorderBox => content_width.saturating_add(extras),
                };
                style.width = Some(LengthPercentage::Px(specified.max(1) as f32));
            }
        }

        style.margin = MarginEdges {
            top: MarginValue::ZERO,
            right: MarginValue::ZERO,
            bottom: MarginValue::ZERO,
            left: MarginValue::ZERO,
        };

        let document = self.document;
        let images = self.images;
        let generated_images = self.generated_images;
        let computed_styles = self.computed_styles;
        let mut local = Context {
            document,
            images,
            generated_images,
            computed_styles,
            measurer: &mut *self.measurer,
            inline_boxes: InlineBoxes::default(),
            width: available,
            y: 0,
            pending_margin: None,
            block_epoch: 0,
            decorations: Vec::new(),
            text: Vec::new(),
            images_out: Vec::new(),
            order: Vec::new(),
        };
        let metrics = local.table_box(&children, id, style, 0, available);

        for decoration in &mut local.decorations {
            decoration.x = decoration.x.saturating_add(margin_left);
            decoration.y = decoration.y.saturating_add(margin_top);
        }
        for text in &mut local.text {
            text.x = text.x.saturating_add(margin_left);
            text.y = text.y.saturating_add(margin_top);
        }
        for image in &mut local.images_out {
            image.x = image.x.saturating_add(margin_left);
            image.y = image.y.saturating_add(margin_top);
        }

        InlineAtomic {
            width: margin_left
                .saturating_add(metrics.width)
                .saturating_add(margin_right)
                .max(1),
            height: margin_top
                .saturating_add(metrics.height)
                .saturating_add(margin_bottom)
                .max(0),
            baseline: margin_top
                .saturating_add(metrics.baseline)
                .clamp(0, margin_top.saturating_add(metrics.height).max(0)),
            decorations: local.decorations,
            text_boxes: local.text,
            image_boxes: local.images_out,
            order: local.order,
        }
    }

    fn anonymous_table(
        &mut self,
        children: &[NodeId],
        inherited_from: NodeId,
        inherited: Style,
        containing_x: i32,
        containing_width: i32,
    ) {
        let style = anonymous_table_style(inherited);
        self.table_box(
            children,
            inherited_from,
            style,
            containing_x,
            containing_width,
        );
    }

    fn layout_table_captions(
        &mut self,
        children: &[NodeId],
        inherited: Style,
        side: CaptionSide,
        containing_x: i32,
        containing_width: i32,
    ) {
        for child in children {
            let Some(element) = self.document.element(*child) else {
                continue;
            };
            if self.element_display(*child, &element.tag_name) != Display::TableCaption {
                continue;
            }
            let caption_style = self.element_style(*child, &element.tag_name, inherited);
            if caption_style.caption_side != side {
                continue;
            }
            self.block(
                BlockContent::Element(*child),
                None,
                caption_style,
                containing_x,
                containing_width,
            );
            self.flush_pending_margin();
        }
    }

    fn table_box(
        &mut self,
        children: &[NodeId],
        inherited_from: NodeId,
        style: Style,
        containing_x: i32,
        containing_width: i32,
    ) -> TableBoxMetrics {
        self.block_epoch = self.block_epoch.saturating_add(1);
        let children = self.table_fixup_expand_structural_contents(children);
        let used = resolve_block_horizontal(style, containing_width);
        let margin_top = resolve_vertical_margin(style.margin.top, containing_width);
        let margin_bottom = resolve_vertical_margin(style.margin.bottom, containing_width);

        self.apply_collapsed_margin(margin_top);
        let wrapper_y = self.y;
        let border_x = containing_x.saturating_add(used.margin_left);
        self.layout_table_captions(
            &children,
            style,
            CaptionSide::Top,
            border_x,
            used.border_width.max(1),
        );
        let border_y = self.y;
        let content_x = border_x
            .saturating_add(used.border.left.width)
            .saturating_add(used.padding.left);
        let content_width = used.content_width.max(1);

        let has_border = used.border.top.width > 0
            || used.border.right.width > 0
            || used.border.bottom.width > 0
            || used.border.left.width > 0;
        let table_decoration = if style.background.alpha > 0 || has_border {
            let index = self.decorations.len();
            self.decorations.push(BoxDecoration {
                x: border_x,
                y: border_y,
                width: used.border_width,
                height: 0,
                background: style.background,
                border_top: DecorationBorder {
                    width: used.border.top.width,
                    color: used.border.top.color,
                },
                border_right: DecorationBorder {
                    width: used.border.right.width,
                    color: used.border.right.color,
                },
                border_bottom: DecorationBorder {
                    width: used.border.bottom.width,
                    color: used.border.bottom.color,
                },
                border_left: DecorationBorder {
                    width: used.border.left.width,
                    color: used.border.left.color,
                },
            });
            Some(index)
        } else {
            None
        };

        self.y = self
            .y
            .saturating_add(used.border.top.width)
            .saturating_add(used.padding.top);

        let grid = self.build_table_grid(&children, inherited_from);
        let mut first_row_baseline = None;
        if grid.row_count > 0 && grid.columns > 0 {
            let (horizontal_spacing, vertical_spacing) = used_table_spacing(style);
            let column_widths = if style.table_layout == TableLayout::Fixed && style.width.is_some()
            {
                self.fixed_table_column_widths(
                    &children,
                    &grid,
                    style,
                    content_width,
                    horizontal_spacing,
                )
            } else {
                let intrinsic = self.table_intrinsic_columns(
                    &children,
                    &grid,
                    style,
                    content_width,
                    horizontal_spacing,
                );
                table_column_widths(content_width, &intrinsic, horizontal_spacing)
            };
            let column_offsets =
                table_column_offsets(content_x, &column_widths, horizontal_spacing);
            let collapsed_borders = (style.border_collapse == BorderCollapse::Collapse)
                .then(|| self.collapsed_table_borders(&grid, style));
            let mut row_heights = vec![0; grid.row_count];
            let mut row_baselines = vec![None; grid.row_count];
            let mut cell_layouts = Vec::new();
            let mut row_top = self.y.saturating_add(vertical_spacing);

            for (row, row_height) in row_heights.iter_mut().enumerate() {
                let current_row_top = row_top;
                let row_cells_start = cell_layouts.len();
                for placement in grid.cells.iter().filter(|cell| cell.row == row) {
                    let slot_width = table_cell_slot_width(
                        &column_widths,
                        placement.column,
                        placement.colspan,
                        horizontal_spacing,
                    );
                    let x = column_offsets
                        .get(placement.column)
                        .copied()
                        .unwrap_or(content_x);
                    let collapsed_border = collapsed_borders
                        .as_ref()
                        .map(|borders| self.collapsed_border_for_cell(placement, &grid, borders));
                    let layout = self.layout_table_cell(
                        placement,
                        x,
                        row_top,
                        slot_width,
                        style,
                        collapsed_border,
                    );
                    *row_height = (*row_height).max(layout.natural_height);
                    cell_layouts.push(layout);
                }
                let mut baseline = 0i32;
                let mut below_baseline = 0i32;
                let mut has_baseline = false;
                for cell in &cell_layouts[row_cells_start..] {
                    if let Some(cell_baseline) = cell.baseline_from_top {
                        has_baseline = true;
                        baseline = baseline.max(cell_baseline);
                        below_baseline =
                            below_baseline.max(cell.natural_height.saturating_sub(cell_baseline));
                    }
                }
                if has_baseline {
                    *row_height = (*row_height).max(baseline.saturating_add(below_baseline));
                    row_baselines[row] = Some(baseline);
                }
                *row_height = (*row_height).max(1);
                if row == 0 {
                    first_row_baseline = row_baselines[row].map(|baseline| {
                        current_row_top
                            .saturating_sub(border_y)
                            .saturating_add(baseline)
                    });
                }
                row_top = row_top
                    .saturating_add(*row_height)
                    .saturating_add(vertical_spacing);
            }

            for cell in cell_layouts {
                let end = cell.row.saturating_add(cell.rowspan).min(row_heights.len());
                let span_height = row_heights[cell.row..end]
                    .iter()
                    .copied()
                    .fold(0i32, i32::saturating_add)
                    .saturating_add(
                        vertical_spacing.saturating_mul(end.saturating_sub(cell.row + 1) as i32),
                    )
                    .max(cell.natural_height);
                if let Some(index) = cell.decoration {
                    self.decorations[index].height = span_height;
                }

                let free_space = span_height
                    .saturating_sub(cell.vertical_extras)
                    .saturating_sub(cell.content_height)
                    .max(0);
                let offset = match cell.vertical_align {
                    VerticalAlign::Top => 0,
                    VerticalAlign::Middle => free_space / 2,
                    VerticalAlign::Bottom => free_space,
                    VerticalAlign::Baseline => row_baselines
                        .get(cell.row)
                        .and_then(|baseline| *baseline)
                        .zip(cell.baseline_from_top)
                        .map_or(0, |(row_baseline, cell_baseline)| {
                            row_baseline.saturating_sub(cell_baseline).min(free_space)
                        }),
                };
                self.shift_table_cell_content(cell, offset);
            }

            self.y = row_top;
        }

        self.y = self
            .y
            .saturating_add(used.padding.bottom)
            .saturating_add(used.border.bottom.width);

        let table_height = self.y.saturating_sub(border_y).max(0);
        if let Some(index) = table_decoration {
            self.decorations[index].height = table_height;
        }

        self.layout_table_captions(
            &children,
            style,
            CaptionSide::Bottom,
            border_x,
            used.border_width.max(1),
        );

        let height = self.y.saturating_sub(wrapper_y).max(0);
        let table_offset = border_y.saturating_sub(wrapper_y).max(0);
        self.pending_margin = Some(margin_bottom);
        TableBoxMetrics {
            width: used.border_width,
            height,
            baseline: table_offset
                .saturating_add(first_row_baseline.unwrap_or(table_height))
                .min(height),
        }
    }

    fn build_table_grid(&self, children: &[NodeId], inherited_from: NodeId) -> TableGrid {
        let rows = self.table_row_sources(children, inherited_from);
        let row_count = rows.len();
        let mut grid = TableGrid {
            columns: self.table_declared_columns(children),
            row_count,
            cells: Vec::new(),
        };
        let mut occupied_until = Vec::<usize>::new();

        for (row_index, row) in rows.into_iter().enumerate() {
            for source in row.cells {
                let (colspan, rowspan) = match &source {
                    TableCellSource::Element(node) => {
                        let Some(element) = self.document.element(*node) else {
                            continue;
                        };
                        let colspan = table_span(element, "colspan", 1, 1000).max(1);
                        let rowspan = match table_span(element, "rowspan", 1, 65534) {
                            0 => row_count.saturating_sub(row_index).max(1),
                            value => value,
                        };
                        (colspan, rowspan)
                    }
                    TableCellSource::Anonymous { .. } => (1, 1),
                };

                let mut column = 0usize;
                loop {
                    let end = column.saturating_add(colspan);
                    if occupied_until.len() < end {
                        occupied_until.resize(end, 0);
                    }
                    if occupied_until[column..end]
                        .iter()
                        .all(|occupied| *occupied <= row_index)
                    {
                        break;
                    }
                    column = column.saturating_add(1);
                }

                let end = column.saturating_add(colspan);
                for occupied in &mut occupied_until[column..end] {
                    *occupied = row_index.saturating_add(rowspan);
                }
                grid.columns = grid.columns.max(end);
                grid.cells.push(TableCellPlacement {
                    source,
                    row: row_index,
                    column,
                    colspan,
                    rowspan,
                });
            }
        }

        grid
    }

    fn table_fixup_flatten_contents(&self, children: &[NodeId]) -> Vec<NodeId> {
        let mut flattened = Vec::new();
        let mut stack = children.iter().rev().copied().collect::<Vec<_>>();

        while let Some(node) = stack.pop() {
            let is_contents = self.document.element(node).is_some_and(|element| {
                self.element_display(node, &element.tag_name) == Display::Contents
            });
            if is_contents {
                stack.extend(self.document.children(node).iter().rev().copied());
            } else {
                flattened.push(node);
            }
        }

        flattened
    }

    fn table_fixup_expand_structural_contents(&self, children: &[NodeId]) -> Vec<NodeId> {
        let mut expanded = Vec::new();

        for child in children.iter().copied() {
            let is_contents = self.document.element(child).is_some_and(|element| {
                self.element_display(child, &element.tag_name) == Display::Contents
            });
            if !is_contents {
                expanded.push(child);
                continue;
            }

            let flattened = self.table_fixup_flatten_contents(self.document.children(child));
            let structural = flattened
                .iter()
                .copied()
                .filter(|node| !self.table_fixup_ignorable(*node))
                .collect::<Vec<_>>();
            if !structural.is_empty()
                && structural
                    .iter()
                    .copied()
                    .all(|node| self.table_internal_node(node))
            {
                expanded.extend(flattened);
            } else {
                expanded.push(child);
            }
        }

        expanded
    }

    fn table_row_sources(
        &self,
        children: &[NodeId],
        inherited_from: NodeId,
    ) -> Vec<TableRowSource> {
        let mut rows = Vec::new();
        let mut anonymous_row_children = Vec::new();

        for child in children.iter().copied() {
            if self.table_fixup_ignorable(child) {
                continue;
            }
            match self.table_fixup_display(child) {
                Some(Display::TableRow) => {
                    self.flush_anonymous_table_row(
                        &mut rows,
                        &mut anonymous_row_children,
                        inherited_from,
                    );
                    rows.push(self.table_row_from_node(child));
                }
                Some(
                    Display::TableHeaderGroup | Display::TableRowGroup | Display::TableFooterGroup,
                ) => {
                    self.flush_anonymous_table_row(
                        &mut rows,
                        &mut anonymous_row_children,
                        inherited_from,
                    );
                    self.append_table_row_group(child, &mut rows);
                }
                Some(Display::TableColumn | Display::TableColumnGroup | Display::TableCaption) => {
                    self.flush_anonymous_table_row(
                        &mut rows,
                        &mut anonymous_row_children,
                        inherited_from,
                    );
                }
                _ => anonymous_row_children.push(child),
            }
        }
        self.flush_anonymous_table_row(&mut rows, &mut anonymous_row_children, inherited_from);
        rows
    }

    fn append_table_row_group(&self, group: NodeId, rows: &mut Vec<TableRowSource>) {
        let mut anonymous_row_children = Vec::new();
        let children = self.table_fixup_expand_structural_contents(self.document.children(group));
        for child in children {
            if self.table_fixup_ignorable(child) {
                continue;
            }
            if self.table_fixup_display(child) == Some(Display::TableRow) {
                self.flush_anonymous_table_row(rows, &mut anonymous_row_children, group);
                rows.push(self.table_row_from_node(child));
            } else {
                anonymous_row_children.push(child);
            }
        }
        self.flush_anonymous_table_row(rows, &mut anonymous_row_children, group);
    }

    fn flush_anonymous_table_row(
        &self,
        rows: &mut Vec<TableRowSource>,
        children: &mut Vec<NodeId>,
        inherited_from: NodeId,
    ) {
        if children.is_empty() {
            return;
        }
        rows.push(self.table_row_from_children(std::mem::take(children), inherited_from));
    }

    fn table_row_from_node(&self, row: NodeId) -> TableRowSource {
        self.table_row_from_children(self.document.children(row).to_vec(), row)
    }

    fn table_row_from_children(
        &self,
        children: Vec<NodeId>,
        inherited_from: NodeId,
    ) -> TableRowSource {
        let mut cells = Vec::new();
        let mut anonymous_cell_children = Vec::new();
        let children = self.table_fixup_expand_structural_contents(&children);

        for child in children {
            if self.table_fixup_ignorable(child) {
                continue;
            }
            if self.table_fixup_display(child) == Some(Display::TableCell) {
                if !anonymous_cell_children.is_empty() {
                    cells.push(TableCellSource::Anonymous {
                        nodes: std::mem::take(&mut anonymous_cell_children),
                        inherited_from,
                    });
                }
                cells.push(TableCellSource::Element(child));
            } else {
                anonymous_cell_children.push(child);
            }
        }

        if !anonymous_cell_children.is_empty() {
            cells.push(TableCellSource::Anonymous {
                nodes: anonymous_cell_children,
                inherited_from,
            });
        }
        TableRowSource { cells }
    }

    fn table_fixup_display(&self, node: NodeId) -> Option<Display> {
        let element = self.document.element(node)?;
        Some(self.element_display(node, &element.tag_name))
    }

    fn table_fixup_ignorable(&self, node: NodeId) -> bool {
        let Some(node_data) = self.document.node(node) else {
            return true;
        };
        match &node_data.kind {
            NodeKind::Text(text) => text
                .chars()
                .all(|ch| matches!(ch, ' ' | '\t' | '\n' | '\r' | '\u{c}')),
            NodeKind::Element(element) => {
                self.element_display(node, &element.tag_name) == Display::None
            }
            NodeKind::Comment(_) | NodeKind::DocumentType(_) | NodeKind::Document => true,
        }
    }

    fn table_declared_columns(&self, children: &[NodeId]) -> usize {
        let mut columns = 0usize;
        for child in children {
            let Some(element) = self.document.element(*child) else {
                continue;
            };
            match self.element_display(*child, &element.tag_name) {
                Display::TableColumn => {
                    columns = columns.saturating_add(table_span(element, "span", 1, 1000).max(1));
                }
                Display::TableColumnGroup => {
                    let child_columns = self
                        .document
                        .children(*child)
                        .iter()
                        .filter_map(|column| {
                            let element = self.document.element(*column)?;
                            (self.element_display(*column, &element.tag_name)
                                == Display::TableColumn)
                                .then_some(table_span(element, "span", 1, 1000).max(1))
                        })
                        .sum::<usize>();
                    columns = columns.saturating_add(if child_columns == 0 {
                        table_span(element, "span", 1, 1000).max(1)
                    } else {
                        child_columns
                    });
                }
                _ => {}
            }
        }
        columns
    }

    fn fixed_table_column_widths(
        &self,
        children: &[NodeId],
        grid: &TableGrid,
        inherited: Style,
        content_width: i32,
        spacing: i32,
    ) -> Vec<i32> {
        if grid.columns == 0 {
            return Vec::new();
        }

        let columns = grid.columns;
        let total_spacing = spacing.saturating_mul(columns.saturating_add(1) as i32);
        let usable = content_width
            .saturating_sub(total_spacing)
            .max(columns as i32);
        let mut widths = vec![None; columns];

        self.apply_fixed_column_hints(children, inherited, usable, &mut widths);

        for placement in grid.cells.iter().filter(|cell| cell.row == 0) {
            let style = self.table_cell_source_style(&placement.source, inherited);
            let Some(width) = style.width else {
                continue;
            };
            let border_box = fixed_cell_border_box_width(style, width, usable);
            apply_fixed_span_width(
                &mut widths,
                placement.column,
                placement.colspan,
                border_box,
                spacing,
            );
        }

        resolve_fixed_track_widths(widths, usable)
    }

    fn apply_fixed_column_hints(
        &self,
        children: &[NodeId],
        inherited: Style,
        basis: i32,
        widths: &mut [Option<i32>],
    ) {
        let mut cursor = 0usize;
        for child in children {
            let Some(element) = self.document.element(*child) else {
                continue;
            };
            match self.element_display(*child, &element.tag_name) {
                Display::TableColumn => {
                    let style = self.element_style(*child, &element.tag_name, inherited);
                    let span = table_span(element, "span", 1, 1000).max(1);
                    apply_fixed_column_length(widths, &mut cursor, span, style.width, basis, false);
                }
                Display::TableColumnGroup => {
                    let group_style = self.element_style(*child, &element.tag_name, inherited);
                    let mut child_columns = 0usize;
                    for column in self.document.children(*child) {
                        let Some(column_element) = self.document.element(*column) else {
                            continue;
                        };
                        if self.element_display(*column, &column_element.tag_name)
                            != Display::TableColumn
                        {
                            continue;
                        }
                        let style =
                            self.element_style(*column, &column_element.tag_name, group_style);
                        let span = table_span(column_element, "span", 1, 1000).max(1);
                        apply_fixed_column_length(
                            widths,
                            &mut cursor,
                            span,
                            style.width.or(group_style.width),
                            basis,
                            false,
                        );
                        child_columns = child_columns.saturating_add(span);
                    }
                    if child_columns == 0 {
                        let span = table_span(element, "span", 1, 1000).max(1);
                        apply_fixed_column_length(
                            widths,
                            &mut cursor,
                            span,
                            group_style.width,
                            basis,
                            true,
                        );
                    }
                }
                _ => {}
            }
        }
    }

    fn table_intrinsic_columns(
        &mut self,
        children: &[NodeId],
        grid: &TableGrid,
        inherited: Style,
        content_width: i32,
        spacing: i32,
    ) -> Vec<TableColumnIntrinsic> {
        let mut columns = vec![TableColumnIntrinsic::default(); grid.columns];
        self.apply_table_column_width_hints(children, inherited, content_width, &mut columns);

        for placement in grid.cells.iter().filter(|cell| cell.colspan == 1) {
            let cell_style = self.table_cell_source_style(&placement.source, inherited);
            let (min, max) =
                self.table_cell_intrinsic_widths(&placement.source, inherited, content_width);
            if let Some(column) = columns.get_mut(placement.column) {
                column.min = column.min.max(min);
                column.max = column.max.max(max).max(column.min);
                if let Some(LengthPercentage::Percent(percent)) = cell_style.width {
                    column.percent = column.percent.max(percent.max(0.0));
                }
            }
        }

        for placement in grid.cells.iter().filter(|cell| cell.colspan > 1) {
            let end = placement
                .column
                .saturating_add(placement.colspan)
                .min(columns.len());
            if placement.column >= end {
                continue;
            }

            let cell_style = self.table_cell_source_style(&placement.source, inherited);
            let (required_min, required_max) =
                self.table_cell_intrinsic_widths(&placement.source, inherited, content_width);
            let internal_spacing =
                spacing.saturating_mul(end.saturating_sub(placement.column + 1) as i32);
            grow_intrinsic_span(
                &mut columns[placement.column..end],
                required_min.saturating_sub(internal_spacing).max(1),
                required_max.saturating_sub(internal_spacing).max(1),
            );
            if let Some(LengthPercentage::Percent(percent)) = cell_style.width {
                grow_percent_span(&mut columns[placement.column..end], percent.max(0.0));
            }
        }

        columns
    }

    fn apply_table_column_width_hints(
        &self,
        children: &[NodeId],
        inherited: Style,
        content_width: i32,
        columns: &mut [TableColumnIntrinsic],
    ) {
        let mut cursor = 0usize;
        for child in children {
            let Some(element) = self.document.element(*child) else {
                continue;
            };
            match self.element_display(*child, &element.tag_name) {
                Display::TableColumn => {
                    let style = self.element_style(*child, &element.tag_name, inherited);
                    let span = table_span(element, "span", 1, 1000).max(1);
                    apply_column_length_hint(
                        columns,
                        &mut cursor,
                        span,
                        style.width,
                        content_width,
                        false,
                    );
                }
                Display::TableColumnGroup => {
                    let group_style = self.element_style(*child, &element.tag_name, inherited);
                    let group_hint = group_style.width;
                    let mut child_columns = 0usize;
                    for column in self.document.children(*child) {
                        let Some(column_element) = self.document.element(*column) else {
                            continue;
                        };
                        if self.element_display(*column, &column_element.tag_name)
                            != Display::TableColumn
                        {
                            continue;
                        }
                        let style =
                            self.element_style(*column, &column_element.tag_name, group_style);
                        let span = table_span(column_element, "span", 1, 1000).max(1);
                        apply_column_length_hint(
                            columns,
                            &mut cursor,
                            span,
                            style.width.or(group_hint),
                            content_width,
                            false,
                        );
                        child_columns = child_columns.saturating_add(span);
                    }

                    if child_columns == 0 {
                        let span = table_span(element, "span", 1, 1000).max(1);
                        apply_column_length_hint(
                            columns,
                            &mut cursor,
                            span,
                            group_hint,
                            content_width,
                            true,
                        );
                    }
                }
                _ => {}
            }
        }
    }

    fn table_cell_intrinsic_widths(
        &mut self,
        source: &TableCellSource,
        inherited: Style,
        width_basis: i32,
    ) -> (i32, i32) {
        let style = self.table_cell_source_style(source, inherited);
        let padding = Edges {
            top: resolve_length(style.padding.top, width_basis).max(0),
            right: resolve_length(style.padding.right, width_basis).max(0),
            bottom: resolve_length(style.padding.bottom, width_basis).max(0),
            left: resolve_length(style.padding.left, width_basis).max(0),
        };
        let border = resolve_border_edges(style.border);
        let extras = padding
            .left
            .saturating_add(padding.right)
            .saturating_add(border.left.width)
            .saturating_add(border.right.width);

        let mut text = String::new();
        let image_width = match source {
            TableCellSource::Element(node) => {
                for child in self.document.children(*node) {
                    self.collect_table_intrinsic_text(*child, &mut text);
                }
                self.table_intrinsic_image_width(*node, style, width_basis)
            }
            TableCellSource::Anonymous { nodes, .. } => {
                for node in nodes {
                    self.collect_table_intrinsic_text(*node, &mut text);
                }
                nodes
                    .iter()
                    .map(|node| self.table_intrinsic_image_width(*node, style, width_basis))
                    .max()
                    .unwrap_or(0)
            }
        };
        let (text_min, text_max) = self.measure_table_intrinsic_text(&text, style.inline);
        let mut min = text_min.max(image_width).saturating_add(extras).max(1);
        let mut max = text_max
            .max(text_min)
            .max(image_width)
            .saturating_add(extras)
            .max(min);

        let border_box_width = |value: LengthPercentage| {
            let resolved = resolve_length(value, width_basis).max(0);
            match style.box_sizing {
                BoxSizing::ContentBox => resolved.saturating_add(extras),
                BoxSizing::BorderBox => resolved.max(extras),
            }
        };

        if let Some(width) = style.width {
            let width = border_box_width(width).max(1);
            min = min.max(width);
            max = max.max(width);
        }

        let min_constraint = border_box_width(style.min_width).max(1);
        min = min.max(min_constraint);
        max = max.max(min);

        if let Some(max_width) = style.max_width {
            let cap = border_box_width(max_width).max(1);
            max = max.min(cap).max(min);
        }

        (min, max)
    }

    fn table_cell_source_style(&self, source: &TableCellSource, inherited: Style) -> Style {
        match source {
            TableCellSource::Element(node) => self
                .document
                .element(*node)
                .map_or_else(default_style, |element| {
                    self.element_style(*node, &element.tag_name, inherited)
                }),
            TableCellSource::Anonymous { inherited_from, .. } => {
                let parent = self
                    .document
                    .element(*inherited_from)
                    .map_or(inherited, |element| {
                        self.element_style(*inherited_from, &element.tag_name, inherited)
                    });
                anonymous_table_cell_style(parent)
            }
        }
    }

    fn collect_table_intrinsic_text(&self, node: NodeId, output: &mut String) {
        let Some(node_data) = self.document.node(node) else {
            return;
        };
        match &node_data.kind {
            NodeKind::Text(text) => output.push_str(text),
            NodeKind::Element(element) => {
                let display = self.element_display(node, &element.tag_name);
                if display == Display::None {
                    return;
                }
                if element.tag_name == "img" {
                    if !self.images.contains_key(&node)
                        && let Some(alt) = attribute(element, "alt")
                    {
                        output.push_str(alt);
                    }
                    return;
                }
                if element.tag_name == "br" {
                    output.push('\n');
                    return;
                }

                let block_boundary =
                    display == Display::Block || is_table_internal_display(display);
                if block_boundary && !output.ends_with([' ', '\n']) {
                    output.push('\n');
                }
                for child in self.document.children(node) {
                    self.collect_table_intrinsic_text(*child, output);
                }
                if block_boundary && !output.ends_with([' ', '\n']) {
                    output.push('\n');
                }
            }
            _ => {}
        }
    }

    fn table_intrinsic_image_width(&self, node: NodeId, inherited: Style, width_basis: i32) -> i32 {
        let Some(node_data) = self.document.node(node) else {
            return 0;
        };
        let NodeKind::Element(element) = &node_data.kind else {
            return 0;
        };
        if self.element_display(node, &element.tag_name) == Display::None {
            return 0;
        }

        let style = self.element_style(node, &element.tag_name, inherited);
        if element.tag_name == "img" {
            if let Some(image) = self.images.get(&node)
                && let Some((width, _)) = resolve_image_size(
                    style,
                    (image.width(), image.height()),
                    None,
                    width_basis,
                    width_basis,
                )
            {
                return width.max(0);
            }
            if let Some(width) = style.width {
                return resolve_length(width, width_basis).max(0);
            }
            return 0;
        }

        self.document
            .children(node)
            .iter()
            .map(|child| self.table_intrinsic_image_width(*child, style, width_basis))
            .max()
            .unwrap_or(0)
    }

    fn measure_table_intrinsic_text(&mut self, text: &str, style: InlineStyle) -> (i32, i32) {
        if text.is_empty() {
            return (0, 0);
        }

        let normalized = match style.white_space {
            WhiteSpace::Pre | WhiteSpace::PreWrap => text.to_owned(),
            WhiteSpace::Normal | WhiteSpace::NoWrap | WhiteSpace::PreLine => {
                collapse_intrinsic_whitespace(text)
            }
        };

        let max = normalized
            .lines()
            .map(|line| measure_intrinsic_text_run(self.measurer, line, style))
            .max()
            .unwrap_or(0);

        if matches!(style.white_space, WhiteSpace::NoWrap | WhiteSpace::Pre) {
            return (max, max);
        }

        let min = normalized
            .split_whitespace()
            .map(|word| measure_intrinsic_text_run(self.measurer, word, style))
            .max()
            .unwrap_or(0);
        (min.min(max), max.max(min))
    }

    fn collapsed_table_borders(&self, grid: &TableGrid, inherited: Style) -> TableCollapsedBorders {
        let mut borders = TableCollapsedBorders::new(grid.row_count, grid.columns);

        for placement in &grid.cells {
            let TableCellSource::Element(node) = &placement.source else {
                continue;
            };
            let Some(element) = self.document.element(*node) else {
                continue;
            };
            let style = self.element_style(*node, &element.tag_name, inherited);
            let cell = resolve_border_edges(style.border);
            let column_end = placement
                .column
                .saturating_add(placement.colspan)
                .min(grid.columns);
            let row_end = placement
                .row
                .saturating_add(placement.rowspan)
                .min(grid.row_count);

            for row in placement.row..row_end {
                if let Some(boundary) = borders.vertical_mut(row, placement.column) {
                    choose_collapsed_border(boundary, cell.left);
                }
                if let Some(boundary) = borders.vertical_mut(row, column_end) {
                    choose_collapsed_border(boundary, cell.right);
                }
            }
            for column in placement.column..column_end {
                if let Some(boundary) = borders.horizontal_mut(placement.row, column) {
                    choose_collapsed_border(boundary, cell.top);
                }
                if let Some(boundary) = borders.horizontal_mut(row_end, column) {
                    choose_collapsed_border(boundary, cell.bottom);
                }
            }
        }

        borders
    }

    fn collapsed_border_for_cell(
        &self,
        placement: &TableCellPlacement,
        grid: &TableGrid,
        borders: &TableCollapsedBorders,
    ) -> UsedBorderEdges {
        let column_end = placement
            .column
            .saturating_add(placement.colspan)
            .min(grid.columns);
        let row_end = placement
            .row
            .saturating_add(placement.rowspan)
            .min(grid.row_count);

        UsedBorderEdges {
            top: borders.horizontal_winner(placement.row, placement.column, column_end),
            right: if column_end == grid.columns {
                borders.vertical_winner(placement.row, row_end, column_end)
            } else {
                empty_used_border_side()
            },
            bottom: if row_end == grid.row_count {
                borders.horizontal_winner(row_end, placement.column, column_end)
            } else {
                empty_used_border_side()
            },
            left: borders.vertical_winner(placement.row, row_end, placement.column),
        }
    }

    fn shift_table_cell_content(&mut self, cell: TableCellLayout, offset: i32) {
        if offset <= 0 {
            return;
        }
        for decoration in &mut self.decorations[cell.decoration_start..cell.decoration_end] {
            decoration.y = decoration.y.saturating_add(offset);
        }
        for text in &mut self.text[cell.text_start..cell.text_end] {
            text.y = text.y.saturating_add(offset);
        }
        for image in &mut self.images_out[cell.image_start..cell.image_end] {
            image.y = image.y.saturating_add(offset);
        }
    }

    fn table_cell_baseline_from_top(
        &mut self,
        row_top: i32,
        text_start: usize,
        text_end: usize,
        image_start: usize,
        image_end: usize,
        fallback: i32,
    ) -> i32 {
        if let Some((y, text, font_size, weight, font_style)) = self
            .text
            .get(text_start..text_end)
            .and_then(|items| items.first())
            .map(|item| {
                (
                    item.y,
                    item.text.clone(),
                    item.font_size,
                    item.weight,
                    item.style,
                )
            })
        {
            let ascent = self
                .measurer
                .measure(&text, font_size, weight, font_style)
                .ascent;
            return y.saturating_add(ascent).saturating_sub(row_top).max(0);
        }

        if let Some(image) = self
            .images_out
            .get(image_start..image_end)
            .and_then(|items| items.first())
        {
            return image
                .y
                .saturating_add(image.height)
                .saturating_sub(row_top)
                .max(0);
        }

        fallback.max(0)
    }

    fn layout_table_cell(
        &mut self,
        placement: &TableCellPlacement,
        x: i32,
        row_top: i32,
        slot_width: i32,
        inherited: Style,
        border_override: Option<UsedBorderEdges>,
    ) -> TableCellLayout {
        let style = self.table_cell_source_style(&placement.source, inherited);
        let padding = Edges {
            top: resolve_length(style.padding.top, slot_width).max(0),
            right: resolve_length(style.padding.right, slot_width).max(0),
            bottom: resolve_length(style.padding.bottom, slot_width).max(0),
            left: resolve_length(style.padding.left, slot_width).max(0),
        };
        let border = border_override.unwrap_or_else(|| resolve_border_edges(style.border));
        let horizontal_extras = padding
            .left
            .saturating_add(padding.right)
            .saturating_add(border.left.width)
            .saturating_add(border.right.width);
        let content_width = slot_width.saturating_sub(horizontal_extras).max(1);

        let has_border = border.top.width > 0
            || border.right.width > 0
            || border.bottom.width > 0
            || border.left.width > 0;
        let decoration = if style.background.alpha > 0 || has_border {
            let index = self.decorations.len();
            self.decorations.push(BoxDecoration {
                x,
                y: row_top,
                width: slot_width.max(1),
                height: 0,
                background: style.background,
                border_top: DecorationBorder {
                    width: border.top.width,
                    color: border.top.color,
                },
                border_right: DecorationBorder {
                    width: border.right.width,
                    color: border.right.color,
                },
                border_bottom: DecorationBorder {
                    width: border.bottom.width,
                    color: border.bottom.color,
                },
                border_left: DecorationBorder {
                    width: border.left.width,
                    color: border.left.color,
                },
            });
            Some(index)
        } else {
            None
        };

        let decoration_start = self.decorations.len();
        let text_start = self.text.len();
        let image_start = self.images_out.len();

        let saved_y = self.y;
        let saved_margin = self.pending_margin.take();
        let content_x = x
            .saturating_add(border.left.width)
            .saturating_add(padding.left);
        let content_top = row_top
            .saturating_add(border.top.width)
            .saturating_add(padding.top);
        self.y = content_top;
        self.pending_margin = None;

        let mut items = Vec::new();
        match &placement.source {
            TableCellSource::Element(node) => {
                self.collect_generated(
                    *node,
                    PseudoElement::Before,
                    None,
                    style,
                    (content_x, content_width),
                    &mut items,
                );
                let children = self.document.children(*node).to_vec();
                self.collect_children(
                    *node,
                    &children,
                    None,
                    style,
                    (content_x, content_width),
                    &mut items,
                );
                self.collect_generated(
                    *node,
                    PseudoElement::After,
                    None,
                    style,
                    (content_x, content_width),
                    &mut items,
                );
            }
            TableCellSource::Anonymous {
                nodes,
                inherited_from,
            } => {
                self.collect_children(
                    *inherited_from,
                    nodes,
                    None,
                    style,
                    (content_x, content_width),
                    &mut items,
                );
            }
        }
        self.emit(&mut items, style, content_x, content_width);
        self.flush_pending_margin();

        let decoration_end = self.decorations.len();
        let text_end = self.text.len();
        let image_end = self.images_out.len();
        let natural_content_height = self.y.saturating_sub(content_top).max(0);
        let baseline_from_top = (style.vertical_align == VerticalAlign::Baseline).then(|| {
            self.table_cell_baseline_from_top(
                row_top,
                text_start,
                text_end,
                image_start,
                image_end,
                content_top
                    .saturating_sub(row_top)
                    .saturating_add(natural_content_height),
            )
        });
        let target_content_height = resolve_block_content_height(
            style,
            natural_content_height,
            padding.top,
            padding.bottom,
            border,
        );
        let vertical_extras = padding
            .top
            .saturating_add(padding.bottom)
            .saturating_add(border.top.width)
            .saturating_add(border.bottom.width);
        let natural_height = target_content_height.saturating_add(vertical_extras).max(1);

        self.y = saved_y;
        self.pending_margin = saved_margin;

        TableCellLayout {
            decoration,
            row: placement.row,
            rowspan: placement.rowspan,
            natural_height,
            content_height: natural_content_height,
            vertical_extras,
            vertical_align: style.vertical_align,
            baseline_from_top,
            decoration_start,
            decoration_end,
            text_start,
            text_end,
            image_start,
            image_end,
        }
    }

    fn apply_collapsed_margin(&mut self, next: i32) {
        let used = self
            .pending_margin
            .take()
            .map_or(next, |previous| collapse_margins(previous, next));
        self.y = self.y.saturating_add(used);
    }

    fn merge_pending_margin(&mut self, next: i32) {
        self.pending_margin = Some(
            self.pending_margin
                .take()
                .map_or(next, |previous| collapse_margins(previous, next)),
        );
    }

    fn self_collapsing_block_margin(
        &self,
        id: NodeId,
        style: Style,
        containing_width: i32,
    ) -> Option<i32> {
        self.self_collapsing_block_margin_inner(id, style, containing_width, 0)
    }

    fn self_collapsing_block_margin_inner(
        &self,
        id: NodeId,
        style: Style,
        containing_width: i32,
        depth: usize,
    ) -> Option<i32> {
        if depth >= 256 {
            return None;
        }

        let used = resolve_block_horizontal(style, containing_width);
        if used.padding.top != 0
            || used.padding.bottom != 0
            || used.border.top.width != 0
            || used.border.bottom.width != 0
            || resolve_block_content_height(
                style,
                0,
                used.padding.top,
                used.padding.bottom,
                used.border,
            ) != 0
        {
            return None;
        }

        if self
            .computed_styles
            .pseudo_style_for(id, PseudoElement::Before)
            .is_some()
            || self
                .computed_styles
                .pseudo_style_for(id, PseudoElement::After)
                .is_some()
        {
            return None;
        }

        let own = collapse_margins(
            resolve_vertical_margin(style.margin.top, containing_width),
            resolve_vertical_margin(style.margin.bottom, containing_width),
        );
        let descendants =
            self.self_collapsing_children_margin(id, style, containing_width, depth + 1)?;
        Some(collapse_margins(own, descendants))
    }

    fn self_collapsing_children_margin(
        &self,
        parent: NodeId,
        inherited: Style,
        containing_width: i32,
        depth: usize,
    ) -> Option<i32> {
        if depth >= 256 {
            return None;
        }

        let mut collapsed = 0;
        for child in self.document.children(parent) {
            let Some(node) = self.document.node(*child) else {
                continue;
            };
            match &node.kind {
                NodeKind::Text(text) => {
                    if !text.is_empty()
                        && (!text.chars().all(char::is_whitespace)
                            || !matches!(
                                inherited.inline.white_space,
                                WhiteSpace::Normal | WhiteSpace::NoWrap
                            ))
                    {
                        return None;
                    }
                }
                NodeKind::Element(element) => {
                    let tag = element.tag_name.as_str();
                    let display = self.element_display(*child, tag);
                    if display == Display::None {
                        continue;
                    }
                    if matches!(tag, "img" | "br") {
                        return None;
                    }
                    let child_style = self.element_style(*child, tag, inherited);
                    let margin = match display {
                        Display::Block => self.self_collapsing_block_margin_inner(
                            *child,
                            child_style,
                            containing_width,
                            depth + 1,
                        )?,
                        Display::Inline => {
                            if resolve_inline_box_style(*child, None, child_style, containing_width)
                                .is_some()
                                || self
                                    .computed_styles
                                    .pseudo_style_for(*child, PseudoElement::Before)
                                    .is_some()
                                || self
                                    .computed_styles
                                    .pseudo_style_for(*child, PseudoElement::After)
                                    .is_some()
                            {
                                return None;
                            }
                            self.self_collapsing_children_margin(
                                *child,
                                child_style,
                                containing_width,
                                depth + 1,
                            )?
                        }
                        Display::Contents => self.self_collapsing_children_margin(
                            *child,
                            child_style,
                            containing_width,
                            depth + 1,
                        )?,
                        _ => return None,
                    };
                    collapsed = collapse_margins(collapsed, margin);
                }
                NodeKind::Document | NodeKind::Comment(_) | NodeKind::DocumentType { .. } => {}
            }
        }
        Some(collapsed)
    }

    fn flush_pending_margin(&mut self) {
        if let Some(margin) = self.pending_margin.take() {
            self.y = self.y.saturating_add(margin);
        }
    }

    fn collect_children(
        &mut self,
        parent: NodeId,
        children: &[NodeId],
        href: Option<&'a str>,
        inherited: Style,
        containing: (i32, i32),
        items: &mut Vec<Item<'a>>,
    ) {
        let (containing_x, containing_width) = containing;
        let mut index = 0usize;
        while index < children.len() {
            let child = children[index];
            if !self.table_internal_node(child) {
                self.collect(
                    child,
                    href,
                    inherited,
                    containing_x,
                    containing_width,
                    items,
                );
                index = index.saturating_add(1);
                continue;
            }

            let mut group = Vec::new();
            while index < children.len() {
                let candidate = children[index];
                if self.table_internal_node(candidate) {
                    group.push(candidate);
                    index = index.saturating_add(1);
                    continue;
                }

                if self.table_fixup_ignorable(candidate) {
                    let mut next = index.saturating_add(1);
                    while next < children.len() && self.table_fixup_ignorable(children[next]) {
                        next = next.saturating_add(1);
                    }
                    if next < children.len() && self.table_internal_node(children[next]) {
                        index = next;
                        continue;
                    }
                }
                break;
            }

            self.emit(items, inherited, containing_x, containing_width);
            self.anonymous_table(&group, parent, inherited, containing_x, containing_width);
        }
    }

    fn table_internal_node(&self, node: NodeId) -> bool {
        self.table_fixup_display(node)
            .is_some_and(is_table_internal_display)
    }

    fn collect(
        &mut self,
        id: NodeId,
        href: Option<&'a str>,
        inherited: Style,
        containing_x: i32,
        containing_width: i32,
        items: &mut Vec<Item<'a>>,
    ) {
        let Some(node) = self.document.node(id) else {
            return;
        };
        match &node.kind {
            NodeKind::Text(text) => items.extend(text.chars().map(|ch| {
                Item::Char(InlineChar {
                    ch,
                    href,
                    style: inherited.inline,
                })
            })),
            NodeKind::Element(element) => {
                let tag = element.tag_name.as_str();
                let display = self.element_display(id, tag);
                if display == Display::None {
                    return;
                }
                let mut current = self.element_style(id, tag, inherited);
                if display == Display::Inline {
                    current.inline.boxes = if tag == "img"
                        && (self.images.contains_key(&id)
                            || attribute(element, "alt").is_none_or(str::is_empty))
                    {
                        inherited.inline.boxes
                    } else {
                        resolve_inline_box_style(id, None, current, containing_width)
                            .map(|box_style| {
                                self.inline_boxes.push(box_style, inherited.inline.boxes)
                            })
                            .or(inherited.inline.boxes)
                    };
                }
                let href = if tag == "a" {
                    attribute(element, "href")
                } else {
                    href
                };

                if display == Display::InlineTable {
                    current.inline.boxes = inherited.inline.boxes;
                    let mut atomic = self.inline_table_atomic(id, current, containing_width);
                    if let Some(href) = href {
                        inherit_atomic_href(&mut atomic, href);
                    }
                    items.push(Item::Atomic(atomic, current.inline, current.vertical_align));
                } else if display == Display::Table {
                    self.emit(items, inherited, containing_x, containing_width);
                    self.table(id, current, containing_x, containing_width);
                } else if tag == "img" {
                    if display == Display::Block {
                        self.emit(items, inherited, containing_x, containing_width);
                        let image = self.images.get(&id).cloned();
                        if image.is_some() || attribute(element, "alt").is_none_or(str::is_empty) {
                            self.block_image(
                                (id, None),
                                href,
                                current,
                                image.as_ref(),
                                (containing_x, containing_width),
                            );
                            return;
                        }
                        self.block(
                            BlockContent::ImageAlt(id),
                            href,
                            current,
                            containing_x,
                            containing_width,
                        );
                    } else {
                        self.collect_image(id, element, href, current, containing_width, items);
                    }
                } else if tag == "br" {
                    if display == Display::Block {
                        self.emit(items, inherited, containing_x, containing_width);
                        let mut line = vec![Item::Break];
                        self.emit(&mut line, current, containing_x, containing_width);
                    } else {
                        items.push(Item::Break);
                    }
                } else if display == Display::Block || is_table_internal_display(display) {
                    self.emit(items, inherited, containing_x, containing_width);
                    self.block(
                        BlockContent::Element(id),
                        href,
                        current,
                        containing_x,
                        containing_width,
                    );
                } else {
                    let initial_len = items.len();
                    let initial_epoch = self.block_epoch;
                    self.collect_generated(
                        id,
                        PseudoElement::Before,
                        href,
                        current,
                        (containing_x, containing_width),
                        items,
                    );
                    self.collect_children(
                        id,
                        &node.children,
                        href,
                        current,
                        (containing_x, containing_width),
                        items,
                    );
                    self.collect_generated(
                        id,
                        PseudoElement::After,
                        href,
                        current,
                        (containing_x, containing_width),
                        items,
                    );
                    if self.block_epoch == initial_epoch
                        && current.inline.boxes.is_some_and(|box_id| {
                            let box_style = self.inline_boxes.style(box_id);
                            box_style.node == id
                                && box_style.pseudo.is_none()
                                && box_style.reserves_empty_fragment()
                        })
                        && items.get(initial_len..).is_some_and(|collected| {
                            collected.iter().all(|item| matches!(item,
                                Item::Char(ch)
                                    if matches!(ch.ch, ' ' | '\t' | '\n' | '\r' | '\u{c}')
                                        && matches!(ch.style.white_space, WhiteSpace::Normal | WhiteSpace::NoWrap)
                            ))
                        })
                    {
                        items.push(Item::EmptyInline(current.inline));
                    }
                }
            }
            NodeKind::Document => {
                self.collect_children(
                    id,
                    &node.children,
                    href,
                    inherited,
                    (containing_x, containing_width),
                    items,
                );
            }
            NodeKind::Comment(_) | NodeKind::DocumentType(_) => {}
        }
    }

    fn apply_first_letter_style(&self, host: NodeId, items: &mut [Item<'a>]) {
        let Some(pseudo) = self
            .computed_styles
            .pseudo_style_for(host, PseudoElement::FirstLetter)
        else {
            return;
        };

        let Some(start) = items
            .iter()
            .position(|item| matches!(item, Item::Char(ch) if !ch.ch.is_whitespace()))
        else {
            return;
        };

        let mut source = String::new();
        for item in &items[start..] {
            let Item::Char(ch) = item else {
                break;
            };
            source.push(ch.ch);
        }
        let Some(grapheme) = source.graphemes(true).next() else {
            return;
        };
        let mut remaining = grapheme.chars().count();
        let pseudo_inline = computed_style(pseudo.style).inline;
        let properties = pseudo.fragment_properties;

        for item in &mut items[start..] {
            if remaining == 0 {
                break;
            }
            let Item::Char(ch) = item else {
                break;
            };
            let current = ch.style;
            ch.style = InlineStyle {
                font_size: if properties.font_size {
                    pseudo_inline.font_size
                } else {
                    current.font_size
                },
                line_height: if properties.line_height || properties.font_size {
                    pseudo_inline.line_height
                } else {
                    current.line_height
                },
                weight: if properties.font_weight {
                    pseudo_inline.weight
                } else {
                    current.weight
                },
                font_style: if properties.font_style {
                    pseudo_inline.font_style
                } else {
                    current.font_style
                },
                decoration: if properties.text_decoration {
                    pseudo_inline.decoration
                } else {
                    current.decoration
                },
                white_space: if properties.white_space {
                    pseudo_inline.white_space
                } else {
                    current.white_space
                },
                letter_spacing: if properties.letter_spacing {
                    pseudo_inline.letter_spacing
                } else {
                    current.letter_spacing
                },
                word_spacing: if properties.word_spacing {
                    pseudo_inline.word_spacing
                } else {
                    current.word_spacing
                },
                text_transform: if properties.text_transform {
                    pseudo_inline.text_transform
                } else {
                    current.text_transform
                },
                color: if properties.color {
                    pseudo_inline.color
                } else {
                    current.color
                },
                boxes: current.boxes,
            };
            remaining -= 1;
        }
    }

    fn collect_generated(
        &mut self,
        id: NodeId,
        pseudo: PseudoElement,
        href: Option<&'a str>,
        host_style: Style,
        containing: (i32, i32),
        items: &mut Vec<Item<'a>>,
    ) {
        let (containing_x, containing_width) = containing;
        let Some(generated) = self.computed_styles.pseudo_style_for(id, pseudo) else {
            return;
        };
        if generated.style.display == Display::None {
            return;
        }

        let mut style = computed_style(generated.style);
        if generated.replaced_image && generated.style.display == Display::Block {
            let image = self.generated_images.get(&(id, pseudo, 0)).cloned();
            self.emit(items, host_style, containing_x, containing_width);
            self.block_image((id, Some(pseudo)), href, style, image.as_ref(), containing);
            return;
        }
        if generated.style.display == Display::Block {
            self.emit(items, host_style, containing_x, containing_width);
            self.block(
                BlockContent::Generated(id, pseudo),
                href,
                style,
                containing_x,
                containing_width,
            );
            return;
        }
        let own_box = resolve_inline_box_style(id, Some(pseudo), style, containing_width);
        let reserves_empty_fragment = own_box.is_some_and(InlineBoxStyle::reserves_empty_fragment);
        style.inline.boxes = own_box
            .map(|box_style| self.inline_boxes.push(box_style, host_style.inline.boxes))
            .or(host_style.inline.boxes);
        if generated.items.is_empty() {
            if reserves_empty_fragment {
                items.push(Item::EmptyInline(style.inline));
            }
            return;
        }
        let initial_len = items.len();
        self.collect_generated_items((id, pseudo), href, style, containing_width, items);
        if items.len() == initial_len && reserves_empty_fragment {
            items.push(Item::EmptyInline(style.inline));
        }
    }

    fn collect_generated_items(
        &self,
        target: (NodeId, PseudoElement),
        href: Option<&'a str>,
        style: Style,
        containing_width: i32,
        items: &mut Vec<Item<'a>>,
    ) {
        let Some(generated) = self.computed_styles.pseudo_style_for(target.0, target.1) else {
            return;
        };
        for (index, item) in generated.items.iter().enumerate() {
            match item {
                op_css::GeneratedContentItem::Text(text) => items.extend(text.chars().map(|ch| {
                    Item::Char(InlineChar {
                        ch,
                        href,
                        style: style.inline,
                    })
                })),
                op_css::GeneratedContentItem::Image { .. } => {
                    let image = self.generated_images.get(&(target.0, target.1, index));
                    if image.is_some() || generated.replaced_image {
                        let mut image_style = style.inline;
                        let mut own_box = None;
                        let sizing = if generated.replaced_image
                            && generated.style.display == Display::Inline
                        {
                            own_box = resolve_inline_box_style(
                                target.0,
                                Some(target.1),
                                style,
                                containing_width,
                            );
                            if own_box.is_some()
                                && let Some(box_id) = image_style.boxes
                            {
                                image_style.boxes = self.inline_boxes.parent(box_id);
                            }
                            style
                        } else {
                            default_style()
                        };
                        let Some((width, height)) = resolve_image_size(
                            sizing,
                            image.map_or((0, 0), |pixels| (pixels.width(), pixels.height())),
                            own_box,
                            containing_width,
                            containing_width
                                .saturating_sub(self.inline_boxes.horizontal(image_style.boxes)),
                        ) else {
                            continue;
                        };
                        items.push(Item::Image(
                            InlineImage {
                                width,
                                height,
                                image: image.cloned(),
                                href: href.map(str::to_owned),
                            },
                            image_style,
                            own_box,
                        ));
                    }
                }
            }
        }
    }

    fn collect_image(
        &mut self,
        id: NodeId,
        element: &'a op_dom::ElementData,
        href: Option<&'a str>,
        style: Style,
        available_width: i32,
        items: &mut Vec<Item<'a>>,
    ) {
        let image = self.images.get(&id);
        if image.is_some() || attribute(element, "alt").is_none_or(str::is_empty) {
            let image_style = style.inline;
            let own_box = resolve_inline_box_style(id, None, style, available_width);
            let Some((width, height)) = resolve_image_size(
                style,
                image.map_or((0, 0), |pixels| (pixels.width(), pixels.height())),
                own_box,
                available_width,
                available_width.saturating_sub(self.inline_boxes.horizontal(image_style.boxes)),
            ) else {
                return;
            };
            items.push(Item::Image(
                InlineImage {
                    width,
                    height,
                    image: image.cloned(),
                    href: href.map(str::to_owned),
                },
                image_style,
                own_box,
            ));
        } else {
            if style
                .width
                .is_some_and(|value| resolve_length(value, available_width) == 0)
                || matches!(style.height, Some(LengthPercentage::Px(0.0)))
            {
                return;
            }
            items.extend(
                attribute(element, "alt")
                    .unwrap_or_default()
                    .chars()
                    .map(|ch| {
                        Item::Char(InlineChar {
                            ch,
                            href,
                            style: style.inline,
                        })
                    }),
            );
        }
    }

    fn block_image(
        &mut self,
        target: (NodeId, Option<PseudoElement>),
        href: Option<&str>,
        style: Style,
        image: Option<&Arc<RasterImage>>,
        containing: (i32, i32),
    ) {
        self.block_epoch = self.block_epoch.saturating_add(1);
        let (containing_x, containing_width) = containing;
        let box_style = resolve_inline_box_style(target.0, target.1, style, containing_width);
        let left = box_style.map_or(0, InlineBoxStyle::left_extra);
        let right = box_style.map_or(0, InlineBoxStyle::right_extra);
        let top = box_style.map_or(0, InlineBoxStyle::top_extra);
        let bottom = box_style.map_or(0, InlineBoxStyle::bottom_extra);
        let margin_left = resolve_margin(style.margin.left, containing_width);
        let margin_right = resolve_margin(style.margin.right, containing_width);
        let fit_width = containing_width
            .saturating_sub(margin_left.unwrap_or(0))
            .saturating_sub(margin_right.unwrap_or(0));
        let Some((width, height)) = resolve_image_size(
            style,
            image.map_or((0, 0), |pixels| (pixels.width(), pixels.height())),
            box_style,
            containing_width,
            fit_width,
        ) else {
            return;
        };
        let outer_width = width.saturating_add(left).saturating_add(right);
        let outer_height = height.saturating_add(top).saturating_add(bottom);
        let remaining = containing_width.saturating_sub(outer_width);
        let used_left = match (margin_left, margin_right) {
            (None, None) => remaining.max(0) / 2,
            (None, Some(right)) => remaining.saturating_sub(right).max(0),
            (Some(left), _) => left,
        };
        self.apply_collapsed_margin(resolve_vertical_margin(style.margin.top, containing_width));
        let x = containing_x.saturating_add(used_left);
        if let Some(box_style) = box_style {
            self.decorations.push(BoxDecoration {
                x,
                y: self.y,
                width: outer_width,
                height: outer_height,
                background: box_style.background,
                border_top: box_style.border_top,
                border_right: box_style.border_right,
                border_bottom: box_style.border_bottom,
                border_left: box_style.border_left,
            });
        }
        if let Some(image) = image {
            self.order.push(LayoutItem::Image(self.images_out.len()));
            self.images_out.push(ImageBox {
                x: x.saturating_add(left),
                y: self.y.saturating_add(top),
                width,
                height,
                image: image.clone(),
                href: href.map(str::to_owned),
            });
        }
        self.y = self.y.saturating_add(outer_height);
        self.pending_margin = Some(resolve_vertical_margin(
            style.margin.bottom,
            containing_width,
        ));
    }

    fn element_display(&self, id: NodeId, tag: &str) -> Display {
        self.computed_styles
            .style_for(id)
            .map_or_else(|| fallback_display(tag), |style| style.display)
    }

    fn element_style(&self, id: NodeId, tag: &str, inherited: Style) -> Style {
        self.computed_styles
            .style_for(id)
            .copied()
            .map(computed_style)
            .unwrap_or_else(|| fallback_style(tag, inherited))
    }
}

fn resolve_image_size(
    style: Style,
    natural: (u32, u32),
    box_style: Option<InlineBoxStyle>,
    available_width: i32,
    fit_width: i32,
) -> Option<(i32, i32)> {
    let horizontal_extras = box_style.map_or(0, |value| {
        value.left_extra().saturating_add(value.right_extra())
    });
    let vertical_extras = box_style.map_or(0, |value| {
        value.top_extra().saturating_add(value.bottom_extra())
    });
    let width_value =
        |value| size_to_content_width(value, style.box_sizing, available_width, horizontal_extras);
    let height_value = |value| match value {
        LengthPercentage::Px(_) => Some(size_to_content_width(
            value,
            style.box_sizing,
            0,
            vertical_extras,
        )),
        LengthPercentage::Percent(_) => None,
    };
    let (width, height) = super::replaced::dimensions(
        natural,
        style.width.map(width_value),
        style.height.and_then(height_value),
        (
            width_value(style.min_width),
            style.max_width.map(width_value),
        ),
        (
            height_value(style.min_height).unwrap_or(0),
            style.max_height.and_then(height_value),
        ),
    );
    let transparent = natural == (0, 0);
    if (!transparent && (width == 0 || height == 0))
        || (transparent
            && width == 0
            && height == 0
            && horizontal_extras == 0
            && vertical_extras == 0)
    {
        return None;
    }
    let (mut width, mut height) = (width as u32, height as u32);
    let available_width = fit_width.saturating_sub(horizontal_extras).max(1) as u32;
    if width > available_width {
        height = (u64::from(height) * u64::from(available_width) / u64::from(width))
            .max(u64::from(height > 0)) as u32;
        width = available_width;
    }
    if height > op_image::MAX_DIMENSION {
        width = (u64::from(width) * u64::from(op_image::MAX_DIMENSION) / u64::from(height))
            .max(u64::from(width > 0)) as u32;
        height = op_image::MAX_DIMENSION;
    }
    Some((width as i32, height as i32))
}

fn resolve_inline_box_style(
    node: NodeId,
    pseudo: Option<PseudoElement>,
    style: Style,
    containing_width: i32,
) -> Option<InlineBoxStyle> {
    let padding_top = resolve_length(style.padding.top, containing_width).max(0);
    let padding_right = resolve_length(style.padding.right, containing_width).max(0);
    let padding_bottom = resolve_length(style.padding.bottom, containing_width).max(0);
    let padding_left = resolve_length(style.padding.left, containing_width).max(0);
    let border = resolve_border_edges(style.border);
    let visible = style.background.alpha > 0
        || padding_top > 0
        || padding_right > 0
        || padding_bottom > 0
        || padding_left > 0
        || border.top.width > 0
        || border.right.width > 0
        || border.bottom.width > 0
        || border.left.width > 0;
    visible.then_some(InlineBoxStyle {
        node,
        pseudo,
        padding_top,
        padding_right,
        padding_bottom,
        padding_left,
        background: style.background,
        border_top: DecorationBorder {
            width: border.top.width,
            color: border.top.color,
        },
        border_right: DecorationBorder {
            width: border.right.width,
            color: border.right.color,
        },
        border_bottom: DecorationBorder {
            width: border.bottom.width,
            color: border.bottom.color,
        },
        border_left: DecorationBorder {
            width: border.left.width,
            color: border.left.color,
        },
    })
}

fn table_span(element: &op_dom::ElementData, name: &str, default: usize, maximum: usize) -> usize {
    element
        .attributes
        .iter()
        .find(|attribute| attribute.name.eq_ignore_ascii_case(name))
        .and_then(|attribute| attribute.value.trim().parse::<usize>().ok())
        .map(|value| value.min(maximum))
        .unwrap_or(default)
}

fn apply_fixed_column_length(
    widths: &mut [Option<i32>],
    cursor: &mut usize,
    span: usize,
    hint: Option<LengthPercentage>,
    basis: i32,
    divide_across_span: bool,
) {
    let divisor = if divide_across_span {
        i32::try_from(span).unwrap_or(i32::MAX).max(1)
    } else {
        1
    };
    let resolved = hint.map(|value| resolve_length(value, basis).max(1) / divisor);
    for _ in 0..span {
        if let Some(slot) = widths.get_mut(*cursor)
            && slot.is_none()
            && let Some(value) = resolved
        {
            *slot = Some(value.max(1));
        }
        *cursor = cursor.saturating_add(1);
    }
}

fn fixed_cell_border_box_width(style: Style, width: LengthPercentage, basis: i32) -> i32 {
    let padding_left = resolve_length(style.padding.left, basis).max(0);
    let padding_right = resolve_length(style.padding.right, basis).max(0);
    let border = resolve_border_edges(style.border);
    let extras = padding_left
        .saturating_add(padding_right)
        .saturating_add(border.left.width)
        .saturating_add(border.right.width);
    let resolved = resolve_length(width, basis).max(0);
    match style.box_sizing {
        BoxSizing::ContentBox => resolved.saturating_add(extras),
        BoxSizing::BorderBox => resolved.max(extras),
    }
    .max(1)
}

fn apply_fixed_span_width(
    widths: &mut [Option<i32>],
    start: usize,
    span: usize,
    required_width: i32,
    spacing: i32,
) {
    let end = start.saturating_add(span).min(widths.len());
    if start >= end {
        return;
    }

    let range = &mut widths[start..end];
    let known = range
        .iter()
        .filter_map(|width| *width)
        .fold(0i32, i32::saturating_add);
    let unresolved = range.iter().filter(|width| width.is_none()).count();
    if unresolved == 0 {
        return;
    }

    let internal_spacing = spacing.saturating_mul(range.len().saturating_sub(1) as i32);
    let remaining = required_width
        .saturating_sub(internal_spacing)
        .saturating_sub(known)
        .max(unresolved as i32);
    let each = remaining / unresolved as i32;
    let mut remainder = remaining % unresolved as i32;
    for width in range {
        if width.is_some() {
            continue;
        }
        let extra = i32::from(remainder > 0);
        remainder = remainder.saturating_sub(extra);
        *width = Some(each.saturating_add(extra).max(1));
    }
}

fn resolve_fixed_track_widths(widths: Vec<Option<i32>>, usable: i32) -> Vec<i32> {
    if widths.is_empty() {
        return Vec::new();
    }

    let columns = widths.len();
    let minimum_usable = usable.max(columns as i32);
    let known_sum = widths
        .iter()
        .filter_map(|width| *width)
        .fold(0i32, i32::saturating_add);
    let unresolved = widths.iter().filter(|width| width.is_none()).count();

    if unresolved > 0 && known_sum < minimum_usable {
        let remaining = minimum_usable.saturating_sub(known_sum);
        let each = remaining / unresolved as i32;
        let mut remainder = remaining % unresolved as i32;
        return widths
            .into_iter()
            .map(|width| {
                width.unwrap_or_else(|| {
                    let extra = i32::from(remainder > 0);
                    remainder = remainder.saturating_sub(extra);
                    each.saturating_add(extra).max(1)
                })
            })
            .collect();
    }

    let mut resolved: Vec<i32> = widths
        .into_iter()
        .map(|width| width.unwrap_or(1).max(1))
        .collect();
    let current = resolved.iter().copied().fold(0i32, i32::saturating_add);
    if current < minimum_usable {
        let weights = resolved.clone();
        distribute_table_width(
            &mut resolved,
            &weights,
            minimum_usable.saturating_sub(current),
        );
    } else if current > minimum_usable {
        let mut squeezed = vec![1; columns];
        let weights: Vec<i32> = resolved
            .iter()
            .map(|width| width.saturating_sub(1).max(0))
            .collect();
        distribute_table_width(
            &mut squeezed,
            &weights,
            minimum_usable.saturating_sub(columns as i32),
        );
        resolved = squeezed;
    }
    resolved
}

fn apply_column_length_hint(
    columns: &mut [TableColumnIntrinsic],
    cursor: &mut usize,
    span: usize,
    hint: Option<LengthPercentage>,
    basis: i32,
    divide_across_span: bool,
) {
    let divisor = if divide_across_span {
        i32::try_from(span).unwrap_or(i32::MAX).max(1) as f32
    } else {
        1.0
    };

    for _ in 0..span {
        if let Some(column) = columns.get_mut(*cursor)
            && let Some(hint) = hint
        {
            match hint {
                LengthPercentage::Px(value) => {
                    let hint = bounded_round(value / divisor).max(1);
                    column.min = column.min.max(hint);
                    column.max = column.max.max(hint).max(column.min);
                }
                LengthPercentage::Percent(value) => {
                    let percent = (value / divisor).max(0.0);
                    column.percent = column.percent.max(percent);
                    let resolved = bounded_round(percent * basis.max(0) as f32).max(1);
                    column.max = column.max.max(resolved).max(column.min);
                }
            }
        }
        *cursor = cursor.saturating_add(1);
    }
}

fn grow_percent_span(columns: &mut [TableColumnIntrinsic], required: f32) {
    if columns.is_empty() || required <= 0.0 {
        return;
    }
    let current: f32 = columns.iter().map(|column| column.percent.max(0.0)).sum();
    let deficit = (required - current).max(0.0);
    if deficit <= f32::EPSILON {
        return;
    }
    let each = deficit / columns.len() as f32;
    for column in columns {
        column.percent = (column.percent + each).max(0.0);
    }
}

fn grow_intrinsic_span(columns: &mut [TableColumnIntrinsic], required_min: i32, required_max: i32) {
    if columns.is_empty() {
        return;
    }

    let current_min = columns
        .iter()
        .map(|column| column.min)
        .fold(0i32, i32::saturating_add);
    let min_deficit = required_min.saturating_sub(current_min).max(0);
    distribute_intrinsic_deficit(columns, min_deficit, true);

    for column in columns.iter_mut() {
        column.max = column.max.max(column.min);
    }

    let current_max = columns
        .iter()
        .map(|column| column.max)
        .fold(0i32, i32::saturating_add);
    let max_deficit = required_max
        .max(required_min)
        .saturating_sub(current_max)
        .max(0);
    distribute_intrinsic_deficit(columns, max_deficit, false);
}

fn distribute_intrinsic_deficit(columns: &mut [TableColumnIntrinsic], amount: i32, minimum: bool) {
    if columns.is_empty() || amount <= 0 {
        return;
    }

    let count = i32::try_from(columns.len()).unwrap_or(i32::MAX).max(1);
    let each = amount / count;
    let mut remainder = amount % count;
    for column in columns {
        let extra = each.saturating_add(i32::from(remainder > 0));
        remainder = remainder.saturating_sub(1).max(0);
        if minimum {
            column.min = column.min.saturating_add(extra);
            column.max = column.max.max(column.min);
        } else {
            column.max = column.max.saturating_add(extra).max(column.min);
        }
    }
}

fn collapse_intrinsic_whitespace(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut pending_space = false;
    for ch in text.chars() {
        if matches!(ch, ' ' | '\t' | '\n' | '\r' | '\u{c}') {
            pending_space = !output.is_empty();
            continue;
        }
        if pending_space {
            output.push(' ');
            pending_space = false;
        }
        output.push(ch);
    }
    output
}

fn transform_intrinsic_text(text: &str, transform: TextTransform) -> String {
    let mut output = String::new();
    let mut capitalize_next = true;
    for ch in text.chars() {
        match transform {
            TextTransform::None => output.push(ch),
            TextTransform::Uppercase => output.extend(ch.to_uppercase()),
            TextTransform::Lowercase => output.extend(ch.to_lowercase()),
            TextTransform::Capitalize if capitalize_next && ch.is_alphabetic() => {
                output.extend(ch.to_uppercase());
            }
            TextTransform::Capitalize => output.push(ch),
        }
        capitalize_next = ch.is_whitespace();
    }
    output
}

fn measure_intrinsic_text_run(
    measurer: &mut dyn TextMeasurer,
    text: &str,
    style: InlineStyle,
) -> i32 {
    if text.is_empty() {
        return 0;
    }

    let text = transform_intrinsic_text(text, style.text_transform);
    let base = measurer
        .measure(&text, style.font_size, style.weight, style.font_style)
        .width;
    let gaps = text.chars().count().saturating_sub(1) as i32;
    let spaces = text.chars().filter(|ch| *ch == ' ').count() as i32;
    base.saturating_add(style.letter_spacing.saturating_mul(gaps))
        .saturating_add(style.word_spacing.saturating_mul(spaces))
        .max(0)
}

fn inherit_atomic_href(atomic: &mut InlineAtomic, href: &str) {
    for text in &mut atomic.text_boxes {
        if text.links.is_empty() && !text.text.is_empty() {
            text.links.push(LinkSpan {
                start: 0,
                end: text.text.len(),
                href: href.to_owned(),
            });
        }
    }
    for image in &mut atomic.image_boxes {
        if image.href.is_none() {
            image.href = Some(href.to_owned());
        }
    }
}

fn table_horizontal_extras(style: Style, basis: i32) -> i32 {
    let border = resolve_border_edges(style.border);
    resolve_length(style.padding.left, basis)
        .max(0)
        .saturating_add(resolve_length(style.padding.right, basis).max(0))
        .saturating_add(border.left.width)
        .saturating_add(border.right.width)
}

fn used_table_spacing(style: Style) -> (i32, i32) {
    if style.border_collapse == BorderCollapse::Collapse {
        return (0, 0);
    }

    (
        style
            .border_spacing
            .horizontal_px
            .round()
            .clamp(0.0, 1_000_000.0) as i32,
        style
            .border_spacing
            .vertical_px
            .round()
            .clamp(0.0, 1_000_000.0) as i32,
    )
}

fn table_column_widths(
    content_width: i32,
    intrinsic: &[TableColumnIntrinsic],
    spacing: i32,
) -> Vec<i32> {
    if intrinsic.is_empty() {
        return Vec::new();
    }

    let columns = intrinsic.len();
    let total_spacing = spacing.saturating_mul(columns.saturating_add(1) as i32);
    let usable = content_width
        .saturating_sub(total_spacing)
        .max(columns as i32);

    let minimums: Vec<i32> = intrinsic
        .iter()
        .map(|column| {
            let percentage = bounded_round(column.percent.max(0.0) * usable as f32).max(0);
            column.min.max(percentage).max(1)
        })
        .collect();
    let min_sum = minimums.iter().copied().fold(0i32, i32::saturating_add);

    if usable <= min_sum {
        let mut widths = vec![1; columns];
        let weights: Vec<i32> = minimums
            .iter()
            .map(|width| width.saturating_sub(1).max(0))
            .collect();
        distribute_table_width(&mut widths, &weights, usable.saturating_sub(columns as i32));
        return widths;
    }

    let preferred: Vec<i32> = intrinsic
        .iter()
        .zip(&minimums)
        .map(|(column, minimum)| column.max.max(*minimum).max(1))
        .collect();
    let preferred_sum = preferred.iter().copied().fold(0i32, i32::saturating_add);

    if usable < preferred_sum {
        let mut widths = minimums;
        let weights: Vec<i32> = preferred
            .iter()
            .zip(&widths)
            .map(|(preferred, minimum)| preferred.saturating_sub(*minimum).max(0))
            .collect();
        distribute_table_width(&mut widths, &weights, usable.saturating_sub(min_sum));
        return widths;
    }

    let mut widths = preferred;
    let has_auto_columns = intrinsic.iter().any(|column| column.percent <= 0.0);
    let weights: Vec<i32> = widths
        .iter()
        .zip(intrinsic)
        .map(|(width, column)| {
            if has_auto_columns && column.percent > 0.0 {
                0
            } else {
                (*width).max(1)
            }
        })
        .collect();
    distribute_table_width(&mut widths, &weights, usable.saturating_sub(preferred_sum));
    widths
}

fn distribute_table_width(widths: &mut [i32], weights: &[i32], amount: i32) {
    if widths.is_empty() || amount <= 0 {
        return;
    }

    let weight_sum: i64 = weights
        .iter()
        .map(|weight| i64::from((*weight).max(0)))
        .sum();
    let even = weight_sum == 0;
    let denominator = if even {
        widths.len() as i64
    } else {
        weight_sum
    }
    .max(1);

    let mut assigned = 0i32;
    for (index, width) in widths.iter_mut().enumerate() {
        let weight = if even {
            1i64
        } else {
            i64::from(weights.get(index).copied().unwrap_or(0).max(0))
        };
        let extra =
            ((i64::from(amount) * weight) / denominator).clamp(0, i64::from(i32::MAX)) as i32;
        *width = width.saturating_add(extra);
        assigned = assigned.saturating_add(extra);
    }

    let mut remainder = amount.saturating_sub(assigned);
    let mut index = 0usize;
    while remainder > 0 && !widths.is_empty() {
        widths[index] = widths[index].saturating_add(1);
        remainder -= 1;
        index = (index + 1) % widths.len();
    }
}

fn table_column_offsets(content_x: i32, widths: &[i32], spacing: i32) -> Vec<i32> {
    let mut offsets = Vec::with_capacity(widths.len());
    let mut x = content_x.saturating_add(spacing);
    for width in widths {
        offsets.push(x);
        x = x.saturating_add(*width).saturating_add(spacing);
    }
    offsets
}

fn table_cell_slot_width(widths: &[i32], column: usize, colspan: usize, spacing: i32) -> i32 {
    let end = column.saturating_add(colspan).min(widths.len());
    widths[column.min(widths.len())..end]
        .iter()
        .copied()
        .fold(0i32, i32::saturating_add)
        .saturating_add(spacing.saturating_mul(end.saturating_sub(column + 1) as i32))
        .max(1)
}

fn resolve_block_horizontal(style: Style, containing_width: i32) -> UsedBlockHorizontal {
    let containing_width = containing_width.max(1);
    let padding = Edges {
        top: resolve_length(style.padding.top, containing_width).max(0),
        right: resolve_length(style.padding.right, containing_width).max(0),
        bottom: resolve_length(style.padding.bottom, containing_width).max(0),
        left: resolve_length(style.padding.left, containing_width).max(0),
    };
    let border = resolve_border_edges(style.border);
    let horizontal_extras = padding
        .left
        .saturating_add(padding.right)
        .saturating_add(border.left.width)
        .saturating_add(border.right.width);

    let margin_left = resolve_margin(style.margin.left, containing_width);
    let margin_right = resolve_margin(style.margin.right, containing_width);
    let specified_width = style.width.map(|value| {
        size_to_content_width(value, style.box_sizing, containing_width, horizontal_extras)
    });

    let mut content_width = specified_width.unwrap_or_else(|| {
        containing_width
            .saturating_sub(margin_left.unwrap_or(0))
            .saturating_sub(margin_right.unwrap_or(0))
            .saturating_sub(horizontal_extras)
    });
    let min_width = size_to_content_width(
        style.min_width,
        style.box_sizing,
        containing_width,
        horizontal_extras,
    );
    content_width = content_width.max(min_width);
    if let Some(max_width) = style.max_width {
        let max_width = size_to_content_width(
            max_width,
            style.box_sizing,
            containing_width,
            horizontal_extras,
        );
        content_width = content_width.min(max_width.max(0));
    }
    content_width = content_width.max(0);

    let border_width = content_width.saturating_add(horizontal_extras).max(1);
    let (margin_left, margin_right) = if specified_width.is_none() {
        (margin_left.unwrap_or(0), margin_right.unwrap_or(0))
    } else {
        distribute_auto_margins(
            margin_left,
            margin_right,
            containing_width.saturating_sub(border_width),
        )
    };

    let _ = margin_right;
    UsedBlockHorizontal {
        margin_left,
        padding,
        border,
        content_width,
        border_width,
    }
}

fn distribute_auto_margins(
    left: Option<i32>,
    right: Option<i32>,
    available_for_margins: i32,
) -> (i32, i32) {
    match (left, right) {
        (None, None) => {
            let left = available_for_margins / 2;
            (left, available_for_margins - left)
        }
        (None, Some(right)) => (available_for_margins.saturating_sub(right), right),
        (Some(left), None) => (left, available_for_margins.saturating_sub(left)),
        (Some(left), Some(right)) => (left, right),
    }
}

fn resolve_margin(value: MarginValue, basis: i32) -> Option<i32> {
    match value {
        MarginValue::Auto => None,
        MarginValue::Length(value) => Some(resolve_length(value, basis)),
    }
}

fn resolve_vertical_margin(value: MarginValue, basis: i32) -> i32 {
    resolve_margin(value, basis).unwrap_or(0)
}

fn resolve_length(value: LengthPercentage, basis: i32) -> i32 {
    match value {
        LengthPercentage::Px(value) => bounded_round(value),
        LengthPercentage::Percent(value) => bounded_round(value * basis as f32),
    }
}

fn bounded_round(value: f32) -> i32 {
    value.round().clamp(-1_000_000.0, 1_000_000.0) as i32
}

pub(super) fn collapse_margins(first: i32, second: i32) -> i32 {
    let positive = first.max(second).max(0);
    let negative = first.min(second).min(0);
    positive.saturating_add(negative)
}

fn size_to_content_width(
    value: LengthPercentage,
    box_sizing: BoxSizing,
    basis: i32,
    horizontal_extras: i32,
) -> i32 {
    let resolved = resolve_length(value, basis).max(0);
    match box_sizing {
        BoxSizing::ContentBox => resolved,
        BoxSizing::BorderBox => resolved.saturating_sub(horizontal_extras).max(0),
    }
}

fn empty_used_border_side() -> UsedBorderSide {
    UsedBorderSide {
        width: 0,
        color: TextColor {
            red: 0,
            green: 0,
            blue: 0,
            alpha: 0,
        },
    }
}

fn choose_collapsed_border(current: &mut UsedBorderSide, candidate: UsedBorderSide) {
    if candidate.width > current.width
        || (candidate.width == current.width
            && candidate.width > 0
            && current.color.alpha == 0
            && candidate.color.alpha > 0)
    {
        *current = candidate;
    }
}

fn resolve_border_edges(border: BorderEdges) -> UsedBorderEdges {
    UsedBorderEdges {
        top: resolve_border_side(border.top),
        right: resolve_border_side(border.right),
        bottom: resolve_border_side(border.bottom),
        left: resolve_border_side(border.left),
    }
}

fn resolve_border_side(border: op_css::ComputedBorder) -> UsedBorderSide {
    UsedBorderSide {
        width: if border.style == BorderStyle::Solid {
            bounded_round(border.width_px).clamp(0, 4096)
        } else {
            0
        },
        color: border.color.into(),
    }
}

fn resolve_block_content_height(
    style: Style,
    natural_height: i32,
    padding_top: i32,
    padding_bottom: i32,
    border: UsedBorderEdges,
) -> i32 {
    let vertical_extras = padding_top
        .saturating_add(padding_bottom)
        .saturating_add(border.top.width)
        .saturating_add(border.bottom.width);
    let to_content = |value: LengthPercentage| -> Option<i32> {
        let LengthPercentage::Px(value) = value else {
            return None;
        };
        let resolved = bounded_round(value).max(0);
        Some(match style.box_sizing {
            BoxSizing::ContentBox => resolved,
            BoxSizing::BorderBox => resolved.saturating_sub(vertical_extras).max(0),
        })
    };

    let mut target = style.height.and_then(to_content).unwrap_or(natural_height);
    if let Some(min_height) = to_content(style.min_height) {
        target = target.max(min_height);
    }
    if let Some(max_height) = style.max_height.and_then(to_content) {
        target = target.min(max_height);
    }
    target.max(0)
}

fn computed_style(style: ComputedStyle) -> Style {
    let font_size = style.font_size_px.round().clamp(1.0, 4096.0) as i32;
    Style {
        inline: InlineStyle {
            font_size,
            line_height: used_line_height(style.line_height, font_size),
            weight: match style.font_weight {
                ComputedFontWeight::Normal => FontWeight::Normal,
                ComputedFontWeight::Bold => FontWeight::Bold,
            },
            font_style: match style.font_style {
                CssFontStyle::Normal => FontStyle::Normal,
                CssFontStyle::Italic | CssFontStyle::Oblique => FontStyle::Italic,
            },
            decoration: TextDecoration {
                underline: style.text_decoration_line.underline,
                line_through: style.text_decoration_line.line_through,
            },
            white_space: style.white_space,
            letter_spacing: style.letter_spacing_px.round().clamp(-4096.0, 4096.0) as i32,
            word_spacing: style.word_spacing_px.round().clamp(-4096.0, 4096.0) as i32,
            text_transform: style.text_transform,
            color: style.color.into(),
            boxes: None,
        },
        text_align: style.text_align,
        vertical_align: style.vertical_align,
        margin: style.margin,
        padding: style.padding,
        background: style.background_color.into(),
        border: style.border,
        width: style.width,
        min_width: style.min_width,
        max_width: style.max_width,
        height: style.height,
        min_height: style.min_height,
        max_height: style.max_height,
        box_sizing: style.box_sizing,
        border_collapse: style.border_collapse,
        border_spacing: style.border_spacing,
        table_layout: style.table_layout,
        caption_side: style.caption_side,
    }
}

fn used_line_height(value: ComputedLineHeight, font_size: i32) -> i32 {
    let value = match value {
        ComputedLineHeight::Normal => font_size as f32 * 1.35,
        ComputedLineHeight::Number(multiplier) => font_size as f32 * multiplier,
        ComputedLineHeight::Px(value) => value,
    };
    value.round().clamp(0.0, 100_000.0) as i32
}

fn fallback_inline_style(tag: &str, inherited: InlineStyle) -> InlineStyle {
    match tag {
        "h1" => InlineStyle {
            font_size: 34,
            weight: FontWeight::Bold,
            ..inherited
        },
        "h2" => InlineStyle {
            font_size: 28,
            weight: FontWeight::Bold,
            ..inherited
        },
        "h3" => InlineStyle {
            font_size: 23,
            weight: FontWeight::Bold,
            ..inherited
        },
        "h4" | "h5" | "h6" => InlineStyle {
            font_size: 18,
            weight: FontWeight::Bold,
            ..inherited
        },
        _ => inherited,
    }
}

fn is_table_internal_display(display: Display) -> bool {
    matches!(
        display,
        Display::TableCaption
            | Display::TableColumnGroup
            | Display::TableColumn
            | Display::TableHeaderGroup
            | Display::TableRowGroup
            | Display::TableFooterGroup
            | Display::TableRow
            | Display::TableCell
    )
}

fn fallback_display(tag: &str) -> Display {
    if hidden_tag(tag) {
        return Display::None;
    }

    match tag {
        "table" => Display::Table,
        "caption" => Display::TableCaption,
        "colgroup" => Display::TableColumnGroup,
        "col" => Display::TableColumn,
        "thead" => Display::TableHeaderGroup,
        "tbody" => Display::TableRowGroup,
        "tfoot" => Display::TableFooterGroup,
        "tr" => Display::TableRow,
        "td" | "th" => Display::TableCell,
        _ if is_block(tag) => Display::Block,
        _ => Display::Inline,
    }
}

fn anonymous_table_style(parent: Style) -> Style {
    let mut style = default_style();
    style.inline = InlineStyle {
        boxes: None,
        ..parent.inline
    };
    style.text_align = parent.text_align;
    style.border_collapse = parent.border_collapse;
    style.border_spacing = parent.border_spacing;
    style
}

fn anonymous_table_cell_style(parent: Style) -> Style {
    let mut style = default_style();
    style.inline = InlineStyle {
        boxes: None,
        ..parent.inline
    };
    style.text_align = parent.text_align;
    style.border_collapse = parent.border_collapse;
    style.border_spacing = parent.border_spacing;
    style
}

fn default_style() -> Style {
    Style {
        inline: InlineStyle {
            font_size: 18,
            line_height: 24,
            weight: FontWeight::Normal,
            font_style: FontStyle::Normal,
            decoration: TextDecoration::NONE,
            white_space: WhiteSpace::Normal,
            letter_spacing: 0,
            word_spacing: 0,
            text_transform: TextTransform::None,
            color: TextColor {
                red: 0,
                green: 0,
                blue: 0,
                alpha: 255,
            },
            boxes: None,
        },
        text_align: TextAlign::Start,
        vertical_align: VerticalAlign::Baseline,
        margin: MarginEdges::ZERO,
        padding: PaddingEdges::ZERO,
        background: TextColor {
            red: 0,
            green: 0,
            blue: 0,
            alpha: 0,
        },
        border: BorderEdges::NONE,
        width: None,
        min_width: LengthPercentage::ZERO,
        max_width: None,
        height: None,
        min_height: LengthPercentage::ZERO,
        max_height: None,
        box_sizing: BoxSizing::ContentBox,
        border_collapse: BorderCollapse::Separate,
        border_spacing: BorderSpacing::ZERO,
        table_layout: TableLayout::Auto,
        caption_side: CaptionSide::Top,
    }
}

fn fallback_style(tag: &str, inherited: Style) -> Style {
    let (top, bottom) = match tag {
        "h1" => (8, 16),
        "h2" => (8, 14),
        "h3" => (6, 12),
        "h4" | "h5" | "h6" => (6, 12),
        "p" | "li" => (0, 12),
        _ => (0, 0),
    };
    Style {
        inline: fallback_inline_style(tag, inherited.inline),
        text_align: inherited.text_align,
        vertical_align: VerticalAlign::Baseline,
        margin: MarginEdges {
            top: MarginValue::Length(LengthPercentage::Px(top as f32)),
            right: MarginValue::ZERO,
            bottom: MarginValue::Length(LengthPercentage::Px(bottom as f32)),
            left: MarginValue::ZERO,
        },
        padding: PaddingEdges::ZERO,
        background: default_style().background,
        border: BorderEdges::NONE,
        width: None,
        min_width: LengthPercentage::ZERO,
        max_width: None,
        height: None,
        min_height: LengthPercentage::ZERO,
        max_height: None,
        box_sizing: BoxSizing::ContentBox,
        border_collapse: inherited.border_collapse,
        table_layout: TableLayout::Auto,
        caption_side: inherited.caption_side,
        border_spacing: if tag == "table" {
            BorderSpacing {
                horizontal_px: 2.0,
                vertical_px: 2.0,
            }
        } else {
            inherited.border_spacing
        },
    }
}

fn attribute<'a>(element: &'a op_dom::ElementData, name: &str) -> Option<&'a str> {
    element
        .attributes
        .iter()
        .find(|a| a.name == name)
        .map(|a| a.value.as_str())
}

fn hidden_tag(tag: &str) -> bool {
    matches!(
        tag,
        "head" | "title" | "style" | "script" | "meta" | "link" | "template"
    )
}

fn is_block(tag: &str) -> bool {
    matches!(
        tag,
        "html"
            | "body"
            | "div"
            | "main"
            | "article"
            | "section"
            | "nav"
            | "header"
            | "footer"
            | "aside"
            | "ul"
            | "ol"
            | "blockquote"
            | "p"
            | "li"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "pre"
            | "address"
            | "dl"
            | "dt"
            | "dd"
            | "figure"
            | "figcaption"
    )
}
