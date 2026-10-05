//! GDI surface lifetime and raster drawing only; no loading, decoding or layout.

use op_paint::PaintCommand;
use std::ffi::c_void;
use std::mem::{size_of, zeroed};
use std::ptr::{copy_nonoverlapping, null_mut};
use windows_sys::Win32::Graphics::Gdi::*;

struct Surface {
    dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    bits: *mut u8,
}

impl Surface {
    fn new(target: HDC, width: i32, height: i32) -> Option<Self> {
        if width <= 0 || height <= 0 {
            return None;
        }
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
        })
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
