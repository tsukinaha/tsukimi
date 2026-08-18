use std::sync::Arc;

use super::*;
use pango::{
    Context,
    Layout,
};

const SCROLL_DURATION_MS: f32 = 10000.0;
const CENTER_DURATION_MS: f32 = 5000.0;
const RESET_DELTA_MS: f32 = 1000.0;
const SEEK_PREROLL_STEP_MS: f64 = 50.0;

const BAKE_BATCH_MIN: usize = 32;
const BAKE_MAX_WAIT_MS: f64 = 1500.0;

struct FontMetrics {
    spacing_factor: f32,
    font_px: f64,
    line_height: f32,
    spacing: f32,
}

impl FontMetrics {
    fn compute(font_px: f64, spacing_factor: f32) -> Self {
        Self {
            spacing_factor,
            font_px,
            line_height: font_px as f32 * spacing_factor,
            spacing: font_px as f32,
        }
    }

    fn is_stale(&self, font_px: f64, spacing_factor: f32) -> bool {
        self.font_px != font_px || self.spacing_factor != spacing_factor
    }
}

fn font_px_for(context: &Context, desc: &pango::FontDescription, font_size: f64) -> f64 {
    context
        .load_font(desc)
        .map(|font| font.describe_with_absolute_size().size() as f64 / pango::SCALE as f64)
        .unwrap_or(font_size * 96.0 / 72.0)
}

struct CenterRowTracker {
    occupied: Vec<bool>,
    overlay_hint: usize,
}

impl CenterRowTracker {
    fn new(max_rows: usize) -> Self {
        Self {
            occupied: vec![false; max_rows],
            overlay_hint: 0,
        }
    }

    fn max_rows(&self) -> usize {
        self.occupied.len()
    }

    fn find_row(&mut self, allow_overlay: bool) -> Option<usize> {
        if let Some(row) = self.occupied.iter().position(|occ| !occ) {
            self.occupied[row] = true;
            return Some(row);
        }

        if allow_overlay && !self.occupied.is_empty() {
            let row = self.overlay_hint % self.occupied.len();
            self.overlay_hint = self.overlay_hint.wrapping_add(1);
            Some(row)
        } else {
            None
        }
    }

    fn release(&mut self, row: usize) {
        if let Some(occ) = self.occupied.get_mut(row) {
            *occ = false;
        }
    }

    fn resize(&mut self, size: usize) {
        self.occupied.resize(size, false);
    }

    fn clear(&mut self) {
        self.occupied.fill(false);
    }
}

pub struct DanmakwRenderer {
    pub danmaku_queue: DanmakuQueue,
    pub last_time: f64,

    pub paused: bool,

    pub scroll_danmaku: Vec<ScrollingDanmaku>,
    pub scroll_max_rows: usize,

    pub top_center_danmaku: Vec<CenterDanmaku>,
    pub bottom_center_danmaku: Vec<CenterDanmaku>,

    pub line_height: f32,
    pub top_padding: f32,
    pub font_size: f64,
    pub font_name: Arc<str>,
    pub font_weight: pango::Weight,
    pub spacing_factor: f32,
    spacing: f32,
    pub outline_px: f64,
    pub shadow_offset: f64,
    pub scale_factor: f64,

    pub gsk_renderer: Option<gtk::gsk::Renderer>,
    pub speed_factor: f64,
    pub screen_height: f32,
    intensity: Intensity,
    overlay_scroll_hint: usize,

    top_center_tracker: CenterRowTracker,
    bottom_center_tracker: CenterRowTracker,
    cached_metrics: Option<FontMetrics>,

    defer_bake: bool,
    bake_deadline: Option<f64>,
}

impl Default for DanmakwRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl DanmakwRenderer {
    pub fn new() -> Self {
        let scroll_max_rows = 25;
        let font_size = 24.0_f64;
        let font_px_logical = font_size * (96.0 / 72.0);
        let spacing_factor = 1.5_f32;
        let line_height = font_px_logical as f32 * spacing_factor;
        let spacing = font_px_logical as f32;
        let top_padding = 10.0;

        Self {
            font_name: Arc::from(""),
            font_size,
            font_weight: pango::Weight::Normal,
            spacing_factor,
            spacing,
            outline_px: 1.0,
            shadow_offset: 1.0,
            danmaku_queue: DanmakuQueue::new(),
            scroll_danmaku: Vec::new(),
            top_center_danmaku: Vec::new(),
            bottom_center_danmaku: Vec::new(),
            scroll_max_rows,
            line_height,
            top_padding,
            scale_factor: 1.0,
            gsk_renderer: None,
            speed_factor: 1.0,
            top_center_tracker: CenterRowTracker::new(10),
            bottom_center_tracker: CenterRowTracker::new(10),
            paused: false,
            last_time: 0.0,
            screen_height: 0.0,
            intensity: Intensity::default(),
            overlay_scroll_hint: 0,
            cached_metrics: None,
            defer_bake: false,
            bake_deadline: None,
        }
    }

    pub fn recompute_max_rows(&mut self) {
        if self.screen_height <= 0.0 || self.line_height <= 0.0 {
            return;
        }

        let total_rows = ((self.screen_height - self.top_padding) / self.line_height) as usize;
        let total_rows = total_rows.max(1);

        let scroll = ((total_rows as f32 * self.intensity.row_fraction()) as usize).max(1);
        let center = (scroll / 5).max(1);

        self.scroll_max_rows = scroll;
        self.top_center_tracker.resize(center);
        self.bottom_center_tracker.resize(center);
    }

    fn scroll_row_is_free(
        &self, row: usize, width: f32, reach_edge_time: f32, spacing: f32,
    ) -> bool {
        let last = self
            .scroll_danmaku
            .iter()
            .filter(|d| d.row == row)
            .map(|d| (d.x, d.width, d.velocity_x))
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        match last {
            None => true,
            Some((last_x, last_width, last_vel)) => {
                let leave_time = (last_x + last_width + spacing) / last_vel.abs();
                leave_time < reach_edge_time && width > last_width + spacing + last_x
            }
        }
    }

    fn find_scroll_row(&self, width: f32, reach_edge_time: f32, spacing: f32) -> Option<usize> {
        (0..self.scroll_max_rows)
            .find(|&row| self.scroll_row_is_free(row, width, reach_edge_time, spacing))
    }

    fn add_scroll_danmaku(
        &mut self, visual: DanmakuVisual, text_width: f32, width: f32, danmaku: Danmaku,
    ) {
        let velocity_x = -(width + text_width) / SCROLL_DURATION_MS * self.speed_factor as f32;

        let reach_edge_time = width / velocity_x.abs();
        let spacing = self.spacing;

        let target_row = if let Some(row) = self.find_scroll_row(width, reach_edge_time, spacing) {
            row
        } else if self.intensity.allows_overlay() {
            let row = self.overlay_scroll_hint % self.scroll_max_rows;
            self.overlay_scroll_hint = self.overlay_scroll_hint.wrapping_add(1);
            row
        } else {
            return;
        };

        self.scroll_danmaku.push(ScrollingDanmaku {
            danmaku,
            visual,
            x: width,
            row: target_row,
            velocity_x,
            width: text_width,
        });
    }

    fn add_topcenter_danmaku(&mut self, visual: DanmakuVisual, text_width: f32, danmaku: Danmaku) {
        let Some(target_row) = self
            .top_center_tracker
            .find_row(self.intensity.allows_overlay())
        else {
            return;
        };

        self.top_center_danmaku.push(CenterDanmaku {
            danmaku,
            visual,
            width: text_width,
            row: target_row,
            remaining_time: CENTER_DURATION_MS,
        });
    }

    fn add_bottomcenter_danmaku(
        &mut self, visual: DanmakuVisual, text_width: f32, danmaku: Danmaku,
    ) {
        let Some(target_row) = self
            .bottom_center_tracker
            .find_row(self.intensity.allows_overlay())
        else {
            return;
        };

        self.bottom_center_danmaku.push(CenterDanmaku {
            danmaku,
            visual,
            width: text_width,
            row: target_row,
            remaining_time: CENTER_DURATION_MS,
        });
    }

    pub fn rebuild_visible_state_at(
        &mut self, context: &Context, screen_width: f32, time_milis: f64,
    ) {
        let preroll_ms = SCROLL_DURATION_MS.max(CENTER_DURATION_MS) as f64;
        let start_time = (time_milis - preroll_ms).max(0.0);

        self.scroll_danmaku.clear();
        self.top_center_danmaku.clear();
        self.bottom_center_danmaku.clear();
        self.top_center_tracker.clear();
        self.bottom_center_tracker.clear();

        self.danmaku_queue.reset_time(start_time);
        self.last_time = start_time;

        self.defer_bake = true;
        let mut simulated_time = start_time;
        while simulated_time + SEEK_PREROLL_STEP_MS < time_milis {
            simulated_time += SEEK_PREROLL_STEP_MS;
            self.update(context, screen_width, simulated_time);
        }
        self.update(context, screen_width, time_milis);
        self.defer_bake = false;

        self.bake_pending(time_milis, true);
    }

    fn bake_pending(&mut self, now: f64, force: bool) {
        if self.defer_bake || self.gsk_renderer.is_none() {
            return;
        }
        let scale = self.scale_factor;

        let waiting = self
            .scroll_danmaku
            .iter()
            .filter(|d| d.visual.needs_bake(scale))
            .count()
            + self
                .top_center_danmaku
                .iter()
                .filter(|d| d.visual.needs_bake(scale))
                .count()
            + self
                .bottom_center_danmaku
                .iter()
                .filter(|d| d.visual.needs_bake(scale))
                .count();

        if waiting == 0 {
            self.bake_deadline = None;
            return;
        }
        let deadline = *self.bake_deadline.get_or_insert(now + BAKE_MAX_WAIT_MS);
        if !force && waiting < BAKE_BATCH_MIN && now < deadline {
            return;
        }
        self.bake_deadline = None;

        let gsk_renderer = self.gsk_renderer.clone().expect("checked above");
        let (scroll, top, bottom) = (
            &mut self.scroll_danmaku,
            &mut self.top_center_danmaku,
            &mut self.bottom_center_danmaku,
        );
        let mut pending: Vec<&mut DanmakuVisual> = scroll
            .iter_mut()
            .map(|d| &mut d.visual)
            .chain(top.iter_mut().map(|d| &mut d.visual))
            .chain(bottom.iter_mut().map(|d| &mut d.visual))
            .filter(|v| v.needs_bake(scale))
            .collect();

        if !pending.is_empty() {
            crate::bake_batch(&gsk_renderer, scale, &mut pending);
        }
    }

    pub fn update(&mut self, context: &Context, screen_width: f32, time_milis: f64) {
        let delta_time = (time_milis - self.last_time) as f32;
        self.last_time = time_milis;

        self.bake_pending(time_milis, false);

        if delta_time.abs() > RESET_DELTA_MS {
            self.danmaku_queue.reset_time(time_milis);
            return;
        }

        let mut danmaku_queue = std::mem::take(&mut self.danmaku_queue);
        for next_danmaku in danmaku_queue.pop_to_time_iter(time_milis) {
            self.add_danmaku(context, screen_width, next_danmaku.clone());
        }
        self.danmaku_queue = danmaku_queue;

        let speed = self.speed_factor as f32;
        for text in self.scroll_danmaku.iter_mut() {
            text.x += text.velocity_x * delta_time * speed;
        }

        self.scroll_danmaku.retain(|text| text.x + text.width > 0.0);

        let (top_danmaku, top_tracker) =
            (&mut self.top_center_danmaku, &mut self.top_center_tracker);
        for text in top_danmaku.iter_mut() {
            text.remaining_time -= delta_time;
        }
        top_danmaku.retain(|text| {
            if text.remaining_time <= 0.0 {
                top_tracker.release(text.row);
                false
            } else {
                true
            }
        });

        let (bottom_danmaku, bottom_tracker) = (
            &mut self.bottom_center_danmaku,
            &mut self.bottom_center_tracker,
        );
        for text in bottom_danmaku.iter_mut() {
            text.remaining_time -= delta_time;
        }
        bottom_danmaku.retain(|text| {
            if text.remaining_time <= 0.0 {
                bottom_tracker.release(text.row);
                false
            } else {
                true
            }
        });
    }

    pub fn add_danmaku(&mut self, context: &Context, screen_width: f32, danmaku: Danmaku) {
        let mut font_desc = pango::FontDescription::default();
        font_desc.set_family(&self.font_name);
        font_desc.set_weight(self.font_weight);
        font_desc.set_size((self.font_size * pango::SCALE as f64).round() as i32);

        let font_px = font_px_for(context, &font_desc, self.font_size);
        if self
            .cached_metrics
            .as_ref()
            .is_none_or(|m| m.is_stale(font_px, self.spacing_factor))
        {
            let m = FontMetrics::compute(font_px, self.spacing_factor);
            self.line_height = m.line_height;
            self.spacing = m.spacing;
            self.cached_metrics = Some(m);
            self.recompute_max_rows();
        }

        let layout = Layout::new(context);
        layout.set_font_description(Some(&font_desc));
        layout.set_text(&danmaku.content);

        let text_width = layout.pixel_size().0 as f32;

        let Some(visual) =
            DanmakuVisual::new(&layout, self.outline_px, self.shadow_offset, danmaku.color)
        else {
            return;
        };

        match danmaku.mode {
            DanmakuMode::Scroll => {
                self.add_scroll_danmaku(visual, text_width, screen_width, danmaku);
            }
            DanmakuMode::TopCenter => {
                self.add_topcenter_danmaku(visual, text_width, danmaku);
            }
            DanmakuMode::BottomCenter => {
                self.add_bottomcenter_danmaku(visual, text_width, danmaku);
            }
        }
    }

    pub fn clear_danmaku(&mut self) {
        self.scroll_danmaku.clear();
        self.top_center_danmaku.clear();
        self.bottom_center_danmaku.clear();
        self.top_center_tracker.clear();
        self.bottom_center_tracker.clear();
    }

    pub fn scrolled_top_y(&self, row: usize) -> f32 {
        self.top_padding + row as f32 * self.line_height
    }

    pub fn top_center_y(&self, row: usize) -> f32 {
        self.top_padding + row as f32 * self.line_height
    }

    pub fn bottom_center_y(&self, row: usize, screen_height: f32) -> f32 {
        screen_height - self.top_padding - (row + 1) as f32 * self.line_height
    }

    pub fn set_font_weight_index(&mut self, index: u32) {
        self.font_weight = Self::pango_weight_from_index(index);
    }

    fn pango_weight_from_index(index: u32) -> pango::Weight {
        match index {
            0 => pango::Weight::Thin,
            1 => pango::Weight::Ultralight,
            2 => pango::Weight::Light,
            3 => pango::Weight::Semilight,
            4 => pango::Weight::Book,
            5 => pango::Weight::Normal,
            6 => pango::Weight::Medium,
            7 => pango::Weight::Semibold,
            8 => pango::Weight::Bold,
            9 => pango::Weight::Ultrabold,
            10 => pango::Weight::Heavy,
            11 => pango::Weight::Ultraheavy,
            _ => pango::Weight::Normal,
        }
    }

    pub fn intensity(&self) -> Intensity {
        self.intensity
    }

    pub fn set_intensity(&mut self, intensity: Intensity) {
        self.intensity = intensity;
        self.recompute_max_rows();
    }

    pub fn top_center_max_rows(&self) -> usize {
        self.top_center_tracker.max_rows()
    }

    pub fn bottom_center_max_rows(&self) -> usize {
        self.bottom_center_tracker.max_rows()
    }
}
