use gtk::{
    gdk,
    prelude::*,
};

const TARGET_SAMPLES: usize = 4_096;
const QUANTIZATION_LEVELS: usize = 16;
const BIN_COUNT: usize = QUANTIZATION_LEVELS * QUANTIZATION_LEVELS * QUANTIZATION_LEVELS;

/// Colors used to build an immersive backdrop from a cover image.
///
/// Hosts that already have palette metadata should construct this type directly.
/// Otherwise, [`CoverPalette::from_texture`] performs one synchronous texture
/// download and samples it; callers should invoke it only when the cover changes.
#[derive(Clone, Debug, PartialEq)]
pub struct CoverPalette {
    pub dominant: gdk::RGBA,
    pub accent: gdk::RGBA,
    pub secondary: gdk::RGBA,
    pub foreground: gdk::RGBA,
    pub scrim: gdk::RGBA,
}

impl Default for CoverPalette {
    fn default() -> Self {
        Self {
            dominant: rgba(0.055, 0.071, 0.12, 1.0),
            accent: rgba(0.36, 0.29, 0.92, 1.0),
            secondary: rgba(0.80, 0.24, 0.56, 1.0),
            foreground: rgba(1.0, 1.0, 1.0, 1.0),
            scrim: rgba(0.0, 0.0, 0.0, 0.42),
        }
    }
}

impl CoverPalette {
    pub fn new(dominant: gdk::RGBA, accent: gdk::RGBA, secondary: gdk::RGBA) -> Self {
        let foreground = readable_foreground(&dominant);
        let scrim = readable_scrim(&foreground);
        Self {
            dominant: opaque(dominant),
            accent: opaque(accent),
            secondary: opaque(secondary),
            foreground,
            scrim,
        }
    }

    /// Extracts a palette from a texture once.
    ///
    /// This method performs a GPU readback for non-memory textures. Prefer
    /// [`CoverPalette::from_rgba8`] or a host-provided palette when decoded
    /// cover pixels are already available.
    pub fn from_texture(texture: &gdk::Texture) -> Self {
        let mut downloader = gdk::TextureDownloader::new(texture);
        downloader.set_format(gdk::MemoryFormat::R8g8b8a8);
        let (bytes, stride) = downloader.download_bytes();
        Self::from_rgba8(
            bytes.as_ref(),
            texture.width() as usize,
            texture.height() as usize,
            stride,
        )
    }

    /// Extracts a palette from unpremultiplied RGBA8 pixels.
    pub fn from_rgba8(pixels: &[u8], width: usize, height: usize, stride: usize) -> Self {
        if !valid_buffer(pixels, width, height, stride) {
            return Self::default();
        }

        let bins = quantize(pixels, width, height, stride);
        let colors = populated_colors(&bins);
        let Some(dominant) = colors.iter().max_by_key(|color| color.count) else {
            return Self::default();
        };

        let accent =
            select_accent(&colors, dominant, None).unwrap_or_else(|| derived_accent(dominant));
        let secondary = select_accent(&colors, dominant, Some(&accent))
            .unwrap_or_else(|| derived_secondary(dominant, &accent));

        Self::new(dominant.rgba(), accent.rgba(), secondary.rgba())
    }

    pub fn with_scrim_alpha(mut self, alpha: f32) -> Self {
        self.scrim.set_alpha(alpha.clamp(0.0, 1.0));
        self
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct ColorBin {
    count: u32,
    red: u64,
    green: u64,
    blue: u64,
}

#[derive(Clone, Copy, Debug)]
struct SampledColor {
    count: u32,
    red: f32,
    green: f32,
    blue: f32,
}

impl SampledColor {
    fn rgba(&self) -> gdk::RGBA {
        rgba(self.red, self.green, self.blue, 1.0)
    }

    fn saturation(&self) -> f32 {
        let max = self.red.max(self.green).max(self.blue);
        let min = self.red.min(self.green).min(self.blue);
        if max <= f32::EPSILON {
            0.0
        } else {
            (max - min) / max
        }
    }

    fn distance(&self, other: &Self) -> f32 {
        let red = self.red - other.red;
        let green = self.green - other.green;
        let blue = self.blue - other.blue;
        (red * red + green * green + blue * blue).sqrt()
    }
}

fn valid_buffer(pixels: &[u8], width: usize, height: usize, stride: usize) -> bool {
    width > 0
        && height > 0
        && stride >= width.saturating_mul(4)
        && pixels.len() >= stride.saturating_mul(height)
}

fn quantize(pixels: &[u8], width: usize, height: usize, stride: usize) -> Vec<ColorBin> {
    let mut bins = vec![ColorBin::default(); BIN_COUNT];
    let step = sampling_step(width, height);

    for y in (0..height).step_by(step) {
        for x in (0..width).step_by(step) {
            let offset = y * stride + x * 4;
            let alpha = pixels[offset + 3];
            if alpha < 96 {
                continue;
            }
            add_sample(
                &mut bins,
                pixels[offset],
                pixels[offset + 1],
                pixels[offset + 2],
            );
        }
    }

    bins
}

fn sampling_step(width: usize, height: usize) -> usize {
    let area = width.saturating_mul(height);
    if area <= TARGET_SAMPLES {
        return 1;
    }
    ((area as f64 / TARGET_SAMPLES as f64).sqrt().ceil() as usize).max(1)
}

fn add_sample(bins: &mut [ColorBin], red: u8, green: u8, blue: u8) {
    let index = ((red as usize >> 4) << 8) | ((green as usize >> 4) << 4) | (blue as usize >> 4);
    let bin = &mut bins[index];
    bin.count += 1;
    bin.red += red as u64;
    bin.green += green as u64;
    bin.blue += blue as u64;
}

fn populated_colors(bins: &[ColorBin]) -> Vec<SampledColor> {
    bins.iter()
        .filter(|bin| bin.count > 0)
        .map(|bin| {
            let denominator = bin.count as f32 * 255.0;
            SampledColor {
                count: bin.count,
                red: bin.red as f32 / denominator,
                green: bin.green as f32 / denominator,
                blue: bin.blue as f32 / denominator,
            }
        })
        .collect()
}

fn select_accent(
    colors: &[SampledColor], dominant: &SampledColor, excluded: Option<&SampledColor>,
) -> Option<SampledColor> {
    colors
        .iter()
        .copied()
        .filter(|candidate| candidate.distance(dominant) >= 0.12)
        .filter(|candidate| excluded.is_none_or(|color| candidate.distance(color) >= 0.16))
        .max_by(|left, right| {
            accent_score(left, dominant).total_cmp(&accent_score(right, dominant))
        })
}

fn accent_score(candidate: &SampledColor, dominant: &SampledColor) -> f32 {
    let population = (candidate.count as f32).sqrt();
    let saturation = 0.25 + candidate.saturation();
    let separation = 0.4 + candidate.distance(dominant);
    population * saturation * separation
}

fn derived_accent(dominant: &SampledColor) -> SampledColor {
    let (red, green, blue) = if relative_luminance(&dominant.rgba()) > 0.45 {
        (
            dominant.red * 0.58,
            dominant.green * 0.52,
            dominant.blue * 0.72,
        )
    } else {
        (
            (dominant.red + 0.36).min(1.0),
            (dominant.green + 0.24).min(1.0),
            (dominant.blue + 0.44).min(1.0),
        )
    };
    SampledColor {
        count: 1,
        red,
        green,
        blue,
    }
}

fn derived_secondary(dominant: &SampledColor, accent: &SampledColor) -> SampledColor {
    SampledColor {
        count: 1,
        red: (accent.blue * 0.72 + dominant.red * 0.28).clamp(0.0, 1.0),
        green: (accent.red * 0.62 + dominant.green * 0.38).clamp(0.0, 1.0),
        blue: (accent.green * 0.68 + dominant.blue * 0.32).clamp(0.0, 1.0),
    }
}

fn readable_foreground(background: &gdk::RGBA) -> gdk::RGBA {
    let white = rgba(1.0, 1.0, 1.0, 1.0);
    let black = rgba(0.035, 0.035, 0.04, 1.0);
    if contrast_ratio(background, &white) >= contrast_ratio(background, &black) {
        white
    } else {
        black
    }
}

fn readable_scrim(foreground: &gdk::RGBA) -> gdk::RGBA {
    if foreground.red() > 0.5 {
        rgba(0.0, 0.0, 0.0, 0.42)
    } else {
        rgba(1.0, 1.0, 1.0, 0.30)
    }
}

fn contrast_ratio(left: &gdk::RGBA, right: &gdk::RGBA) -> f32 {
    let left = relative_luminance(left);
    let right = relative_luminance(right);
    (left.max(right) + 0.05) / (left.min(right) + 0.05)
}

fn relative_luminance(color: &gdk::RGBA) -> f32 {
    0.2126 * linear(color.red()) + 0.7152 * linear(color.green()) + 0.0722 * linear(color.blue())
}

fn linear(component: f32) -> f32 {
    if component <= 0.04045 {
        component / 12.92
    } else {
        ((component + 0.055) / 1.055).powf(2.4)
    }
}

fn opaque(mut color: gdk::RGBA) -> gdk::RGBA {
    color.set_alpha(1.0);
    color
}

fn rgba(red: f32, green: f32, blue: f32, alpha: f32) -> gdk::RGBA {
    gdk::RGBA::new(red, green, blue, alpha)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_distinct_accents() {
        let pixels = [
            16, 24, 48, 255, 16, 24, 48, 255, 230, 48, 96, 255, 36, 180, 220, 255,
        ];
        let palette = CoverPalette::from_rgba8(&pixels, 4, 1, 16);

        assert_ne!(palette.dominant, palette.accent);
        assert_ne!(palette.accent, palette.secondary);
    }

    #[test]
    fn invalid_buffers_use_fallback_palette() {
        assert_eq!(
            CoverPalette::from_rgba8(&[], 10, 10, 40),
            CoverPalette::default()
        );
    }
}
