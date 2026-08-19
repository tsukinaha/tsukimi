use std::{
    ops::Range,
    sync::Arc,
};

use gtk::{
    graphene,
    gsk,
    prelude::*,
};

use crate::{
    render::{
        batch::blur_for_distance,
        cache::RenderCache,
        highlight::RevealPart,
        layout::{
            LaneLayout,
            LineLayout,
        },
        style::{
            LyricsStyle,
            TransitionEasing,
        },
        visual::{
            GapPhase,
            LaneVisual,
            LineVisual,
        },
    },
    time::LyricTime,
};

#[derive(Clone, Copy, Debug)]
pub(crate) struct LineMotionLeg {
    started_us: i64,
    duration_us: i64,
    stagger_us: i64,
    max_stagger_lines: usize,
    scroll_delta: f32,
    focus_line: Option<usize>,
    easing: TransitionEasing,
}

impl LineMotionLeg {
    pub(crate) fn new(
        started_us: i64, duration_us: i64, stagger_us: i64, max_stagger_lines: usize,
        scroll_delta: f32, focus_line: Option<usize>, easing: TransitionEasing,
    ) -> Self {
        Self {
            started_us,
            duration_us: duration_us.max(0),
            stagger_us: stagger_us.max(0),
            max_stagger_lines,
            scroll_delta,
            focus_line,
            easing,
        }
    }

    pub(crate) fn finished(self, now_us: i64) -> bool {
        now_us.saturating_sub(self.started_us) >= self.total_duration_us()
    }

    pub(crate) fn overall_progress(self, now_us: i64) -> f32 {
        animation_progress(
            now_us.saturating_sub(self.started_us),
            self.total_duration_us(),
        )
    }

    fn total_duration_us(self) -> i64 {
        let stagger_lines = i64::try_from(self.max_stagger_lines).unwrap_or(i64::MAX);
        self.duration_us
            .saturating_add(self.stagger_us.saturating_mul(stagger_lines))
    }

    fn line_offset(self, index: usize, now_us: i64) -> f32 {
        let distance = self
            .focus_line
            .map_or(0, |focus| index.abs_diff(focus))
            .min(self.max_stagger_lines);
        let distance = i64::try_from(distance).unwrap_or(i64::MAX);
        let delay = self.stagger_us.saturating_mul(distance);
        let elapsed = now_us.saturating_sub(self.started_us).saturating_sub(delay);
        let progress = if elapsed <= 0 {
            0.0
        } else if self.duration_us <= 0 || elapsed >= self.duration_us {
            1.0
        } else {
            self.easing.apply(elapsed as f32 / self.duration_us as f32)
        };
        self.scroll_delta * (1.0 - progress)
    }
}

#[derive(Clone, Debug)]
pub struct LineMotion {
    sampled_at_us: i64,
    legs: Arc<[LineMotionLeg]>,
}

impl LineMotion {
    pub(crate) fn new(sampled_at_us: i64, legs: Arc<[LineMotionLeg]>) -> Self {
        Self {
            sampled_at_us,
            legs,
        }
    }

    pub(crate) fn line_offset(&self, index: usize) -> f32 {
        self.legs
            .iter()
            .map(|leg| leg.line_offset(index, self.sampled_at_us))
            .sum()
    }
}

fn animation_progress(elapsed_us: i64, duration_us: i64) -> f32 {
    if elapsed_us <= 0 {
        return 0.0;
    }
    if duration_us <= 0 || elapsed_us >= duration_us {
        return 1.0;
    }
    elapsed_us as f32 / duration_us as f32
}

#[derive(Clone, Debug)]
pub struct SnapshotFrame {
    pub position: LyricTime,
    pub visible: Range<usize>,
    pub current_line: Option<usize>,
    pub scroll_offset: f32,

    pub gap_phase: Option<GapPhase>,
    pub gap_line: Option<usize>,
    pub gap_layout_animating: bool,
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub scale_factor: f64,
    pub scale_generation: u64,
    pub manual_scroll: bool,
    pub line_motion: Option<LineMotion>,
    pub hover_line: Option<usize>,
    pub hover_opacity: f32,
    pub activation_line: Option<usize>,
    pub activation_progress: f32,
}

pub fn snapshot_lyrics(
    snapshot: &gtk::Snapshot, cache: &RenderCache, style: &LyricsStyle, frame: &SnapshotFrame,
) {
    if frame.viewport_width <= 0.0 || frame.viewport_height <= 0.0 {
        return;
    }
    let viewport = graphene::Rect::new(0.0, 0.0, frame.viewport_width, frame.viewport_height);
    snapshot.push_clip(&viewport);
    snapshot.save();
    snapshot.translate(&graphene::Point::new(0.0, -frame.scroll_offset));
    append_interaction_background(snapshot, cache, style, frame);
    let static_node = if frame.gap_layout_animating || frame.line_motion.is_some() {
        None
    } else {
        cache.static_node(
            frame.visible.clone(),
            frame.current_line,
            frame.scale_generation,
            frame.scale_factor,
            frame.manual_scroll,
        )
    };
    if frame.line_motion.is_some() {
        append_transition_lines(snapshot, cache, frame);
    } else if frame.gap_layout_animating
        && let Some(node) = cache.unbatched_static_node(
            frame.visible.clone(),
            frame.current_line,
            frame.manual_scroll,
        )
    {
        snapshot.append_node(&node);
    }
    if let Some(node) = static_node {
        snapshot.append_node(&node);
    }
    append_current_line(snapshot, cache, style, frame);
    append_gap(snapshot, cache, frame);
    snapshot.restore();
    snapshot.pop();
}

#[derive(Clone, Copy)]
struct LineBackground {
    line: usize,
    color: gtk::gdk::RGBA,
    opacity: f32,
    scale: f32,
}

fn append_interaction_background(
    snapshot: &gtk::Snapshot, cache: &RenderCache, style: &LyricsStyle, frame: &SnapshotFrame,
) {
    if let Some(line) = frame.hover_line {
        append_line_background(
            snapshot,
            cache,
            style,
            frame,
            LineBackground {
                line,
                color: style.interaction.hover_fill,
                opacity: frame.hover_opacity,
                scale: 1.0,
            },
        );
    }
    if let Some(line) = frame.activation_line {
        let eased = 1.0 - (1.0 - frame.activation_progress.clamp(0.0, 1.0)).powi(3);
        append_line_background(
            snapshot,
            cache,
            style,
            frame,
            LineBackground {
                line,
                color: style.interaction.activated_fill,
                opacity: 1.0 - eased,
                scale: 0.96 + 0.04 * eased,
            },
        );
    }
}

fn append_line_background(
    snapshot: &gtk::Snapshot, cache: &RenderCache, style: &LyricsStyle, frame: &SnapshotFrame,
    background: LineBackground,
) {
    let LineBackground {
        line: line_index,
        color,
        opacity,
        scale,
    } = background;
    let Some((line, _)) = cache.line_scene(line_index) else {
        return;
    };
    let offset = line_motion_offset(frame, line_index);
    let rect = line_background_rect(line, frame.viewport_width, offset);
    let center = graphene::Point::new(
        rect.x() + rect.width() * 0.5,
        rect.y() + rect.height() * 0.5,
    );
    let rounded = gsk::RoundedRect::from_rect(rect, style.interaction.corner_radius.max(0.0));
    snapshot.save();
    snapshot.translate(&center);
    snapshot.scale(scale, scale);
    snapshot.translate(&graphene::Point::new(-center.x(), -center.y()));
    snapshot.push_opacity(opacity.clamp(0.0, 1.0) as f64);
    snapshot.push_rounded_clip(&rounded);
    snapshot.append_color(&color, &rect);
    snapshot.pop();
    snapshot.pop();
    snapshot.restore();
}

fn line_background_rect(line: &LineLayout, viewport_width: f32, offset: f32) -> graphene::Rect {
    let (content_left, content_right) = line
        .lanes
        .iter()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(left, right), lane| {
            (left.min(lane.x), right.max(lane.x + lane.width))
        });
    let (content_left, content_right) =
        if content_left.is_finite() && content_right.is_finite() && content_right > content_left {
            (content_left, content_right)
        } else {
            (0.0, viewport_width.max(1.0))
        };
    let left = (content_left - line.interaction_insets.left)
        .max(0.0)
        .min(viewport_width.max(0.0));
    let right = (content_right + line.interaction_insets.right)
        .max(left + 1.0)
        .min(viewport_width.max(left + 1.0));
    graphene::Rect::new(
        left,
        line.top + line.content_top - line.interaction_insets.top + offset,
        (right - left).max(1.0),
        line.content_height + line.interaction_insets.top + line.interaction_insets.bottom,
    )
}

fn append_transition_lines(snapshot: &gtk::Snapshot, cache: &RenderCache, frame: &SnapshotFrame) {
    for index in frame.visible.clone() {
        if Some(index) == frame.current_line {
            continue;
        }
        let Some((line, visual)) = cache.line_scene(index) else {
            continue;
        };
        let offset = line_motion_offset(frame, index);
        snapshot.save();
        snapshot.translate(&graphene::Point::new(0.0, offset));
        let blur = if frame.manual_scroll {
            0.0
        } else {
            let screen_center = line.center_y() - frame.scroll_offset + offset;
            blur_for_distance((screen_center - frame.viewport_height * 0.5).abs())
        };
        if blur > 0.0 {
            snapshot.push_blur(blur);
        }
        visual.append_normal(snapshot, line);
        if blur > 0.0 {
            snapshot.pop();
        }
        snapshot.restore();
    }
}

fn line_motion_offset(frame: &SnapshotFrame, index: usize) -> f32 {
    frame
        .line_motion
        .as_ref()
        .map(|motion| motion.line_offset(index))
        .unwrap_or(0.0)
}

fn append_gap(snapshot: &gtk::Snapshot, cache: &RenderCache, frame: &SnapshotFrame) {
    let Some(phase) = frame.gap_phase else {
        return;
    };
    let Some(dot_node) = cache.gap_node() else {
        return;
    };
    let Some(gap_line) = frame.gap_line else {
        return;
    };
    let Some(center_y) = cache
        .layout()
        .and_then(|layout| layout.gap_center(gap_line))
    else {
        return;
    };
    let center_x = frame.viewport_width * 0.5;
    for dot in phase.dots() {
        snapshot.save();
        snapshot.translate(&graphene::Point::new(
            center_x + dot.offset_x,
            center_y + dot.offset_y,
        ));
        snapshot.scale(dot.scale, dot.scale);
        snapshot.push_opacity(dot.opacity as f64);
        snapshot.append_node(dot_node);
        snapshot.pop();
        snapshot.restore();
    }
}

fn append_current_line(
    snapshot: &gtk::Snapshot, cache: &RenderCache, style: &LyricsStyle, frame: &SnapshotFrame,
) {
    let Some(index) = frame.current_line else {
        return;
    };
    let Some((line, visual)) = cache.line_scene(index) else {
        return;
    };
    let offset = line_motion_offset(frame, index);
    snapshot.save();
    snapshot.translate(&graphene::Point::new(0.0, offset));
    let blur = if frame.manual_scroll {
        0.0
    } else {
        let screen_center = line.center_y() - frame.scroll_offset + offset;
        blur_for_distance((screen_center - frame.viewport_height * 0.5).abs())
    };
    if blur > 0.0 {
        snapshot.push_blur(blur);
    }
    append_current_lanes(snapshot, line, visual, style, frame.position);
    if blur > 0.0 {
        snapshot.pop();
    }
    snapshot.restore();
}

fn append_current_lanes(
    snapshot: &gtk::Snapshot, line: &LineLayout, visual: &LineVisual, style: &LyricsStyle,
    position: LyricTime,
) {
    for (lane_index, lane) in line.lanes.iter().enumerate() {
        let Some(lane_visual) = visual.lane(lane_index) else {
            continue;
        };
        snapshot.save();
        snapshot.translate(&graphene::Point::new(lane.x, line.top + lane.y));
        if lane.has_timing() {
            append_timed_lane(
                snapshot,
                lane,
                lane_visual,
                position,
                style.interaction.highlight_lift,
            );
        } else {
            snapshot.append_node_if_some(lane_visual.current());
        }
        snapshot.restore();
    }
}

fn append_timed_lane(
    snapshot: &gtk::Snapshot, lane: &LaneLayout, visual: &LaneVisual, position: LyricTime,
    highlight_lift: f32,
) {
    snapshot.append_node_if_some(visual.inactive());
    let Some(active) = visual.active() else {
        return;
    };

    snapshot.save();
    snapshot.translate(&graphene::Point::new(0.0, -highlight_lift.max(0.0)));
    snapshot.push_mask(gsk::MaskMode::Alpha);
    for (segment, range) in lane.segment_times.iter().copied().enumerate() {
        let progress = segment_progress(range, position);
        if progress <= 0.0 {
            continue;
        }
        lane.highlights
            .visit_gradient_reveal(segment, progress, |part| {
                append_reveal_mask(snapshot, part);
            });
    }
    snapshot.pop();
    snapshot.append_node(active);
    snapshot.pop();
    snapshot.restore();
}

fn append_reveal_mask(snapshot: &gtk::Snapshot, part: RevealPart) {
    if !part.feather {
        snapshot.append_color(&gtk::gdk::RGBA::WHITE, &part.rect);
        return;
    }

    const MAX_FEATHER: f32 = 16.0;
    let settle = 1.0 - smoothstep(0.82, 1.0, part.fraction);
    let feather = part.rect.width().min(MAX_FEATHER) * settle;
    if feather <= 0.5 {
        snapshot.append_color(&gtk::gdk::RGBA::WHITE, &part.rect);
        return;
    }

    let (solid, fade, start, end) = if part.rtl {
        let fade = graphene::Rect::new(part.rect.x(), part.rect.y(), feather, part.rect.height());
        let solid = graphene::Rect::new(
            part.rect.x() + feather,
            part.rect.y(),
            part.rect.width() - feather,
            part.rect.height(),
        );
        let start = graphene::Point::new(fade.x(), fade.y());
        let end = graphene::Point::new(fade.x() + fade.width(), fade.y());
        (solid, fade, start, end)
    } else {
        let solid_width = part.rect.width() - feather;
        let solid = graphene::Rect::new(
            part.rect.x(),
            part.rect.y(),
            solid_width,
            part.rect.height(),
        );
        let fade = graphene::Rect::new(
            part.rect.x() + solid_width,
            part.rect.y(),
            feather,
            part.rect.height(),
        );
        let start = graphene::Point::new(fade.x(), fade.y());
        let end = graphene::Point::new(fade.x() + fade.width(), fade.y());
        (solid, fade, start, end)
    };
    if solid.width() > 0.0 {
        snapshot.append_color(&gtk::gdk::RGBA::WHITE, &solid);
    }
    let stops = if part.rtl {
        [
            gsk::ColorStop::new(0.0, gtk::gdk::RGBA::TRANSPARENT),
            gsk::ColorStop::new(1.0, gtk::gdk::RGBA::WHITE),
        ]
    } else {
        [
            gsk::ColorStop::new(0.0, gtk::gdk::RGBA::WHITE),
            gsk::ColorStop::new(1.0, gtk::gdk::RGBA::TRANSPARENT),
        ]
    };
    snapshot.append_linear_gradient(&fade, &start, &end, &stops);
}

fn smoothstep(edge0: f32, edge1: f32, value: f32) -> f32 {
    let progress = ((value - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    progress * progress * (3.0 - 2.0 * progress)
}

fn segment_progress(range: crate::time::TimeRange, position: LyricTime) -> f32 {
    if position < range.start {
        return 0.0;
    }
    let Some(end) = range.end else {
        return 1.0;
    };
    let duration = end.as_micros().saturating_sub(range.start.as_micros());
    if duration <= 0 || position >= end {
        return 1.0;
    }
    let elapsed = position.as_micros().saturating_sub(range.start.as_micros());
    (elapsed as f64 / duration as f64).clamp(0.0, 1.0) as f32
}

trait SnapshotNodeExt {
    fn append_node_if_some(&self, node: Option<&gsk::RenderNode>);
}

impl SnapshotNodeExt for gtk::Snapshot {
    fn append_node_if_some(&self, node: Option<&gsk::RenderNode>) {
        if let Some(node) = node {
            self.append_node(node);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn motion(
        sampled_at_us: i64, started_us: i64, duration_us: i64, stagger_us: i64,
        max_stagger_lines: usize, scroll_delta: f32, focus_line: Option<usize>,
        easing: TransitionEasing,
    ) -> LineMotion {
        LineMotion::new(
            sampled_at_us,
            Arc::from([LineMotionLeg::new(
                started_us,
                duration_us,
                stagger_us,
                max_stagger_lines,
                scroll_delta,
                focus_line,
                easing,
            )]),
        )
    }

    #[test]
    fn hover_background_surrounds_the_reserved_text_area() {
        let layout = gtk::pango::Layout::new(&gtk::pango::Context::new());
        let lane = LaneLayout {
            lane_index: 0,
            slot: crate::render::style::LaneSlot::Original,
            layout: layout.clone(),
            x: 20.0,
            y: 0.0,
            width: 600.0,
            height: 40.0,
            segment_times: Vec::new(),
            highlights: crate::render::highlight::HighlightGeometry::new(&layout, "", &[]),
        };
        let line = LineLayout {
            timeline_index: 0,
            start: LyricTime::ZERO,
            top: 100.0,
            content_top: 0.0,
            content_height: 40.0,
            height: 80.0,
            gap_before: None,
            gap_height: 0.0,
            interaction_insets: crate::render::layout::InteractionInsets {
                left: 12.0,
                right: 12.0,
                top: 10.0,
                bottom: 10.0,
            },
            lanes: vec![lane],
        };

        assert_eq!(
            line_background_rect(&line, 640.0, 0.0),
            graphene::Rect::new(8.0, 90.0, 624.0, 60.0)
        );
    }

    #[test]
    fn staggered_lines_finish_at_different_times_without_double_scroll() {
        let near = motion(
            800_000,
            0,
            800_000,
            90_000,
            4,
            100.0,
            Some(0),
            TransitionEasing::EaseOut,
        );
        assert_eq!(near.line_offset(0), 0.0);
        assert!(near.line_offset(4) > 0.0);

        let start = motion(
            0,
            0,
            800_000,
            90_000,
            4,
            100.0,
            Some(0),
            TransitionEasing::EaseOut,
        );
        let start_offset = start.line_offset(0);
        let to_scroll = 200.0;
        assert_eq!(to_scroll - start_offset, 100.0);

        let finished = motion(
            1_160_000,
            0,
            800_000,
            90_000,
            4,
            100.0,
            Some(0),
            TransitionEasing::EaseOut,
        );
        assert_eq!(finished.line_offset(4), 0.0);
    }

    #[test]
    fn line_motion_uses_configured_easing_and_caps_stagger_distance() {
        let linear = motion(
            400_000,
            0,
            800_000,
            0,
            2,
            100.0,
            Some(0),
            TransitionEasing::Linear,
        );
        let ease_in = motion(
            400_000,
            0,
            800_000,
            0,
            2,
            100.0,
            Some(0),
            TransitionEasing::EaseIn,
        );
        assert_eq!(linear.line_offset(0), 50.0);
        assert!(ease_in.line_offset(0) > linear.line_offset(0));

        let capped = motion(
            979_999,
            0,
            800_000,
            90_000,
            2,
            100.0,
            Some(0),
            TransitionEasing::Linear,
        );
        assert!(capped.line_offset(20) > 0.0);
        let finished = motion(
            980_000,
            0,
            800_000,
            90_000,
            2,
            100.0,
            Some(0),
            TransitionEasing::Linear,
        );
        assert_eq!(finished.line_offset(20), 0.0);
    }

    #[test]
    fn whole_line_mask_is_absolute_for_dense_and_reverse_timing() {
        let start = LyricTime::from_micros(17_000_000);
        let zero = crate::time::TimeRange::new(start, Some(start));
        assert_eq!(segment_progress(zero, start), 1.0);
        assert_eq!(
            segment_progress(zero, LyricTime::from_micros(16_999_999)),
            0.0
        );

        let range = crate::time::TimeRange::new(start, Some(LyricTime::from_micros(17_001_000)));
        let later = segment_progress(range, LyricTime::from_micros(17_000_750));
        let earlier = segment_progress(range, LyricTime::from_micros(17_000_250));
        assert!(later > earlier);
        assert_eq!(later, 0.75);
        assert_eq!(earlier, 0.25);
    }
}
