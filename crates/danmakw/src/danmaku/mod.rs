mod queue;
mod sort;

pub use queue::DanmakuQueue;

use crate::DanmakuVisual;

#[derive(Debug, Clone, PartialEq)]
pub struct Danmaku {
    pub content: String,
    // milliseconds
    pub start: f64,
    pub color: Color,
    pub mode: DanmakuMode,
}

pub struct ScrollingDanmaku {
    pub danmaku: Danmaku,
    pub visual: DanmakuVisual,
    pub x: f32,
    pub row: usize,
    pub velocity_x: f32,
    pub width: f32,
}

pub struct CenterDanmaku {
    pub danmaku: Danmaku,
    pub visual: DanmakuVisual,
    pub width: f32,
    pub row: usize,
    pub remaining_time: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum DanmakuMode {
    #[default]
    Scroll,
    TopCenter,
    BottomCenter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, glib::Enum)]
#[enum_type(name = "DanmakwIntensity")]
pub enum Intensity {
    #[enum_value(name = "Quarter", nick = "quarter")]
    Quarter,
    #[default]
    #[enum_value(name = "Half", nick = "half")]
    Half,
    #[enum_value(name = "Full", nick = "full")]
    Full,
    #[enum_value(name = "Overlay", nick = "overlay")]
    Overlay,
}

impl Intensity {
    pub fn row_fraction(self) -> f32 {
        match self {
            Self::Quarter => 0.25,
            Self::Half => 0.5,
            Self::Full | Self::Overlay => 1.0,
        }
    }

    pub fn allows_overlay(self) -> bool {
        matches!(self, Self::Overlay)
    }
}

impl From<u32> for Intensity {
    fn from(index: u32) -> Self {
        match index {
            0 => Self::Quarter,
            1 => Self::Half,
            2 => Self::Full,
            3 => Self::Overlay,
            _ => Self::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Default for Color {
    fn default() -> Self {
        Self {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        }
    }
}

impl From<Color> for gtk::gdk::RGBA {
    fn from(color: Color) -> Self {
        Self::new(
            color.r as f32 / 255.0,
            color.g as f32 / 255.0,
            color.b as f32 / 255.0,
            color.a as f32 / 255.0,
        )
    }
}
