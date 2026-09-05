mod backend;
mod error;
mod layout;
mod mpv;
mod play;
mod player;

pub use backend::*;
pub use error::*;
pub use layout::*;
pub use mpv::*;
pub use play::*;
pub use player::*;

use gtk::prelude::*;

pub fn init() {
    MutsumiVideoLayout::ensure_type();
}
