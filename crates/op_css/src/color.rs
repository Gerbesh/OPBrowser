//! CSS Color 4 conversion helpers.
//!
//! The parser lives in computed.rs because it owns CSS token semantics. This module keeps the
//! floating-point color-space math isolated from cascade/computed-style code. All conversions end
//! in encoded sRGB because the current Win32/GDI surface is an 8-bit sRGB-like raster target.

type Vec3 = [f64; 3];

const D50_TO_D65: [[f64; 3]; 3] = [
    [0.955473421488075, -0.02309845494876471, 0.06325924320057072],
    [
        -0.0283697093338637,
        1.0099953980813041,
        0.021041441191917323,
    ],
    [
        0.012314014864481998,
        -0.020507649298898964,
        1.330365926242124,
    ],
];

const XYZ_D65_TO_LINEAR_SRGB: [[f64; 3]; 3] = [
    [12831.0 / 3959.0, -329.0 / 214.0, -1974.0 / 3959.0],
    [
        -851781.0 / 878810.0,
        1648619.0 / 878810.0,
        36519.0 / 878810.0,
    ],
    [705.0 / 12673.0, -2585.0 / 12673.0, 705.0 / 667.0],
];

const LINEAR_P3_TO_XYZ_D65: [[f64; 3]; 3] = [
    [
        608311.0 / 1250200.0,
        189793.0 / 714400.0,
        198249.0 / 1000160.0,
    ],
    [
        35783.0 / 156275.0,
        247089.0 / 357200.0,
        198249.0 / 2500400.0,
    ],
    [0.0, 32229.0 / 714400.0, 5220557.0 / 5000800.0],
];

const LINEAR_A98_TO_XYZ_D65: [[f64; 3]; 3] = [
    [
        573536.0 / 994567.0,
        263643.0 / 1420810.0,
        187206.0 / 994567.0,
    ],
    [
        591459.0 / 1989134.0,
        6239551.0 / 9945670.0,
        374412.0 / 4972835.0,
    ],
    [
        53769.0 / 1989134.0,
        351524.0 / 4972835.0,
        4929758.0 / 4972835.0,
    ],
];

const LINEAR_PROPHOTO_TO_XYZ_D50: [[f64; 3]; 3] = [
    [0.7977666449006423, 0.13518129740053308, 0.0313477341283922],
    [0.2880748288194013, 0.711835234241873, 0.00008993693872564],
    [0.0, 0.0, 0.8251046025104602],
];

const LINEAR_REC2020_TO_XYZ_D65: [[f64; 3]; 3] = [
    [
        63426534.0 / 99577255.0,
        20160776.0 / 139408157.0,
        47086771.0 / 278816314.0,
    ],
    [
        26158966.0 / 99577255.0,
        472592308.0 / 697040785.0,
        8267143.0 / 139408157.0,
    ],
    [0.0, 19567812.0 / 697040785.0, 295819943.0 / 278816314.0],
];

const OKLAB_TO_LMS: [[f64; 3]; 3] = [
    [1.0, 0.3963377773761749, 0.2158037573099136],
    [1.0, -0.1055613458156586, -0.0638541728258133],
    [1.0, -0.0894841775298119, -1.2914855480194092],
];

const LMS_TO_XYZ_D65: [[f64; 3]; 3] = [
    [1.2268798758459243, -0.5578149944602171, 0.2813910456659647],
    [-0.0405757452148008, 1.112286803280317, -0.0717110580655164],
    [-0.0763729366746601, -0.4214933324022432, 1.5869240198367816],
];

fn multiply(matrix: [[f64; 3]; 3], vector: Vec3) -> Vec3 {
    matrix.map(|row| row[0] * vector[0] + row[1] * vector[1] + row[2] * vector[2])
}

fn gamma_encode_srgb(value: f64) -> f64 {
    let sign = if value < 0.0 { -1.0 } else { 1.0 };
    let absolute = value.abs();
    if absolute > 0.003_130_8 {
        sign * (1.055 * absolute.powf(1.0 / 2.4) - 0.055)
    } else {
        12.92 * value
    }
}

fn linearize_srgb(value: f64) -> f64 {
    let sign = if value < 0.0 { -1.0 } else { 1.0 };
    let absolute = value.abs();
    if absolute <= 0.04045 {
        value / 12.92
    } else {
        sign * ((absolute + 0.055) / 1.055).powf(2.4)
    }
}

fn xyz_d65_to_srgb(xyz: Vec3) -> Vec3 {
    multiply(XYZ_D65_TO_LINEAR_SRGB, xyz).map(gamma_encode_srgb)
}

fn xyz_d50_to_srgb(xyz: Vec3) -> Vec3 {
    xyz_d65_to_srgb(multiply(D50_TO_D65, xyz))
}

fn lab_to_xyz_d50(lab: Vec3) -> Vec3 {
    const KAPPA: f64 = 24389.0 / 27.0;
    const EPSILON: f64 = 216.0 / 24389.0;
    const D50: Vec3 = [0.3457 / 0.3585, 1.0, (1.0 - 0.3457 - 0.3585) / 0.3585];

    let f1 = (lab[0] + 16.0) / 116.0;
    let f0 = lab[1] / 500.0 + f1;
    let f2 = f1 - lab[2] / 200.0;
    let f0_cube = f0.powi(3);
    let f2_cube = f2.powi(3);
    let relative = [
        if f0_cube > EPSILON {
            f0_cube
        } else {
            (116.0 * f0 - 16.0) / KAPPA
        },
        if lab[0] > KAPPA * EPSILON {
            f1.powi(3)
        } else {
            lab[0] / KAPPA
        },
        if f2_cube > EPSILON {
            f2_cube
        } else {
            (116.0 * f2 - 16.0) / KAPPA
        },
    ];

    [
        relative[0] * D50[0],
        relative[1] * D50[1],
        relative[2] * D50[2],
    ]
}

pub(crate) fn lab_to_srgb(lab: Vec3) -> Vec3 {
    xyz_d50_to_srgb(lab_to_xyz_d50(lab))
}

pub(crate) fn oklab_to_srgb(oklab: Vec3) -> Vec3 {
    let nonlinear_lms = multiply(OKLAB_TO_LMS, oklab);
    let linear_lms = nonlinear_lms.map(|component| component.powi(3));
    xyz_d65_to_srgb(multiply(LMS_TO_XYZ_D65, linear_lms))
}

pub(crate) fn predefined_to_srgb(space: &str, components: Vec3) -> Option<Vec3> {
    let normalized = space.to_ascii_lowercase();
    let srgb = match normalized.as_str() {
        "srgb" => components,
        "srgb-linear" => components.map(gamma_encode_srgb),
        "display-p3" => {
            let xyz = multiply(LINEAR_P3_TO_XYZ_D65, components.map(linearize_srgb));
            xyz_d65_to_srgb(xyz)
        }
        "display-p3-linear" => {
            let xyz = multiply(LINEAR_P3_TO_XYZ_D65, components);
            xyz_d65_to_srgb(xyz)
        }
        "a98-rgb" => {
            let linear = components.map(|value| {
                let sign = if value < 0.0 { -1.0 } else { 1.0 };
                sign * value.abs().powf(563.0 / 256.0)
            });
            xyz_d65_to_srgb(multiply(LINEAR_A98_TO_XYZ_D65, linear))
        }
        "prophoto-rgb" => {
            let linear = components.map(|value| {
                let sign = if value < 0.0 { -1.0 } else { 1.0 };
                let absolute = value.abs();
                if absolute <= 16.0 / 512.0 {
                    value / 16.0
                } else {
                    sign * absolute.powf(1.8)
                }
            });
            xyz_d50_to_srgb(multiply(LINEAR_PROPHOTO_TO_XYZ_D50, linear))
        }
        "rec2020" => {
            // CSS Color 4 (2026 CRD) uses the BT.1886 display-referred gamma 2.40 transfer.
            // Keep this distinct from the older piecewise Rec.2020 camera/OETF transform.
            let linear = components.map(|value| {
                let sign = if value < 0.0 { -1.0 } else { 1.0 };
                sign * value.abs().powf(2.4)
            });
            xyz_d65_to_srgb(multiply(LINEAR_REC2020_TO_XYZ_D65, linear))
        }
        "xyz" | "xyz-d65" => xyz_d65_to_srgb(components),
        "xyz-d50" => xyz_d50_to_srgb(components),
        _ => return None,
    };
    Some(srgb)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(rgb: Vec3) -> [u8; 3] {
        rgb.map(|component| (component.clamp(0.0, 1.0) * 255.0).round() as u8)
    }

    #[test]
    fn lab_examples_land_on_expected_srgb_bytes() {
        assert_eq!(bytes(lab_to_srgb([0.0, 0.0, 0.0])), [0, 0, 0]);
        assert_eq!(bytes(lab_to_srgb([70.0, -45.0, 0.0])), [27, 193, 169]);
        assert_eq!(
            bytes(lab_to_srgb([
                46.2775,
                67.9892 * 134.3912_f64.to_radians().cos(),
                67.9892 * 134.3912_f64.to_radians().sin(),
            ])),
            [0, 128, 0]
        );
    }

    #[test]
    fn oklab_examples_land_on_expected_srgb_bytes() {
        assert_eq!(bytes(oklab_to_srgb([0.5, 0.05, 0.0])), [124, 87, 98]);
        assert_eq!(bytes(oklab_to_srgb([0.5, 0.2, 0.0])), [180, 6, 95]);
    }

    #[test]
    fn predefined_spaces_preserve_basic_reference_colors() {
        assert_eq!(
            bytes(predefined_to_srgb("display-p3", [0.0, 0.501_960_8, 0.0]).unwrap()),
            [0, 130, 0]
        );
        assert_eq!(
            bytes(predefined_to_srgb("xyz-d65", [0.9505, 1.0, 1.089]).unwrap()),
            [255, 255, 255]
        );
        assert_eq!(
            bytes(predefined_to_srgb("xyz-d50", [0.9643, 1.0, 0.8251]).unwrap()),
            [255, 255, 255]
        );
    }
}
