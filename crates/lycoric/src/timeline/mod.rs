mod frame;
mod index;

use std::ops::Range;

use crate::{
    model::{
        LaneKind,
        LyricTrack,
        LyricsDocument,
    },
    time::LyricTime,
};

pub use frame::{
    ActiveLine,
    SegmentProgress,
    TimelineFrame,
    TimelineLine,
};
use index::LineIndex;

#[derive(Clone, Debug, Default)]
pub struct Timeline {
    index: LineIndex,
    events: Vec<LyricTime>,
}

impl Timeline {
    pub fn new(document: &LyricsDocument) -> Self {
        let selected = document
            .tracks
            .iter()
            .position(|track| track.kind == LaneKind::Original && !track.lines.is_empty())
            .or_else(|| {
                document
                    .tracks
                    .iter()
                    .position(|track| !track.lines.is_empty())
            });
        selected.map_or_else(Self::default, |track_index| {
            Self::build(&document.tracks[track_index], track_index)
        })
    }

    pub fn from_document_track(document: &LyricsDocument, track_index: usize) -> Option<Self> {
        document
            .tracks
            .get(track_index)
            .map(|track| Self::build(track, track_index))
    }

    pub fn from_track(track: &LyricTrack) -> Self {
        Self::build(track, 0)
    }

    pub fn len(&self) -> usize {
        self.index.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn lines(&self) -> impl ExactSizeIterator<Item = &TimelineLine> {
        self.index.lines()
    }

    pub fn frame_at(&self, position: LyricTime) -> TimelineFrame {
        let active_lines = self.index.active_at(position);
        let active_line = active_lines.last().cloned();
        let upper = self.index.upper_bound(position);
        let (previous_index, next_index) = active_line.as_ref().map_or_else(
            || (upper.checked_sub(1), Some(upper)),
            |active| {
                (
                    active.line.timeline_index.checked_sub(1),
                    Some(active.line.timeline_index + 1),
                )
            },
        );

        TimelineFrame {
            position,
            active_lines,
            active_line,
            previous_line: previous_index
                .and_then(|index| self.index.line(index))
                .cloned(),
            next_line: next_index.and_then(|index| self.index.line(index)).cloned(),
            next_event: self.next_event_after(position),
        }
    }

    pub fn visible_range(&self, position: LyricTime, before: usize, after: usize) -> Range<usize> {
        if self.is_empty() {
            return 0..0;
        }
        let upper = self.index.upper_bound(position);
        let anchor = upper.saturating_sub(1).min(self.len() - 1);
        let (first_active, last_active) = self
            .index
            .active_index_bounds(position)
            .unwrap_or((anchor, anchor));
        let start = first_active.saturating_sub(before);
        let end = last_active
            .saturating_add(after)
            .saturating_add(1)
            .min(self.len());
        start..end
    }

    pub fn next_event_after(&self, position: LyricTime) -> Option<LyricTime> {
        let index = self.events.partition_point(|event| *event <= position);
        self.events.get(index).copied()
    }

    pub fn previous_event_at_or_before(&self, position: LyricTime) -> Option<LyricTime> {
        let index = self.events.partition_point(|event| *event <= position);
        index
            .checked_sub(1)
            .and_then(|index| self.events.get(index))
            .copied()
    }

    fn build(track: &LyricTrack, track_index: usize) -> Self {
        let mut events = Vec::new();
        for line in &track.lines {
            events.push(line.range.start);
            events.extend(line.range.end);
            for lane in &line.lanes {
                for segment in &lane.segments {
                    events.push(segment.range.start);
                    events.extend(segment.range.end);
                }
            }
        }
        events.sort_unstable();
        events.dedup();
        Self {
            index: LineIndex::new(track, track_index),
            events,
        }
    }
}
