use std::sync::atomic::{
    AtomicBool,
    Ordering,
};

use once_cell::sync::Lazy;

pub mod account_item;
pub mod actions;
pub mod background_paintable;
pub mod core_song;
pub mod descriptor;
pub mod dropdown_factory;
pub mod image_tags;
pub mod tu_item;
pub mod tu_object;

pub static IS_ADMIN: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

pub fn set_admin(value: bool) {
    IS_ADMIN.store(value, Ordering::SeqCst);
}
