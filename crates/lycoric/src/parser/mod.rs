mod lrc;

use std::sync::Arc;

use crate::{
    diagnostics::{
        Diagnostic,
        DiagnosticSeverity,
    },
    model::{
        LaneKind,
        LyricsDocument,
    },
    time::LyricTime,
};

pub use lrc::parse_lrc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ParseMode {
    Strict,
    #[default]
    Lenient,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseOptions {
    pub mode: ParseMode,
    pub lane_kind: LaneKind,
    pub language: Option<Arc<str>>,
    pub media_duration: Option<LyricTime>,
    pub fallback_duration: Option<LyricTime>,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            mode: ParseMode::Lenient,
            lane_kind: LaneKind::Original,
            language: None,
            media_duration: None,
            fallback_duration: Some(LyricTime::from_secs(5)),
        }
    }
}

impl ParseOptions {
    pub fn strict() -> Self {
        Self {
            mode: ParseMode::Strict,
            ..Self::default()
        }
    }

    pub fn lenient() -> Self {
        Self::default()
    }
}

#[derive(Clone, Debug)]
pub struct ParseReport {
    pub document: Option<LyricsDocument>,
    pub diagnostics: Vec<Diagnostic>,
}

impl ParseReport {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
    }

    pub fn into_result(self) -> Result<LyricsDocument, Vec<Diagnostic>> {
        if self.has_errors() {
            Err(self.diagnostics)
        } else {
            self.document.ok_or(self.diagnostics)
        }
    }
}
