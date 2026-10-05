//! Bounded raster buffers; Windows WIC is used only as an infrastructure codec.

use std::fmt;

pub const MAX_ENCODED_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_PIXELS: u32 = 4 * 1024 * 1024;
pub const MAX_DIMENSION: u32 = 4096;

/// Top-down premultiplied BGRA, ready for GDI AlphaBlend. Fields stay validated.
#[derive(Debug, PartialEq, Eq)]
pub struct RasterImage {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl RasterImage {
    pub fn from_premultiplied_bgra(
        width: u32,
        height: u32,
        pixels: Vec<u8>,
    ) -> Result<Self, ImageError> {
        let length = pixel_length(width, height, usize::MAX)?;
        if pixels.len() != length
            || pixels
                .as_chunks::<4>()
                .0
                .iter()
                .any(|p| p[..3].iter().any(|c| *c > p[3]))
        {
            return Err(ImageError::InvalidPixels);
        }
        Ok(Self {
            width,
            height,
            pixels,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
    }
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ImageError {
    UnsupportedFormat,
    TooLarge,
    InvalidPixels,
    Decode(String),
}

impl fmt::Display for ImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedFormat => write!(f, "unsupported raster format"),
            Self::TooLarge => write!(f, "image exceeds the byte/pixel budget"),
            Self::InvalidPixels => write!(f, "invalid premultiplied raster pixels"),
            Self::Decode(message) => write!(f, "image decode failed: {message}"),
        }
    }
}
impl std::error::Error for ImageError {}

fn pixel_length(width: u32, height: u32, budget: usize) -> Result<usize, ImageError> {
    let pixels = width.checked_mul(height).ok_or(ImageError::TooLarge)?;
    if width == 0
        || height == 0
        || width > MAX_DIMENSION
        || height > MAX_DIMENSION
        || pixels > MAX_PIXELS
    {
        return Err(ImageError::TooLarge);
    }
    let length = pixels as usize * 4;
    if length > budget {
        return Err(ImageError::TooLarge);
    }
    Ok(length)
}

pub fn decode(bytes: &[u8], pixel_budget: usize) -> Result<RasterImage, ImageError> {
    if bytes.len() > MAX_ENCODED_BYTES {
        return Err(ImageError::TooLarge);
    }
    #[cfg(windows)]
    {
        wic::decode(bytes, pixel_budget)
    }
    #[cfg(not(windows))]
    {
        let _ = pixel_budget;
        Err(ImageError::UnsupportedFormat)
    }
}

#[cfg(windows)]
mod wic {
    use super::*;
    use windows::Win32::Graphics::Imaging::*;
    use windows::Win32::System::Com::*;

    struct Apartment;
    impl Drop for Apartment {
        fn drop(&mut self) {
            unsafe { CoUninitialize() };
        }
    }

    pub(super) fn decode(bytes: &[u8], pixel_budget: usize) -> Result<RasterImage, ImageError> {
        let container = if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            GUID_ContainerFormatPng
        } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
            GUID_ContainerFormatJpeg
        } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
            GUID_ContainerFormatGif
        } else if bytes.starts_with(b"BM") {
            GUID_ContainerFormatBmp
        } else {
            return Err(ImageError::UnsupportedFormat);
        };
        let operation = || -> windows::core::Result<RasterImage> {
            // Balance each successful COM initialization, including S_FALSE.
            unsafe {
                CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
            }
            let _apartment = Apartment;
            // COM objects are local to this call and drop before the apartment.
            // Input memory remains alive and unmodified until the decoder drops.
            unsafe {
                let factory: IWICImagingFactory =
                    CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
                let stream = factory.CreateStream()?;
                stream.InitializeFromMemory(bytes)?;
                // Explicit formats and Microsoft vendor avoid arbitrary codec discovery.
                let decoder = factory.CreateDecoder(&container, &GUID_VendorMicrosoft)?;
                decoder.Initialize(&stream, WICDecodeMetadataCacheOnDemand)?;
                let frame = decoder.GetFrame(0)?;
                let (mut width, mut height) = (0, 0);
                frame.GetSize(&mut width, &mut height)?;
                // Size/budget validation happens before allocating or copying pixels.
                let length = match pixel_length(width, height, pixel_budget) {
                    Ok(length) => length,
                    Err(_) => {
                        return Err(windows::core::Error::from_hresult(windows::core::HRESULT(
                            0x8007000e_u32 as i32,
                        )));
                    }
                };
                let converter = factory.CreateFormatConverter()?;
                converter.Initialize(
                    &frame,
                    &GUID_WICPixelFormat32bppPBGRA,
                    WICBitmapDitherTypeNone,
                    None,
                    0.0,
                    WICBitmapPaletteTypeCustom,
                )?;
                let mut pixels = vec![0; length];
                converter.CopyPixels(std::ptr::null(), width * 4, &mut pixels)?;
                Ok(RasterImage {
                    width,
                    height,
                    pixels,
                })
            }
        };
        operation().map_err(|error| {
            if error.code().0 == 0x8007000e_u32 as i32 {
                ImageError::TooLarge
            } else {
                ImageError::Decode(error.to_string())
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(windows)]
    fn decodes_builtin_formats_and_premultiplied_png_without_unbounded_pixels() {
        let png = include_bytes!("../../../examples/images/colors.png");
        let image = decode(png, 16).unwrap();
        assert_eq!((image.width(), image.height()), (2, 2));
        assert_eq!(
            image.pixels(),
            [0, 0, 255, 255, 0, 255, 0, 255, 128, 0, 0, 128, 0, 0, 0, 0]
        );
        for bytes in [
            include_bytes!("../../../examples/images/colors.jpg").as_slice(),
            include_bytes!("../../../examples/images/colors.gif").as_slice(),
            include_bytes!("../../../examples/images/colors.bmp").as_slice(),
        ] {
            let image = decode(bytes, 16).unwrap();
            assert_eq!((image.width(), image.height()), (2, 2));
            assert_eq!(image.pixels().len(), 16);
        }
        assert_eq!(decode(png, 15), Err(ImageError::TooLarge));
        assert_eq!(
            decode(
                include_bytes!("../../../examples/images/oversized.png"),
                usize::MAX
            ),
            Err(ImageError::TooLarge)
        );
        assert!(decode(&png[..16], 100).is_err());
        assert_eq!(
            decode(b"<svg></svg>", 100),
            Err(ImageError::UnsupportedFormat)
        );
    }

    #[test]
    fn rejects_bad_raster_shapes_and_premultiplication() {
        assert_eq!(
            pixel_length(4096, 4096, usize::MAX),
            Err(ImageError::TooLarge)
        );
        assert_eq!(pixel_length(1, 1, 3), Err(ImageError::TooLarge));
        assert!(RasterImage::from_premultiplied_bgra(0, 1, vec![]).is_err());
        assert!(RasterImage::from_premultiplied_bgra(1, 1, vec![0; 3]).is_err());
        assert!(RasterImage::from_premultiplied_bgra(1, 1, vec![255, 0, 0, 128]).is_err());
        assert!(RasterImage::from_premultiplied_bgra(1, 1, vec![0, 0, 128, 128]).is_ok());
    }
}
