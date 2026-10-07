//! Intrinsic replaced-element sizing before OPBrowser's viewport/draw-size fitting policy.

use op_image::IntrinsicSize;

pub(super) fn dimensions(
    intrinsic: Option<IntrinsicSize>,
    width: Option<i32>,
    height: Option<i32>,
    width_limits: (i32, Option<i32>),
    height_limits: (i32, Option<i32>),
) -> (i32, i32) {
    let min_width = f64::from(width_limits.0.max(0));
    let min_height = f64::from(height_limits.0.max(0));
    let max_width = width_limits.1.map_or(f64::INFINITY, |value| {
        f64::from(value.max(0)).max(min_width)
    });
    let max_height = height_limits.1.map_or(f64::INFINITY, |value| {
        f64::from(value.max(0)).max(min_height)
    });
    let clamp_width = |value: f64| value.clamp(min_width, max_width);
    let clamp_height = |value: f64| value.clamp(min_height, max_height);

    let Some(intrinsic) = intrinsic else {
        return (
            clamp_width(width.map_or(0.0, f64::from)) as i32,
            clamp_height(height.map_or(0.0, f64::from)) as i32,
        );
    };

    let ratio = intrinsic
        .ratio
        .filter(|(width, height)| *width > 0 && *height > 0)
        .map(|(width, height)| f64::from(width) / f64::from(height));
    if ratio.is_none() {
        let natural_width = intrinsic.width.map_or(300.0, f64::from);
        let natural_height = intrinsic.height.map_or(150.0, f64::from);
        return (
            clamp_width(width.map_or(natural_width, f64::from)) as i32,
            clamp_height(height.map_or(natural_height, f64::from)) as i32,
        );
    }

    let ratio = ratio.expect("checked above");
    let natural_width = intrinsic
        .width
        .map(f64::from)
        .or_else(|| intrinsic.height.map(|height| f64::from(height) * ratio))
        .unwrap_or(300.0);
    let natural_height = intrinsic
        .height
        .map(f64::from)
        .or_else(|| intrinsic.width.map(|width| f64::from(width) / ratio))
        .unwrap_or(natural_width / ratio);

    let (width, height) = match (width, height) {
        (Some(width), Some(height)) => (
            clamp_width(f64::from(width)),
            clamp_height(f64::from(height)),
        ),
        (Some(width), None) => {
            let width = clamp_width(f64::from(width));
            (width, clamp_height(width / ratio))
        }
        (None, Some(height)) => {
            let height = clamp_height(f64::from(height));
            (clamp_width(height * ratio), height)
        }
        (None, None) => {
            let lower = (min_width / natural_width).max(min_height / natural_height);
            let upper = (max_width / natural_width).min(max_height / natural_height);
            let scale = if upper < 1.0 { upper } else { lower.max(1.0) };
            (
                clamp_width(natural_width * scale),
                clamp_height(natural_height * scale),
            )
        }
    };
    let pixel = |value: f64| {
        if value > 0.0 {
            value.floor().max(1.0) as i32
        } else {
            0
        }
    };
    (pixel(width), pixel(height))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raster(width: u32, height: u32) -> Option<IntrinsicSize> {
        Some(IntrinsicSize::raster(width, height))
    }

    fn intrinsic(
        width: Option<u32>,
        height: Option<u32>,
        ratio: Option<(u32, u32)>,
    ) -> Option<IntrinsicSize> {
        Some(IntrinsicSize {
            width,
            height,
            ratio,
        })
    }

    #[test]
    fn unavailable_natural_dimensions_have_no_ratio_and_keep_css_axes_independent() {
        for (width, height, limits, expected) in [
            (None, None, ((0, None), (0, None)), (0, 0)),
            (Some(80), None, ((0, None), (0, None)), (80, 0)),
            (None, Some(30), ((0, None), (0, None)), (0, 30)),
            (Some(80), Some(30), ((0, None), (0, None)), (80, 30)),
            (None, None, ((20, None), (10, None)), (20, 10)),
            (
                Some(80),
                Some(30),
                ((0, Some(40)), (50, Some(20))),
                (40, 50),
            ),
        ] {
            assert_eq!(
                dimensions(None, width, height, limits.0, limits.1),
                expected
            );
        }
        assert_eq!(
            dimensions(
                intrinsic(None, Some(20), None),
                Some(40),
                None,
                (0, None),
                (0, None)
            ),
            (40, 20)
        );
    }

    #[test]
    fn resolves_natural_explicit_auto_and_zero_dimensions() {
        for (width, height, expected) in [
            (None, None, (200, 100)),
            (Some(80), None, (80, 40)),
            (None, Some(30), (60, 30)),
            (Some(80), Some(30), (80, 30)),
            (Some(0), None, (0, 0)),
            (None, Some(0), (0, 0)),
        ] {
            assert_eq!(
                dimensions(raster(200, 100), width, height, (0, None), (0, None)),
                expected
            );
        }
    }

    #[test]
    fn intrinsic_auto_constraints_follow_replaced_ratio_and_conflict_rules() {
        for (width_limits, height_limits, expected) in [
            ((0, Some(100)), (0, None), (100, 50)),
            ((300, None), (0, None), (300, 150)),
            ((0, None), (0, Some(40)), (80, 40)),
            ((0, None), (150, None), (300, 150)),
            ((0, Some(100)), (0, Some(80)), (100, 50)),
            ((0, Some(180)), (0, Some(40)), (80, 40)),
            ((300, None), (200, None), (400, 200)),
            ((500, None), (200, None), (500, 250)),
            ((300, None), (0, Some(40)), (300, 40)),
            ((0, Some(100)), (150, None), (100, 150)),
            ((0, Some(100)), (80, None), (100, 80)),
            ((300, Some(100)), (0, None), (300, 150)),
        ] {
            assert_eq!(
                dimensions(raster(200, 100), None, None, width_limits, height_limits),
                expected,
                "{width_limits:?} {height_limits:?}"
            );
        }
    }

    #[test]
    fn explicit_side_constraints_recompute_auto_side_and_allow_stretching() {
        assert_eq!(
            dimensions(raster(200, 100), Some(300), None, (0, Some(100)), (0, None)),
            (100, 50)
        );
        assert_eq!(
            dimensions(raster(200, 100), Some(100), None, (0, None), (80, None)),
            (100, 80)
        );
        assert_eq!(
            dimensions(raster(200, 100), None, Some(100), (300, None), (0, None)),
            (300, 100)
        );
        assert_eq!(
            dimensions(raster(200, 100), Some(50), Some(20), (80, None), (40, None)),
            (80, 40)
        );
        assert_eq!(
            dimensions(raster(1, 4096), None, Some(1), (0, None), (0, None)),
            (1, 1)
        );
    }

    #[test]
    fn svg_partial_and_ratio_only_intrinsics_follow_css21_replaced_defaults() {
        let ratio_two = Some((2, 1));
        for (source, expected) in [
            (intrinsic(Some(50), Some(25), ratio_two), (40, 20)),
            (intrinsic(None, Some(25), ratio_two), (40, 20)),
            (intrinsic(Some(50), None, ratio_two), (40, 20)),
            (intrinsic(None, Some(25), None), (300, 20)),
            (intrinsic(Some(50), None, None), (50, 20)),
            (intrinsic(None, None, ratio_two), (40, 20)),
            (intrinsic(None, None, None), (300, 20)),
        ] {
            assert_eq!(
                dimensions(source, None, Some(20), (0, None), (0, None)),
                expected,
                "{source:?}"
            );
        }

        for (source, expected) in [
            (intrinsic(Some(50), Some(25), ratio_two), (40, 20)),
            (intrinsic(None, Some(25), ratio_two), (40, 20)),
            (intrinsic(Some(50), None, ratio_two), (40, 20)),
            (intrinsic(None, Some(25), None), (40, 25)),
            (intrinsic(Some(50), None, None), (40, 150)),
            (intrinsic(None, None, ratio_two), (40, 20)),
            (intrinsic(None, None, None), (40, 150)),
        ] {
            assert_eq!(
                dimensions(source, None, None, (0, Some(40)), (0, None)),
                expected,
                "{source:?}"
            );
        }
    }
}
