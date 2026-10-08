use op_html::parse_document;
use op_image::RasterImage;
use op_layout::{
    FontStyle, FontWeight, GeneratedImageResources, ImageResources, LayoutTree, TextMeasurer,
    TextMetrics, layout_document_with_computed_styles_and_metrics,
    layout_document_with_resources_and_metrics,
};
use std::sync::Arc;

struct Fixed;
impl TextMeasurer for Fixed {
    fn measure(&mut self, text: &str, _: i32, _: FontWeight, _: FontStyle) -> TextMetrics {
        TextMetrics {
            width: text.chars().count() as i32 * 10,
            ascent: 14,
            descent: 4,
        }
    }
}

fn layout(html: &str, width: i32, measurer: &mut dyn TextMeasurer) -> LayoutTree {
    let document = parse_document(html);
    let mut images = ImageResources::new();
    let mut stack = vec![document.root()];
    let image = Arc::new(RasterImage::from_premultiplied_bgra(2, 2, vec![255; 16]).unwrap());
    while let Some(id) = stack.pop() {
        if document.element(id).is_some_and(|e| {
            e.tag_name == "img"
                && e.attributes
                    .iter()
                    .any(|a| a.name == "src" && a.value == "ok")
        }) {
            images.insert(id, image.clone());
        }
        stack.extend(document.children(id));
    }
    let computed =
        op_css::compute_styles(&document, &op_css::collect_author_styles(&document).styles);
    layout_document_with_computed_styles_and_metrics(&document, width, &images, &computed, measurer)
}

#[test]
fn inline_block_baselines_and_inline_backgrounds_are_independent_of_line_height() {
    let page = layout(
        "<div style='margin:0'><div style='display:inline-block;font-size:50px;line-height:200px;color:transparent'><span style='background:blue'>AA</span></div><div style='display:inline-block;font-size:50px;line-height:30px;color:transparent'><span style='background:blue'>AA</span></div><div style='display:inline-block;font-size:50px;line-height:normal;color:transparent'><span style='background:blue'>AA</span></div></div>",
        800,
        &mut Fixed,
    );
    let blue: Vec<_> = page
        .box_decorations
        .iter()
        .filter(|b| b.background == op_css::CssColor::BLUE.into())
        .collect();
    assert_eq!(blue.len(), 3);
    let y = blue[0].y;
    for fragment in blue {
        assert_eq!(fragment.y, y);
        assert_eq!(fragment.height, 18);
    }
}

#[test]
fn undecorated_relative_inline_establishes_containing_block_for_absolute_child() {
    let page = layout(
        "<div style='margin:0'>Before <span style='position:relative;left:20px;top:9px'>AB<span style='position:absolute;display:block;left:12px;top:7px;width:20px;height:10px;background:blue'></span>CD</span> after</div>",
        800,
        &mut Fixed,
    );
    let positioned = page
        .box_decorations
        .iter()
        .find(|box_| box_.background == op_css::CssColor::BLUE.into())
        .unwrap();
    let containing = page
        .box_decorations
        .iter()
        .find(|box_| box_.background.alpha == 0 && box_.width == 40)
        .unwrap();
    assert_eq!(
        (positioned.x, positioned.y),
        (containing.x + 12, containing.y + 7)
    );
    assert_eq!((positioned.width, positioned.height), (20, 10));
    let before = page
        .text_boxes
        .iter()
        .find(|text| text.text == "Before ")
        .unwrap();
    let inner = page
        .text_boxes
        .iter()
        .find(|text| text.text == "AB")
        .unwrap();
    let after = page
        .text_boxes
        .iter()
        .find(|text| text.text == " after")
        .unwrap();
    assert_eq!(inner.x, before.x + before.width + 20);
    assert_eq!(inner.y, before.y + 9);
    assert_eq!(after.x, before.x + before.width + 40);
    assert_eq!(after.y, before.y);
}

#[test]
fn nearest_relative_inline_wins_over_nested_relative_block_and_fixed_uses_viewport() {
    let page = layout(
        "<div style='margin:0;position:relative;width:300px;height:70px;background:red'><span style='position:relative;left:15px;top:5px'>AA<span style='position:relative;left:7px;top:4px'>BB<span style='position:absolute;left:3px;top:6px;width:10px;height:10px;background:blue'></span></span><span style='position:fixed;left:20px;top:30px;width:10px;height:10px;background:green'></span></span></div>",
        800,
        &mut Fixed,
    );
    let blue = page
        .box_decorations
        .iter()
        .find(|box_| box_.background == op_css::CssColor::BLUE.into())
        .unwrap();
    let green = page
        .box_decorations
        .iter()
        .find(|box_| box_.background == op_css::CssColor::GREEN.into())
        .unwrap();
    let bb = page
        .text_boxes
        .iter()
        .find(|text| text.text == "BB")
        .unwrap();
    let inner = page
        .box_decorations
        .iter()
        .find(|box_| box_.background.alpha == 0 && box_.x == bb.x)
        .unwrap();
    assert_eq!((blue.x, blue.y), (inner.x + 3, inner.y + 6));
    assert_eq!((green.x, green.y), (20, 30));
}

#[test]
fn wrapped_relative_inline_uses_first_and_last_fragment_padding_edges() {
    let page = layout(
        "<div style='margin:0;width:70px'><span style='position:relative;left:5px;top:3px;background:red'>abc def ghi<span style='position:absolute;left:0;top:0;width:8px;height:8px;background:blue'></span></span></div>",
        800,
        &mut Fixed,
    );
    let fragments: Vec<_> = page
        .box_decorations
        .iter()
        .filter(|box_| box_.background == op_css::CssColor::RED.into())
        .collect();
    assert!(fragments.len() >= 2);
    let blue = page
        .box_decorations
        .iter()
        .find(|box_| box_.background == op_css::CssColor::BLUE.into())
        .unwrap();
    assert_eq!((blue.x, blue.y), (fragments[0].x, fragments[0].y));
    assert_eq!(
        page.text_boxes
            .iter()
            .filter(|text| text.text.contains("abc"))
            .count(),
        1
    );
}

#[test]
fn rtl_relative_inline_uses_last_fragment_left_and_first_fragment_right() {
    let page = layout(
        "<div style='margin:0;width:85px'>XX <span style='position:relative;direction:rtl;background:red'>abc def ghi<span style='position:absolute;left:0;top:0;width:8px;height:8px;background:blue'></span></span></div>",
        800,
        &mut Fixed,
    );
    let fragments: Vec<_> = page
        .box_decorations
        .iter()
        .filter(|box_| box_.background == op_css::CssColor::RED.into())
        .collect();
    assert!(fragments.len() >= 2);
    assert_ne!(fragments[0].x, fragments.last().unwrap().x);
    let child = page
        .box_decorations
        .iter()
        .find(|box_| box_.background == op_css::CssColor::BLUE.into())
        .unwrap();
    assert_eq!(child.x, fragments.last().unwrap().x);
    assert_eq!(child.y, fragments[0].y);
}

#[test]
fn non_positioned_inline_does_not_reparent_block_level_absolute_static_position() {
    let page = layout(
        "<div style='margin:0'><span style='margin-right:-10px'>x<div style='position:absolute;width:10px;height:10px;background:blue'></div></span></div>",
        800,
        &mut Fixed,
    );
    let reference = layout(
        "<div style='margin:0'>x<br><span style='position:absolute;width:10px;height:10px;background:blue'></span></div>",
        800,
        &mut Fixed,
    );
    let actual = page
        .box_decorations
        .iter()
        .find(|box_| box_.background == op_css::CssColor::BLUE.into())
        .unwrap();
    let expected = reference
        .box_decorations
        .iter()
        .find(|box_| box_.background == op_css::CssColor::BLUE.into())
        .unwrap();
    assert_eq!((actual.x, actual.y), (expected.x, expected.y));
}

#[test]
fn split_inline_containing_block_spans_distinct_formatter_runs() {
    let page = layout(
        "<div style='margin:0;width:300px'><span style='position:relative;background:red'>AA<div style='height:30px'></div>BBBB<span style='position:absolute;left:0;right:0;top:0;bottom:0;background:blue'></span></span></div>",
        800,
        &mut Fixed,
    );
    let fragments: Vec<_> = page
        .box_decorations
        .iter()
        .filter(|box_| box_.background == op_css::CssColor::RED.into())
        .collect();
    assert_eq!(fragments.len(), 2);
    let blue = page
        .box_decorations
        .iter()
        .find(|box_| box_.background == op_css::CssColor::BLUE.into())
        .unwrap();
    let first = fragments[0];
    let last = fragments[1];
    assert_eq!(blue.x, first.x);
    assert_eq!(blue.y, first.y);
    assert_eq!(blue.width, last.x + last.width - first.x);
    assert_eq!(blue.height, last.y + last.height - first.y);
}

#[test]
fn earlier_absolute_child_waits_for_later_split_inline_fragments() {
    let page = layout(
        "<div style='margin:0;width:300px'><span style='position:relative;background:red'>AA<span style='position:absolute;left:0;top:0;bottom:0;width:9px;background:blue'></span><div style='height:30px'></div>BBBB</span></div>",
        800,
        &mut Fixed,
    );
    let fragments: Vec<_> = page
        .box_decorations
        .iter()
        .filter(|box_| box_.background == op_css::CssColor::RED.into())
        .collect();
    assert_eq!(fragments.len(), 2);
    let blue = page
        .box_decorations
        .iter()
        .find(|box_| box_.background == op_css::CssColor::BLUE.into())
        .unwrap();
    assert_eq!((blue.x, blue.y), (fragments[0].x, fragments[0].y));
    assert_eq!(
        blue.height,
        fragments[1].y + fragments[1].height - fragments[0].y
    );
}

#[test]
fn relative_outer_block_translates_deferred_inline_descendant() {
    let page = layout(
        "<div style='position:relative;left:35px;top:14px;margin:0'><span style='position:relative;background:red'>AA<span style='position:absolute;left:0;top:0;width:10px;height:10px;background:blue'></span></span></div>",
        800,
        &mut Fixed,
    );
    let red = page
        .box_decorations
        .iter()
        .find(|box_| box_.background == op_css::CssColor::RED.into())
        .unwrap();
    let blue = page
        .box_decorations
        .iter()
        .find(|box_| box_.background == op_css::CssColor::BLUE.into())
        .unwrap();
    assert_eq!((blue.x, blue.y), (red.x, red.y));
    assert_eq!(blue.x, 67);
}

#[test]
fn linked_and_unlinked_nested_inline_backgrounds_keep_same_green_geometry() {
    let original = layout(
        "<style>*{background-color:white}div *:not(:link):not(:visited){background-color:green}</style><div><a href='#'>Unvisited (<span>Green</span>)</a><a href='#'>Visited (<span>Green</span>)</a><span>Green</span></div>",
        800,
        &mut Fixed,
    );
    let reference = layout(
        "<style>span{background-color:green}</style><div><a href='#'>Unvisited (<span>Green</span>)</a><a href='#'>Visited (<span>Green</span>)</a><span>Green</span></div>",
        800,
        &mut Fixed,
    );
    let green = op_css::CssColor::GREEN.into();
    let tested: Vec<_> = original
        .box_decorations
        .iter()
        .filter(|b| b.background == green)
        .map(|b| (b.x, b.y, b.width, b.height))
        .collect();
    let expected: Vec<_> = reference
        .box_decorations
        .iter()
        .filter(|b| b.background == green)
        .map(|b| (b.x, b.y, b.width, b.height))
        .collect();
    assert_eq!(tested, expected);
}

#[test]
fn rtl_static_absolute_block_uses_its_hypothetical_flow_width() {
    for direction in ["rtl", "ltr"] {
        let html = format!(
            "<div style='direction:{direction};width:200px;margin:0;background:red'><div style='position:absolute;width:30px;height:10px;background:blue'></div><span>Flow</span></div>"
        );
        let page = layout(&html, 800, &mut Fixed);
        let container = page
            .box_decorations
            .iter()
            .find(|box_| box_.background == op_css::CssColor::RED.into())
            .unwrap();
        let child = page
            .box_decorations
            .iter()
            .find(|box_| box_.background == op_css::CssColor::BLUE.into())
            .unwrap();
        let offset = if direction == "rtl" { 170 } else { 0 };
        assert_eq!(child.x, container.x + offset, "{direction}");
        assert_eq!((child.width, child.height), (30, 10));
    }
}

#[test]
fn rtl_fixed_static_block_uses_parent_flow_width_not_full_viewport() {
    let page = layout(
        "<div style='direction:rtl;width:200px;margin:0;background:red'><div style='position:fixed;width:30px;height:10px;background:blue'></div>Flow</div>",
        800,
        &mut Fixed,
    );
    let container = page
        .box_decorations
        .iter()
        .find(|box_| box_.background == op_css::CssColor::RED.into())
        .unwrap();
    let child = page
        .box_decorations
        .iter()
        .find(|box_| box_.background == op_css::CssColor::BLUE.into())
        .unwrap();
    assert_eq!(child.x, container.x + 170);
}

#[test]
fn rtl_split_inline_uses_later_continuation_as_left_padding_edge() {
    let page = layout(
        "<div style='margin:0;width:180px'>prefix <span style='direction:rtl;position:relative;background:red'>AA<div style='height:20px'></div>BBBB<span style='position:absolute;left:0;top:0;width:6px;height:8px;background:blue'></span></span></div>",
        800,
        &mut Fixed,
    );
    let reds: Vec<_> = page
        .box_decorations
        .iter()
        .filter(|box_| box_.background == op_css::CssColor::RED.into())
        .collect();
    assert_eq!(reds.len(), 2);
    let blue = page
        .box_decorations
        .iter()
        .find(|box_| box_.background == op_css::CssColor::BLUE.into())
        .unwrap();
    assert_ne!(reds[0].x, reds[1].x);
    assert_eq!(blue.x, reds[1].x);
    assert_eq!(blue.y, reds[0].y);
}

#[test]
fn nested_deferred_absolute_subtrees_finish_after_their_parent() {
    let page = layout(
        "<div style='margin:0'><span style='position:relative;background:red'>AAA<span style='position:absolute;left:12px;top:7px;width:80px;height:35px;background:blue'><span style='position:relative;background:#00ffff'>B<span style='position:absolute;left:3px;top:5px;width:8px;height:9px;background:green'></span></span></span></span></div>",
        800,
        &mut Fixed,
    );
    let red = page
        .box_decorations
        .iter()
        .find(|box_| box_.background == op_css::CssColor::RED.into())
        .unwrap();
    let blue = page
        .box_decorations
        .iter()
        .find(|box_| box_.background == op_css::CssColor::BLUE.into())
        .unwrap();
    let yellow = page
        .box_decorations
        .iter()
        .find(|box_| {
            box_.background
                == op_layout::TextColor {
                    red: 0,
                    green: 255,
                    blue: 255,
                    alpha: 255,
                }
        })
        .unwrap();
    let green = page
        .box_decorations
        .iter()
        .find(|box_| box_.background == op_css::CssColor::GREEN.into())
        .unwrap();
    assert_eq!((blue.x, blue.y), (red.x + 12, red.y + 7));
    assert_eq!((green.x, green.y), (yellow.x + 3, yellow.y + 5));
}

#[test]
fn nested_decorations_keep_outer_geometry_and_paint_order_across_text_styles() {
    let page = layout(
        "<p style='line-height:18px'><span style='padding:2px 3px;border:1px solid red;background:red'>A<span style='padding:4px 5px;border:2px solid blue;background:blue'><b>B</b></span>C</span>Z</p>",
        300,
        &mut Fixed,
    );
    assert_eq!(page.box_decorations.len(), 2);
    let (outer, inner) = (&page.box_decorations[0], &page.box_decorations[1]);
    assert_eq!((outer.x, outer.width, outer.height), (32, 52, 24));
    assert_eq!(
        (inner.x, inner.y, inner.width, inner.height),
        (46, outer.y - 3, 24, 30)
    );
    assert_eq!(
        page.text_boxes
            .iter()
            .map(|text| (text.text.as_str(), text.x))
            .collect::<Vec<_>>(),
        [("A", 36), ("B", 53), ("C", 70), ("Z", 84)]
    );
    assert!(
        page.text_boxes
            .iter()
            .all(|text| text.y == page.text_boxes[0].y)
    );
    assert_eq!(page.text_boxes[1].weight, FontWeight::Bold);
}

#[test]
fn nested_fragments_reserve_all_edges_on_each_wrapped_line() {
    let page = layout(
        "<p style='line-height:18px;width:80px'><span style='padding:2px 3px;border:1px solid red'><span style='padding:4px 5px;border:2px solid blue'>aa aa aa</span></span></p>",
        144,
        &mut Fixed,
    );
    assert_eq!(page.box_decorations.len(), 4);
    assert_eq!(
        page.text_boxes
            .iter()
            .map(|text| text.text.as_str())
            .collect::<Vec<_>>(),
        ["aa aa", "aa"]
    );
    for (index, text) in page.text_boxes.iter().enumerate() {
        let outer = &page.box_decorations[index * 2];
        let inner = &page.box_decorations[index * 2 + 1];
        assert_eq!(
            (outer.x, outer.width, outer.height),
            (32, text.width + 22, 24)
        );
        assert_eq!(
            (inner.x, inner.width, inner.height),
            (36, text.width + 14, 30)
        );
        assert_eq!(text.x, 43);
        assert_eq!(inner.y, outer.y - 3);
        assert!(outer.x + outer.width <= 112);
    }
    assert_eq!(page.box_decorations[2].y, page.box_decorations[0].y + 36);
}

#[test]
fn nested_image_and_empty_boxes_share_continuous_ancestor_fragments() {
    let page = layout(
        "<p style='line-height:18px'><a href=next style='padding:2px 3px;border:1px solid red'>A<span style='padding:4px 5px;border:2px solid blue'><img src=ok style='width:20px;height:12px;padding:1px;border:1px solid green'>B<span style='padding:2px;border:1px solid black'></span>C</span>D</a>Z</p>",
        300,
        &mut Fixed,
    );
    assert_eq!(page.box_decorations.len(), 4);
    let (outer, inner, own_image, empty) = (
        &page.box_decorations[0],
        &page.box_decorations[1],
        &page.box_decorations[2],
        &page.box_decorations[3],
    );
    assert_eq!((outer.x, outer.width), (32, 92));
    assert_eq!((inner.x, inner.width), (46, 64));
    assert_eq!(
        (own_image.x, own_image.width, own_image.height),
        (53, 24, 16)
    );
    assert_eq!((empty.x, empty.width), (87, 6));
    let image = &page.image_boxes[0];
    assert_eq!((image.x, image.width, image.height), (55, 20, 12));
    assert_eq!(image.href.as_deref(), Some("next"));
    assert!(own_image.y >= inner.y && own_image.y + own_image.height <= inner.y + inner.height);
    assert_eq!(page.text_boxes.last().unwrap().x, 124);
}

#[test]
fn nested_image_fitting_reserves_ancestors_without_changing_percentage_basis() {
    for (css_width, expected_width) in [("50%", 50), ("100%", 74)] {
        let html = format!(
            "<p style='width:100px'><span style='padding:2px 3px;border:1px solid red'><span style='padding:4px 5px;border:2px solid blue'><img src=ok style='width:{css_width};padding:1px;border:1px solid green'></span></span></p>"
        );
        let page = layout(&html, 300, &mut Fixed);
        assert_eq!(page.image_boxes[0].width, expected_width);
        assert_eq!(page.box_decorations[0].width, expected_width + 26);
        assert!(page.box_decorations[0].width <= 100);
    }
}

#[test]
fn deep_inline_box_stacks_accumulate_geometry_without_recursive_fragment_closing() {
    let mut html = "<p>".to_owned();
    for _ in 0..128 {
        html.push_str("<span style='padding:1px'>");
    }
    html.push('X');
    for _ in 0..128 {
        html.push_str("</span>");
    }
    html.push_str("</p>");
    let page = layout(&html, 500, &mut Fixed);
    assert_eq!(page.box_decorations.len(), 128);
    assert_eq!(page.text_boxes[0].x, 32 + 128);
    assert_eq!(page.box_decorations[0].width, 266);
    for pair in page.box_decorations.windows(2) {
        assert_eq!(pair[1].x, pair[0].x + 1);
        assert_eq!(pair[1].y, pair[0].y);
        assert_eq!(pair[1].width, pair[0].width - 2);
        assert_eq!(pair[1].height, pair[0].height);
    }
}

#[test]
fn generated_replacements_and_mixed_lists_keep_host_and_pseudo_box_stacks() {
    let document = parse_document(
        "<style>a { padding:2px;border:1px solid red } a::before { content:url(ok);width:20px;height:12px;padding:1px;border:1px solid green } a::after { content:'' url(ok) 'T';padding:3px;border:1px solid blue }</style><p><a href=next>B</a>Z</p>",
    );
    let computed =
        op_css::compute_styles(&document, &op_css::collect_author_styles(&document).styles);
    let mut generated = GeneratedImageResources::new();
    let image = Arc::new(RasterImage::from_premultiplied_bgra(2, 2, vec![255; 16]).unwrap());
    let mut nodes = vec![document.root()];
    while let Some(node) = nodes.pop() {
        for pseudo in [op_css::PseudoElement::Before, op_css::PseudoElement::After] {
            if computed.pseudo_style_for(node, pseudo).is_some() {
                generated.insert((node, pseudo, 0), image.clone());
            }
        }
        nodes.extend(document.children(node));
    }
    let page = layout_document_with_resources_and_metrics(
        &document,
        300,
        &ImageResources::new(),
        &generated,
        &computed,
        &mut Fixed,
    );
    assert_eq!(page.box_decorations.len(), 3);
    let (host, before, after) = (
        &page.box_decorations[0],
        &page.box_decorations[1],
        &page.box_decorations[2],
    );
    assert_eq!((host.x, host.width), (32, 60));
    assert_eq!((before.x, before.width), (35, 24));
    assert_eq!((after.x, after.width), (69, 20));
    assert_eq!((page.image_boxes[0].x, page.image_boxes[0].width), (37, 20));
    assert_eq!((page.image_boxes[1].x, page.image_boxes[1].width), (73, 2));
    assert_eq!(
        page.text_boxes
            .iter()
            .map(|text| (text.text.as_str(), text.x))
            .collect::<Vec<_>>(),
        [("B", 59), ("T", 75), ("Z", 92)]
    );
    assert!(
        page.image_boxes
            .iter()
            .all(|image| image.href.as_deref() == Some("next"))
    );
}

#[test]
fn unavailable_generated_replacements_keep_atomic_css_boxes_inside_ancestors() {
    let page = layout(
        "<style>#s::before { content:url(missing);width:30px;height:20px;padding:2px;border:1px solid green;background:blue }</style><p><a id=s href=next style='padding:2px;border:1px solid red'>B</a>Z</p>",
        300,
        &mut Fixed,
    );
    assert!(page.image_boxes.is_empty());
    assert_eq!(page.box_decorations.len(), 2);
    assert_eq!(
        (page.box_decorations[0].x, page.box_decorations[0].width),
        (32, 52)
    );
    assert_eq!(
        (
            page.box_decorations[1].x,
            page.box_decorations[1].width,
            page.box_decorations[1].height
        ),
        (35, 36, 26)
    );
    assert_eq!(
        page.text_boxes
            .iter()
            .map(|text| (text.text.as_str(), text.x))
            .collect::<Vec<_>>(),
        [("B", 71), ("Z", 84)]
    );
    assert_eq!(page.text_boxes[0].links[0].href, "next");
}

#[test]
fn unavailable_block_replacements_use_exact_sizes_margins_and_zero_natural_axes() {
    for content in [
        "<style>span::before { content:url(missing);display:block;width:100px;height:40px;box-sizing:border-box;padding:4px;border:2px solid red;background:blue;margin:8px auto 12px }</style><span>Tail</span>",
        "<img src=bad alt='' style='display:block;width:100px;height:40px;box-sizing:border-box;padding:4px;border:2px solid red;background:blue;margin:8px auto 12px'>Tail",
        "<img src=bad style='display:block;width:100px;height:40px;box-sizing:border-box;padding:4px;border:2px solid red;background:blue;margin:8px auto 12px'>Tail",
    ] {
        let page = layout(content, 300, &mut Fixed);
        assert!(page.image_boxes.is_empty());
        let box_style = &page.box_decorations[0];
        assert_eq!(
            (box_style.x, box_style.y, box_style.width, box_style.height),
            (100, 36, 100, 40)
        );
        assert_eq!(page.text_boxes.len(), 1);
        assert_eq!(page.text_boxes[0].text, "Tail");
        assert!(page.text_boxes[0].y >= 88);
    }
}

#[test]
fn unavailable_empty_alt_and_mixed_pseudos_keep_edges_without_fake_pixels_or_labels() {
    let page = layout(
        "<style>#s::before { content:'' url(missing);padding:3px;border:1px solid blue;width:100px }</style><p><img src=bad alt='' style='width:30px;padding:2px;border:1px solid red'><img src=bad alt='' style='height:10px;border:2px solid green'><span id=s></span>Z</p>",
        300,
        &mut Fixed,
    );
    assert!(page.image_boxes.is_empty());
    assert_eq!(page.box_decorations.len(), 3);
    assert_eq!(
        page.box_decorations
            .iter()
            .map(|box_style| (box_style.x, box_style.width))
            .collect::<Vec<_>>(),
        [(32, 36), (68, 4), (72, 8)]
    );
    assert_eq!(
        (
            page.box_decorations[0].height,
            page.box_decorations[1].height
        ),
        (6, 14)
    );
    assert_eq!(page.text_boxes.len(), 1);
    assert_eq!(
        (page.text_boxes[0].text.as_str(), page.text_boxes[0].x),
        ("Z", 80)
    );
}

#[test]
fn failed_alt_text_has_own_inline_or_block_style_without_replaced_inline_dimensions() {
    let page = layout(
        "<p><span style='padding:2px;border:1px solid red'><img src=bad alt='XY' style='width:100px;height:50px;padding:3px;border:1px solid blue'></span>Z</p>",
        300,
        &mut Fixed,
    );
    assert_eq!(page.box_decorations.len(), 2);
    assert_eq!(
        (page.box_decorations[0].x, page.box_decorations[0].width),
        (32, 34)
    );
    assert_eq!(
        (page.box_decorations[1].x, page.box_decorations[1].width),
        (35, 28)
    );
    assert_eq!(
        page.text_boxes
            .iter()
            .map(|text| (text.text.as_str(), text.x))
            .collect::<Vec<_>>(),
        [("XY", 39), ("Z", 66)]
    );
    let page = layout(
        "<img src=bad alt='XY' style='display:block;width:100px;height:40px;box-sizing:border-box;padding:4px;border:2px solid red;background:blue;margin:8px auto 12px'>Tail",
        300,
        &mut Fixed,
    );
    let box_style = &page.box_decorations[0];
    assert_eq!(
        (box_style.x, box_style.y, box_style.width, box_style.height),
        (100, 36, 100, 40)
    );
    assert_eq!(page.text_boxes[0].x, 106);
    assert!(page.text_boxes[1].y >= 88);
}

#[test]
fn unavailable_inline_replacements_wrap_atomically_and_obey_nowrap() {
    for (white_space, expected_x, wraps) in [("normal", 32, true), ("nowrap", 82, false)] {
        let html = format!(
            "<style>span::before {{ content:url(missing);width:30px;height:12px;padding:1px;border:1px solid blue }}</style><p style='width:60px;white-space:{white_space}'>1234 <span></span>Z</p>"
        );
        let page = layout(&html, 300, &mut Fixed);
        assert!(page.image_boxes.is_empty());
        assert_eq!(page.box_decorations.len(), 1);
        let box_style = &page.box_decorations[0];
        assert_eq!(
            (box_style.x, box_style.width, box_style.height),
            (expected_x, 34, 16)
        );
        assert_eq!(page.text_boxes[1].x, expected_x + 34);
        assert_eq!(page.text_boxes[1].y > page.text_boxes[0].y, wraps);
    }
}

#[test]
fn background_only_empty_inlines_have_zero_geometry_and_do_not_split_spaces() {
    let page = layout(
        "<p>A <span style='background:red'></span> B</p><p><span style='background:blue'></span>Tail</p>",
        300,
        &mut Fixed,
    );
    assert!(page.box_decorations.is_empty());
    assert_eq!(
        page.text_boxes
            .iter()
            .map(|text| text.text.as_str())
            .collect::<String>(),
        "A BTail"
    );

    let reference = layout("<p>A B</p><p>Tail</p>", 300, &mut Fixed);
    assert_eq!(page.text_boxes.len(), reference.text_boxes.len());
    for (actual, expected) in page.text_boxes.iter().zip(&reference.text_boxes) {
        assert_eq!(
            (
                actual.x,
                actual.y,
                actual.width,
                actual.height,
                actual.text.as_str()
            ),
            (
                expected.x,
                expected.y,
                expected.width,
                expected.height,
                expected.text.as_str()
            )
        );
    }
}

#[test]
fn background_only_empty_generated_inline_has_zero_geometry() {
    let page = layout(
        "<style>#host::before { content:''; background:red }</style><p id=host>Body</p>",
        300,
        &mut Fixed,
    );
    let reference = layout("<p>Body</p>", 300, &mut Fixed);
    assert!(page.box_decorations.is_empty());
    assert_eq!(page.text_boxes.len(), reference.text_boxes.len());
    assert_eq!(
        (
            page.text_boxes[0].x,
            page.text_boxes[0].y,
            page.text_boxes[0].text.as_str()
        ),
        (
            reference.text_boxes[0].x,
            reference.text_boxes[0].y,
            reference.text_boxes[0].text.as_str()
        )
    );
}

#[test]
fn visually_empty_inline_descendants_keep_own_frames_without_fake_text() {
    for descendants in [
        "<em style='display:none'>hidden</em>",
        "<em></em>",
        " \n\t ",
        "<span></span><em style='display:none'>hidden</em>",
        "<style>ignored</style>",
    ] {
        let html = format!(
            "<p style='line-height:18px;width:100px;text-align:center'><span style='padding:2px;border:1px solid red;background:blue'>{descendants}</span>Z</p>"
        );
        let page = layout(&html, 300, &mut Fixed);
        assert_eq!(page.box_decorations.len(), 1, "{descendants}");
        let box_style = &page.box_decorations[0];
        assert_eq!(
            (box_style.x, box_style.width, box_style.height),
            (74, 6, 24),
            "{descendants}"
        );
        assert_eq!(page.text_boxes.len(), 1, "{descendants}");
        assert_eq!(
            (page.text_boxes[0].text.as_str(), page.text_boxes[0].x),
            ("Z", 80),
            "{descendants}"
        );
    }
}

#[test]
fn nested_empty_descendants_do_not_duplicate_ancestors_and_split_at_block_boundaries() {
    let page = layout(
        "<p><span style='padding:2px;border:1px solid red'><span style='padding:1px;border:1px solid blue'><em style='display:none'>hidden</em></span></span>Z</p>",
        300,
        &mut Fixed,
    );
    assert_eq!(page.box_decorations.len(), 2);
    assert_eq!(
        (page.box_decorations[0].x, page.box_decorations[0].width),
        (32, 10)
    );
    assert_eq!(
        (page.box_decorations[1].x, page.box_decorations[1].width),
        (35, 4)
    );
    assert_eq!(page.text_boxes.len(), 1);
    assert_eq!(page.text_boxes[0].x, 42);
    let split = layout(
        "<div><span style='padding:2px;border:1px solid red'><div style='height:0'></div></span>Z</div>",
        300,
        &mut Fixed,
    );
    let reference = layout(
        "<div><span style='padding:2px;padding-right:0;border:1px solid red;border-right-width:0'></span><div style='height:0'></div><span style='padding:2px;padding-left:0;border:1px solid red;border-left-width:0'></span>Z</div>",
        300,
        &mut Fixed,
    );
    assert_eq!(split.box_decorations, reference.box_decorations);
    assert_eq!(split.text_boxes, reference.text_boxes);
    let page = layout(
        "<p><span style='padding:2px;border:1px solid red;white-space:pre'>  </span>Z</p>",
        300,
        &mut Fixed,
    );
    assert_eq!(page.box_decorations.len(), 1);
    assert_eq!(page.box_decorations[0].width, 26);
    assert_eq!(page.text_boxes[0].text, "  ");
}

#[test]
fn shares_baseline_and_exact_horizontal_extents_between_text_image_and_unicode_link() {
    let page = layout(
        "<p>Before <a href='/next'><img src=ok width=40 height=32> Привет 😀</a> after</p>",
        800,
        &mut Fixed,
    );
    assert_eq!(page.image_boxes.len(), 1);
    assert_eq!(page.text_boxes.len(), 3);
    let image = &page.image_boxes[0];
    let (before, linked, after) = (
        &page.text_boxes[0],
        &page.text_boxes[1],
        &page.text_boxes[2],
    );
    assert_eq!(before.text, "Before ");
    assert_eq!(image.x, before.x + before.width);
    assert_eq!(linked.x, image.x + image.width);
    assert_eq!(after.x, linked.x + linked.width);
    assert_eq!(image.y + image.height, before.y + 14);
    assert_eq!(after.y, before.y);
    assert_eq!(linked.y, before.y);
    assert_eq!(image.href.as_deref(), Some("/next"));
    assert_eq!(
        &linked.text[linked.links[0].start..linked.links[0].end],
        " Привет 😀"
    );
    assert_eq!(linked.links[0].href, "/next");
    assert!(linked.decoration.underline);
    assert!(!after.decoration.underline);
    assert!(after.links.is_empty());
    assert!(after.x + after.width <= 768);
}

#[test]
fn wraps_after_a_filled_mixed_line_without_leading_spaces_or_overlap() {
    let page = layout(
        "<p>one <img src=ok width=120 height=20> two</p>",
        240,
        &mut Fixed,
    );
    assert_eq!(page.image_boxes[0].x, 72);
    assert_eq!(page.text_boxes[0].text, "one ");
    assert_eq!(page.text_boxes[1].text, "two");
    assert_eq!(page.text_boxes[1].x, 32);
    assert!(page.text_boxes[1].y >= page.image_boxes[0].y + 20);
}

#[test]
fn groups_inline_siblings_and_honors_block_and_repeated_br_boundaries() {
    let page = layout(
        "<div>one<span> two</span><br><br>three<p>block</p>tail</div>",
        800,
        &mut Fixed,
    );
    assert_eq!(
        page.text_boxes
            .iter()
            .map(|b| b.text.as_str())
            .collect::<Vec<_>>(),
        ["one two", "three", "block", "tail"]
    );
    assert_eq!(page.text_boxes[1].y - page.text_boxes[0].y, 48);
    assert!(page.text_boxes[2].y > page.text_boxes[1].y);
    assert!(page.text_boxes[3].y >= page.text_boxes[2].y + page.text_boxes[2].height);
}

#[test]
fn collapses_only_html_spaces_and_retains_nonbreaking_unicode_spacing() {
    let page = layout(
        "<p> \tlongword&nbsp;<span>longword</span>  \n tail\r </p>",
        240,
        &mut Fixed,
    );
    assert_eq!(page.text_boxes[0].text, "longword\u{a0}longword");
    assert_eq!(page.text_boxes[1].text, "tail");
    let page = layout("<p>x&ThickSpace;y</p>", 800, &mut Fixed);
    assert_eq!(page.text_boxes[0].text, "x\u{205f}\u{200a}y");
}

#[test]
fn aligns_adjacent_images_and_wraps_them_as_atomic_boxes() {
    let page = layout(
        "<p><img src=ok width=80 height=16><img src=ok width=80 height=32><img src=ok width=80 height=24></p>",
        240,
        &mut Fixed,
    );
    assert!(page.text_boxes.is_empty());
    let images = &page.image_boxes;
    assert_eq!(images[1].x, images[0].x + 80);
    assert_eq!(images[0].y + 16, images[1].y + 32);
    assert_eq!(images[2].x, 32);
    assert!(images[2].y >= images[1].y + 32);
}

#[test]
fn split_inline_whitespace_between_blocks_matches_explicit_fragments() {
    let split = layout(
        "<style>body>span{border:3px solid blue}</style><body><span><div>One</div><div>Two</div></span></body>",
        800,
        &mut Fixed,
    );
    let reference = layout(
        "<style>body>span{border:3px solid blue}.notstart{border-left-width:0}.notend{border-right-width:0}</style><body><span class=notend></span><div>One</div><span class='notstart notend'></span><div>Two</div><span class=notstart></span></body>",
        800,
        &mut Fixed,
    );
    assert_eq!(split.box_decorations, reference.box_decorations);
    assert_eq!(split.text_boxes, reference.text_boxes);
}

#[test]
fn preserves_linked_alt_text_and_skips_zero_sized_images_inside_a_line() {
    let page = layout(
        "<p>a<a href='/x'><img src=bad alt='Русский'></a><img src=ok width=0>z</p>",
        800,
        &mut Fixed,
    );
    assert!(page.image_boxes.is_empty());
    assert_eq!(
        page.text_boxes
            .iter()
            .map(|text| text.text.as_str())
            .collect::<Vec<_>>(),
        ["a", "Русский", "z"]
    );
    let text = &page.text_boxes[1];
    assert_eq!(
        &text.text[text.links[0].start..text.links[0].end],
        "Русский"
    );
    assert!(text.decoration.underline);
    assert_eq!(text.x, page.text_boxes[0].x + page.text_boxes[0].width);
    assert_eq!(page.text_boxes[2].x, text.x + text.width);
}

#[test]
fn splits_long_unicode_words_without_quadratic_suffix_measurement() {
    struct Counting {
        chars: usize,
    }
    impl TextMeasurer for Counting {
        fn measure(&mut self, text: &str, _: i32, _: FontWeight, _: FontStyle) -> TextMetrics {
            self.chars += text.chars().count();
            Fixed.measure(text, 18, FontWeight::Normal, FontStyle::Normal)
        }
    }
    let text = "я😀".repeat(2500);
    let mut measurer = Counting { chars: 0 };
    let page = layout(&format!("<a href='/x'>{text}</a>"), 240, &mut measurer);
    assert_eq!(
        page.text_boxes
            .iter()
            .map(|b| b.text.as_str())
            .collect::<String>(),
        text
    );
    assert!(
        measurer.chars < 5000 * 60,
        "measured {} chars",
        measurer.chars
    );
    for line in page.text_boxes {
        assert!(line.width <= 176);
        assert_eq!(line.links.len(), 1);
        assert_eq!(
            &line.text[line.links[0].start..line.links[0].end],
            line.text
        );
    }
}
