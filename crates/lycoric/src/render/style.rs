use gtk::{
    gdk,
    pango,
};

pub const DEFAULT_TRANSITION_DURATION_US: i64 = 800_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LaneSlot {
    Original,
    Other,
}

impl LaneSlot {
    pub const ALL: [Self; 2] = [Self::Original, Self::Other];
}

/// Colors and opacities used for one lyric lane.
#[derive(Clone, Debug, PartialEq)]
pub struct LaneColors {
    pub current: gdk::RGBA,
    pub non_current: gdk::RGBA,
    pub highlight_active: gdk::RGBA,
    pub highlight_inactive: gdk::RGBA,
    pub current_opacity: f32,
    pub non_current_opacity: f32,
}

impl LaneColors {
    pub fn new(
        current: gdk::RGBA, non_current: gdk::RGBA, highlight_active: gdk::RGBA,
        highlight_inactive: gdk::RGBA, current_opacity: f32, non_current_opacity: f32,
    ) -> Self {
        Self {
            current,
            non_current,
            highlight_active,
            highlight_inactive,
            current_opacity,
            non_current_opacity,
        }
    }

    pub fn clamped_current_opacity(&self) -> f32 {
        self.current_opacity.clamp(0.0, 1.0)
    }

    pub fn clamped_non_current_opacity(&self) -> f32 {
        self.non_current_opacity.clamp(0.0, 1.0)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LaneStyle {
    pub font: pango::FontDescription,
    pub colors: LaneColors,
    /// Extra logical pixels following this lane, in addition to global lane spacing.
    pub spacing_after: f32,
}

impl LaneStyle {
    pub fn new(font: pango::FontDescription, colors: LaneColors) -> Self {
        Self {
            font,
            colors,
            spacing_after: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LaneStyles {
    pub original: LaneStyle,
    pub other: LaneStyle,
}

impl LaneStyles {
    pub fn get(&self, lane: LaneSlot) -> &LaneStyle {
        match lane {
            LaneSlot::Original => &self.original,
            LaneSlot::Other => &self.other,
        }
    }

    pub fn get_mut(&mut self, lane: LaneSlot) -> &mut LaneStyle {
        match lane {
            LaneSlot::Original => &mut self.original,
            LaneSlot::Other => &mut self.other,
        }
    }
}

impl Default for LaneStyles {
    fn default() -> Self {
        let current = rgba(1.0, 1.0, 1.0, 1.0);
        let non_current = rgba(1.0, 1.0, 1.0, 1.0);
        let highlight_active = rgba(1.0, 1.0, 1.0, 1.0);
        let highlight_inactive = rgba(1.0, 1.0, 1.0, 0.48);

        let primary = LaneColors::new(
            current,
            non_current,
            highlight_active,
            highlight_inactive,
            1.0,
            0.46,
        );
        let secondary = LaneColors::new(
            current,
            non_current,
            highlight_active,
            highlight_inactive,
            1.0,
            0.38,
        );

        Self {
            original: LaneStyle::new(font("Sans Bold 30"), primary),
            other: LaneStyle::new(font("Sans 20"), secondary),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutStyle {
    pub line_spacing: f32,
    pub lane_spacing: f32,
    /// Maximum line width in logical pixels. `None` uses all allocated width.
    pub max_width: Option<f32>,
    pub wrap: pango::WrapMode,
    pub alignment: pango::Alignment,
}

impl LayoutStyle {
    pub fn content_width(&self, allocation_width: f32) -> f32 {
        let allocation_width = finite_non_negative(allocation_width);
        match self
            .max_width
            .filter(|width| width.is_finite() && *width > 0.0)
        {
            Some(max_width) => allocation_width.min(max_width),
            None => allocation_width,
        }
    }
}

impl Default for LayoutStyle {
    fn default() -> Self {
        Self {
            line_spacing: 64.0,
            lane_spacing: 6.0,
            max_width: Some(960.0),
            wrap: pango::WrapMode::WordChar,
            alignment: pango::Alignment::Left,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct OutlineStyle {
    pub color: gdk::RGBA,
    pub width: f32,
}

impl OutlineStyle {
    pub fn is_visible(&self) -> bool {
        self.width.is_finite() && self.width > 0.0 && self.color.alpha() > 0.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShadowStyle {
    pub color: gdk::RGBA,
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur_radius: f32,
}

impl ShadowStyle {
    pub fn is_visible(&self) -> bool {
        self.color.alpha() > 0.0
            && self.blur_radius.is_finite()
            && self.blur_radius >= 0.0
            && self.offset_x.is_finite()
            && self.offset_y.is_finite()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextEffects {
    pub outline: Option<OutlineStyle>,
    pub shadow: Option<ShadowStyle>,
}

impl TextEffects {
    pub fn has_visible_effect(&self) -> bool {
        self.outline.as_ref().is_some_and(OutlineStyle::is_visible)
            || self.shadow.as_ref().is_some_and(ShadowStyle::is_visible)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TransitionEasing {
    Linear,
    EaseIn,
    EaseOut,
    #[default]
    EaseInOut,
}

impl TransitionEasing {
    pub fn apply(self, progress: f32) -> f32 {
        let progress = progress.clamp(0.0, 1.0);
        match self {
            Self::Linear => progress,
            Self::EaseIn => progress * progress,
            Self::EaseOut => 1.0 - (1.0 - progress) * (1.0 - progress),
            Self::EaseInOut => progress * progress * (3.0 - 2.0 * progress),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransitionStyle {
    pub duration_us: i64,
    pub easing: TransitionEasing,
}

impl TransitionStyle {
    pub const fn new(duration_us: i64, easing: TransitionEasing) -> Self {
        Self {
            duration_us,
            easing,
        }
    }

    pub fn progress(self, elapsed_us: i64) -> f32 {
        if elapsed_us <= 0 {
            return 0.0;
        }
        if self.duration_us <= 0 || elapsed_us >= self.duration_us {
            return 1.0;
        }

        self.easing
            .apply(elapsed_us as f32 / self.duration_us as f32)
    }
}

impl Default for TransitionStyle {
    fn default() -> Self {
        Self::new(DEFAULT_TRANSITION_DURATION_US, TransitionEasing::EaseOut)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Overscan {
    pub before: usize,
    pub after: usize,
}

impl Overscan {
    pub const fn new(before: usize, after: usize) -> Self {
        Self { before, after }
    }

    pub const fn total(self) -> usize {
        self.before.saturating_add(self.after)
    }
}

impl Default for Overscan {
    fn default() -> Self {
        Self::new(4, 6)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct InteractionStyle {
    pub hover_fill: gdk::RGBA,
    pub activated_fill: gdk::RGBA,
    pub corner_radius: f32,
    pub hover_duration_us: i64,
    pub activation_duration_us: i64,
    pub highlight_lift: f32,
    pub line_stagger_us: i64,
}

impl Default for InteractionStyle {
    fn default() -> Self {
        Self {
            hover_fill: rgba(1.0, 1.0, 1.0, 0.10),
            activated_fill: rgba(1.0, 1.0, 1.0, 0.16),
            corner_radius: 14.0,
            hover_duration_us: 140_000,
            activation_duration_us: 560_000,
            highlight_lift: 1.5,
            line_stagger_us: 90_000,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LyricsStyle {
    pub lanes: LaneStyles,
    pub layout: LayoutStyle,
    pub effects: TextEffects,
    pub transition: TransitionStyle,
    pub interaction: InteractionStyle,
    pub overscan: Overscan,
}

impl LyricsStyle {
    pub fn lane(&self, lane: LaneSlot) -> &LaneStyle {
        self.lanes.get(lane)
    }

    pub fn lane_mut(&mut self, lane: LaneSlot) -> &mut LaneStyle {
        self.lanes.get_mut(lane)
    }
}

fn font(description: &str) -> pango::FontDescription {
    pango::FontDescription::from_string(description)
}

fn rgba(red: f32, green: f32, blue: f32, alpha: f32) -> gdk::RGBA {
    gdk::RGBA::new(red, green, blue, alpha)
}

fn finite_non_negative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}
