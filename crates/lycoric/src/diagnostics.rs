use std::{
    fmt,
    sync::Arc,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DiagnosticSeverity {
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DiagnosticCode {
    MalformedTag,
    MissingTimestamp,
    InvalidTimestamp,
    InvalidOffset,
    InvalidMetadata,
    UnknownMetadata,
    OutOfOrderTimestamp,
    InvalidRange,
    TimeOverflow,
    AmbiguousMatch,
    FuzzyMatch,
    LaneConflict,
    UnmatchedSidecarLine,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SourcePosition {
    pub line: usize,
    pub column: usize,
    pub byte_offset: usize,
}

impl SourcePosition {
    pub const fn new(line: usize, column: usize, byte_offset: usize) -> Self {
        Self {
            line,
            column,
            byte_offset,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SourceSpan {
    pub start: SourcePosition,
    pub end: SourcePosition,
}

impl SourceSpan {
    pub const fn new(start: SourcePosition, end: SourcePosition) -> Self {
        Self { start, end }
    }

    pub fn at(line: usize, column: usize, byte_offset: usize, byte_len: usize) -> Self {
        Self {
            start: SourcePosition::new(line, column, byte_offset),
            end: SourcePosition::new(line, column + byte_len, byte_offset + byte_len),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub code: DiagnosticCode,
    pub message: Arc<str>,
    pub span: Option<SourceSpan>,
}

impl Diagnostic {
    pub fn warning(code: DiagnosticCode, message: impl Into<Arc<str>>) -> Self {
        Self {
            severity: DiagnosticSeverity::Warning,
            code,
            message: message.into(),
            span: None,
        }
    }

    pub fn error(code: DiagnosticCode, message: impl Into<Arc<str>>) -> Self {
        Self {
            severity: DiagnosticSeverity::Error,
            code,
            message: message.into(),
            span: None,
        }
    }

    pub fn with_span(mut self, span: SourceSpan) -> Self {
        self.span = Some(span);
        self
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(span) = self.span {
            write!(
                formatter,
                "{}:{}: {:?}: {}",
                span.start.line, span.start.column, self.code, self.message
            )
        } else {
            write!(formatter, "{:?}: {}", self.code, self.message)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_carry_optional_source_locations() {
        let diagnostic = Diagnostic::warning(DiagnosticCode::MalformedTag, "bad tag")
            .with_span(SourceSpan::at(2, 4, 10, 3));
        assert_eq!(diagnostic.span.unwrap().start.line, 2);
        assert!(diagnostic.to_string().contains("2:4"));
    }
}
