//! Bounded raster buffers; Windows WIC is used only as an infrastructure codec.

use std::fmt;

pub const MAX_ENCODED_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_PIXELS: u32 = 4 * 1024 * 1024;
pub const MAX_DIMENSION: u32 = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntrinsicSize {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub ratio: Option<(u32, u32)>,
}

impl IntrinsicSize {
    pub const fn raster(width: u32, height: u32) -> Self {
        Self {
            width: Some(width),
            height: Some(height),
            ratio: Some((width, height)),
        }
    }
}

/// Top-down premultiplied BGRA, ready for GDI AlphaBlend. Fields stay validated.
#[derive(Debug, PartialEq, Eq)]
pub struct RasterImage {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    intrinsic: IntrinsicSize,
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
            intrinsic: IntrinsicSize::raster(width, height),
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

    pub fn intrinsic_size(&self) -> IntrinsicSize {
        self.intrinsic
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

fn looks_like_svg(bytes: &[u8]) -> bool {
    let Ok(source) = std::str::from_utf8(bytes) else {
        return false;
    };
    let head = &source[..source.len().min(1024)];
    head.find("<svg").is_some_and(|index| {
        head[index + 4..]
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_whitespace() || matches!(ch, '>' | '/'))
    })
}

fn tag_attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    for (index, _) in tag.match_indices(name) {
        let before = tag[..index].chars().next_back();
        if before.is_some_and(|ch| !ch.is_ascii_whitespace() && ch != '<') {
            continue;
        }
        let mut rest = tag[index + name.len()..].trim_start();
        if !rest.starts_with('=') {
            continue;
        }
        rest = rest[1..].trim_start();
        let quote = rest.chars().next()?;
        if !matches!(quote, '"' | '\'') {
            continue;
        }
        let value = &rest[quote.len_utf8()..];
        let end = value.find(quote)?;
        return Some(&value[..end]);
    }
    None
}

fn svg_length(value: &str) -> Option<f64> {
    let value = value.trim();
    if value.ends_with('%') {
        return None;
    }
    let value = value.strip_suffix("px").unwrap_or(value).trim();
    let number = value.parse::<f64>().ok()?;
    number.is_finite().then_some(number)
}

fn svg_view_box(tag: &str) -> Option<(f64, f64, f64, f64)> {
    let value = tag_attribute(tag, "viewBox")?;
    let values = value
        .split(|ch: char| ch.is_ascii_whitespace() || ch == ',')
        .filter(|part| !part.is_empty())
        .map(str::parse::<f64>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    let [x, y, width, height] = values.as_slice() else {
        return None;
    };
    (*width > 0.0 && *height > 0.0 && width.is_finite() && height.is_finite())
        .then_some((*x, *y, *width, *height))
}

fn ratio_pair(width: f64, height: f64) -> Option<(u32, u32)> {
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return None;
    }
    let scale = 10_000.0;
    let width = (width * scale).round().clamp(1.0, f64::from(u32::MAX)) as u32;
    let height = (height * scale).round().clamp(1.0, f64::from(u32::MAX)) as u32;
    Some((width, height))
}

fn raster_dimension(value: f64) -> Result<u32, ImageError> {
    if !value.is_finite() || value <= 0.0 || value > f64::from(MAX_DIMENSION) {
        return Err(ImageError::TooLarge);
    }
    Ok(value.round().max(1.0) as u32)
}

fn svg_raster_size(intrinsic: IntrinsicSize) -> Result<(u32, u32), ImageError> {
    let ratio = intrinsic
        .ratio
        .map(|(width, height)| f64::from(width) / f64::from(height));
    let (width, height) = match (intrinsic.width, intrinsic.height, ratio) {
        (Some(width), Some(height), _) => (f64::from(width), f64::from(height)),
        (Some(width), None, Some(ratio)) => (f64::from(width), f64::from(width) / ratio),
        (None, Some(height), Some(ratio)) => (f64::from(height) * ratio, f64::from(height)),
        (Some(width), None, None) => (f64::from(width), 150.0),
        (None, Some(height), None) => (300.0, f64::from(height)),
        (None, None, Some(ratio)) if ratio >= 2.0 => (300.0, 300.0 / ratio),
        (None, None, Some(ratio)) => (150.0 * ratio, 150.0),
        (None, None, None) => (300.0, 150.0),
    };
    Ok((raster_dimension(width)?, raster_dimension(height)?))
}

fn svg_color(value: &str) -> Option<(u8, u8, u8, u8)> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("none") {
        return Some((0, 0, 0, 0));
    }
    if let Some(hex) = value.strip_prefix('#') {
        if !hex.is_ascii() {
            return None;
        }
        return match hex.len() {
            3 => {
                let mut channels = [0_u8; 3];
                for (index, ch) in hex.as_bytes().iter().copied().enumerate() {
                    let digit = (ch as char).to_digit(16)? as u8;
                    channels[index] = digit * 17;
                }
                Some((channels[0], channels[1], channels[2], 255))
            }
            6 => {
                let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;
                Some((red, green, blue, 255))
            }
            _ => None,
        };
    }
    let rgb = match value.to_ascii_lowercase().as_str() {
        "black" => (0, 0, 0),
        "white" => (255, 255, 255),
        "red" => (255, 0, 0),
        "green" => (0, 128, 0),
        "lime" => (0, 255, 0),
        "blue" => (0, 0, 255),
        "fuchsia" | "magenta" => (255, 0, 255),
        "silver" => (192, 192, 192),
        "gray" | "grey" => (128, 128, 128),
        "orange" => (255, 165, 0),
        "aqua" | "cyan" => (0, 255, 255),
        "yellow" => (255, 255, 0),
        _ => return None,
    };
    Some((rgb.0, rgb.1, rgb.2, 255))
}

fn svg_axis_value(
    value: &str,
    raster_extent: u32,
    user_min: f64,
    user_extent: Option<f64>,
    is_extent: bool,
) -> Option<f64> {
    let value = value.trim();
    if let Some(percent) = value.strip_suffix('%') {
        let percent = percent.trim().parse::<f64>().ok()? / 100.0;
        return percent
            .is_finite()
            .then_some(percent * f64::from(raster_extent));
    }
    let value = svg_length(value)?;
    if let Some(user_extent) = user_extent {
        let value = if is_extent { value } else { value - user_min };
        Some(value * f64::from(raster_extent) / user_extent)
    } else {
        Some(value)
    }
}

fn decode_svg(bytes: &[u8], pixel_budget: usize) -> Result<RasterImage, ImageError> {
    let source = std::str::from_utf8(bytes)
        .map_err(|_| ImageError::Decode("SVG is not valid UTF-8".to_owned()))?;
    let svg_start = source
        .find("<svg")
        .ok_or_else(|| ImageError::Decode("missing SVG root".to_owned()))?;
    let svg_end = source[svg_start..]
        .find('>')
        .map(|offset| svg_start + offset + 1)
        .ok_or_else(|| ImageError::Decode("unterminated SVG root".to_owned()))?;
    let root = &source[svg_start..svg_end];

    let intrinsic_width = tag_attribute(root, "width")
        .and_then(svg_length)
        .filter(|value| *value > 0.0)
        .map(raster_dimension)
        .transpose()?;
    let intrinsic_height = tag_attribute(root, "height")
        .and_then(svg_length)
        .filter(|value| *value > 0.0)
        .map(raster_dimension)
        .transpose()?;
    let view_box = svg_view_box(root);
    let ratio = match (intrinsic_width, intrinsic_height) {
        (Some(width), Some(height)) => Some((width, height)),
        _ => view_box.and_then(|(_, _, width, height)| ratio_pair(width, height)),
    };
    let intrinsic = IntrinsicSize {
        width: intrinsic_width,
        height: intrinsic_height,
        ratio,
    };
    let (width, height) = svg_raster_size(intrinsic)?;
    let length = pixel_length(width, height, pixel_budget)?;
    let mut pixels = vec![0_u8; length];

    let (view_x, view_y, view_width, view_height) =
        view_box.unwrap_or((0.0, 0.0, f64::from(width), f64::from(height)));
    let mut cursor = svg_end;
    let mut rectangle_count = 0_usize;
    while let Some(relative) = source[cursor..].find("<rect") {
        rectangle_count = rectangle_count.saturating_add(1);
        if rectangle_count > 4096 {
            return Err(ImageError::TooLarge);
        }
        let start = cursor + relative;
        let Some(relative_end) = source[start..].find('>') else {
            break;
        };
        let end = start + relative_end + 1;
        let tag = &source[start..end];
        let fill = tag_attribute(tag, "fill")
            .and_then(svg_color)
            .unwrap_or((0, 0, 0, 255));
        let x = tag_attribute(tag, "x")
            .and_then(|value| svg_axis_value(value, width, view_x, Some(view_width), false))
            .unwrap_or(0.0);
        let y = tag_attribute(tag, "y")
            .and_then(|value| svg_axis_value(value, height, view_y, Some(view_height), false))
            .unwrap_or(0.0);
        let rect_width = tag_attribute(tag, "width")
            .and_then(|value| svg_axis_value(value, width, 0.0, Some(view_width), true))
            .unwrap_or(0.0);
        let rect_height = tag_attribute(tag, "height")
            .and_then(|value| svg_axis_value(value, height, 0.0, Some(view_height), true))
            .unwrap_or(0.0);

        let x0 = x.floor().clamp(0.0, f64::from(width)) as u32;
        let y0 = y.floor().clamp(0.0, f64::from(height)) as u32;
        let x1 = (x + rect_width).ceil().clamp(0.0, f64::from(width)) as u32;
        let y1 = (y + rect_height).ceil().clamp(0.0, f64::from(height)) as u32;
        let (red, green, blue, alpha) = fill;
        let red = (u16::from(red) * u16::from(alpha) / 255) as u8;
        let green = (u16::from(green) * u16::from(alpha) / 255) as u8;
        let blue = (u16::from(blue) * u16::from(alpha) / 255) as u8;
        for row in y0..y1 {
            for column in x0..x1 {
                let index = ((row * width + column) * 4) as usize;
                pixels[index..index + 4].copy_from_slice(&[blue, green, red, alpha]);
            }
        }
        cursor = end;
    }

    Ok(RasterImage {
        width,
        height,
        pixels,
        intrinsic,
    })
}

pub fn decode(bytes: &[u8], pixel_budget: usize) -> Result<RasterImage, ImageError> {
    if bytes.len() > MAX_ENCODED_BYTES {
        return Err(ImageError::TooLarge);
    }
    if looks_like_svg(bytes) {
        return decode_svg(bytes, pixel_budget);
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

/// Convert one RGB color expressed in an embedded ICC source profile to
/// output sRGB using Windows Image Component's color-transform pipeline.
/// Profile loading is separately bounded by the caller.
pub fn convert_icc_rgb(profile: &[u8], rgb: [u8; 3]) -> Result<[u8; 3], ImageError> {
    if profile.is_empty() || profile.len() > 1024 * 1024 {
        return Err(ImageError::TooLarge);
    }
    #[cfg(windows)]
    {
        wic::convert_icc_rgb(profile, rgb)
    }
    #[cfg(not(windows))]
    {
        let _ = rgb;
        Err(ImageError::UnsupportedFormat)
    }
}

#[cfg(windows)]
mod wic {
    use super::*;
    use windows::Win32::Graphics::Imaging::*;
    use windows::Win32::System::Com::*;
    use windows::core::Interface;

    struct Apartment;
    impl Drop for Apartment {
        fn drop(&mut self) {
            unsafe { CoUninitialize() };
        }
    }

    pub(super) fn convert_icc_rgb(profile: &[u8], rgb: [u8; 3]) -> Result<[u8; 3], ImageError> {
        let operation = || -> windows::core::Result<[u8; 3]> {
            unsafe {
                CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
            }
            let _apartment = Apartment;
            unsafe {
                let factory: IWICImagingFactory =
                    CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
                let source_context = factory.CreateColorContext()?;
                source_context.InitializeFromMemory(profile)?;
                let destination = factory.CreateColorContext()?;
                destination.InitializeFromExifColorSpace(1)?;
                let pixel = [rgb[2], rgb[1], rgb[0], 255];
                let bitmap = factory.CreateBitmapFromMemory(
                    1,
                    1,
                    &GUID_WICPixelFormat32bppBGRA,
                    4,
                    &pixel,
                )?;
                let transform = factory.CreateColorTransformer()?;
                transform.Initialize(
                    &bitmap,
                    &source_context,
                    &destination,
                    &GUID_WICPixelFormat32bppBGRA,
                )?;
                let mut result = [0; 4];
                transform.CopyPixels(std::ptr::null(), 4, &mut result)?;
                Ok([result[2], result[1], result[0]])
            }
        };
        operation().map_err(|error| ImageError::Decode(error.to_string()))
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
                // WIC does not implicitly apply embedded ICC profiles when
                // converting frame pixels to PBGRA. Explicitly transform the
                // embedded source space to the display's sRGB target first.
                // Unprofiled images and unsupported/corrupt profiles keep the
                // normal decode path instead of making the image disappear.
                let mut actual = 0u32;
                let count_status = frame.GetColorContexts(&mut [], &mut actual);
                let mut contexts = (0..actual.min(8))
                    .filter_map(|_| factory.CreateColorContext().ok().map(Some))
                    .collect::<Vec<_>>();
                let color_context_status = if count_status.is_ok() && !contexts.is_empty() {
                    frame.GetColorContexts(&mut contexts, &mut actual)
                } else {
                    count_status
                };

                let profiled_source = if color_context_status.is_ok() && actual > 0 {
                    contexts
                        .into_iter()
                        .flatten()
                        .find(|context| {
                            context
                                .GetType()
                                .is_ok_and(|kind| kind == WICColorContextProfile)
                        })
                        .and_then(|source_context| {
                            let destination = factory.CreateColorContext().ok()?;
                            destination.InitializeFromExifColorSpace(1).ok()?;
                            let transformed = factory.CreateColorTransformer().ok()?;
                            transformed
                                .Initialize(
                                    &frame,
                                    &source_context,
                                    &destination,
                                    &GUID_WICPixelFormat32bppBGRA,
                                )
                                .ok()?;
                            Some(transformed)
                        })
                } else {
                    None
                };
                let converter = factory.CreateFormatConverter()?;
                let source: IWICBitmapSource = if let Some(transform) = profiled_source {
                    transform.cast()?
                } else {
                    frame.cast()?
                };
                converter.Initialize(
                    &source,
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
                    intrinsic: IntrinsicSize::raster(width, height),
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
    fn standalone_icc_colors_transform_to_output_srgb() {
        let path =
            std::path::Path::new("../../target/compat-wpt/css/css-color/support/swapped.icc");
        if !path.is_file() {
            return;
        }
        let bytes = std::fs::read(path).unwrap();
        assert_eq!(convert_icc_rgb(&bytes, [153, 0, 0]).unwrap(), [0, 153, 0]);
        assert_eq!(convert_icc_rgb(&bytes, [0, 153, 0]).unwrap(), [153, 0, 0]);
        assert!(convert_icc_rgb(&[0, 1, 2, 3], [153, 0, 0]).is_err());
    }

    #[test]
    #[cfg(windows)]
    fn embedded_icc_png_uses_profile_when_wpt_fixtures_are_available() {
        // The pinned upstream fixtures live in ignored target/compat-wpt.
        // CI without that checkout still runs the decoder's built-in tests.
        let root = "../../target/compat-wpt/css/css-color/support/";
        let profiled = std::path::Path::new(root).join("swap-990000-iCCP.png");
        let srgb = std::path::Path::new(root).join("009900-sRGB.png");
        if !profiled.is_file() || !srgb.is_file() {
            return;
        }
        for path in [&profiled, &srgb] {
            let image = decode(&std::fs::read(path).unwrap(), 200 * 200 * 4).unwrap();
            assert_eq!((image.width(), image.height()), (200, 200));
            let color = &image.pixels()[..4];
            assert_eq!(color, &[0, 153, 0, 255], "{}", path.display());
        }
    }

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
    }

    #[test]
    fn decodes_bounded_svg_rects_and_preserves_intrinsic_metadata() {
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="50" height="25" viewBox="0 0 1000 1000">
            <rect fill="#ff00ff" x="0" y="0" width="1000" height="1000"/>
        </svg>"##;
        let image = decode(svg, 50 * 25 * 4).unwrap();
        assert_eq!((image.width(), image.height()), (50, 25));
        assert_eq!(
            image.intrinsic_size(),
            IntrinsicSize {
                width: Some(50),
                height: Some(25),
                ratio: Some((50, 25)),
            }
        );
        assert_eq!(&image.pixels()[..4], &[255, 0, 255, 255]);

        let ratio_only =
            br#"<svg viewBox="0 0 1000 500"><rect fill="blue" width="100%" height="100%"/></svg>"#;
        let image = decode(ratio_only, 300 * 150 * 4).unwrap();
        assert_eq!((image.width(), image.height()), (300, 150));
        assert_eq!(
            image.intrinsic_size(),
            IntrinsicSize {
                width: None,
                height: None,
                ratio: Some((10_000_000, 5_000_000)),
            }
        );
        assert_eq!(&image.pixels()[..4], &[255, 0, 0, 255]);

        let no_ratio = br#"<svg height="25"><rect fill="aqua" width="100%" height="100%"/></svg>"#;
        let image = decode(no_ratio, 300 * 25 * 4).unwrap();
        assert_eq!((image.width(), image.height()), (300, 25));
        assert_eq!(
            image.intrinsic_size(),
            IntrinsicSize {
                width: None,
                height: Some(25),
                ratio: None,
            }
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
