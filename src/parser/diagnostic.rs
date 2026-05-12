use crate::parser::span::Span;

/// How severe is a diagnostic?
#[allow(dead_code)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// A problem to report to the human.
#[allow(dead_code)]
pub struct Diagnostic {
    pub severity: Severity,
    pub span: Span,
    pub message: String,
}

impl Diagnostic {
    pub fn error(at: impl Into<Span>, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            span: at.into(),
            message: message.into(),
        }
    }
}
