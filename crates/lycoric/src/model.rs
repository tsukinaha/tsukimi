use std::{
    ops::Range,
    sync::Arc,
};

use crate::time::{
    LyricTime,
    TimeRange,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum LaneKind {
    Original,
    Other(Arc<str>),
}

impl LaneKind {
    pub fn custom(name: impl Into<Arc<str>>) -> Self {
        Self::Other(name.into())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LineId(u64);

impl LineId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MetadataTag {
    pub key: Arc<str>,
    pub value: Arc<str>,
}

impl MetadataTag {
    pub fn new(key: impl Into<Arc<str>>, value: impl Into<Arc<str>>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LyricsMetadata {
    pub artist: Option<Arc<str>>,
    pub album: Option<Arc<str>>,
    pub title: Option<Arc<str>>,
    pub author: Option<Arc<str>>,
    pub creator: Option<Arc<str>>,
    pub editor: Option<Arc<str>>,
    pub version: Option<Arc<str>>,
    pub length: Option<LyricTime>,
    pub offset: LyricTime,
    pub unknown: Vec<MetadataTag>,
}

impl LyricsMetadata {
    pub fn value(&self, key: &str) -> Option<&str> {
        match key.to_ascii_lowercase().as_str() {
            "ar" => self.artist.as_deref(),
            "al" => self.album.as_deref(),
            "ti" => self.title.as_deref(),
            "au" => self.author.as_deref(),
            "by" => self.creator.as_deref(),
            "re" => self.editor.as_deref(),
            "ve" => self.version.as_deref(),
            key => self
                .unknown
                .iter()
                .rev()
                .find(|tag| tag.key.eq_ignore_ascii_case(key))
                .map(|tag| tag.value.as_ref()),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LyricsDocument {
    pub metadata: LyricsMetadata,
    pub tracks: Vec<LyricTrack>,
    pub duration: Option<LyricTime>,
}

impl LyricsDocument {
    fn primary_track_index(&self) -> Option<usize> {
        self.tracks
            .iter()
            .position(|track| track.kind == LaneKind::Original && !track.lines.is_empty())
            .or_else(|| self.tracks.iter().position(|track| !track.lines.is_empty()))
    }

    pub fn primary_track(&self) -> Option<&LyricTrack> {
        self.primary_track_index().map(|index| &self.tracks[index])
    }

    pub fn primary_track_mut(&mut self) -> Option<&mut LyricTrack> {
        let index = self.primary_track_index()?;
        Some(&mut self.tracks[index])
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LyricTrack {
    pub language: Option<Arc<str>>,
    pub kind: LaneKind,
    pub lines: Vec<LyricLine>,
}

impl LyricTrack {
    pub fn new(kind: LaneKind) -> Self {
        Self {
            language: None,
            kind,
            lines: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LyricLine {
    pub id: LineId,
    pub range: TimeRange,
    pub lanes: Vec<LyricLane>,
}

impl LyricLine {
    pub fn has_visible_text(&self) -> bool {
        self.lanes.iter().any(|lane| !lane.text.trim().is_empty())
    }

    pub fn lane(&self, kind: &LaneKind) -> Option<&LyricLane> {
        self.lanes.iter().find(|lane| &lane.kind == kind)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LyricLane {
    pub kind: LaneKind,
    pub language: Option<Arc<str>>,
    pub text: Arc<str>,
    pub segments: Vec<TimedSegment>,
}

impl LyricLane {
    pub fn new(kind: LaneKind, text: impl Into<Arc<str>>) -> Self {
        Self {
            kind,
            language: None,
            text: text.into(),
            segments: Vec::new(),
        }
    }

    pub fn has_valid_text_ranges(&self) -> bool {
        self.segments.iter().all(|segment| {
            segment.text_range.start <= segment.text_range.end
                && segment.text_range.end <= self.text.len()
                && self.text.is_char_boundary(segment.text_range.start)
                && self.text.is_char_boundary(segment.text_range.end)
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimedSegment {
    pub range: TimeRange,
    pub text_range: Range<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_unicode_segment_boundaries() {
        let mut lane = LyricLane::new(LaneKind::custom("annotation"), "歌かな");
        lane.segments.push(TimedSegment {
            range: TimeRange::new(LyricTime::ZERO, None),
            text_range: 0..3,
        });
        assert!(lane.has_valid_text_ranges());

        lane.segments[0].text_range = 0..1;
        assert!(!lane.has_valid_text_ranges());
    }

    #[test]
    fn primary_track_prefers_nonempty_original_then_first_nonempty_track() {
        let empty_original = LyricTrack::new(LaneKind::Original);
        let sidecar_kind = LaneKind::custom("sidecar");
        let mut sidecar_track = LyricTrack::new(sidecar_kind.clone());
        sidecar_track.lines.push(LyricLine {
            id: LineId::new(1),
            range: TimeRange::new(LyricTime::ZERO, None),
            lanes: Vec::new(),
        });
        let mut document = LyricsDocument {
            tracks: vec![empty_original, sidecar_track],
            ..LyricsDocument::default()
        };

        assert_eq!(
            document.primary_track().map(|track| &track.kind),
            Some(&sidecar_kind)
        );
        assert_eq!(
            document.primary_track_mut().map(|track| &track.kind),
            Some(&sidecar_kind)
        );
        assert!(
            LyricsDocument {
                tracks: vec![LyricTrack::new(LaneKind::Original)],
                ..LyricsDocument::default()
            }
            .primary_track()
            .is_none()
        );
    }

    #[test]
    fn custom_lanes_and_unknown_metadata_are_retained() {
        let kind = LaneKind::custom("annotation");
        assert_eq!(kind, LaneKind::Other(Arc::from("annotation")));

        let mut metadata = LyricsMetadata::default();
        metadata.unknown.push(MetadataTag::new("x-vendor", "value"));
        assert_eq!(metadata.value("X-VENDOR"), Some("value"));
    }
}
