use crate::{
    model::{
        LyricLine,
        LyricTrack,
    },
    time::{
        LyricTime,
        TimeRange,
    },
};

use super::frame::{
    ActiveLine,
    SegmentProgress,
    TimelineLine,
};

#[derive(Clone, Debug, Default)]
pub(super) struct LineIndex {
    entries: Vec<IndexedLine>,
    max_end_tree: Vec<EndBound>,
}

impl LineIndex {
    pub(super) fn new(track: &LyricTrack, track_index: usize) -> Self {
        let mut entries: Vec<_> = track
            .lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.has_visible_text())
            .map(|(line_index, line)| IndexedLine::new(line, track_index, line_index))
            .collect();
        entries.sort_by(|left, right| {
            left.summary
                .range
                .start
                .cmp(&right.summary.range.start)
                .then(left.summary.line_index.cmp(&right.summary.line_index))
        });
        for (timeline_index, entry) in entries.iter_mut().enumerate() {
            entry.summary.timeline_index = timeline_index;
        }

        let mut index = Self {
            max_end_tree: vec![EndBound::default(); entries.len().saturating_mul(4)],
            entries,
        };
        if !index.entries.is_empty() {
            index.build_tree(1, 0, index.entries.len());
        }
        index
    }

    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(super) fn lines(&self) -> impl ExactSizeIterator<Item = &TimelineLine> {
        self.entries.iter().map(|entry| &entry.summary)
    }

    pub(super) fn line(&self, index: usize) -> Option<&TimelineLine> {
        self.entries.get(index).map(|entry| &entry.summary)
    }

    pub(super) fn active_at(&self, position: LyricTime) -> Vec<ActiveLine> {
        self.active_indices_at(position)
            .into_iter()
            .map(|index| self.entries[index].active_at(position))
            .collect()
    }

    pub(super) fn active_index_bounds(&self, position: LyricTime) -> Option<(usize, usize)> {
        let indices = self.active_indices_at(position);
        Some((*indices.first()?, *indices.last()?))
    }

    fn active_indices_at(&self, position: LyricTime) -> Vec<usize> {
        if self.entries.is_empty() {
            return Vec::new();
        }
        let mut indices = Vec::new();
        self.query_tree(1, 0, self.entries.len(), position, &mut indices);
        indices
    }

    pub(super) fn upper_bound(&self, position: LyricTime) -> usize {
        self.entries
            .partition_point(|entry| entry.summary.range.start <= position)
    }

    fn build_tree(&mut self, node: usize, left: usize, right: usize) -> EndBound {
        if right - left == 1 {
            let bound = EndBound::from_range(self.entries[left].summary.range);
            self.max_end_tree[node] = bound;
            return bound;
        }
        let middle = left + (right - left) / 2;
        let left_bound = self.build_tree(node * 2, left, middle);
        let right_bound = self.build_tree(node * 2 + 1, middle, right);
        let bound = left_bound.union(right_bound);
        self.max_end_tree[node] = bound;
        bound
    }

    fn query_tree(
        &self, node: usize, left: usize, right: usize, position: LyricTime, output: &mut Vec<usize>,
    ) {
        if self.entries[left].summary.range.start > position
            || !self.max_end_tree[node].extends_past(position)
        {
            return;
        }
        if right - left == 1 {
            if self.entries[left].summary.range.contains(position) {
                output.push(left);
            }
            return;
        }

        let middle = left + (right - left) / 2;
        self.query_tree(node * 2, left, middle, position, output);
        if middle < right && self.entries[middle].summary.range.start <= position {
            self.query_tree(node * 2 + 1, middle, right, position, output);
        }
    }
}

#[derive(Clone, Debug)]
struct IndexedLine {
    summary: TimelineLine,
    lanes: Vec<Vec<IndexedSegment>>,
}

impl IndexedLine {
    fn new(line: &LyricLine, track_index: usize, line_index: usize) -> Self {
        let lanes = line
            .lanes
            .iter()
            .map(|lane| {
                let mut segments: Vec<_> = lane
                    .segments
                    .iter()
                    .enumerate()
                    .map(|(segment_index, segment)| IndexedSegment {
                        segment_index,
                        range: segment.range,
                    })
                    .collect();
                segments.sort_by(|left, right| {
                    left.range
                        .start
                        .cmp(&right.range.start)
                        .then(left.segment_index.cmp(&right.segment_index))
                });
                segments
            })
            .collect();
        Self {
            summary: TimelineLine {
                timeline_index: 0,
                track_index,
                line_index,
                id: line.id,
                range: line.range,
            },
            lanes,
        }
    }

    fn active_at(&self, position: LyricTime) -> ActiveLine {
        let active_segments = self
            .lanes
            .iter()
            .enumerate()
            .filter_map(|(lane_index, segments)| {
                active_segment(segments, position).map(|segment| SegmentProgress {
                    lane_index,
                    segment_index: segment.segment_index,
                    range: segment.range,
                    progress: segment_progress(segment.range, position),
                })
            })
            .collect();
        ActiveLine {
            line: self.summary.clone(),
            active_segments,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct IndexedSegment {
    segment_index: usize,
    range: TimeRange,
}

fn active_segment(segments: &[IndexedSegment], position: LyricTime) -> Option<&IndexedSegment> {
    let upper = segments.partition_point(|segment| segment.range.start <= position);
    segments[..upper]
        .iter()
        .rev()
        .find(|segment| segment_is_active(segment.range, position))
}

fn segment_is_active(range: TimeRange, position: LyricTime) -> bool {
    range.contains(position) || (range.end == Some(range.start) && position == range.start)
}

fn segment_progress(range: TimeRange, position: LyricTime) -> f64 {
    let Some(end) = range.end else {
        return 0.0;
    };
    let duration = end.as_micros().saturating_sub(range.start.as_micros());
    if duration <= 0 {
        return 1.0;
    }
    let elapsed = position.as_micros().saturating_sub(range.start.as_micros());
    (elapsed as f64 / duration as f64).clamp(0.0, 1.0)
}

#[derive(Clone, Copy, Debug, Default)]
struct EndBound {
    latest: LyricTime,
    open: bool,
}

impl EndBound {
    fn from_range(range: TimeRange) -> Self {
        match range.end {
            Some(end) => Self {
                latest: end,
                open: false,
            },
            None => Self {
                latest: LyricTime::MAX,
                open: true,
            },
        }
    }

    fn union(self, other: Self) -> Self {
        Self {
            latest: self.latest.max(other.latest),
            open: self.open || other.open,
        }
    }

    fn extends_past(self, position: LyricTime) -> bool {
        self.open || self.latest > position
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::model::{
        LaneKind,
        LineId,
        LyricLane,
        TimedSegment,
    };

    fn line(id: u64, start: i64, end: Option<i64>) -> LyricLine {
        LyricLine {
            id: LineId::new(id),
            range: TimeRange::new(LyricTime::from_secs(start), end.map(LyricTime::from_secs)),
            lanes: vec![LyricLane {
                kind: LaneKind::Original,
                language: None,
                text: Arc::from("word"),
                segments: Vec::new(),
            }],
        }
    }

    #[test]
    fn interval_index_finds_overlaps_and_open_ranges() {
        let track = LyricTrack {
            language: None,
            kind: LaneKind::Original,
            lines: vec![line(1, 1, Some(5)), line(2, 2, Some(3)), line(3, 9, None)],
        };
        let index = LineIndex::new(&track, 0);
        assert_eq!(index.active_at(LyricTime::from_secs(2)).len(), 2);
        assert_eq!(
            index.active_at(LyricTime::from_secs(4))[0].line.id,
            LineId::new(1)
        );
        assert_eq!(index.active_at(LyricTime::MAX)[0].line.id, LineId::new(3));
    }

    #[test]
    fn ended_overlapping_segment_does_not_mask_an_earlier_active_segment() {
        let mut timed = line(1, 1, Some(6));
        timed.lanes[0].segments = vec![
            TimedSegment {
                range: TimeRange::new(LyricTime::from_secs(1), Some(LyricTime::from_secs(5))),
                text_range: 0..2,
            },
            TimedSegment {
                range: TimeRange::new(LyricTime::from_secs(2), Some(LyricTime::from_secs(3))),
                text_range: 2..4,
            },
        ];
        let track = LyricTrack {
            language: None,
            kind: LaneKind::Original,
            lines: vec![timed],
        };

        let active = LineIndex::new(&track, 0).active_at(LyricTime::from_secs(4));
        assert_eq!(active[0].active_segments[0].segment_index, 0);
        assert_eq!(active[0].active_segments[0].progress, 0.75);
    }

    #[test]
    fn open_and_zero_duration_segments_have_stable_progress_semantics() {
        assert_eq!(
            segment_progress(
                TimeRange::new(LyricTime::from_secs(1), None),
                LyricTime::from_secs(2)
            ),
            0.0
        );
        assert_eq!(
            segment_progress(
                TimeRange::new(LyricTime::from_secs(2), Some(LyricTime::from_secs(2)),),
                LyricTime::from_secs(2),
            ),
            1.0
        );

        let segments = [IndexedSegment {
            segment_index: 7,
            range: TimeRange::new(LyricTime::from_secs(2), Some(LyricTime::from_secs(2))),
        }];
        assert_eq!(
            active_segment(&segments, LyricTime::from_secs(2)).map(|segment| segment.segment_index),
            Some(7)
        );
    }

    #[test]
    fn computes_segment_progress() {
        let mut timed = line(1, 1, Some(4));
        timed.lanes[0].segments.push(TimedSegment {
            range: TimeRange::new(LyricTime::from_secs(1), Some(LyricTime::from_secs(3))),
            text_range: 0..4,
        });
        let track = LyricTrack {
            language: None,
            kind: LaneKind::Original,
            lines: vec![timed],
        };
        let active = LineIndex::new(&track, 0).active_at(LyricTime::from_secs(2));
        assert_eq!(active[0].active_segments[0].progress, 0.5);
    }
}
