#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectId(pub(crate) usize);

#[derive(Debug, Clone, PartialEq)]
pub enum JsValue {
    Undefined,
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
    Object(ObjectId),
}

fn parse_radix_number(digits: &str, base: u32) -> f64 {
    if digits.is_empty() {
        return f64::NAN;
    }
    let mut value = 0.0;
    for ch in digits.chars() {
        let Some(digit) = ch.to_digit(base) else {
            return f64::NAN;
        };
        value = value * base as f64 + digit as f64;
    }
    value
}

impl JsValue {
    pub fn is_truthy(&self) -> bool {
        match self {
            Self::Undefined | Self::Null => false,
            Self::Boolean(value) => *value,
            Self::Number(value) => *value != 0.0 && !value.is_nan(),
            Self::String(value) => !value.is_empty(),
            Self::Object(_) => true,
        }
    }

    pub(crate) fn to_number(&self) -> f64 {
        match self {
            Self::Undefined => f64::NAN,
            Self::Null => 0.0,
            Self::Boolean(false) => 0.0,
            Self::Boolean(true) => 1.0,
            Self::Number(value) => *value,
            Self::String(value) => {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    0.0
                } else if let Some(hex) = trimmed
                    .strip_prefix("0x")
                    .or_else(|| trimmed.strip_prefix("0X"))
                {
                    parse_radix_number(hex, 16)
                } else if let Some(binary) = trimmed
                    .strip_prefix("0b")
                    .or_else(|| trimmed.strip_prefix("0B"))
                {
                    parse_radix_number(binary, 2)
                } else if let Some(octal) = trimmed
                    .strip_prefix("0o")
                    .or_else(|| trimmed.strip_prefix("0O"))
                {
                    parse_radix_number(octal, 8)
                } else {
                    trimmed.parse().unwrap_or(f64::NAN)
                }
            }
            Self::Object(_) => f64::NAN,
        }
    }

    pub(crate) fn to_js_string(&self) -> String {
        match self {
            Self::Undefined => "undefined".into(),
            Self::Null => "null".into(),
            Self::Boolean(value) => value.to_string(),
            Self::Number(value) if value.is_nan() => "NaN".into(),
            Self::Number(value) if *value == f64::INFINITY => "Infinity".into(),
            Self::Number(value) if *value == f64::NEG_INFINITY => "-Infinity".into(),
            Self::Number(value) if value.fract() == 0.0 => format!("{value:.0}"),
            Self::Number(value) => value.to_string(),
            Self::String(value) => value.clone(),
            Self::Object(_) => "[object Object]".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn number_string_conversion_supports_standard_radices() {
        for (source, expected) in [
            ("0xff", 255.0),
            ("0XFF", 255.0),
            ("0b101", 5.0),
            ("0B11", 3.0),
            ("0o77", 63.0),
            ("0O10", 8.0),
            ("  0x20  ", 32.0),
            (" +12 ", 12.0),
        ] {
            assert_eq!(JsValue::String(source.into()).to_number(), expected);
        }
        for invalid in ["0x", "0b2", "-0x1", "0o9", "0x 1"] {
            assert!(
                JsValue::String(invalid.into()).to_number().is_nan(),
                "{invalid}"
            );
        }
    }
}
