//! Font extents only; original op_layout owns all line breaking and positioning.

#[cfg(not(windows))]
pub(super) struct Measurer;
#[cfg(not(windows))]
impl Measurer {
    pub(super) fn new() -> Self {
        Self
    }
}

#[cfg(not(windows))]
impl op_layout::TextMeasurer for Measurer {
    fn measure(
        &mut self,
        text: &str,
        size: i32,
        weight: op_layout::FontWeight,
    ) -> op_layout::TextMetrics {
        op_layout::ApproximateTextMeasurer.measure(text, size, weight)
    }
}

#[cfg(windows)]
pub(super) use native::Measurer;

#[cfg(windows)]
mod native {
    use op_layout::{ApproximateTextMeasurer, FontWeight, TextMeasurer, TextMetrics};
    use std::collections::HashMap;
    use std::mem::zeroed;
    use std::ptr::null_mut;
    use windows_sys::Win32::Foundation::SIZE;
    use windows_sys::Win32::Graphics::Gdi::*;

    struct Font {
        dc: HDC,
        font: HFONT,
        previous: HGDIOBJ,
        ascent: i32,
        descent: i32,
    }

    impl Font {
        fn new(size: i32, bold: bool) -> Option<Self> {
            let guard = op_paint::GDI_TEXT_LOCK
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let dc = unsafe { CreateCompatibleDC(null_mut()) };
            if dc.is_null() {
                return None;
            }
            let family: Vec<u16> = op_paint::TEXT_FONT_FAMILY
                .encode_utf16()
                .chain(Some(0))
                .collect();
            let font = unsafe {
                CreateFontW(
                    -size,
                    0,
                    0,
                    0,
                    if bold { FW_BOLD } else { FW_NORMAL } as i32,
                    0,
                    0,
                    0,
                    DEFAULT_CHARSET as u32,
                    OUT_DEFAULT_PRECIS as u32,
                    CLIP_DEFAULT_PRECIS as u32,
                    CLEARTYPE_QUALITY as u32,
                    (DEFAULT_PITCH | FF_DONTCARE) as u32,
                    family.as_ptr(),
                )
            };
            if font.is_null() {
                unsafe {
                    DeleteDC(dc);
                }
                return None;
            }
            let previous = unsafe { SelectObject(dc, font) };
            if previous.is_null() || previous as isize == -1 {
                unsafe {
                    DeleteObject(font);
                    DeleteDC(dc);
                }
                return None;
            }
            let mut metrics: TEXTMETRICW = unsafe { zeroed() };
            let measured = unsafe { GetTextMetricsW(dc, &mut metrics) };
            let result = Self {
                dc,
                font,
                previous,
                ascent: metrics.tmAscent,
                descent: metrics.tmDescent,
            };
            drop(guard);
            if measured == 0 { None } else { Some(result) }
        }
    }

    impl Drop for Font {
        fn drop(&mut self) {
            let _guard = op_paint::GDI_TEXT_LOCK
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            unsafe {
                SelectObject(self.dc, self.previous);
                DeleteObject(self.font);
                DeleteDC(self.dc);
            }
        }
    }

    pub(crate) struct Measurer {
        fonts: HashMap<(i32, bool), Option<Font>>,
    }
    impl Measurer {
        pub(crate) fn new() -> Self {
            Self {
                fonts: HashMap::new(),
            }
        }
    }

    impl TextMeasurer for Measurer {
        fn measure(&mut self, text: &str, size: i32, weight: FontWeight) -> TextMetrics {
            let bold = weight == FontWeight::Bold;
            let font = self
                .fonts
                .entry((size, bold))
                .or_insert_with(|| Font::new(size, bold));
            if let Some(font) = font {
                let utf16: Vec<u16> = text.encode_utf16().collect();
                let _guard = op_paint::GDI_TEXT_LOCK
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                let mut extent: SIZE = unsafe { zeroed() };
                if utf16.is_empty()
                    || unsafe {
                        GetTextExtentPoint32W(
                            font.dc,
                            utf16.as_ptr(),
                            utf16.len() as i32,
                            &mut extent,
                        )
                    } != 0
                {
                    return TextMetrics {
                        width: extent.cx,
                        ascent: font.ascent,
                        descent: font.descent,
                    };
                }
            }
            ApproximateTextMeasurer.measure(text, size, weight)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn measures_unicode_variable_width_and_reuses_worker_fonts() {
            let mut measurer = Measurer::new();
            let narrow = measurer.measure("iiii", 18, FontWeight::Normal);
            let wide = measurer.measure("WWWW", 18, FontWeight::Normal);
            assert!(wide.width > narrow.width);
            let unicode = measurer.measure("Привет 😀", 18, FontWeight::Normal);
            assert!(unicode.width > 0 && unicode.ascent > 0 && unicode.descent >= 0);
            assert_eq!(measurer.fonts.len(), 1);
            assert!(measurer.fonts.values().all(Option::is_some));
        }

        #[test]
        fn concurrent_font_realization_and_measurement_keep_layout_stable() {
            let html = "<h1>OPBrowser</h1><p>Windows-1251: Ё ё №</p><p>Привет 😀</p>";
            let expected = crate::Engine::new().render_html(html, 800, 600);
            std::thread::scope(|scope| {
                for _ in 0..8 {
                    let expected = &expected;
                    scope.spawn(move || {
                        for _ in 0..16 {
                            assert_eq!(crate::Engine::new().render_html(html, 800, 600), *expected);
                        }
                    });
                }
            });
        }

        #[test]
        fn mixed_display_order_and_image_positions_match_measured_text_and_baseline() {
            use op_paint::PaintCommand;
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples/images/inline.html");
            let page = crate::Engine::new()
                .render_source(path.to_str().unwrap(), 1200, 700)
                .unwrap();
            let commands = &page.display_list.commands;
            let index = commands
                .iter()
                .position(|c| matches!(c, PaintCommand::Image { href: Some(_), .. }))
                .unwrap();
            let PaintCommand::Text {
                x,
                y,
                text,
                font_size,
                ..
            } = &commands[index - 1]
            else {
                panic!("text must precede the inline image");
            };
            assert_eq!(text, "До картинки ");
            let measured = Measurer::new().measure(text, *font_size, FontWeight::Normal);
            let PaintCommand::Image {
                x: image_x,
                y: image_y,
                width,
                height,
                ..
            } = &commands[index]
            else {
                unreachable!()
            };
            assert_eq!(*image_x, x + measured.width);
            assert_eq!(image_y + height, y + measured.ascent);
            let PaintCommand::Text {
                x: next_x,
                y: next_y,
                ..
            } = &commands[index + 1]
            else {
                panic!("text must follow the image on the same line");
            };
            assert_eq!(*next_x, image_x + width);
            assert_eq!(next_y, y);
        }
    }
}
