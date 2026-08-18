mod alloc;
mod danmaku;
mod layout;
mod parse;
mod pool;
mod render;
mod timer;

pub use alloc::*;
pub use danmaku::*;
pub use layout::*;
pub use parse::*;
pub use pool::*;
pub use render::*;
pub use timer::*;

use gtk::prelude::*;

pub fn init() {
    Danmakw::ensure_type();
}
