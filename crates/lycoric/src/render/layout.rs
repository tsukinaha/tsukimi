use std::ops::Range;

use gtk::pango;

use crate::{
    model::{
        LaneKind,
        LyricsDocument,
    },
    render::{
        highlight::HighlightGeometry,
        style::{
            LaneSlot,
            LyricsStyle,
            Overscan,
        },
    },
    time::{
        LyricTime,
        TimeRange,
    },
    timeline::{
        Timeline,
        TimelineLine,
    },
};

const GAP_SLOT_HEIGHT: f32 = 48.0;
const GAP_MIN_DURATION_US: i64 = 800_000;
const LINE_FRAME_MARGIN_X: f32 = 8.0;
const LINE_INTERACTION_BASE_PADDING_X: f32 = 12.0;
const LINE_INTERACTION_BASE_PADDING_Y: f32 = 10.0;
const LINE_INTERACTION_MIN_GAP_Y: f32 = 20.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct InteractionInsets {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LaneVisibility(u8);

impl LaneVisibility {
    const ORIGINAL: u8 = 1 << 0;
    const OTHER: u8 = 1 << 1;

    pub const ALL: Self = Self(Self::ORIGINAL | Self::OTHER);

    pub const fn bits(self) -> u8 {
        self.0
    }

    pub const fn is_visible(self, slot: LaneSlot) -> bool {
        self.0 & slot_bit(slot) != 0
    }

    pub fn set_visible(&mut self, slot: LaneSlot, visible: bool) {
        let bit = slot_bit(slot);
        if visible {
            self.0 |= bit;
        } else {
            self.0 &= !bit;
        }
    }
}

impl Default for LaneVisibility {
    fn default() -> Self {
        Self::ALL
    }
}

#[derive(Clone)]
pub struct LaneLayout {
    pub lane_index: usize,
    pub slot: LaneSlot,
    pub layout: pango::Layout,
    pub x: f32,
    /// Y is relative to the owning line. This keeps cached nodes valid when a
    /// measurement before the line changes the prefix index.
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub segment_times: Vec<TimeRange>,
    pub highlights: HighlightGeometry,
}

impl LaneLayout {
    pub fn has_timing(&self) -> bool {
        !self.segment_times.is_empty()
    }
}

#[derive(Clone)]
pub struct LineLayout {
    pub timeline_index: usize,
    pub start: LyricTime,
    pub top: f32,
    pub content_top: f32,
    pub content_height: f32,
    pub height: f32,
    pub gap_before: Option<TimeRange>,
    pub gap_height: f32,
    pub(crate) interaction_insets: InteractionInsets,
    pub lanes: Vec<LaneLayout>,
}

impl LineLayout {
    pub fn center_y(&self) -> f32 {
        self.top + self.content_top + self.content_height * 0.5
    }

    pub fn gap_center_y(&self) -> Option<f32> {
        (self.gap_before.is_some() && self.gap_height > 0.0)
            .then_some(self.top + self.gap_height * 0.5)
    }

    pub fn contains_interaction_y(&self, content_y: f32) -> bool {
        let top = self.top + self.content_top - self.interaction_insets.top;
        let bottom =
            self.top + self.content_top + self.content_height + self.interaction_insets.bottom;
        content_y >= top && content_y < bottom
    }

    pub fn contains_gap_y(&self, content_y: f32) -> bool {
        self.gap_before.is_some()
            && self.gap_height > 0.0
            && content_y >= self.top
            && content_y < self.top + self.gap_height
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineHit {
    pub timeline_index: usize,
    pub start: LyricTime,
}

#[derive(Clone, Debug)]
struct LineSource {
    timeline_index: usize,
    track_index: usize,
    line_index: usize,
    start: LyricTime,
    gap_before: Option<TimeRange>,
}

#[derive(Clone, Debug, Default)]
struct HeightIndex {
    values: Vec<f32>,
    tree: Vec<f32>,
}

impl HeightIndex {
    fn new(values: Vec<f32>) -> Self {
        let mut index = Self {
            tree: vec![0.0; values.len() + 1],
            values: vec![0.0; values.len()],
        };
        for (line, value) in values.into_iter().enumerate() {
            index.set(line, value);
        }
        index
    }

    fn value(&self, line: usize) -> Option<f32> {
        self.values.get(line).copied()
    }

    fn set(&mut self, line: usize, value: f32) -> bool {
        let Some(previous) = self.values.get_mut(line) else {
            return false;
        };
        let value = finite_height(value);
        let delta = value - *previous;
        if delta.abs() <= 0.01 {
            return false;
        }
        *previous = value;
        let mut node = line + 1;
        while node < self.tree.len() {
            self.tree[node] += delta;
            node += node & node.wrapping_neg();
        }
        true
    }

    fn prefix(&self, end: usize) -> f32 {
        let mut node = end.min(self.values.len());
        let mut sum = 0.0;
        while node > 0 {
            sum += self.tree[node];
            node &= node - 1;
        }
        sum
    }

    fn total(&self) -> f32 {
        self.prefix(self.values.len())
    }

    fn line_at(&self, offset: f32) -> Option<usize> {
        if self.values.is_empty() {
            return None;
        }
        let target = if offset.is_finite() {
            offset.max(0.0)
        } else {
            0.0
        };
        if target >= self.total() {
            return Some(self.values.len() - 1);
        }
        let mut low = 0;
        let mut high = self.values.len();
        while low < high {
            let middle = low + (high - low) / 2;
            if self.prefix(middle + 1) <= target {
                low = middle + 1;
            } else {
                high = middle;
            }
        }
        Some(low.min(self.values.len() - 1))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ViewportAnchor {
    FocusLine(usize),
    FocusGap(usize),
    ScrollOffset(f32),
}

#[derive(Debug, Default)]
pub struct LayoutWindowUpdate {
    pub range: Range<usize>,
    pub rebuilt: Vec<usize>,
    pub scroll_correction: f32,
    pub changed: bool,
}

#[derive(Clone, Default)]
pub struct DocumentLayout {
    width: f32,
    sources: Vec<LineSource>,
    heights: HeightIndex,
    lines: Vec<Option<LineLayout>>,
    gap_expansions: Vec<f32>,
    active_gap: Option<usize>,
    loaded: Range<usize>,
}

impl DocumentLayout {
    pub fn new(
        document: &LyricsDocument, timeline: &Timeline, width: f32, style: &LyricsStyle,
        visibility: LaneVisibility,
    ) -> Self {
        let width = finite_non_negative(width);
        let summaries: Vec<_> = timeline.lines().cloned().collect();
        let sources: Vec<_> = summaries
            .iter()
            .enumerate()
            .map(|(index, summary)| LineSource {
                timeline_index: summary.timeline_index,
                track_index: summary.track_index,
                line_index: summary.line_index,
                start: summary.range.start,
                gap_before: gap_before_range(
                    index
                        .checked_sub(1)
                        .and_then(|previous| summaries.get(previous)),
                    summary,
                ),
            })
            .collect();
        let estimates = sources
            .iter()
            .map(|source| {
                source_line(document, source)
                    .map(|line| estimate_line_height(line, width, style, visibility))
                    .unwrap_or_else(|| fallback_line_height(style))
            })
            .collect();
        Self {
            width,
            heights: HeightIndex::new(estimates),
            lines: vec![None; sources.len()],
            gap_expansions: vec![0.0; sources.len()],
            active_gap: None,
            sources,
            loaded: 0..0,
        }
    }

    pub fn width(&self) -> f32 {
        self.width
    }

    pub fn total_height(&self) -> f32 {
        self.heights.total()
    }

    pub fn len(&self) -> usize {
        self.sources.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }

    pub fn loaded_range(&self) -> Range<usize> {
        self.loaded.clone()
    }

    pub fn loaded_count(&self) -> usize {
        self.lines.iter().filter(|line| line.is_some()).count()
    }

    pub fn line(&self, timeline_index: usize) -> Option<&LineLayout> {
        self.lines
            .get(timeline_index)?
            .as_ref()
            .filter(|line| line.timeline_index == timeline_index)
    }

    pub fn line_center(&self, timeline_index: usize) -> Option<f32> {
        if let Some(line) = self.line(timeline_index) {
            return Some(line.center_y());
        }
        let height = self.heights.value(timeline_index)?;
        Some(self.heights.prefix(timeline_index) + height * 0.5)
    }

    pub fn gap_center(&self, timeline_index: usize) -> Option<f32> {
        self.line(timeline_index)?.gap_center_y()
    }

    pub fn gap_before(&self, timeline_index: usize) -> Option<TimeRange> {
        self.sources.get(timeline_index)?.gap_before
    }

    pub fn set_gap_expansion(&mut self, gap: Option<usize>, expansion: f32) -> bool {
        let mut changed = false;
        if self.active_gap != gap {
            if let Some(previous) = self.active_gap.take() {
                changed |= self.set_line_gap_expansion(previous, 0.0);
            }
            self.active_gap = gap;
        }
        if let Some(index) = gap {
            changed |= self.set_line_gap_expansion(index, expansion);
        }
        if changed {
            self.refresh_loaded_tops();
        }
        changed
    }

    fn set_line_gap_expansion(&mut self, index: usize, expansion: f32) -> bool {
        let Some(source) = self.sources.get(index) else {
            return false;
        };
        let expansion = if source.gap_before.is_some() {
            expansion.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let Some(previous) = self.gap_expansions.get_mut(index) else {
            return false;
        };
        if (*previous - expansion).abs() <= 0.001 {
            return false;
        }
        let delta = GAP_SLOT_HEIGHT * (expansion - *previous);
        *previous = expansion;
        if let Some(height) = self.heights.value(index) {
            self.heights.set(index, height + delta);
        }
        if let Some(line) = self.lines.get_mut(index).and_then(Option::as_mut) {
            line.gap_height += delta;
            line.content_top += delta;
            line.height += delta;
            for lane in &mut line.lanes {
                lane.y += delta;
            }
        }
        true
    }

    pub fn visible_range(
        &self, scroll_offset: f32, viewport_height: f32, overscan: Overscan,
    ) -> Range<usize> {
        if self.is_empty() || !viewport_height.is_finite() || viewport_height <= 0.0 {
            return 0..0;
        }
        let first = self.heights.line_at(scroll_offset.max(0.0)).unwrap_or(0);
        let bottom = (scroll_offset + viewport_height).max(0.0);
        let last_sample = (bottom - 0.01).max(scroll_offset.max(0.0));
        let last = self.heights.line_at(last_sample).unwrap_or(first);
        first.saturating_sub(overscan.before)
            ..last
                .saturating_add(1)
                .saturating_add(overscan.after)
                .min(self.len())
    }

    pub fn ensure_range(
        &mut self, document: &LyricsDocument, context: &pango::Context, style: &LyricsStyle,
        visibility: LaneVisibility, range: Range<usize>, anchor: ViewportAnchor,
    ) -> LayoutWindowUpdate {
        let mut range =
            range.start.min(self.len())..range.end.min(self.len()).max(range.start.min(self.len()));
        if let ViewportAnchor::FocusLine(line) | ViewportAnchor::FocusGap(line) = anchor
            && line < self.len()
        {
            range.start = range.start.min(line);
            range.end = range.end.max(line + 1);
        }
        let old_anchor_position = self.anchor_position(anchor);
        let mut rebuilt = Vec::new();
        let range_changed = self.loaded != range;

        for index in self.loaded.clone() {
            if !range.contains(&index) {
                self.lines[index] = None;
            }
        }
        for index in range.clone() {
            if self.lines[index].is_some() {
                continue;
            }
            let Some(source) = self.sources.get(index) else {
                continue;
            };
            let Some(line) = source_line(document, source) else {
                continue;
            };
            let track_language = document
                .tracks
                .get(source.track_index)
                .and_then(|track| track.language.as_deref());
            let top = self.heights.prefix(index);
            let mut layout = build_line(LineBuild {
                line,
                source,
                track_language,
                context,
                width: self.width,
                style,
                visibility,
                gap_expansion: self.gap_expansions.get(index).copied().unwrap_or(0.0),
            });
            layout.top = top;
            self.heights.set(index, layout.height);
            self.lines[index] = Some(layout);
            rebuilt.push(index);
        }
        self.loaded = range.clone();
        self.refresh_loaded_tops();

        let new_anchor_position = self.anchor_position(anchor);
        let scroll_correction = finite_delta(new_anchor_position - old_anchor_position);
        LayoutWindowUpdate {
            range,
            changed: range_changed || !rebuilt.is_empty() || scroll_correction != 0.0,
            rebuilt,
            scroll_correction,
        }
    }

    pub fn hit_test(&self, content_y: f32) -> Option<LineHit> {
        if !content_y.is_finite() || content_y < 0.0 || content_y >= self.total_height() {
            return None;
        }
        let index = self.heights.line_at(content_y)?;
        for candidate in [Some(index), index.checked_add(1), index.checked_sub(1)]
            .into_iter()
            .flatten()
        {
            if let Some(hit) = self.hit_test_line(candidate, content_y) {
                return Some(hit);
            }
        }
        None
    }

    pub fn hit_test_line(&self, index: usize, content_y: f32) -> Option<LineHit> {
        let source = self.sources.get(index)?;
        let line = self.line(index)?;
        if line.contains_gap_y(content_y) || !line.contains_interaction_y(content_y) {
            return None;
        }
        Some(LineHit {
            timeline_index: source.timeline_index,
            start: source.start,
        })
    }

    fn anchor_position(&self, anchor: ViewportAnchor) -> f32 {
        match anchor {
            ViewportAnchor::FocusLine(line) => self.line_center(line).unwrap_or(0.0),
            ViewportAnchor::FocusGap(line) => self.gap_center(line).unwrap_or(0.0),
            ViewportAnchor::ScrollOffset(offset) => self
                .heights
                .line_at(offset.max(0.0))
                .map(|line| self.heights.prefix(line))
                .unwrap_or(0.0),
        }
    }

    fn refresh_loaded_tops(&mut self) {
        for index in self.loaded.clone() {
            let top = self.heights.prefix(index);
            if let Some(line) = self.lines.get_mut(index).and_then(Option::as_mut) {
                line.top = top;
            }
        }
    }
}

pub fn gap_before_range(previous: Option<&TimelineLine>, next: &TimelineLine) -> Option<TimeRange> {
    let start = match previous {
        None => LyricTime::ZERO,
        Some(previous) => previous.range.end.filter(|end| *end < next.range.start)?,
    };

    let duration = next
        .range
        .start
        .as_micros()
        .saturating_sub(start.as_micros());
    let range = TimeRange::new(start, Some(next.range.start));
    (range.is_valid() && duration >= GAP_MIN_DURATION_US).then_some(range)
}

fn gap_slot_height(gap: Option<TimeRange>, expansion: f32) -> f32 {
    if gap.is_some() {
        GAP_SLOT_HEIGHT * expansion.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn source_line<'a>(
    document: &'a LyricsDocument, source: &LineSource,
) -> Option<&'a crate::model::LyricLine> {
    document
        .tracks
        .get(source.track_index)
        .and_then(|track| track.lines.get(source.line_index))
}

struct LineBuild<'a> {
    line: &'a crate::model::LyricLine,
    source: &'a LineSource,
    track_language: Option<&'a str>,
    context: &'a pango::Context,
    width: f32,
    style: &'a LyricsStyle,
    visibility: LaneVisibility,
    gap_expansion: f32,
}

fn build_line(build: LineBuild<'_>) -> LineLayout {
    let LineBuild {
        line,
        source,
        track_language,
        context,
        width,
        style,
        visibility,
        gap_expansion,
    } = build;
    let interaction_insets = interaction_insets(style);
    let (content_x, content_width) = line_content_geometry(width, style, interaction_insets);
    let mut lanes = build_lanes(
        line,
        track_language,
        context,
        content_x,
        content_width,
        style,
        visibility,
    );
    let content_height = lane_block_height(&lanes);
    let gap_height = gap_slot_height(source.gap_before, gap_expansion);
    let content_top = gap_height;
    let height = gap_height + content_height.max(1.0) + effective_line_spacing(style);
    for lane in &mut lanes {
        lane.y += content_top;
    }
    LineLayout {
        timeline_index: source.timeline_index,
        start: source.start,
        top: 0.0,
        content_top,
        content_height,
        height,
        gap_before: source.gap_before,
        gap_height,
        interaction_insets,
        lanes,
    }
}

fn build_lanes(
    line: &crate::model::LyricLine, track_language: Option<&str>, context: &pango::Context, x: f32,
    width: f32, style: &LyricsStyle, visibility: LaneVisibility,
) -> Vec<LaneLayout> {
    let order = ordered_lane_indices(line);
    let mut y = 0.0;
    let mut lanes = Vec::new();

    for lane_index in order.iter().copied() {
        let lane = &line.lanes[lane_index];
        let slot = slot_for_kind(&lane.kind);
        if lane.text.is_empty() || !visibility.is_visible(slot) {
            continue;
        }
        let lane_style = style.lane(slot);
        let language = shaping_language(lane.language.as_deref(), track_language);
        let layout = make_layout(context, &lane.text, language, width, lane_style, style);
        let (_, height) = layout.pixel_size();
        let (segment_times, ranges) = lane_timing(lane);
        let highlights = HighlightGeometry::new(&layout, &lane.text, &ranges);
        lanes.push(LaneLayout {
            lane_index,
            slot,
            layout,
            x,
            y,
            width,
            height: height.max(1) as f32,
            segment_times,
            highlights,
        });

        y += height.max(1) as f32 + lane_spacing_after(lane_style, style);
    }
    lanes
}

fn ordered_lane_indices(line: &crate::model::LyricLine) -> Vec<usize> {
    let mut indices = Vec::with_capacity(line.lanes.len());
    for wanted in [LaneSlot::Original, LaneSlot::Other] {
        indices.extend(
            line.lanes
                .iter()
                .enumerate()
                .filter(|(_, lane)| slot_for_kind(&lane.kind) == wanted)
                .map(|(index, _)| index),
        );
    }
    indices
}

fn lane_timing(lane: &crate::model::LyricLane) -> (Vec<TimeRange>, Vec<Range<usize>>) {
    (
        lane.segments.iter().map(|segment| segment.range).collect(),
        lane.segments
            .iter()
            .map(|segment| segment.text_range.clone())
            .collect(),
    )
}

fn lane_spacing_after(lane_style: &crate::render::style::LaneStyle, style: &LyricsStyle) -> f32 {
    finite_non_negative(style.layout.lane_spacing) + finite_non_negative(lane_style.spacing_after)
}

fn make_layout(
    context: &pango::Context, text: &str, language: Option<&str>, width: f32,
    lane_style: &crate::render::style::LaneStyle, style: &LyricsStyle,
) -> pango::Layout {
    let layout = pango::Layout::new(context);
    layout.set_text(text);
    layout.set_font_description(Some(&lane_style.font));
    if let Some(language) = language {
        let attributes = pango::AttrList::new();
        attributes.insert(pango::AttrLanguage::new(&pango::Language::from_string(
            language,
        )));
        layout.set_attributes(Some(&attributes));
    }
    layout.set_width(pango_units(width));
    layout.set_wrap(style.layout.wrap);
    layout.set_alignment(style.layout.alignment);
    layout.set_auto_dir(true);
    layout
}

fn shaping_language<'a>(
    lane_language: Option<&'a str>, track_language: Option<&'a str>,
) -> Option<&'a str> {
    lane_language
        .filter(|language| !language.trim().is_empty())
        .or_else(|| track_language.filter(|language| !language.trim().is_empty()))
}

fn interaction_insets(style: &LyricsStyle) -> InteractionInsets {
    let outline = style
        .effects
        .outline
        .as_ref()
        .filter(|outline| outline.is_visible())
        .map(|outline| finite_non_negative(outline.width))
        .unwrap_or(0.0);
    let (shadow_left, shadow_right, shadow_top, shadow_bottom) = style
        .effects
        .shadow
        .as_ref()
        .filter(|shadow| shadow.is_visible())
        .map(|shadow| {
            let blur = finite_non_negative(shadow.blur_radius);
            (
                (blur - shadow.offset_x).max(0.0),
                (blur + shadow.offset_x).max(0.0),
                (blur - shadow.offset_y).max(0.0),
                (blur + shadow.offset_y).max(0.0),
            )
        })
        .unwrap_or((0.0, 0.0, 0.0, 0.0));
    InteractionInsets {
        left: LINE_INTERACTION_BASE_PADDING_X + outline.max(shadow_left),
        right: LINE_INTERACTION_BASE_PADDING_X + outline.max(shadow_right),
        top: LINE_INTERACTION_BASE_PADDING_Y
            + outline
                .max(shadow_top)
                .max(finite_non_negative(style.interaction.highlight_lift)),
        bottom: LINE_INTERACTION_BASE_PADDING_Y + outline.max(shadow_bottom),
    }
}

fn effective_line_spacing(style: &LyricsStyle) -> f32 {
    let insets = interaction_insets(style);
    finite_non_negative(style.layout.line_spacing)
        .max(insets.top + insets.bottom + LINE_INTERACTION_MIN_GAP_Y)
}

fn line_content_geometry(width: f32, style: &LyricsStyle, insets: InteractionInsets) -> (f32, f32) {
    let width = finite_non_negative(width);
    let frame_padding = insets.left.max(insets.right);
    let text_margin = LINE_FRAME_MARGIN_X + frame_padding;
    let available = (width - text_margin * 2.0).max(1.0);
    let content_width = style.layout.content_width(available).clamp(1.0, available);
    (((width - content_width) * 0.5).max(0.0), content_width)
}

fn estimate_line_height(
    line: &crate::model::LyricLine, width: f32, style: &LyricsStyle, visibility: LaneVisibility,
) -> f32 {
    let insets = interaction_insets(style);
    let (_, content_width) = line_content_geometry(width, style, insets);
    let content_width = content_width.max(1.0);
    let mut content_height = 0.0;
    let order = ordered_lane_indices(line);
    for lane_index in order.iter().copied() {
        let lane = &line.lanes[lane_index];
        let slot = slot_for_kind(&lane.kind);
        if lane.text.is_empty() || !visibility.is_visible(slot) {
            continue;
        }
        let lane_style = style.lane(slot);
        let font_size = estimated_font_size(&lane_style.font);
        let average_advance = (font_size * 0.58).max(1.0);
        let estimated_width = lane.text.chars().count() as f32 * average_advance;
        let rows = (estimated_width / content_width).ceil().max(1.0);
        content_height +=
            rows * (font_size * 1.28).max(1.0) + lane_spacing_after(lane_style, style);
    }
    if content_height <= 0.0 {
        return fallback_line_height(style);
    }
    let last_slot = order
        .iter()
        .rev()
        .map(|index| slot_for_kind(&line.lanes[*index].kind))
        .find(|slot| visibility.is_visible(*slot))
        .unwrap_or(LaneSlot::Original);
    content_height -= lane_spacing_after(style.lane(last_slot), style);
    content_height + effective_line_spacing(style)
}

fn estimated_font_size(font: &pango::FontDescription) -> f32 {
    let size = font.size() as f32 / pango::SCALE as f32;
    if size.is_finite() && size > 0.0 {
        size
    } else {
        18.0
    }
}

fn fallback_line_height(style: &LyricsStyle) -> f32 {
    estimated_font_size(&style.lane(LaneSlot::Original).font) * 1.28 + effective_line_spacing(style)
}

fn lane_block_height(lanes: &[LaneLayout]) -> f32 {
    let Some(last) = lanes.last() else {
        return 1.0;
    };
    (last.y + last.height).max(1.0)
}

pub fn slot_for_kind(kind: &LaneKind) -> LaneSlot {
    match kind {
        LaneKind::Original => LaneSlot::Original,
        LaneKind::Other(_) => LaneSlot::Other,
    }
}

const fn slot_bit(slot: LaneSlot) -> u8 {
    match slot {
        LaneSlot::Original => LaneVisibility::ORIGINAL,
        LaneSlot::Other => LaneVisibility::OTHER,
    }
}

fn pango_units(width: f32) -> i32 {
    let units = width.max(1.0) * pango::SCALE as f32;
    units.round().clamp(1.0, i32::MAX as f32) as i32
}

fn finite_non_negative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn finite_height(value: f32) -> f32 {
    if value.is_finite() {
        value.max(1.0)
    } else {
        1.0
    }
}

fn finite_delta(value: f32) -> f32 {
    if value.is_finite() && value.abs() > 0.01 {
        value
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        model::{
            LaneKind,
            LineId,
            LyricLane,
            LyricLine,
            LyricTrack,
            TimedSegment,
        },
        time::TimeRange,
    };

    fn document(line_count: usize) -> LyricsDocument {
        let lines = (0..line_count)
            .map(|index| LyricLine {
                id: LineId::new(index as u64),
                range: TimeRange::new(
                    LyricTime::from_micros(index as i64 * 1_000_000),
                    Some(LyricTime::from_micros((index as i64 + 1) * 1_000_000)),
                ),
                lanes: vec![LyricLane::new(LaneKind::Original, "word")],
            })
            .collect();
        LyricsDocument {
            tracks: vec![LyricTrack {
                language: None,
                kind: LaneKind::Original,
                lines,
            }],
            ..Default::default()
        }
    }

    #[test]
    fn line_content_geometry_reserves_room_for_the_hover_frame() {
        let style = LyricsStyle::default();
        let insets = interaction_insets(&style);
        assert_eq!(line_content_geometry(640.0, &style, insets), (20.0, 600.0));
        assert_eq!(
            line_content_geometry(1_200.0, &style, insets),
            (120.0, 960.0)
        );
    }

    #[test]
    fn interaction_insets_cover_text_effects_and_highlight_lift() {
        let mut style = LyricsStyle::default();
        style.interaction.highlight_lift = 24.0;
        style.effects.outline = Some(crate::render::style::OutlineStyle {
            color: gtk::gdk::RGBA::WHITE,
            width: 16.0,
        });
        style.effects.shadow = Some(crate::render::style::ShadowStyle {
            color: gtk::gdk::RGBA::BLACK,
            offset_x: 20.0,
            offset_y: 12.0,
            blur_radius: 8.0,
        });

        let insets = interaction_insets(&style);
        assert_eq!(
            insets,
            InteractionInsets {
                left: 28.0,
                right: 40.0,
                top: 34.0,
                bottom: 30.0,
            }
        );
        assert!(effective_line_spacing(&style) >= insets.top + insets.bottom + 20.0);
    }

    #[test]
    fn interaction_bounds_leave_space_between_adjacent_hover_frames() {
        let style = LyricsStyle::default();
        let insets = interaction_insets(&style);
        let line = LineLayout {
            timeline_index: 0,
            start: LyricTime::ZERO,
            top: 100.0,
            content_top: 0.0,
            content_height: 40.0,
            height: 40.0 + effective_line_spacing(&style),
            gap_before: None,
            gap_height: 0.0,
            interaction_insets: insets,
            lanes: Vec::new(),
        };
        let frame_top = line.top - insets.top;
        let frame_bottom = line.top + line.content_height + insets.bottom;
        assert!(line.contains_interaction_y(frame_top));
        assert!(line.contains_interaction_y(frame_bottom - 0.01));
        assert!(!line.contains_interaction_y(frame_bottom));

        let next_frame_top = line.top + line.height - insets.top;
        assert!(next_frame_top - frame_bottom >= 20.0);
    }

    #[test]
    fn hit_test_checks_the_next_lines_visible_top_padding() {
        let insets = InteractionInsets {
            left: 12.0,
            right: 12.0,
            top: 10.0,
            bottom: 10.0,
        };
        let first = LineLayout {
            timeline_index: 0,
            start: LyricTime::ZERO,
            top: 0.0,
            content_top: 0.0,
            content_height: 40.0,
            height: 80.0,
            gap_before: None,
            gap_height: 0.0,
            interaction_insets: insets,
            lanes: Vec::new(),
        };
        let second = LineLayout {
            timeline_index: 1,
            start: LyricTime::from_secs(1),
            top: 80.0,
            ..first.clone()
        };
        let layout = DocumentLayout {
            width: 640.0,
            sources: vec![
                LineSource {
                    timeline_index: 0,
                    track_index: 0,
                    line_index: 0,
                    start: LyricTime::ZERO,
                    gap_before: None,
                },
                LineSource {
                    timeline_index: 1,
                    track_index: 0,
                    line_index: 1,
                    start: LyricTime::from_secs(1),
                    gap_before: None,
                },
            ],
            heights: HeightIndex::new(vec![80.0, 80.0]),
            lines: vec![Some(first), Some(second)],
            gap_expansions: vec![0.0, 0.0],
            active_gap: None,
            loaded: 0..2,
        };

        assert_eq!(layout.hit_test(60.0), None);
        assert_eq!(
            layout.hit_test(75.0),
            Some(LineHit {
                timeline_index: 1,
                start: LyricTime::from_secs(1),
            })
        );
    }

    #[test]
    fn original_lane_precedes_custom_lanes_without_reordering_custom_content() {
        let line = LyricLine {
            id: LineId::new(1),
            range: TimeRange::new(LyricTime::ZERO, Some(LyricTime::from_secs(2))),
            lanes: vec![
                LyricLane::new(LaneKind::custom("first"), "first"),
                LyricLane::new(LaneKind::Original, "main"),
                LyricLane::new(LaneKind::custom("second"), "second"),
            ],
        };
        assert_eq!(ordered_lane_indices(&line), vec![1, 0, 2]);
    }

    #[test]
    fn custom_lane_uses_only_its_own_explicit_segment_timing() {
        let range = TimeRange::new(LyricTime::ZERO, Some(LyricTime::from_secs(1)));
        let custom = LyricLane::new(LaneKind::custom("annotation"), "note");
        assert_eq!(lane_timing(&custom), (Vec::new(), Vec::new()));

        let custom = crate::model::LyricLane {
            segments: vec![TimedSegment {
                range,
                text_range: 0..4,
            }],
            ..custom
        };
        assert_eq!(lane_timing(&custom), (vec![range], vec![0..4]));
    }

    #[test]
    fn lane_language_overrides_track_language_for_shaping() {
        assert_eq!(
            shaping_language(Some("ja-JP"), Some("zh-CN")),
            Some("ja-JP")
        );
        assert_eq!(shaping_language(None, Some("zh-TW")), Some("zh-TW"));
        assert_eq!(shaping_language(Some(""), Some("ko-KR")), Some("ko-KR"));
        assert_eq!(shaping_language(None, None), None);

        let style = LyricsStyle::default();
        let layout = make_layout(
            &pango::Context::new(),
            "漢字",
            shaping_language(None, Some("zh-TW")),
            200.0,
            style.lane(LaneSlot::Original),
            &style,
        );
        let language = layout.attributes().and_then(|attributes| {
            attributes.iterator().attrs().iter().find_map(|attribute| {
                attribute
                    .downcast_ref::<pango::AttrLanguage>()
                    .map(pango::AttrLanguage::value)
            })
        });
        assert_eq!(language, Some(pango::Language::from_string("zh-TW")));
    }

    #[test]
    fn gap_slot_is_inserted_and_removed_dynamically() {
        let gap = TimeRange::new(LyricTime::from_secs(3), Some(LyricTime::from_secs(6)));
        let mut layout = DocumentLayout {
            width: 100.0,
            sources: vec![LineSource {
                timeline_index: 0,
                track_index: 0,
                line_index: 0,
                start: LyricTime::from_secs(6),
                gap_before: Some(gap),
            }],
            heights: HeightIndex::new(vec![100.0]),
            lines: vec![None],
            gap_expansions: vec![0.0],
            active_gap: None,
            loaded: 0..0,
        };

        assert_eq!(layout.total_height(), 100.0);
        assert!(layout.set_gap_expansion(Some(0), 0.5));
        assert_eq!(layout.total_height(), 124.0);
        assert!(layout.set_gap_expansion(Some(0), 1.0));
        assert_eq!(layout.total_height(), 148.0);
        assert!(layout.set_gap_expansion(None, 0.0));
        assert_eq!(layout.total_height(), 100.0);
    }

    #[test]
    fn height_index_updates_prefixes_without_rebuilding_all_entries() {
        let mut index = HeightIndex::new(vec![10.0, 20.0, 30.0, 40.0]);
        assert_eq!(index.prefix(3), 60.0);
        assert_eq!(index.line_at(29.0), Some(1));

        assert!(index.set(1, 50.0));
        assert_eq!(index.prefix(3), 90.0);
        assert_eq!(index.line_at(29.0), Some(1));
        assert_eq!(index.line_at(61.0), Some(2));
    }

    #[test]
    fn geometric_window_uses_viewport_and_line_overscan() {
        let layout = DocumentLayout {
            width: 100.0,
            sources: (0..100)
                .map(|index| LineSource {
                    timeline_index: index,
                    track_index: 0,
                    line_index: index,
                    start: LyricTime::ZERO,
                    gap_before: None,
                })
                .collect(),
            heights: HeightIndex::new(vec![20.0; 100]),
            lines: vec![None; 100],
            gap_expansions: vec![0.0; 100],
            active_gap: None,
            loaded: 0..0,
        };

        assert_eq!(
            layout.visible_range(800.0, 100.0, Overscan::new(2, 3)),
            38..48
        );
        assert!(
            layout
                .visible_range(1_900.0, 100.0, Overscan::new(2, 3))
                .end
                <= 100
        );
    }

    #[test]
    fn layout_slots_are_created_and_evicted_only_for_the_requested_window() {
        let document = document(100);
        let timeline = Timeline::new(&document);
        let style = LyricsStyle::default();
        let context = pango::Context::new();
        let mut layout =
            DocumentLayout::new(&document, &timeline, 640.0, &style, LaneVisibility::ALL);

        layout.ensure_range(
            &document,
            &context,
            &style,
            LaneVisibility::ALL,
            5..10,
            ViewportAnchor::ScrollOffset(100.0),
        );
        assert_eq!(layout.loaded_count(), 5);
        assert!(layout.line(7).is_some());
        assert!(layout.line(50).is_none());

        layout.ensure_range(
            &document,
            &context,
            &style,
            LaneVisibility::ALL,
            50..55,
            ViewportAnchor::ScrollOffset(3_000.0),
        );
        assert_eq!(layout.loaded_count(), 5);
        assert!(layout.line(7).is_none());
        assert!(layout.line(52).is_some());
    }

    #[test]
    fn focus_anchor_materializes_the_focus_and_corrects_its_center() {
        let document = document(20);
        let timeline = Timeline::new(&document);
        let style = LyricsStyle::default();
        let context = pango::Context::new();
        let mut layout =
            DocumentLayout::new(&document, &timeline, 640.0, &style, LaneVisibility::ALL);
        let old_center = layout.line_center(12).unwrap();

        let update = layout.ensure_range(
            &document,
            &context,
            &style,
            LaneVisibility::ALL,
            0..3,
            ViewportAnchor::FocusLine(12),
        );
        let new_center = layout.line_center(12).unwrap();

        assert!(update.range.contains(&12));
        assert!(layout.line(12).is_some());
        assert!((update.scroll_correction - (new_center - old_center)).abs() <= 0.01);
    }

    #[test]
    fn viewport_height_change_reuses_materialized_line_layouts() {
        let document = document(20);
        let timeline = Timeline::new(&document);
        let style = LyricsStyle::default();
        let context = pango::Context::new();
        let mut layout =
            DocumentLayout::new(&document, &timeline, 640.0, &style, LaneVisibility::ALL);
        let focus = 12;
        layout.ensure_range(
            &document,
            &context,
            &style,
            LaneVisibility::ALL,
            10..15,
            ViewportAnchor::FocusLine(focus),
        );
        let first = layout.line(focus).unwrap() as *const LineLayout;

        layout.ensure_range(
            &document,
            &context,
            &style,
            LaneVisibility::ALL,
            8..18,
            ViewportAnchor::FocusLine(focus),
        );
        let second = layout.line(focus).unwrap() as *const LineLayout;

        assert_eq!(first, second);
        assert_eq!(layout.width(), 640.0);
    }

    #[test]
    fn unloaded_document_keeps_only_lightweight_geometry() {
        let layout = DocumentLayout {
            width: 100.0,
            sources: (0..10)
                .map(|index| LineSource {
                    timeline_index: index,
                    track_index: 0,
                    line_index: index,
                    start: LyricTime::ZERO,
                    gap_before: None,
                })
                .collect(),
            heights: HeightIndex::new(vec![20.0; 10]),
            lines: vec![None; 10],
            gap_expansions: vec![0.0; 10],
            active_gap: None,
            loaded: 0..0,
        };
        assert_eq!(layout.loaded_count(), 0);
        assert_eq!(layout.total_height(), 200.0);
    }
}
