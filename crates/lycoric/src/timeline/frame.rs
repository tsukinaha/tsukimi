use crate::{
    model::LineId,
    time::{
        LyricTime,
        TimeRange,
    },
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimelineLine {
    pub timeline_index: usize,
    pub track_index: usize,
    pub line_index: usize,
    pub id: LineId,
    pub range: TimeRange,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SegmentProgress {
    pub lane_index: usize,
    pub segment_index: usize,
    pub range: TimeRange,
    pub progress: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActiveLine {
    pub line: TimelineLine,
    pub active_segments: Vec<SegmentProgress>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimelineFrame {
    pub position: LyricTime,
    pub active_lines: Vec<ActiveLine>,
    pub active_line: Option<ActiveLine>,
    pub previous_line: Option<TimelineLine>,
    pub next_line: Option<TimelineLine>,
    pub next_event: Option<LyricTime>,
}

impl TimelineFrame {
    pub fn is_gap(&self) -> bool {
        self.active_lines.is_empty()
    }

    pub fn bounded_gap(&self) -> Option<TimeRange> {
        if !self.is_gap() {
            return None;
        }
        let next = self.next_line.as_ref()?;
        let start = self
            .previous_line
            .as_ref()
            .and_then(|line| line.range.end)
            .unwrap_or(LyricTime::ZERO);
        let range = TimeRange::new(start, Some(next.range.start));
        (range.is_valid() && range.contains(self.position)).then_some(range)
    }

    pub fn active_line_id(&self) -> Option<LineId> {
        self.active_line.as_ref().map(|active| active.line.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(index: usize, start_us: i64, end_us: Option<i64>) -> TimelineLine {
        TimelineLine {
            timeline_index: index,
            track_index: 0,
            line_index: index,
            id: LineId::new(index as u64),
            range: TimeRange::new(
                LyricTime::from_micros(start_us),
                end_us.map(LyricTime::from_micros),
            ),
        }
    }

    fn gap_frame(
        position_us: i64, previous: Option<TimelineLine>, next: Option<TimelineLine>,
    ) -> TimelineFrame {
        TimelineFrame {
            position: LyricTime::from_micros(position_us),
            active_lines: Vec::new(),
            active_line: None,
            previous_line: previous,
            next_line: next,
            next_event: None,
        }
    }

    #[test]
    fn bounded_gap_supports_intro_and_uses_previous_end() {
        let intro = gap_frame(500_000, None, Some(line(0, 2_000_000, Some(3_000_000))));
        assert_eq!(
            intro.bounded_gap(),
            Some(TimeRange::new(
                LyricTime::ZERO,
                Some(LyricTime::from_micros(2_000_000))
            ))
        );

        let between = gap_frame(
            2_500_000,
            Some(line(0, 0, Some(2_000_000))),
            Some(line(1, 4_000_000, Some(5_000_000))),
        );
        assert_eq!(
            between.bounded_gap(),
            Some(TimeRange::new(
                LyricTime::from_micros(2_000_000),
                Some(LyricTime::from_micros(4_000_000))
            ))
        );
    }

    #[test]
    fn bounded_gap_requires_a_next_line_and_gap_position() {
        let outro = gap_frame(3_000_000, Some(line(0, 0, Some(2_000_000))), None);
        assert_eq!(outro.bounded_gap(), None);

        let before_intro = gap_frame(-1, None, Some(line(0, 2_000_000, None)));
        assert_eq!(before_intro.bounded_gap(), None);
    }
}
