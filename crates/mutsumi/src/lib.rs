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

pub fn init() {
    control::init();
    danmakw::init();
}
