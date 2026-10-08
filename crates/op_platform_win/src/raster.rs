//! GDI surface lifetime and raster drawing only; no loading, decoding or layout.

use op_paint::PaintCommand;
use std::ffi::c_void;
use std::mem::{size_of, zeroed};
use std::ptr::{copy_nonoverlapping, null_mut};
use windows_sys::Win32::Graphics::Gdi::*;

pub(super) struct Surface {
    pub(super) dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    pub(super) bits: *mut u8,
    byte_len: usize,
}

impl Surface {
    pub(super) fn new(target: HDC, width: i32, height: i32) -> Option<Self> {
        if width <= 0 || height <= 0 {
            return None;
        }
        let byte_len = (width as usize)
            .checked_mul(height as usize)?
            .checked_mul(4)?;
        let dc = unsafe { CreateCompatibleDC(target) };
        if dc.is_null() {
            return None;
        }
        let mut info: BITMAPINFO = unsafe { zeroed() };
        info.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
        info.bmiHeader.biWidth = width;
        info.bmiHeader.biHeight = -height;
        info.bmiHeader.biPlanes = 1;
        info.bmiHeader.biBitCount = 32;
        info.bmiHeader.biCompression = BI_RGB;
        let mut bits = null_mut();
        let bitmap =
            unsafe { CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut bits, null_mut(), 0) };
        if bitmap.is_null() || bits.is_null() {
            unsafe {
                if !bitmap.is_null() {
                    DeleteObject(bitmap);
                }
                DeleteDC(dc);
            }
            return None;
        }
        let previous = unsafe { SelectObject(dc, bitmap) };
        if previous.is_null() || previous as isize == -1 {
            unsafe {
                DeleteObject(bitmap);
                DeleteDC(dc);
            }
            return None;
        }
        Some(Self {
            dc,
            bitmap,
            previous,
            bits: bits.cast(),
            byte_len,
        })
    }

    pub(super) fn clear_white(&self) {
        unsafe {
            std::ptr::write_bytes(self.bits, 255, self.byte_len);
        }
    }

    pub(super) fn clear_black(&self) {
        unsafe { std::ptr::write_bytes(self.bits, 0, self.byte_len) };
    }

    pub(super) fn pixels(&self) -> Vec<u8> {
        unsafe { std::slice::from_raw_parts(self.bits, self.byte_len) }.to_vec()
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.previous);
            DeleteObject(self.bitmap);
            DeleteDC(self.dc);
        }
    }
}

pub(super) fn paint(target: *mut c_void, command: &PaintCommand) -> bool {
    let PaintCommand::Image {
        x,
        y,
        width,
        height,
        image,
        ..
    } = command
    else {
        return false;
    };
    if *width <= 0 || *height <= 0 {
        return false;
    }
    let bounds = windows_sys::Win32::Foundation::RECT {
        left: *x,
        top: *y,
        right: *x + *width,
        bottom: *y + *height,
    };
    if unsafe { RectVisible(target, &bounds) } == 0 {
        return false;
    }
    let Some(surface) = Surface::new(target, image.width() as i32, image.height() as i32) else {
        return false;
    };
    // RasterImage validates exact BGRA length; this DIB allocates the same storage.
    // The source bitmap is selected only into its own DC and drops after drawing.
    unsafe {
        copy_nonoverlapping(image.pixels().as_ptr(), surface.bits, image.pixels().len());
        let result = AlphaBlend(
            target,
            *x,
            *y,
            *width,
            *height,
            surface.dc,
            0,
            0,
            image.width() as i32,
            image.height() as i32,
            BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            },
        );
        GdiFlush();
        result != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_paint::RasterImage;
    use std::sync::Arc;

    #[test]
    fn linked_text_uses_computed_color_and_optional_underline_in_gdi() {
        for underline in [false, true] {
            let target = Surface::new(null_mut(), 160, 48).unwrap();
            unsafe {
                std::ptr::write_bytes(target.bits, 255, 160 * 48 * 4);
            }
            let command = PaintCommand::Text {
                x: 4,
                y: 4,
                text: "link".into(),
                font_size: 18,
                bold: false,
                italic: false,
                underline,
                line_through: false,
                letter_spacing: 2,
                word_spacing: 0,
                color: op_paint::Color { r: 255, g: 0, b: 0 },
                links: vec![op_paint::LinkSpan {
                    start: 0,
                    end: 4,
                    href: "next.html".into(),
                }],
            };
            let mut regions = Vec::new();
            crate::paint_command(target.dc, &command, &mut regions);
            unsafe {
                GdiFlush();
            }
            assert_eq!(regions.len(), 1);
            assert_eq!(regions[0].href, "next.html");
            assert!(regions[0].bounds.right > regions[0].bounds.left);
            assert_eq!(
                unsafe { GetPixel(target.dc, 6, 24) },
                if underline { 0x0000ff } else { 0xffffff }
            );
            assert!((4..22).any(|y| (4..regions[0].bounds.right).any(|x| {
                let pixel = unsafe { GetPixel(target.dc, x, y) };
                pixel & 255 > 200 && (pixel >> 8) & 255 < 100 && (pixel >> 16) & 255 < 100
            })));
        }
    }

    #[test]
    fn opacity_layer_composites_overlapping_rectangles_once() {
        let target = Surface::new(null_mut(), 180, 180).unwrap();
        target.clear_white();
        let yellow = op_paint::Color {
            r: 255,
            g: 255,
            b: 0,
        };
        let commands = [
            PaintCommand::BeginLayer {
                opacity: 128,
                invert: 0,
            },
            PaintCommand::FillRect {
                x: 0,
                y: 0,
                width: 100,
                height: 150,
                color: yellow,
            },
            PaintCommand::FillRect {
                x: 50,
                y: 0,
                width: 100,
                height: 150,
                color: yellow,
            },
            PaintCommand::EndLayer,
        ];
        crate::paint_commands(target.dc, &commands, &mut Vec::new());
        unsafe {
            GdiFlush();
        }
        let first = unsafe { GetPixel(target.dc, 25, 70) };
        let overlap = unsafe { GetPixel(target.dc, 75, 70) };
        let second = unsafe { GetPixel(target.dc, 125, 70) };
        assert_eq!(first, overlap, "overlap must not apply parent alpha twice");
        assert_eq!(first, second);
        assert!((127..=128).contains(&((first >> 16) & 255)));
        assert_eq!(unsafe { GetPixel(target.dc, 175, 70) }, 0xffffff);
    }

    #[test]
    fn nested_invert_filter_and_opacity_make_uniform_light_blue() {
        let target = Surface::new(null_mut(), 180, 180).unwrap();
        target.clear_white();
        let yellow = op_paint::Color {
            r: 255,
            g: 255,
            b: 0,
        };
        let commands = [
            PaintCommand::BeginLayer {
                opacity: 128,
                invert: 0,
            },
            PaintCommand::BeginLayer {
                opacity: 255,
                invert: 255,
            },
            PaintCommand::FillRect {
                x: 0,
                y: 0,
                width: 100,
                height: 150,
                color: yellow,
            },
            PaintCommand::EndLayer,
            PaintCommand::BeginLayer {
                opacity: 255,
                invert: 255,
            },
            PaintCommand::FillRect {
                x: 50,
                y: 0,
                width: 100,
                height: 150,
                color: yellow,
            },
            PaintCommand::EndLayer,
            PaintCommand::EndLayer,
        ];
        crate::paint_commands(target.dc, &commands, &mut Vec::new());
        unsafe {
            GdiFlush();
        }
        let rgb = |x| {
            let pixel = unsafe { GetPixel(target.dc, x, 75) };
            (pixel & 255, (pixel >> 8) & 255, (pixel >> 16) & 255)
        };
        assert_eq!(rgb(25), rgb(75), "alpha must apply to filtered group once");
        assert_eq!(rgb(75), rgb(125));
        let (red, green, blue) = rgb(75);
        assert!((127..=129).contains(&red), "{red}");
        assert!((127..=129).contains(&green), "{green}");
        assert_eq!(blue, 255);
    }

    #[test]
    fn paints_scaled_color_and_alpha_pixels_into_a_real_gdi_surface() {
        let target = Surface::new(null_mut(), 4, 4).unwrap();
        unsafe {
            std::ptr::write_bytes(target.bits, 255, 4 * 4 * 4);
        }
        let image = RasterImage::from_premultiplied_bgra(
            2,
            2,
            vec![0, 0, 255, 255, 0, 255, 0, 255, 128, 0, 0, 128, 0, 0, 0, 0],
        )
        .unwrap();
        let command = PaintCommand::Image {
            x: 0,
            y: 0,
            width: 4,
            height: 4,
            image: Arc::new(image),
            href: None,
        };
        assert!(paint(target.dc, &command));
        assert_eq!(unsafe { GetPixel(target.dc, 0, 0) }, 0x0000ff);
        assert_eq!(unsafe { GetPixel(target.dc, 3, 0) }, 0x00ff00);
        let blended = unsafe { GetPixel(target.dc, 0, 3) };
        assert!((127..=128).contains(&(blended & 255)));
        assert!((127..=128).contains(&((blended >> 8) & 255)));
        assert_eq!((blended >> 16) & 255, 255);
        assert_eq!(unsafe { GetPixel(target.dc, 3, 3) }, 0xffffff);
    }
}
