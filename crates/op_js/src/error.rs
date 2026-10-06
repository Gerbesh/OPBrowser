use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsErrorKind {
    Syntax,
    Reference,
    Type,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsError {
    pub kind: JsErrorKind,
    pub offset: usize,
    pub message: String,
}

impl JsError {
    pub(crate) fn syntax(offset: usize, message: impl Into<String>) -> Self {
        Self {
            kind: JsErrorKind::Syntax,
            offset,
            message: message.into(),
        }
    }

    pub(crate) fn reference(message: impl Into<String>) -> Self {
        Self {
            kind: JsErrorKind::Reference,
            offset: 0,
            message: message.into(),
        }
    }

    pub(crate) fn type_error(message: impl Into<String>) -> Self {
        Self {
            kind: JsErrorKind::Type,
            offset: 0,
            message: message.into(),
        }
    }
}

impl fmt::Display for JsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.offset == 0 {
            write!(formatter, "{:?}: {}", self.kind, self.message)
        } else {
            write!(
                formatter,
                "{:?} at byte {}: {}",
                self.kind, self.offset, self.message
            )
        }
    }
}

impl std::error::Error for JsError {}
