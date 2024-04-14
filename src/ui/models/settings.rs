use std::ops::Deref;

use gtk::{
    gio,
    glib::{self, thread_guard::ThreadGuard},
    prelude::*,
};

use crate::APP_ID;

pub struct Settings(ThreadGuard<gio::Settings>);

impl Settings {
    const KEY_IS_PROGRESS_ENABLED: &'static str = "is-progress-enabled";
    const KEY_IS_OVERLAY: &'static str = "is-overlay";
    const KEY_IS_FULLSCREEN: &'static str = "is-fullscreen";
    const KEY_IS_RESUME: &'static str = "is-resume";
    const KEY_IS_BLUR_ENABLED: &'static str = "is-blurenabled";
    const KEY_THEME: &'static str = "theme";
    const KEY_PROXY: &'static str = "proxy";
    const KEY_ROOT_PIC: &'static str = "root-pic";
    const KEY_BACKGROUND_HEIGHT: &'static str = "background-height";
    const KEY_IS_BACKGROUND_ENABLED: &'static str = "is-backgroundenabled";
    const KEY_IS_FORCE_WINDOW: &'static str = "is-force-window";
    const KEY_THREADS: &'static str = "threads";
    const KEY_PIC_OPACITY: &'static str = "pic-opacity";
    const KEY_PIC_BLUR: &'static str = "pic-blur";

    pub fn set_forcewindow(&self, force_window: bool) -> Result<(), glib::BoolError> {
        self.set_boolean(Self::KEY_IS_FORCE_WINDOW, force_window)
    }

    pub fn forcewindow(&self) -> bool {
        self.boolean(Self::KEY_IS_FORCE_WINDOW)
    }

    pub fn set_progress(&self, progress: bool) -> Result<(), glib::BoolError> {
        self.set_boolean(Self::KEY_IS_PROGRESS_ENABLED, progress)
    }
    pub fn progress(&self) -> bool {
        self.boolean(Self::KEY_IS_PROGRESS_ENABLED)
    }

    pub fn set_overlay(&self, overlay: bool) -> Result<(), glib::BoolError> {
        self.set_boolean(Self::KEY_IS_OVERLAY, overlay)
    }
    pub fn overlay(&self) -> bool {
        self.boolean(Self::KEY_IS_OVERLAY)
    }

    pub fn set_fullscreen(&self, fullscreen: bool) -> Result<(), glib::BoolError> {
        self.set_boolean(Self::KEY_IS_FULLSCREEN, fullscreen)
    }
    pub fn fullscreen(&self) -> bool {
        self.boolean(Self::KEY_IS_FULLSCREEN)
    }

    pub fn set_resume(&self, resume: bool) -> Result<(), glib::BoolError> {
        self.set_boolean(Self::KEY_IS_RESUME, resume)
    }

    pub fn resume(&self) -> bool {
        self.boolean(Self::KEY_IS_RESUME)
    }

    pub fn set_theme(&self, theme: &str) -> Result<(), glib::BoolError> {
        self.set_string(Self::KEY_THEME, theme)
    }

    pub fn theme(&self) -> String {
        self.string(Self::KEY_THEME).to_string()
    }

    pub fn set_proxy(&self, proxy: &str) -> Result<(), glib::BoolError> {
        self.set_string(Self::KEY_PROXY, proxy)
    }

    pub fn proxy(&self) -> String {
        self.string(Self::KEY_PROXY).to_string()
    }

    pub fn set_root_pic(&self, root_pic: &str) -> Result<(), glib::BoolError> {
        self.set_string(Self::KEY_ROOT_PIC, root_pic)
    }

    pub fn root_pic(&self) -> String {
        self.string(Self::KEY_ROOT_PIC).to_string()
    }

    pub fn set_background_height(&self, background_height: i32) -> Result<(), glib::BoolError> {
        self.set_int(Self::KEY_BACKGROUND_HEIGHT, background_height)
    }

    pub fn background_height(&self) -> i32 {
        self.int(Self::KEY_BACKGROUND_HEIGHT)
    }

    pub fn set_threads(&self, threads: i32) -> Result<(), glib::BoolError> {
        self.set_int(Self::KEY_THREADS, threads)
    }

    pub fn threads(&self) -> i32 {
        self.int(Self::KEY_THREADS)
    }

    pub fn set_pic_opacity(&self, pic_opacity: i32) -> Result<(), glib::BoolError> {
        self.set_int(Self::KEY_PIC_OPACITY, pic_opacity)
    }

    pub fn pic_opacity(&self) -> i32 {
        self.int(Self::KEY_PIC_OPACITY)
    }

    pub fn set_pic_blur(&self, pic_blur: i32) -> Result<(), glib::BoolError> {
        self.set_int(Self::KEY_PIC_BLUR, pic_blur)
    }

    pub fn pic_blur(&self) -> i32 {
        self.int(Self::KEY_PIC_BLUR)
    }

    /*** pub fn connect_auto_lock_changed<F>(&self, callback: F) -> glib::SignalHandlerId
    where
        F: Fn(bool) + 'static,
    {
        self.connect_changed(Some(Self::KEY_AUTO_LOCK), move |settings, _key| {
            callback(settings.boolean(Self::KEY_AUTO_LOCK))
        })
    } ***/

    pub fn set_background_enabled(&self, background_enabled: bool) -> Result<(), glib::BoolError> {
        self.set_boolean(Self::KEY_IS_BACKGROUND_ENABLED, background_enabled)
    }

    pub fn background_enabled(&self) -> bool {
        self.boolean(Self::KEY_IS_BACKGROUND_ENABLED)
    }

    pub fn set_blur_enabled(&self, is_blur_enabled: bool) -> Result<(), glib::BoolError> {
        self.set_boolean(Self::KEY_IS_BLUR_ENABLED, is_blur_enabled)
    }

    pub fn is_blur_enabled(&self) -> bool {
        self.boolean(Self::KEY_IS_BLUR_ENABLED)
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self(ThreadGuard::new(gio::Settings::new(APP_ID)))
    }
}

impl Deref for Settings {
    type Target = gio::Settings;

    fn deref(&self) -> &Self::Target {
        self.0.get_ref()
    }
}

unsafe impl Send for Settings {}
unsafe impl Sync for Settings {}
