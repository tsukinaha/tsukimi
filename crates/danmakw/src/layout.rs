pub trait LayoutExt {
    // default
    fn dx_per_millis(&self, width: f32) -> f32;
}

impl LayoutExt for pango::Layout {
    fn dx_per_millis(&self, width: f32) -> f32 {
        let total_width = width + (self.pixel_size().0 / pango::SCALE) as f32;
        total_width / 3000.0
    }
}
