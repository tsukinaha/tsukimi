use std::{
    mem,
    ops::Range,
    sync::Arc,
};

use crate::{
    diagnostics::{
        Diagnostic,
        DiagnosticCode,
        DiagnosticSeverity,
        SourceSpan,
    },
    model::{
        LineId,
        LyricLane,
        LyricLine,
        LyricTrack,
        LyricsDocument,
        LyricsMetadata,
        MetadataTag,
        TimedSegment,
    },
    time::{
        LyricTime,
        TimeRange,
    },
};

use super::{
    ParseMode,
    ParseOptions,
    ParseReport,
};

pub fn parse_lrc(input: &str, options: &ParseOptions) -> ParseReport {
    LrcParser::new(options).parse(input)
}

struct LrcParser<'a> {
    options: &'a ParseOptions,
    diagnostics: Vec<Diagnostic>,
    metadata: LyricsMetadata,
    raw_lines: Vec<RawLine>,
    next_line_id: u64,
    source_order: usize,
    previous_source_start: Option<LyricTime>,
}

impl<'a> LrcParser<'a> {
    fn new(options: &'a ParseOptions) -> Self {
        Self {
            options,
            diagnostics: Vec::new(),
            metadata: LyricsMetadata::default(),
            raw_lines: Vec::new(),
            next_line_id: 0,
            source_order: 0,
            previous_source_start: None,
        }
    }

    fn parse(mut self, input: &str) -> ParseReport {
        self.parse_lines(input);
        let duration = self.document_duration();
        let mut lines = self.build_lines();
        self.infer_line_ends(&mut lines, duration);
        self.normalize_segment_ranges(&mut lines);
        let duration = self.normalized_document_duration(duration, &lines);

        let track = LyricTrack {
            language: self.options.language.clone(),
            kind: self.options.lane_kind.clone(),
            lines: lines.into_iter().map(|line| line.line).collect(),
        };
        let document = LyricsDocument {
            metadata: self.metadata,
            tracks: vec![track],
            duration,
        };
        let has_errors = self
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error);

        ParseReport {
            document: (!has_errors).then_some(document),
            diagnostics: self.diagnostics,
        }
    }

    fn parse_lines(&mut self, input: &str) {
        let (input, bom_len) = input
            .strip_prefix('\u{feff}')
            .map_or((input, 0), |stripped| (stripped, '\u{feff}'.len_utf8()));
        let mut byte_offset = bom_len;

        for (index, chunk) in input.split_inclusive('\n').enumerate() {
            let line = chunk
                .strip_suffix('\n')
                .unwrap_or(chunk)
                .strip_suffix('\r')
                .unwrap_or_else(|| chunk.strip_suffix('\n').unwrap_or(chunk));
            self.parse_source_line(
                line,
                SourceLine {
                    number: index + 1,
                    byte_offset,
                },
            );
            byte_offset += chunk.len();
        }
    }

    fn parse_source_line(&mut self, line: &str, source: SourceLine) {
        if line.is_empty() {
            return;
        }

        let mut cursor = 0;
        let mut timestamps = Vec::new();

        while line[cursor..].starts_with('[') {
            let Some(relative_end) = line[cursor + 1..].find(']') else {
                let span = source.span(line, cursor, line.len());
                self.recover(DiagnosticCode::MalformedTag, "unclosed LRC tag", Some(span));
                break;
            };
            let end = cursor + 1 + relative_end;
            let tag_span = source.span(line, cursor, end + 1);
            let contents = &line[cursor + 1..end];
            if !timestamps.is_empty() && is_inline_variant_tag(contents) {
                break;
            }

            match parse_timestamp(contents) {
                Ok(timestamp) => timestamps.push((timestamp, tag_span)),
                Err(error) if looks_like_timestamp(contents) => {
                    self.recover(
                        DiagnosticCode::InvalidTimestamp,
                        format!("invalid line timestamp: {error}"),
                        Some(tag_span),
                    );
                }
                Err(_) if contents.contains(':') => {
                    self.parse_metadata(contents, tag_span);
                }
                Err(_) => {
                    self.recover(
                        DiagnosticCode::MalformedTag,
                        format!("unrecognized LRC tag [{contents}]"),
                        Some(tag_span),
                    );
                    if !timestamps.is_empty() {
                        break;
                    }
                }
            }
            cursor = end + 1;
        }

        if timestamps.is_empty() {
            if !line[cursor..].trim().is_empty() {
                self.recover(
                    DiagnosticCode::MissingTimestamp,
                    "lyric text has no valid line timestamp",
                    Some(source.span(line, cursor, line.len())),
                );
            }
            return;
        }

        let raw_body = &line[cursor..];
        let (variant_offset, body) = unwrap_inline_variant(raw_body).unwrap_or((0, raw_body));
        let body_start = cursor + variant_offset;
        let (text, segments) = self.scan_enhanced_text(body, source, line, body_start);
        self.push_timestamped_lines(timestamps, text, segments);
    }

    fn parse_metadata(&mut self, contents: &str, span: SourceSpan) {
        let Some((raw_key, raw_value)) = contents.split_once(':') else {
            self.recover(
                DiagnosticCode::InvalidMetadata,
                "metadata tag has no value separator",
                Some(span),
            );
            return;
        };
        let key = raw_key.trim();
        if key.is_empty() {
            self.recover(
                DiagnosticCode::InvalidMetadata,
                "metadata key cannot be empty",
                Some(span),
            );
            return;
        }
        let normalized_key = key.to_ascii_lowercase();
        let value = raw_value.trim();
        let value: Arc<str> = Arc::from(value);

        match normalized_key.as_str() {
            "ar" => self.metadata.artist = Some(value),
            "al" => self.metadata.album = Some(value),
            "ti" => self.metadata.title = Some(value),
            "au" => self.metadata.author = Some(value),
            "by" => self.metadata.creator = Some(value),
            "re" => self.metadata.editor = Some(value),
            "ve" => self.metadata.version = Some(value),
            "length" => self.parse_length(value.as_ref(), span),
            "offset" => self.parse_offset(value.as_ref(), span),
            _ => {
                self.metadata
                    .unknown
                    .push(MetadataTag::new(key, value.clone()));
                self.diagnostics.push(
                    Diagnostic::warning(
                        DiagnosticCode::UnknownMetadata,
                        format!("unknown metadata tag [{key}:...] was preserved"),
                    )
                    .with_span(span),
                );
            }
        }
    }

    fn parse_length(&mut self, value: &str, span: SourceSpan) {
        match parse_timestamp(value) {
            Ok(length) => self.metadata.length = Some(length),
            Err(error) => self.recover(
                DiagnosticCode::InvalidMetadata,
                format!("invalid length metadata: {error}"),
                Some(span),
            ),
        }
    }

    fn parse_offset(&mut self, value: &str, span: SourceSpan) {
        let offset = value
            .parse::<i64>()
            .ok()
            .and_then(|milliseconds| milliseconds.checked_mul(1_000));
        match offset {
            Some(offset) => self.metadata.offset = LyricTime::from_micros(offset),
            None => self.recover(
                DiagnosticCode::InvalidOffset,
                "offset must be a signed integer number of milliseconds",
                Some(span),
            ),
        }
    }

    fn scan_enhanced_text(
        &mut self, body: &str, source: SourceLine, complete_line: &str, body_start: usize,
    ) -> (Arc<str>, Vec<RawSegment>) {
        let mut text = String::with_capacity(body.len());
        let mut markers = Vec::new();
        let mut cursor = 0;

        while let Some(relative_start) = body[cursor..].find('<') {
            let start = cursor + relative_start;
            text.push_str(&body[cursor..start]);
            let Some(relative_end) = body[start + 1..].find('>') else {
                if looks_like_timestamp(&body[start + 1..]) {
                    self.recover(
                        DiagnosticCode::MalformedTag,
                        "unclosed enhanced word timestamp",
                        Some(source.span(
                            complete_line,
                            body_start + start,
                            body_start + body.len(),
                        )),
                    );
                }
                text.push_str(&body[start..]);
                cursor = body.len();
                break;
            };
            let end = start + 1 + relative_end;
            let contents = &body[start + 1..end];
            let span = source.span(complete_line, body_start + start, body_start + end + 1);

            match parse_timestamp(contents) {
                Ok(mut time) => {
                    if let Some(previous) = markers.last().map(|marker: &WordMarker| marker.time)
                        && time < previous
                    {
                        self.recover(
                            DiagnosticCode::OutOfOrderTimestamp,
                            "enhanced word timestamp moved backwards and was clamped",
                            Some(span),
                        );
                        time = previous;
                    }
                    markers.push(WordMarker {
                        time,
                        text_offset: text.len(),
                    });
                }
                Err(error) if looks_like_timestamp(contents) => {
                    self.recover(
                        DiagnosticCode::InvalidTimestamp,
                        format!("invalid enhanced word timestamp: {error}"),
                        Some(span),
                    );
                    text.push_str(&body[start..=end]);
                }
                Err(_) => text.push_str(&body[start..=end]),
            }
            cursor = end + 1;
        }

        if cursor < body.len() {
            text.push_str(&body[cursor..]);
        }
        let segments = segments_from_markers(&markers, text.len());
        (Arc::from(text), segments)
    }

    fn push_timestamped_lines(
        &mut self, timestamps: Vec<(LyricTime, SourceSpan)>, text: Arc<str>,
        segments: Vec<RawSegment>,
    ) {
        let reference_start = timestamps[0].0;
        self.validate_source_timestamp_order(&timestamps);
        self.previous_source_start = Some(reference_start);

        for (start, span) in timestamps {
            self.raw_lines.push(RawLine {
                id: LineId::new(self.next_line_id),
                start,
                reference_start,
                text: text.clone(),
                segments: segments.clone(),
                source_order: self.source_order,
                span,
            });
            self.next_line_id += 1;
            self.source_order += 1;
        }
    }

    fn validate_source_timestamp_order(&mut self, timestamps: &[(LyricTime, SourceSpan)]) {
        let (first, first_span) = timestamps[0];
        if self
            .previous_source_start
            .is_some_and(|previous| first < previous)
        {
            self.recover(
                DiagnosticCode::OutOfOrderTimestamp,
                "source line timestamp moved backwards; lines were stably sorted",
                Some(first_span),
            );
        }

        for pair in timestamps.windows(2) {
            if pair[1].0 < pair[0].0 {
                self.recover(
                    DiagnosticCode::OutOfOrderTimestamp,
                    "timestamps on the same source line moved backwards; lines were stably sorted",
                    Some(pair[1].1),
                );
            }
        }
    }

    fn document_duration(&mut self) -> Option<LyricTime> {
        if let Some(duration) = self.options.media_duration {
            if duration >= LyricTime::ZERO {
                return Some(duration);
            }
            self.recover(
                DiagnosticCode::InvalidRange,
                "media duration cannot be negative; valid length metadata was used instead",
                None,
            );
        }

        self.metadata
            .length
            .filter(|duration| *duration >= LyricTime::ZERO)
    }

    fn normalized_document_duration(
        &self, duration: Option<LyricTime>, lines: &[ParsedLine],
    ) -> Option<LyricTime> {
        if self.options.mode != ParseMode::Lenient {
            return duration;
        }

        lines
            .iter()
            .filter(|line| line.line.range.is_valid())
            .filter_map(|line| line.line.range.end)
            .fold(duration, |duration, line_end| {
                Some(duration.map_or(line_end, |duration| duration.max(line_end)))
            })
    }

    fn build_lines(&mut self) -> Vec<ParsedLine> {
        let raw_lines = mem::take(&mut self.raw_lines);
        let offset = self.metadata.offset;
        let mut lines = Vec::with_capacity(raw_lines.len());

        for raw in raw_lines {
            let start = self.shift_time(raw.start, offset, raw.span);
            let occurrence_shift = raw.start.saturating_sub(raw.reference_start);
            let mut timed_segments = Vec::with_capacity(raw.segments.len());

            for segment in raw.segments {
                let segment_start = segment
                    .start
                    .map(|time| self.shift_time(time, occurrence_shift, raw.span))
                    .map(|time| self.shift_time(time, offset, raw.span))
                    .unwrap_or(start);
                let segment_end = segment
                    .end
                    .map(|time| self.shift_time(time, occurrence_shift, raw.span))
                    .map(|time| self.shift_time(time, offset, raw.span));
                timed_segments.push(TimedSegment {
                    range: TimeRange::new(segment_start, segment_end),
                    text_range: segment.text_range,
                });
            }

            let lane = LyricLane {
                kind: self.options.lane_kind.clone(),
                language: self.options.language.clone(),
                text: raw.text,
                segments: timed_segments,
            };
            lines.push(ParsedLine {
                line: LyricLine {
                    id: raw.id,
                    range: TimeRange::new(start, None),
                    lanes: vec![lane],
                },
                source_order: raw.source_order,
                span: raw.span,
            });
        }

        lines.sort_by(|left, right| {
            left.line
                .range
                .start
                .cmp(&right.line.range.start)
                .then(left.source_order.cmp(&right.source_order))
        });
        lines
    }

    fn infer_line_ends(&mut self, lines: &mut [ParsedLine], duration: Option<LyricTime>) {
        let mut group_start = 0;
        while group_start < lines.len() {
            let start = lines[group_start].line.range.start;
            let mut next_group = group_start + 1;
            while next_group < lines.len() && lines[next_group].line.range.start == start {
                next_group += 1;
            }

            let next_start = lines.get(next_group).map(|line| line.line.range.start);
            let latest_segment_start = lines[group_start..next_group]
                .iter()
                .flat_map(|line| &line.line.lanes)
                .flat_map(|lane| &lane.segments)
                .map(|segment| segment.range.start)
                .max();
            let inferred_end = match (next_start, latest_segment_start) {
                (Some(next), Some(segment)) => Some(next.max(segment)),
                (Some(next), None) => Some(next),
                (None, Some(segment)) => self
                    .final_line_end(start, duration, lines[group_start].span)
                    .map(|end| end.max(segment))
                    .or(Some(segment)),
                (None, _) => self.final_line_end(start, duration, lines[group_start].span),
            };
            for parsed in &mut lines[group_start..next_group] {
                parsed.line.range.end = inferred_end;
            }
            group_start = next_group;
        }
    }

    fn final_line_end(
        &mut self, start: LyricTime, duration: Option<LyricTime>, span: SourceSpan,
    ) -> Option<LyricTime> {
        if let Some(duration) = duration {
            if duration > start {
                return Some(duration);
            }
            self.recover(
                DiagnosticCode::InvalidRange,
                "media duration does not follow the final line start; fallback was used",
                Some(span),
            );
        }

        let fallback = self.options.fallback_duration?;
        if fallback <= LyricTime::ZERO {
            self.recover(
                DiagnosticCode::InvalidRange,
                "fallback duration must be positive",
                Some(span),
            );
            return None;
        }
        Some(self.shift_time(start, fallback, span))
    }

    fn normalize_segment_ranges(&mut self, lines: &mut [ParsedLine]) {
        for parsed in lines {
            let line_start = parsed.line.range.start;
            let line_end = parsed.line.range.end;
            for lane in &mut parsed.line.lanes {
                for segment_index in 0..lane.segments.len() {
                    let mut start = lane.segments[segment_index].range.start;
                    let mut end = lane.segments[segment_index].range.end.or(line_end);

                    if start < line_start {
                        self.recover(
                            DiagnosticCode::InvalidRange,
                            "segment started before its line and was clamped",
                            Some(parsed.span),
                        );
                        start = line_start;
                    }
                    if let Some(line_end) = line_end
                        && start > line_end
                    {
                        self.recover(
                            DiagnosticCode::InvalidRange,
                            "segment started after its line ended and was clamped",
                            Some(parsed.span),
                        );
                        start = line_end;
                    }
                    if end.is_some_and(|end| end < start) {
                        self.recover(
                            DiagnosticCode::InvalidRange,
                            "segment end preceded its start and was clamped",
                            Some(parsed.span),
                        );
                        end = Some(start);
                    }
                    if let (Some(segment_end), Some(line_end)) = (end, line_end)
                        && segment_end > line_end
                    {
                        self.recover(
                            DiagnosticCode::InvalidRange,
                            "segment ended after its line and was clamped",
                            Some(parsed.span),
                        );
                        end = Some(line_end.max(start));
                    }
                    if let Some(previous) = segment_index.checked_sub(1)
                        && lane.segments[previous]
                            .range
                            .end
                            .is_none_or(|previous_end| start < previous_end)
                    {
                        self.recover(
                            DiagnosticCode::InvalidRange,
                            "segments in the same lane overlapped; the earlier segment was truncated",
                            Some(parsed.span),
                        );
                        lane.segments[previous].range.end = Some(start);
                    }
                    lane.segments[segment_index].range = TimeRange::new(start, end);
                }
            }
        }
    }

    fn shift_time(&mut self, time: LyricTime, delta: LyricTime, span: SourceSpan) -> LyricTime {
        if let Some(shifted) = time.checked_add(delta) {
            shifted
        } else {
            self.recover(
                DiagnosticCode::TimeOverflow,
                "timestamp overflowed and was saturated",
                Some(span),
            );
            time.saturating_add(delta)
        }
    }

    fn recover(
        &mut self, code: DiagnosticCode, message: impl Into<Arc<str>>, span: Option<SourceSpan>,
    ) {
        let mut diagnostic = match self.options.mode {
            ParseMode::Strict => Diagnostic::error(code, message),
            ParseMode::Lenient => Diagnostic::warning(code, message),
        };
        diagnostic.span = span;
        self.diagnostics.push(diagnostic);
    }
}

#[derive(Clone, Copy)]
struct SourceLine {
    number: usize,
    byte_offset: usize,
}

impl SourceLine {
    fn span(self, line: &str, start: usize, end: usize) -> SourceSpan {
        let start_column = line[..start].chars().count() + 1;
        let end_column = line[..end].chars().count() + 1;
        SourceSpan::new(
            crate::diagnostics::SourcePosition::new(
                self.number,
                start_column,
                self.byte_offset + start,
            ),
            crate::diagnostics::SourcePosition::new(
                self.number,
                end_column,
                self.byte_offset + end,
            ),
        )
    }
}

#[derive(Clone)]
struct RawLine {
    id: LineId,
    start: LyricTime,
    reference_start: LyricTime,
    text: Arc<str>,
    segments: Vec<RawSegment>,
    source_order: usize,
    span: SourceSpan,
}

#[derive(Clone)]
struct RawSegment {
    start: Option<LyricTime>,
    end: Option<LyricTime>,
    text_range: Range<usize>,
}

#[derive(Clone)]
struct ParsedLine {
    line: LyricLine,
    source_order: usize,
    span: SourceSpan,
}

struct WordMarker {
    time: LyricTime,
    text_offset: usize,
}

fn segments_from_markers(markers: &[WordMarker], text_len: usize) -> Vec<RawSegment> {
    if markers.is_empty() {
        return Vec::new();
    }

    let mut segments = Vec::with_capacity(markers.len() + 1);
    if markers[0].text_offset > 0 {
        segments.push(RawSegment {
            start: None,
            end: Some(markers[0].time),
            text_range: 0..markers[0].text_offset,
        });
    }
    for (index, marker) in markers.iter().enumerate() {
        let text_end = markers
            .get(index + 1)
            .map_or(text_len, |next| next.text_offset);
        if marker.text_offset < text_end {
            segments.push(RawSegment {
                start: Some(marker.time),
                end: markers.get(index + 1).map(|next| next.time),
                text_range: marker.text_offset..text_end,
            });
        }
    }
    segments
}

#[derive(Clone, Copy, Debug)]
enum TimestampError {
    MissingSeparator,
    InvalidMinutes,
    InvalidSeconds,
    SecondsOutOfRange,
    Overflow,
}

impl std::fmt::Display for TimestampError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::MissingSeparator => "expected mm:ss",
            Self::InvalidMinutes => "minutes are not an unsigned integer",
            Self::InvalidSeconds => "seconds are not a valid decimal",
            Self::SecondsOutOfRange => "seconds must be below 60",
            Self::Overflow => "timestamp is too large",
        };
        formatter.write_str(message)
    }
}

fn is_inline_variant_tag(contents: &str) -> bool {
    contents
        .split_once(':')
        .is_some_and(|(key, _)| is_voice_key(key))
}

fn unwrap_inline_variant(body: &str) -> Option<(usize, &str)> {
    let end = body.strip_suffix(']')?;
    let separator = end.find(':')?;
    let key = &end[1..separator];
    if !body.starts_with('[') || !is_voice_key(key) {
        return None;
    }
    Some((separator + 1, &end[separator + 1..]))
}

fn is_voice_key(key: &str) -> bool {
    let key = key.trim().to_ascii_lowercase();
    key == "bg"
        || key == "background"
        || key.strip_prefix('v').is_some_and(|suffix| {
            !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
        })
}

fn looks_like_timestamp(value: &str) -> bool {
    value.as_bytes().first().is_some_and(u8::is_ascii_digit) && value.contains(':')
}

fn parse_timestamp(value: &str) -> Result<LyricTime, TimestampError> {
    let (minutes, seconds) = value
        .split_once(':')
        .ok_or(TimestampError::MissingSeparator)?;
    if minutes.is_empty() || !minutes.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(TimestampError::InvalidMinutes);
    }
    let minutes = minutes
        .parse::<i64>()
        .map_err(|_| TimestampError::Overflow)?;
    let (whole_seconds, fraction) = seconds
        .split_once('.')
        .map_or((seconds, None), |(whole, fraction)| (whole, Some(fraction)));
    if whole_seconds.is_empty()
        || !whole_seconds.bytes().all(|byte| byte.is_ascii_digit())
        || fraction.is_some_and(|fraction| {
            fraction.is_empty() || !fraction.bytes().all(|byte| byte.is_ascii_digit())
        })
    {
        return Err(TimestampError::InvalidSeconds);
    }
    let seconds = whole_seconds
        .parse::<i64>()
        .map_err(|_| TimestampError::Overflow)?;
    if seconds >= 60 {
        return Err(TimestampError::SecondsOutOfRange);
    }

    let whole_micros = minutes
        .checked_mul(60)
        .and_then(|minutes| minutes.checked_add(seconds))
        .and_then(|seconds| seconds.checked_mul(1_000_000))
        .ok_or(TimestampError::Overflow)?;
    let fraction_micros = fraction.map_or(0, decimal_fraction_to_micros);
    whole_micros
        .checked_add(fraction_micros)
        .map(LyricTime::from_micros)
        .ok_or(TimestampError::Overflow)
}

fn decimal_fraction_to_micros(fraction: &str) -> i64 {
    let mut micros = 0_i64;
    let mut digits = 0;
    for byte in fraction.bytes().take(6) {
        micros = micros * 10 + i64::from(byte - b'0');
        digits += 1;
    }
    for _ in digits..6 {
        micros *= 10;
    }
    micros
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        diagnostics::DiagnosticCode,
        model::LaneKind,
    };

    fn document(input: &str) -> LyricsDocument {
        parse_lrc(input, &ParseOptions::default())
            .document
            .expect("lenient parsing should return a document")
    }

    #[test]
    fn parses_metadata_bom_crlf_offset_and_multiple_timestamps() {
        let report = parse_lrc(
            "\u{feff}[ar:Artist]\r\n[x-vendor:kept]\r\n[offset:-500]\r\n[00:01.00][00:02.1234567]歌\r\n",
            &ParseOptions::default(),
        );
        let document = report.document.unwrap();
        assert_eq!(document.metadata.artist.as_deref(), Some("Artist"));
        assert_eq!(document.metadata.value("x-vendor"), Some("kept"));
        assert_eq!(document.metadata.offset, LyricTime::from_millis(-500));
        assert_eq!(document.tracks[0].lines.len(), 2);
        assert_eq!(
            document.tracks[0].lines[0].range.start,
            LyricTime::from_micros(500_000)
        );
        assert_eq!(
            document.tracks[0].lines[1].range.start,
            LyricTime::from_micros(1_623_456)
        );
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == DiagnosticCode::UnknownMetadata)
        );
    }

    #[test]
    fn parses_inline_voice_wrappers_and_preserves_enhanced_overlap() {
        let source = "[00:38.000][v1:<00:38.000>Main <00:38.120>vocal <00:38.360>here]\n\
                      [00:38.200][bg:<00:38.200>la <00:38.300>la <00:38.500>la]\n\
                      [00:39.000]next";
        let document = document(source);
        let lines = &document.tracks[0].lines;

        assert_eq!(lines[0].lanes[0].text.as_ref(), "Main vocal here");
        assert_eq!(lines[1].lanes[0].text.as_ref(), "la la la");
        assert_eq!(lines[0].range.end, Some(LyricTime::from_millis(38_360)));
        assert!(lines[0].range.end.unwrap() > lines[1].range.start);
    }

    #[test]
    fn parses_enhanced_segments_with_utf8_byte_ranges() {
        let document = document("[00:10.00]<00:10.00>Hello <00:10.50>世界\n[00:12]next");
        let line = &document.tracks[0].lines[0];
        let lane = &line.lanes[0];
        assert_eq!(lane.text.as_ref(), "Hello 世界");
        assert_eq!(&lane.text[lane.segments[0].text_range.clone()], "Hello ");
        assert_eq!(&lane.text[lane.segments[1].text_range.clone()], "世界");
        assert_eq!(
            lane.segments[0].range.end,
            Some(LyricTime::from_millis(10_500))
        );
        assert_eq!(lane.segments[1].range.end, Some(LyricTime::from_secs(12)));
        assert!(lane.has_valid_text_ranges());
    }

    #[test]
    fn shifts_enhanced_segments_for_repeated_line_timestamps() {
        let document = document("[00:10][00:20]<00:10>one <00:11>two\n[00:30]end");
        let repeated = &document.tracks[0].lines[1].lanes[0].segments;
        assert_eq!(repeated[0].range.start, LyricTime::from_secs(20));
        assert_eq!(repeated[1].range.start, LyricTime::from_secs(21));
    }

    #[test]
    fn duplicate_lines_share_the_next_strictly_later_end() {
        let document = document("[00:01]a\n[00:01]b\n[00:02]c");
        let lines = &document.tracks[0].lines;
        assert_eq!(lines[0].range.end, Some(LyricTime::from_secs(2)));
        assert_eq!(lines[1].range.end, Some(LyricTime::from_secs(2)));
        assert_eq!(lines[2].range.end, Some(LyricTime::from_secs(7)));
    }

    #[test]
    fn media_duration_precedes_last_line_fallback() {
        let options = ParseOptions {
            media_duration: Some(LyricTime::from_secs(9)),
            ..ParseOptions::default()
        };
        let report = parse_lrc("[00:01]a", &options);
        assert_eq!(
            report.document.unwrap().tracks[0].lines[0].range.end,
            Some(LyricTime::from_secs(9))
        );
    }

    #[test]
    fn lenient_mode_repairs_order_but_strict_mode_rejects_it() {
        let input = "[00:02]later\n[00:01]earlier";
        let lenient = parse_lrc(input, &ParseOptions::lenient());
        let lines = &lenient.document.unwrap().tracks[0].lines;
        assert_eq!(lines[0].lanes[0].text.as_ref(), "earlier");
        assert!(
            lenient
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == DiagnosticCode::OutOfOrderTimestamp)
        );

        let strict = parse_lrc(input, &ParseOptions::strict());
        assert!(strict.document.is_none());
        assert!(strict.has_errors());
        assert!(strict.diagnostics[0].span.is_some());
    }

    #[test]
    fn malformed_enhanced_tags_are_preserved_in_lenient_text() {
        let report = parse_lrc("[00:01]a<00:x>b<c><00:02\n", &ParseOptions::lenient());
        let document = report.document.unwrap();
        assert_eq!(
            document.tracks[0].lines[0].lanes[0].text.as_ref(),
            "a<00:x>b<c><00:02"
        );
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == DiagnosticCode::MalformedTag)
        );
    }

    #[test]
    fn metadata_followed_by_unstamped_text_is_diagnosed() {
        let report = parse_lrc("[ar:Artist]unexpected", &ParseOptions::strict());
        assert!(report.document.is_none());
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == DiagnosticCode::MissingTimestamp)
        );
    }

    #[test]
    fn strict_mode_allows_reused_timestamps_to_cross_later_source_lines() {
        let report = parse_lrc("[00:10][01:10]reused\n[00:20]next", &ParseOptions::strict());
        assert!(report.document.is_some());
        assert!(
            report
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.code != DiagnosticCode::OutOfOrderTimestamp)
        );
        let starts: Vec<_> = report.document.unwrap().tracks[0]
            .lines
            .iter()
            .map(|line| line.range.start)
            .collect();
        assert_eq!(
            starts,
            vec![
                LyricTime::from_secs(10),
                LyricTime::from_secs(20),
                LyricTime::from_secs(70),
            ]
        );
    }

    #[test]
    fn strict_mode_rejects_backwards_timestamps_within_one_source_line() {
        let report = parse_lrc("[00:20][00:10]bad", &ParseOptions::strict());
        assert!(report.document.is_none());
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == DiagnosticCode::OutOfOrderTimestamp
                && diagnostic.message.contains("same source line")
        }));
    }

    #[test]
    fn invalid_media_duration_falls_back_to_valid_length_metadata() {
        let mut options = ParseOptions::lenient();
        options.media_duration = Some(LyricTime::from_secs(-1));
        let report = parse_lrc("[length:00:08]\n[00:01]line", &options);
        let document = report.document.unwrap();
        assert_eq!(document.duration, Some(LyricTime::from_secs(8)));
        assert_eq!(
            document.tracks[0].lines[0].range.end,
            Some(LyricTime::from_secs(8))
        );
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == DiagnosticCode::InvalidRange)
        );
    }

    #[test]
    fn lenient_document_duration_covers_inferred_line_ends() {
        let mut options = ParseOptions::lenient();
        options.media_duration = Some(LyricTime::from_secs(2));
        let report = parse_lrc("[00:10]late", &options);
        let document = report.document.unwrap();
        assert_eq!(
            document.tracks[0].lines[0].range.end,
            Some(LyricTime::from_secs(15))
        );
        assert_eq!(document.duration, Some(LyricTime::from_secs(15)));
    }

    #[test]
    fn parser_normalization_diagnoses_and_repairs_lane_segment_overlap() {
        let options = ParseOptions::lenient();
        let mut parser = LrcParser::new(&options);
        let span = SourceSpan::at(1, 1, 0, 1);
        let mut lines = vec![ParsedLine {
            line: LyricLine {
                id: LineId::new(0),
                range: TimeRange::new(LyricTime::from_secs(1), Some(LyricTime::from_secs(4))),
                lanes: vec![LyricLane {
                    kind: LaneKind::Original,
                    language: None,
                    text: Arc::from("ab"),
                    segments: vec![
                        TimedSegment {
                            range: TimeRange::new(
                                LyricTime::from_secs(1),
                                Some(LyricTime::from_secs(3)),
                            ),
                            text_range: 0..1,
                        },
                        TimedSegment {
                            range: TimeRange::new(
                                LyricTime::from_secs(2),
                                Some(LyricTime::from_secs(4)),
                            ),
                            text_range: 1..2,
                        },
                    ],
                }],
            },
            source_order: 0,
            span,
        }];

        let strict_options = ParseOptions::strict();
        let mut strict_parser = LrcParser::new(&strict_options);
        let mut strict_lines = lines.clone();
        strict_parser.normalize_segment_ranges(&mut strict_lines);
        assert!(strict_parser.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == DiagnosticCode::InvalidRange
                && diagnostic.severity == DiagnosticSeverity::Error
        }));

        parser.normalize_segment_ranges(&mut lines);

        assert_eq!(
            lines[0].line.lanes[0].segments[0].range.end,
            Some(LyricTime::from_secs(2))
        );
        assert!(parser.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == DiagnosticCode::InvalidRange
                && diagnostic.message.contains("same lane overlapped")
        }));
    }

    #[test]
    fn parse_options_assign_sidecar_lane_kind_and_language() {
        let options = ParseOptions {
            lane_kind: LaneKind::custom("annotation"),
            language: Some(Arc::from("en")),
            ..ParseOptions::default()
        };
        let document = parse_lrc("[00:01]note", &options).document.unwrap();
        assert_eq!(document.tracks[0].kind, LaneKind::custom("annotation"));
        assert_eq!(
            document.tracks[0].lines[0].lanes[0].language.as_deref(),
            Some("en")
        );
    }
}
