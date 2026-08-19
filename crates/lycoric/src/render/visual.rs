use gtk::{
    gdk,
    graphene,
    gsk,
    prelude::*,
};

use crate::{
    render::{
        batch::{
            TextureBudget,
            bake_texture_node,
        },
        layout::{
            LaneLayout,
            LineLayout,
        },
        style::{
            LaneStyle,
            LyricsStyle,
            TextEffects,
        },
    },
    time::{
        LyricTime,
        TimeRange,
    },
};

pub const MIN_GAP_DURATION_US: i64 = 800_000;
const GAP_CYCLE_US: i64 = 1_200_000;
const GAP_EDGE_FADE_US: i64 = 180_000;
const GAP_DOT_DIAMETER: f32 = 8.0;
const GAP_DOT_RADIUS: f32 = GAP_DOT_DIAMETER * 0.5;
const GAP_DOT_SPACING: f32 = 16.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GapPhase {
    phase_bits: u32,
    edge_bits: u32,
    reduced_motion: bool,
}

impl GapPhase {
    pub fn at(range: TimeRange, position: LyricTime, reduced_motion: bool) -> Option<Self> {
        let end = range.end?;
        let duration_us = end.as_micros().saturating_sub(range.start.as_micros());
        if duration_us < MIN_GAP_DURATION_US || !range.contains(position) {
            return None;
        }
        if reduced_motion {
            return Some(Self {
                phase_bits: 0.0f32.to_bits(),
                edge_bits: 1.0f32.to_bits(),
                reduced_motion: true,
            });
        }
        let elapsed_us = position.as_micros().saturating_sub(range.start.as_micros());
        let remaining_us = end.as_micros().saturating_sub(position.as_micros());
        let phase = elapsed_us.rem_euclid(GAP_CYCLE_US) as f32 / GAP_CYCLE_US as f32;
        let edge = edge_opacity(elapsed_us, remaining_us);
        Some(Self {
            phase_bits: phase.to_bits(),
            edge_bits: edge.to_bits(),
            reduced_motion: false,
        })
    }

    pub fn expansion(self) -> f32 {
        f32::from_bits(self.edge_bits).clamp(0.0, 1.0)
    }

    pub fn dots(self) -> [GapDotFrame; 3] {
        if self.reduced_motion {
            return std::array::from_fn(|index| GapDotFrame {
                offset_x: dot_offset(index),
                offset_y: 0.0,
                opacity: 0.72,
                scale: 1.0,
            });
        }
        let phase = f32::from_bits(self.phase_bits);
        let edge = f32::from_bits(self.edge_bits);
        std::array::from_fn(|index| {
            let intensity = pulse_intensity(phase, index);
            GapDotFrame {
                offset_x: dot_offset(index),
                offset_y: -2.0 * intensity,
                opacity: edge * intensity,
                scale: 0.72 + 0.28 * intensity,
            }
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GapDotFrame {
    pub offset_x: f32,
    pub offset_y: f32,
    pub opacity: f32,
    pub scale: f32,
}

#[derive(Clone, Default)]
pub struct GapVisual {
    dot: Option<gsk::RenderNode>,
}

impl GapVisual {
    pub fn build(style: &LyricsStyle) -> Self {
        let colors = &style.lane(crate::render::style::LaneSlot::Original).colors;
        let color = with_opacity(&colors.current, colors.clamped_current_opacity());
        let bounds = graphene::Rect::new(
            -GAP_DOT_RADIUS,
            -GAP_DOT_RADIUS,
            GAP_DOT_DIAMETER,
            GAP_DOT_DIAMETER,
        );
        let color_node: gsk::RenderNode = gsk::ColorNode::new(&color, &bounds).upcast();
        let clip = gsk::RoundedRect::from_rect(bounds, GAP_DOT_RADIUS);
        let dot = gsk::RoundedClipNode::new(&color_node, &clip).upcast();
        Self { dot: Some(dot) }
    }

    pub fn dot(&self) -> Option<&gsk::RenderNode> {
        self.dot.as_ref()
    }
}

fn dot_offset(index: usize) -> f32 {
    (index as f32 - 1.0) * GAP_DOT_SPACING
}

fn edge_opacity(elapsed_us: i64, remaining_us: i64) -> f32 {
    let entering = elapsed_us.max(0) as f32 / GAP_EDGE_FADE_US as f32;
    let leaving = remaining_us.max(0) as f32 / GAP_EDGE_FADE_US as f32;
    entering.min(leaving).clamp(0.0, 1.0)
}

fn pulse_intensity(phase: f32, index: usize) -> f32 {
    let local = (phase - index as f32 * 0.14).rem_euclid(1.0);
    if local < 0.18 {
        local / 0.18
    } else if local < 0.58 {
        1.0 - (local - 0.18) / 0.4
    } else {
        0.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LaneHighlightStatus {
    pub completed_segments: usize,
    pub current_segment: Option<usize>,
    pub progress_bits: u32,
}

impl LaneHighlightStatus {
    pub fn progress(self) -> f32 {
        f32::from_bits(self.progress_bits)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisualSignature {
    pub current_line: Option<usize>,
    pub lanes: Vec<LaneHighlightStatus>,
}

#[derive(Clone, Default)]
struct BakedLaneVisual {
    current: Option<gsk::RenderNode>,
    inactive: Option<gsk::RenderNode>,
    active: Option<gsk::RenderNode>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LineBakeKey {
    scale_generation: u64,
    scale_bits: u64,
    renderer_available: bool,
}

impl LineBakeKey {
    fn new(scale_generation: u64, scale: f64, renderer_available: bool) -> Self {
        Self {
            scale_generation,
            scale_bits: scale.to_bits(),
            renderer_available,
        }
    }
}

#[derive(Clone)]
pub struct LaneVisual {
    normal: Option<gsk::RenderNode>,
    current: Option<gsk::RenderNode>,
    inactive: Option<gsk::RenderNode>,
    active: Option<gsk::RenderNode>,
    baked: BakedLaneVisual,
}

impl LaneVisual {
    pub fn build(layout: &LaneLayout, style: &LaneStyle, effects: &TextEffects) -> Self {
        let colors = &style.colors;
        Self {
            normal: text_node(
                &layout.layout,
                with_opacity(&colors.non_current, colors.clamped_non_current_opacity()),
                effects,
            ),
            current: text_node(
                &layout.layout,
                with_opacity(&colors.current, colors.clamped_current_opacity()),
                effects,
            ),
            inactive: text_node(
                &layout.layout,
                with_opacity(&colors.highlight_inactive, colors.clamped_current_opacity()),
                effects,
            ),
            active: text_node(
                &layout.layout,
                with_opacity(&colors.highlight_active, colors.clamped_current_opacity()),
                effects,
            ),
            baked: BakedLaneVisual::default(),
        }
    }

    pub fn normal(&self) -> Option<&gsk::RenderNode> {
        self.normal.as_ref()
    }

    pub fn current(&self) -> Option<&gsk::RenderNode> {
        preferred_node(self.baked.current.as_ref(), self.current.as_ref())
    }

    pub fn inactive(&self) -> Option<&gsk::RenderNode> {
        preferred_node(self.baked.inactive.as_ref(), self.inactive.as_ref())
    }

    pub fn active(&self) -> Option<&gsk::RenderNode> {
        preferred_node(self.baked.active.as_ref(), self.active.as_ref())
    }

    fn bake_primary(
        &self, layout: &LaneLayout, renderer: &gsk::Renderer, scale: f64,
        budget: &mut TextureBudget,
    ) -> BakedLaneVisual {
        if layout.has_timing() {
            BakedLaneVisual {
                active: bake_optional(renderer, self.active.as_ref(), scale, budget),
                inactive: bake_optional(renderer, self.inactive.as_ref(), scale, budget),
                current: None,
            }
        } else {
            BakedLaneVisual {
                current: bake_optional(renderer, self.current.as_ref(), scale, budget),
                ..Default::default()
            }
        }
    }

    fn replace_baked(&mut self, _layout: &LaneLayout, baked: BakedLaneVisual) {
        self.baked = baked;
    }
}

#[derive(Clone, Default)]
pub struct LineVisual {
    lanes: Vec<LaneVisual>,
    bake_key: Option<LineBakeKey>,
}

impl LineVisual {
    pub fn build(line: &LineLayout, style: &LyricsStyle) -> Self {
        let lanes = line
            .lanes
            .iter()
            .map(|lane| LaneVisual::build(lane, style.lane(lane.slot), &style.effects))
            .collect();
        Self {
            lanes,
            bake_key: None,
        }
    }

    pub(crate) fn prepare_bake(
        &mut self, line: &LineLayout, scale_generation: u64, scale: f64,
        renderer: Option<&gsk::Renderer>,
    ) {
        let key = LineBakeKey::new(scale_generation, scale, renderer.is_some());
        if self.bake_key == Some(key) {
            return;
        }
        self.reset_baked(line);
        let baked = self.bake_lanes(line, renderer, scale);
        for ((layout, visual), baked) in line.lanes.iter().zip(&mut self.lanes).zip(baked) {
            visual.replace_baked(layout, baked);
        }
        self.bake_key = Some(key);
    }

    pub(crate) fn clear_bake(&mut self, line: &LineLayout) {
        if self.bake_key.take().is_none() {
            return;
        }
        self.reset_baked(line);
    }

    fn reset_baked(&mut self, line: &LineLayout) {
        for (layout, visual) in line.lanes.iter().zip(&mut self.lanes) {
            visual.replace_baked(layout, BakedLaneVisual::default());
        }
    }

    fn bake_lanes(
        &self, line: &LineLayout, renderer: Option<&gsk::Renderer>, scale: f64,
    ) -> Vec<BakedLaneVisual> {
        let Some(renderer) = renderer else {
            return vec![BakedLaneVisual::default(); self.lanes.len()];
        };
        let mut budget = TextureBudget::current_line();
        line.lanes
            .iter()
            .zip(&self.lanes)
            .map(|(layout, visual)| visual.bake_primary(layout, renderer, scale, &mut budget))
            .collect()
    }

    pub fn lane(&self, index: usize) -> Option<&LaneVisual> {
        self.lanes.get(index)
    }

    pub fn append_normal(&self, snapshot: &gtk::Snapshot, line: &LineLayout) {
        for (lane, visual) in line.lanes.iter().zip(&self.lanes) {
            append_at(snapshot, visual.normal(), lane.x, line.top + lane.y);
        }
    }
}

fn bake_optional(
    renderer: &gsk::Renderer, node: Option<&gsk::RenderNode>, scale: f64,
    budget: &mut TextureBudget,
) -> Option<gsk::RenderNode> {
    node.and_then(|node| bake_texture_node(renderer, node, scale, budget))
}

fn preferred_node<'a, T>(baked: Option<&'a T>, fallback: Option<&'a T>) -> Option<&'a T> {
    baked.or(fallback)
}

pub fn visual_signature(
    line: Option<(&LineLayout, &LineVisual)>, current_line: Option<usize>, position: LyricTime,
) -> VisualSignature {
    let lanes = line
        .map(|(line, _)| {
            line.lanes
                .iter()
                .map(|lane| highlight_status(lane, position))
                .collect()
        })
        .unwrap_or_default();
    VisualSignature {
        current_line,
        lanes,
    }
}

pub fn highlight_status(lane: &LaneLayout, position: LyricTime) -> LaneHighlightStatus {
    let mut completed_segments = 0;
    for (index, range) in lane.segment_times.iter().copied().enumerate() {
        if position < range.start {
            break;
        }
        let Some(end) = range.end else {
            completed_segments = index + 1;
            continue;
        };
        let duration = end.as_micros().saturating_sub(range.start.as_micros());
        if duration <= 0 || position >= end {
            completed_segments = index + 1;
            continue;
        }
        let elapsed = position.as_micros().saturating_sub(range.start.as_micros());
        let progress = (elapsed as f64 / duration as f64).clamp(0.0, 1.0) as f32;
        return LaneHighlightStatus {
            completed_segments,
            current_segment: Some(index),
            progress_bits: progress.to_bits(),
        };
    }
    LaneHighlightStatus {
        completed_segments,
        current_segment: None,
        progress_bits: 0.0f32.to_bits(),
    }
}

pub fn append_at(snapshot: &gtk::Snapshot, node: Option<&gsk::RenderNode>, x: f32, y: f32) {
    let Some(node) = node else {
        return;
    };
    snapshot.save();
    snapshot.translate(&graphene::Point::new(x, y));
    snapshot.append_node(node);
    snapshot.restore();
}

fn text_node(
    layout: &gtk::pango::Layout, foreground: gdk::RGBA, effects: &TextEffects,
) -> Option<gsk::RenderNode> {
    let snapshot = gtk::Snapshot::new();
    append_shadow(&snapshot, layout, effects);
    append_outline(&snapshot, layout, effects);
    snapshot.append_layout(layout, &foreground);
    snapshot.to_node()
}

fn append_shadow(snapshot: &gtk::Snapshot, layout: &gtk::pango::Layout, effects: &TextEffects) {
    let Some(shadow) = effects.shadow.as_ref().filter(|shadow| shadow.is_visible()) else {
        return;
    };
    snapshot.save();
    snapshot.translate(&graphene::Point::new(shadow.offset_x, shadow.offset_y));
    if shadow.blur_radius > 0.0 {
        snapshot.push_blur(shadow.blur_radius as f64);
    }
    snapshot.append_layout(layout, &shadow.color);
    if shadow.blur_radius > 0.0 {
        snapshot.pop();
    }
    snapshot.restore();
}

fn append_outline(snapshot: &gtk::Snapshot, layout: &gtk::pango::Layout, effects: &TextEffects) {
    let Some(outline) = effects
        .outline
        .as_ref()
        .filter(|outline| outline.is_visible())
    else {
        return;
    };
    let samples = ((outline.width * std::f32::consts::TAU).ceil() as usize).clamp(8, 24);
    let node_snapshot = gtk::Snapshot::new();
    node_snapshot.append_layout(layout, &outline.color);
    let Some(node) = node_snapshot.to_node() else {
        return;
    };
    for index in 0..samples {
        let angle = std::f32::consts::TAU * index as f32 / samples as f32;
        snapshot.save();
        snapshot.translate(&graphene::Point::new(
            angle.cos() * outline.width,
            angle.sin() * outline.width,
        ));
        snapshot.append_node(&node);
        snapshot.restore();
    }
}

fn with_opacity(color: &gdk::RGBA, opacity: f32) -> gdk::RGBA {
    gdk::RGBA::new(
        color.red(),
        color.green(),
        color.blue(),
        color.alpha() * opacity.clamp(0.0, 1.0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::TimeRange;

    fn lane(times: Vec<TimeRange>) -> LaneLayout {
        let context = gtk::pango::Context::new();
        LaneLayout {
            lane_index: 0,
            slot: crate::render::style::LaneSlot::Original,
            layout: gtk::pango::Layout::new(&context),
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 20.0,
            segment_times: times,

            highlights: Default::default(),
        }
    }

    #[test]
    fn gap_phase_requires_a_long_bounded_gap() {
        let start = LyricTime::from_micros(1_000_000);
        let long = TimeRange::new(start, Some(LyricTime::from_micros(1_800_000)));
        let short = TimeRange::new(start, Some(LyricTime::from_micros(1_799_999)));

        assert!(GapPhase::at(long, LyricTime::from_micros(1_400_000), false).is_some());
        assert!(GapPhase::at(short, LyricTime::from_micros(1_400_000), false).is_none());
        assert!(GapPhase::at(long, LyricTime::from_micros(1_800_000), false).is_none());
    }

    #[test]
    fn gap_phase_is_absolute_and_reduced_motion_is_static() {
        let range = TimeRange::new(LyricTime::ZERO, Some(LyricTime::from_micros(2_000_000)));
        let position = LyricTime::from_micros(360_000);
        let first = GapPhase::at(range, position, false).unwrap();
        let second = GapPhase::at(range, position, false).unwrap();
        let later = GapPhase::at(range, LyricTime::from_micros(400_000), false).unwrap();
        assert_eq!(first, second);
        assert_ne!(first, later);
        let dots = first.dots();
        assert!(dots[1].opacity > dots[2].opacity);

        let reduced_a = GapPhase::at(range, position, true).unwrap().dots();
        let reduced_b = GapPhase::at(range, LyricTime::from_micros(900_000), true)
            .unwrap()
            .dots();
        assert_eq!(reduced_a, reduced_b);
        assert!(
            reduced_a
                .iter()
                .all(|dot| dot.opacity == 0.72 && dot.scale == 1.0)
        );
    }

    #[test]
    fn gap_uses_an_overall_edge_fade() {
        let range = TimeRange::new(LyricTime::ZERO, Some(LyricTime::from_micros(2_000_000)));
        let entering = GapPhase::at(range, LyricTime::ZERO, false).unwrap().dots();
        assert!(entering.iter().all(|dot| dot.opacity == 0.0));
    }

    #[test]
    fn stable_getters_prefer_baked_values_and_keep_fallbacks() {
        assert_eq!(preferred_node(Some(&2), Some(&1)), Some(&2));
        assert_eq!(preferred_node::<i32>(None, Some(&1)), Some(&1));
        assert_eq!(preferred_node::<i32>(None, None), None);
    }

    #[test]
    fn bake_state_changes_with_scale_renderer_or_generation() {
        let base = LineBakeKey::new(4, 2.0, true);
        assert_ne!(base, LineBakeKey::new(5, 2.0, true));
        assert_ne!(base, LineBakeKey::new(4, 1.0, true));
        assert_ne!(base, LineBakeKey::new(4, 2.0, false));
    }

    #[test]
    fn only_one_segment_is_dynamic_and_completed_prefix_is_stable() {
        let lane = lane(vec![
            TimeRange::new(LyricTime::from_micros(0), Some(LyricTime::from_micros(100))),
            TimeRange::new(
                LyricTime::from_micros(100),
                Some(LyricTime::from_micros(200)),
            ),
        ]);
        let first = highlight_status(&lane, LyricTime::from_micros(50));
        assert_eq!(first.completed_segments, 0);
        assert_eq!(first.current_segment, Some(0));

        let second = highlight_status(&lane, LyricTime::from_micros(150));
        assert_eq!(second.completed_segments, 1);
        assert_eq!(second.current_segment, Some(1));
    }
}
