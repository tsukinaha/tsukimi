#![cfg_attr(docsrs, feature(doc_cfg))]

macro_rules! assert_initialized_main_thread {
    () => {};
}

macro_rules! skip_assert_initialized {
    () => {};
}

#[allow(unused_imports)]
mod auto;
pub use crate::auto::*;

#[doc(hidden)]
pub static INITIALIZED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub use ffi;

pub fn init() -> Result<(), &'static str> {
    unsafe {
        use glib::translate::*;
        use std::ptr;

        if from_glib(ffi::clapper_init_check(ptr::null_mut(), ptr::null_mut())) {
            crate::INITIALIZED.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        } else {
            Err("Failed to initialize Clapper")
        }
    }
}
