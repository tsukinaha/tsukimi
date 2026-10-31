pub mod control;
pub mod video;

pub use control::*;
pub use danmakw::*;
pub use video::*;

pub fn force_gl_renderer() {
    unsafe {
        std::env::set_var("GSK_RENDERER", "gl");
    }
}

pub fn enable_gdk_hdr() {
    unsafe {
        std::env::set_var("GDK_DEBUG", "hdr");
    }
}

fn gdk_hdr_enabled() -> bool {
    std::env::var("GDK_DEBUG")
        .map(|v| v.contains("hdr"))
        .unwrap_or(false)
}

pub fn init() {
    video::init();
    control::init();
    danmakw::init();
}
