#[derive(Debug, Clone, PartialEq)]
pub enum JsValue {
    Undefined,
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
}

impl JsValue {
    pub fn is_truthy(&self) -> bool {
        match self {
            Self::Undefined | Self::Null => false,
            Self::Boolean(value) => *value,
            Self::Number(value) => *value != 0.0 && !value.is_nan(),
            Self::String(value) => !value.is_empty(),
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
                } else {
                    trimmed.parse().unwrap_or(f64::NAN)
                }
            }
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
        }
    }
}
