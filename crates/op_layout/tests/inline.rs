use op_html::parse_document;
use op_image::RasterImage;
use op_layout::{
    FontStyle, FontWeight, ImageResources, LayoutTree, TextMeasurer, TextMetrics,
    layout_document_with_metrics,
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
    layout_document_with_metrics(&document, width, &images, measurer)
}

#[test]
fn shares_baseline_and_exact_horizontal_extents_between_text_image_and_unicode_link() {
    let page = layout(
        "<p>Before <a href='/next'><img src=ok width=40 height=32> Привет 😀</a> after</p>",
        800,
        &mut Fixed,
    );
    assert_eq!(page.image_boxes.len(), 1);
    assert_eq!(page.text_boxes.len(), 2);
    let image = &page.image_boxes[0];
    let (before, after) = (&page.text_boxes[0], &page.text_boxes[1]);
    assert_eq!(before.text, "Before ");
    assert_eq!(image.x, before.x + before.width);
    assert_eq!(after.x, image.x + image.width);
    assert_eq!(image.y + image.height, before.y + 14);
    assert_eq!(after.y, before.y);
    assert_eq!(image.href.as_deref(), Some("/next"));
    assert_eq!(
        &after.text[after.links[0].start..after.links[0].end],
        " Привет 😀"
    );
    assert_eq!(after.links[0].href, "/next");
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
fn preserves_linked_alt_text_and_skips_zero_sized_images_inside_a_line() {
    let page = layout(
        "<p>a<a href='/x'><img src=bad alt='Русский'></a><img src=ok width=0>z</p>",
        800,
        &mut Fixed,
    );
    assert!(page.image_boxes.is_empty());
    assert_eq!(page.text_boxes[0].text, "aРусскийz");
    let text = &page.text_boxes[0];
    assert_eq!(
        &text.text[text.links[0].start..text.links[0].end],
        "Русский"
    );
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
