use crate::{
    diagnostics::{
        Diagnostic,
        DiagnosticCode,
    },
    model::{
        LyricLane,
        LyricLine,
        LyricTrack,
        LyricsDocument,
        LyricsMetadata,
        TimedSegment,
    },
    time::{
        LyricTime,
        TimeRange,
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchStrategy {
    ExactTimestamp,
    TimestampTolerance { tolerance: LyricTime },
    ExplicitLineId,
    Ordered,
    KeepSeparate,
}

pub type MergeStrategy = MatchStrategy;
pub type SidecarMatchStrategy = MatchStrategy;
pub type VariantMatchStrategy = MatchStrategy;

#[derive(Clone, Debug)]
pub struct AssemblyReport {
    pub document: LyricsDocument,
    pub diagnostics: Vec<Diagnostic>,
}

impl AssemblyReport {
    pub fn into_document(self) -> LyricsDocument {
        self.document
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VariantAssembler {
    strategy: MatchStrategy,
}

impl VariantAssembler {
    pub const fn new(strategy: MatchStrategy) -> Self {
        Self { strategy }
    }

    pub const fn strategy(&self) -> MatchStrategy {
        self.strategy
    }

    pub fn merge(&self, mut primary: LyricsDocument, sidecar: LyricsDocument) -> AssemblyReport {
        let diagnostics = self.merge_into(&mut primary, sidecar);
        AssemblyReport {
            document: primary,
            diagnostics,
        }
    }

    pub fn assemble<I>(&self, mut primary: LyricsDocument, sidecars: I) -> AssemblyReport
    where
        I: IntoIterator<Item = LyricsDocument>,
    {
        let mut diagnostics = Vec::new();
        for sidecar in sidecars {
            diagnostics.extend(self.merge_into(&mut primary, sidecar));
        }
        AssemblyReport {
            document: primary,
            diagnostics,
        }
    }

    pub fn merge_into(
        &self, primary: &mut LyricsDocument, sidecar: LyricsDocument,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        if let MatchStrategy::TimestampTolerance { tolerance } = self.strategy
            && tolerance < LyricTime::ZERO
        {
            diagnostics.push(Diagnostic::warning(
                DiagnosticCode::InvalidRange,
                "negative timestamp tolerance was treated as zero",
            ));
        }
        merge_metadata(&mut primary.metadata, &sidecar.metadata);
        primary.duration = later_time(primary.duration, sidecar.duration);

        if self.strategy == MatchStrategy::KeepSeparate || primary.tracks.is_empty() {
            primary.tracks.extend(sidecar.tracks);
            return diagnostics;
        }

        let target_index = primary
            .tracks
            .iter()
            .position(|track| track.kind == crate::model::LaneKind::Original)
            .unwrap_or(0);
        for sidecar_track in sidecar.tracks {
            let unmatched = merge_track(
                &mut primary.tracks[target_index],
                sidecar_track,
                self.strategy,
                &mut diagnostics,
            );
            if let Some(unmatched) = unmatched {
                primary.tracks.push(unmatched);
            }
        }
        diagnostics
    }
}

fn merge_track(
    target: &mut LyricTrack, sidecar: LyricTrack, strategy: MatchStrategy,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<LyricTrack> {
    let LyricTrack {
        language,
        kind,
        lines,
    } = sidecar;
    let mut unmatched = Vec::new();

    for (ordered_index, sidecar_line) in lines.into_iter().enumerate() {
        match find_match(&target.lines, &sidecar_line, strategy, ordered_index) {
            MatchResult::Found {
                index,
                fuzzy_distance,
            } => {
                if let Some(distance) = fuzzy_distance {
                    diagnostics.push(Diagnostic::warning(
                        DiagnosticCode::FuzzyMatch,
                        format!(
                            "sidecar line {} matched within a {}µs timestamp tolerance",
                            sidecar_line.id.get(),
                            distance
                        ),
                    ));
                }
                append_lanes(
                    &mut target.lines[index],
                    sidecar_line,
                    strategy,
                    diagnostics,
                );
            }
            MatchResult::Ambiguous => {
                diagnostics.push(Diagnostic::warning(
                    DiagnosticCode::AmbiguousMatch,
                    format!(
                        "sidecar line {} matched more than one primary line and was kept separate",
                        sidecar_line.id.get()
                    ),
                ));
                unmatched.push(sidecar_line);
            }
            MatchResult::Missing => {
                diagnostics.push(Diagnostic::warning(
                    DiagnosticCode::UnmatchedSidecarLine,
                    format!(
                        "sidecar line {} had no safe primary match and was kept separate",
                        sidecar_line.id.get()
                    ),
                ));
                unmatched.push(sidecar_line);
            }
        }
    }

    (!unmatched.is_empty()).then_some(LyricTrack {
        language,
        kind,
        lines: unmatched,
    })
}

fn append_lanes(
    target: &mut LyricLine, mut sidecar: LyricLine, strategy: MatchStrategy,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let target_id = target.id;
    let target_range = target.range;
    if strategy == MatchStrategy::Ordered {
        let delta = time_delta(
            target_range.start,
            sidecar.range.start,
            sidecar.id.get(),
            diagnostics,
        );
        for lane in &mut sidecar.lanes {
            shift_segments(&mut lane.segments, delta, sidecar.id.get(), diagnostics);
        }
    }

    for mut lane in sidecar.lanes {
        normalize_segments_to_line(&mut lane, target_range, target_id.get(), diagnostics);
        let conflicts = target
            .lanes
            .iter()
            .any(|existing| existing.kind == lane.kind && existing.language == lane.language);
        if conflicts {
            diagnostics.push(Diagnostic::warning(
                DiagnosticCode::LaneConflict,
                format!(
                    "line {} already contained this lane kind and language; both lanes were preserved",
                    target.id.get()
                ),
            ));
        }
        target.lanes.push(lane);
    }
}

fn time_delta(
    target_start: LyricTime, sidecar_start: LyricTime, sidecar_id: u64,
    diagnostics: &mut Vec<Diagnostic>,
) -> LyricTime {
    target_start.checked_sub(sidecar_start).unwrap_or_else(|| {
        diagnostics.push(Diagnostic::warning(
            DiagnosticCode::TimeOverflow,
            format!("sidecar line {sidecar_id} start delta overflowed and was saturated"),
        ));
        target_start.saturating_sub(sidecar_start)
    })
}

fn shift_segments(
    segments: &mut [TimedSegment], delta: LyricTime, sidecar_id: u64,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for segment in segments {
        segment.range.start =
            shift_segment_time(segment.range.start, delta, sidecar_id, diagnostics);
        segment.range.end = segment
            .range
            .end
            .map(|end| shift_segment_time(end, delta, sidecar_id, diagnostics));
    }
}

fn shift_segment_time(
    time: LyricTime, delta: LyricTime, sidecar_id: u64, diagnostics: &mut Vec<Diagnostic>,
) -> LyricTime {
    time.checked_add(delta).unwrap_or_else(|| {
        diagnostics.push(Diagnostic::warning(
            DiagnosticCode::TimeOverflow,
            format!("enhanced timing on sidecar line {sidecar_id} overflowed and was saturated"),
        ));
        time.saturating_add(delta)
    })
}

fn normalize_segments_to_line(
    lane: &mut LyricLane, target: TimeRange, target_id: u64, diagnostics: &mut Vec<Diagnostic>,
) {
    let target = if target.is_valid() {
        target
    } else {
        diagnostics.push(Diagnostic::warning(
            DiagnosticCode::InvalidRange,
            format!("target line {target_id} had an invalid range; its end was clamped"),
        ));
        TimeRange::new(target.start, Some(target.start))
    };

    for (segment_index, segment) in lane.segments.iter_mut().enumerate() {
        let original = segment.range;
        let outside_before = original
            .end
            .is_some_and(|end| end < target.start || (end == target.start && original.start < end));
        let outside_after = target.end.is_some_and(|end| original.start >= end);
        if outside_before || outside_after {
            segment.range = target;
            diagnostics.push(segment_range_diagnostic(target_id, segment_index));
            continue;
        }

        let start = original.start.max(target.start);
        let mut end = original.end.or(target.end);
        if let (Some(segment_end), Some(target_end)) = (end, target.end) {
            end = Some(segment_end.min(target_end));
        }
        if end.is_some_and(|end| end < start) {
            end = Some(start);
        }

        let normalized = TimeRange::new(start, end);
        if normalized != original {
            diagnostics.push(segment_range_diagnostic(target_id, segment_index));
            segment.range = normalized;
        }
    }
}

fn segment_range_diagnostic(target_id: u64, segment_index: usize) -> Diagnostic {
    Diagnostic::warning(
        DiagnosticCode::InvalidRange,
        format!(
            "enhanced segment {segment_index} had timing outside target line {target_id} and was normalized"
        ),
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MatchResult {
    Found {
        index: usize,
        fuzzy_distance: Option<u64>,
    },
    Ambiguous,
    Missing,
}

fn find_match(
    target: &[LyricLine], sidecar: &LyricLine, strategy: MatchStrategy, ordered_index: usize,
) -> MatchResult {
    match strategy {
        MatchStrategy::ExactTimestamp => unique_match(
            target
                .iter()
                .enumerate()
                .filter(|(_, line)| line.range.start == sidecar.range.start)
                .map(|(index, _)| (index, 0)),
            false,
        ),
        MatchStrategy::TimestampTolerance { tolerance } => {
            let tolerance = tolerance.as_micros().max(0) as u64;
            unique_match(
                target
                    .iter()
                    .enumerate()
                    .map(|(index, line)| (index, line.range.start.abs_diff(sidecar.range.start)))
                    .filter(|(_, distance)| *distance <= tolerance),
                true,
            )
        }
        MatchStrategy::ExplicitLineId => unique_match(
            target
                .iter()
                .enumerate()
                .filter(|(_, line)| line.id == sidecar.id)
                .map(|(index, _)| (index, 0)),
            false,
        ),
        MatchStrategy::Ordered => {
            target
                .get(ordered_index)
                .map_or(MatchResult::Missing, |_| MatchResult::Found {
                    index: ordered_index,
                    fuzzy_distance: None,
                })
        }
        MatchStrategy::KeepSeparate => MatchResult::Missing,
    }
}

fn unique_match(candidates: impl Iterator<Item = (usize, u64)>, report_fuzzy: bool) -> MatchResult {
    let mut best: Option<(usize, u64)> = None;
    let mut tied = false;
    for candidate in candidates {
        match best {
            None => best = Some(candidate),
            Some((_, best_distance)) if candidate.1 < best_distance => {
                best = Some(candidate);
                tied = false;
            }
            Some((_, best_distance)) if candidate.1 == best_distance => tied = true,
            Some(_) => {}
        }
    }

    match (best, tied) {
        (None, _) => MatchResult::Missing,
        (Some(_), true) => MatchResult::Ambiguous,
        (Some((index, distance)), false) => MatchResult::Found {
            index,
            fuzzy_distance: (report_fuzzy && distance != 0).then_some(distance),
        },
    }
}

fn merge_metadata(primary: &mut LyricsMetadata, sidecar: &LyricsMetadata) {
    primary.artist = primary.artist.clone().or_else(|| sidecar.artist.clone());
    primary.album = primary.album.clone().or_else(|| sidecar.album.clone());
    primary.title = primary.title.clone().or_else(|| sidecar.title.clone());
    primary.author = primary.author.clone().or_else(|| sidecar.author.clone());
    primary.creator = primary.creator.clone().or_else(|| sidecar.creator.clone());
    primary.editor = primary.editor.clone().or_else(|| sidecar.editor.clone());
    primary.version = primary.version.clone().or_else(|| sidecar.version.clone());
    primary.length = later_time(primary.length, sidecar.length);
    primary.unknown.extend(sidecar.unknown.iter().cloned());
}

fn later_time(left: Option<LyricTime>, right: Option<LyricTime>) -> Option<LyricTime> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::{
        model::{
            LaneKind,
            LyricLane,
        },
        time::TimeRange,
    };

    fn line(id: u64, seconds: i64, kind: LaneKind, text: &str) -> LyricLine {
        LyricLine {
            id: crate::model::LineId::new(id),
            range: TimeRange::new(
                LyricTime::from_secs(seconds),
                Some(LyricTime::from_secs(seconds + 1)),
            ),
            lanes: vec![LyricLane {
                kind,
                language: None,
                text: Arc::from(text),
                segments: Vec::new(),
            }],
        }
    }

    fn line_with_segment(
        id: u64, line_start_millis: i64, line_end_millis: i64, segment_start_millis: i64,
        segment_end_millis: Option<i64>, kind: LaneKind,
    ) -> LyricLine {
        let mut line = LyricLine {
            id: crate::model::LineId::new(id),
            range: TimeRange::new(
                LyricTime::from_millis(line_start_millis),
                Some(LyricTime::from_millis(line_end_millis)),
            ),
            lanes: vec![LyricLane::new(kind, "word")],
        };
        line.lanes[0].segments.push(TimedSegment {
            range: TimeRange::new(
                LyricTime::from_millis(segment_start_millis),
                segment_end_millis.map(LyricTime::from_millis),
            ),
            text_range: 0..4,
        });
        line
    }

    fn document(kind: LaneKind, lines: Vec<LyricLine>) -> LyricsDocument {
        LyricsDocument {
            metadata: LyricsMetadata::default(),
            tracks: vec![LyricTrack {
                language: None,
                kind,
                lines,
            }],
            duration: None,
        }
    }

    #[test]
    fn exact_timestamp_merges_a_sidecar_lane() {
        let primary = document(
            LaneKind::Original,
            vec![line(10, 1, LaneKind::Original, "original")],
        );
        let sidecar = document(
            LaneKind::custom("sidecar"),
            vec![line(20, 1, LaneKind::custom("sidecar"), "sidecar")],
        );
        let report = VariantAssembler::new(MatchStrategy::ExactTimestamp).merge(primary, sidecar);
        assert!(report.diagnostics.is_empty());
        assert_eq!(report.document.tracks.len(), 1);
        assert_eq!(report.document.tracks[0].lines[0].lanes.len(), 2);
    }

    #[test]
    fn tolerance_reports_fuzzy_matches() {
        let primary = document(
            LaneKind::Original,
            vec![line(1, 10, LaneKind::Original, "a")],
        );
        let mut sidecar_line = line(2, 10, LaneKind::custom("sidecar"), "b");
        sidecar_line.range.start += LyricTime::from_millis(20);
        let sidecar = document(LaneKind::custom("sidecar"), vec![sidecar_line]);
        let report = VariantAssembler::new(MatchStrategy::TimestampTolerance {
            tolerance: LyricTime::from_millis(50),
        })
        .merge(primary, sidecar);
        assert_eq!(report.diagnostics[0].code, DiagnosticCode::FuzzyMatch);
        assert_eq!(report.document.tracks[0].lines[0].lanes.len(), 2);
    }

    #[test]
    fn ambiguous_matches_remain_in_a_separate_track() {
        let primary = document(
            LaneKind::Original,
            vec![
                line(1, 1, LaneKind::Original, "a"),
                line(2, 1, LaneKind::Original, "b"),
            ],
        );
        let sidecar = document(
            LaneKind::custom("sidecar"),
            vec![line(3, 1, LaneKind::custom("sidecar"), "c")],
        );
        let report = VariantAssembler::new(MatchStrategy::ExactTimestamp).merge(primary, sidecar);
        assert_eq!(report.diagnostics[0].code, DiagnosticCode::AmbiguousMatch);
        assert_eq!(report.document.tracks.len(), 2);
        assert_eq!(
            report.document.tracks[1].lines[0].lanes[0].text.as_ref(),
            "c"
        );
    }

    #[test]
    fn ordered_merge_preserves_conflicting_lanes() {
        let mut primary_line = line(1, 1, LaneKind::Original, "a");
        primary_line
            .lanes
            .push(LyricLane::new(LaneKind::custom("sidecar"), "old"));
        let primary = document(LaneKind::Original, vec![primary_line]);
        let sidecar = document(
            LaneKind::custom("sidecar"),
            vec![line(2, 99, LaneKind::custom("sidecar"), "new")],
        );
        let report = VariantAssembler::new(MatchStrategy::Ordered).merge(primary, sidecar);
        assert_eq!(report.diagnostics[0].code, DiagnosticCode::LaneConflict);
        assert_eq!(report.document.tracks[0].lines[0].lanes.len(), 3);
    }

    #[test]
    fn ordered_merge_shifts_enhanced_timing_to_the_target_start() {
        let primary = document(
            LaneKind::Original,
            vec![line(1, 1, LaneKind::Original, "a")],
        );
        let sidecar = document(
            LaneKind::custom("sidecar"),
            vec![line_with_segment(
                2,
                99_000,
                100_000,
                99_200,
                Some(99_800),
                LaneKind::custom("sidecar"),
            )],
        );

        let report = VariantAssembler::new(MatchStrategy::Ordered).merge(primary, sidecar);
        let segment = &report.document.tracks[0].lines[0].lanes[1].segments[0];
        assert_eq!(segment.range.start, LyricTime::from_millis(1_200));
        assert_eq!(segment.range.end, Some(LyricTime::from_millis(1_800)));
        assert!(
            report
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.code != DiagnosticCode::InvalidRange)
        );
    }

    #[test]
    fn matched_sidecar_segments_are_normalized_for_every_matching_strategy() {
        let cases = [
            (
                MatchStrategy::ExactTimestamp,
                line_with_segment(
                    2,
                    10_000,
                    11_000,
                    9_000,
                    Some(12_000),
                    LaneKind::custom("sidecar"),
                ),
            ),
            (
                MatchStrategy::TimestampTolerance {
                    tolerance: LyricTime::from_millis(50),
                },
                line_with_segment(
                    2,
                    10_020,
                    11_020,
                    9_000,
                    Some(12_000),
                    LaneKind::custom("sidecar"),
                ),
            ),
            (
                MatchStrategy::ExplicitLineId,
                line_with_segment(
                    1,
                    99_000,
                    100_000,
                    99_000,
                    Some(100_000),
                    LaneKind::custom("sidecar"),
                ),
            ),
        ];

        for (strategy, sidecar_line) in cases {
            let primary = document(
                LaneKind::Original,
                vec![line(1, 10, LaneKind::Original, "a")],
            );
            let sidecar = document(LaneKind::custom("sidecar"), vec![sidecar_line]);
            let report = VariantAssembler::new(strategy).merge(primary, sidecar);
            let target = &report.document.tracks[0].lines[0];
            let segment = &target.lanes[1].segments[0];
            assert_eq!(segment.range, target.range, "strategy: {strategy:?}");
            assert!(
                report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == DiagnosticCode::InvalidRange),
                "strategy: {strategy:?}"
            );
        }
    }

    #[test]
    fn keep_separate_is_lossless_and_quiet() {
        let primary = document(
            LaneKind::Original,
            vec![line(1, 1, LaneKind::Original, "a")],
        );
        let sidecar = document(
            LaneKind::custom("annotation"),
            vec![line(2, 1, LaneKind::custom("annotation"), "annotation")],
        );
        let report = VariantAssembler::new(MatchStrategy::KeepSeparate).merge(primary, sidecar);
        assert!(report.diagnostics.is_empty());
        assert_eq!(report.document.tracks.len(), 2);
    }

    #[test]
    fn negative_tolerance_is_not_silent() {
        let primary = document(
            LaneKind::Original,
            vec![line(1, 1, LaneKind::Original, "a")],
        );
        let sidecar = document(
            LaneKind::custom("sidecar"),
            vec![line(2, 1, LaneKind::custom("sidecar"), "b")],
        );
        let report = VariantAssembler::new(MatchStrategy::TimestampTolerance {
            tolerance: LyricTime::from_millis(-1),
        })
        .merge(primary, sidecar);
        assert_eq!(report.diagnostics[0].code, DiagnosticCode::InvalidRange);
    }
}
