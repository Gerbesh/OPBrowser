//! Intrinsic raster sizing before OPBrowser's viewport/draw-size fitting policy.

pub(super) fn dimensions(
    natural: (u32, u32),
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
    let natural_width = f64::from(natural.0);
    let natural_height = f64::from(natural.1);
    let clamp_width = |value: f64| value.clamp(min_width, max_width);
    let clamp_height = |value: f64| value.clamp(min_height, max_height);
    let (width, height) = match (width, height) {
        (Some(width), Some(height)) => (
            clamp_width(f64::from(width)),
            clamp_height(f64::from(height)),
        ),
        (Some(width), None) => {
            let width = clamp_width(f64::from(width));
            (width, clamp_height(width * natural_height / natural_width))
        }
        (None, Some(height)) => {
            let height = clamp_height(f64::from(height));
            (clamp_width(height * natural_width / natural_height), height)
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
                dimensions((200, 100), width, height, (0, None), (0, None)),
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
                dimensions((200, 100), None, None, width_limits, height_limits),
                expected,
                "{width_limits:?} {height_limits:?}"
            );
        }
    }

    #[test]
    fn explicit_side_constraints_recompute_auto_side_and_allow_stretching() {
        assert_eq!(
            dimensions((200, 100), Some(300), None, (0, Some(100)), (0, None)),
            (100, 50)
        );
        assert_eq!(
            dimensions((200, 100), Some(100), None, (0, None), (80, None)),
            (100, 80)
        );
        assert_eq!(
            dimensions((200, 100), None, Some(100), (300, None), (0, None)),
            (300, 100)
        );
        assert_eq!(
            dimensions((200, 100), Some(50), Some(20), (80, None), (40, None)),
            (80, 40)
        );
        assert_eq!(
            dimensions((1, 4096), None, Some(1), (0, None), (0, None)),
            (1, 1)
        );
    }
}
