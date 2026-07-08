use std::{
    cell::RefCell,
    sync::atomic::{
        AtomicBool,
        Ordering,
    },
};

use gtk::{
    CssProvider,
    gdk::Display,
};

use crate::{
    Window,
    ui::SETTINGS,
};

pub mod cursor;
pub mod focus;
pub mod osk;

pub use focus::{
    controller_navigation_enabled,
    set_tv_focused,
};

pub static TV_MODE_ACTIVE: AtomicBool = AtomicBool::new(false);
static TV_CSS_ATTACHED: AtomicBool = AtomicBool::new(false);

thread_local! {
    static TV_PROVIDER: RefCell<Option<CssProvider>> = const { RefCell::new(None) };
}

pub fn is_tv_mode_active() -> bool {
    TV_MODE_ACTIVE.load(Ordering::Relaxed)
}

pub fn set_tv_mode_active(active: bool) {
    TV_MODE_ACTIVE.store(active, Ordering::Relaxed);
}

/// Resolve whether TV visual mode should be active for this session.
pub fn resolve_tv_mode(cli_tv_mode: bool, cli_fullscreen: bool) -> bool {
    cli_tv_mode || crate::steam::is_steam_big_picture() || SETTINGS.tv_mode() || cli_fullscreen
}

fn build_tv_css() -> String {
    let scale = SETTINGS.tv_ui_scale();
    let mut css = format!(
        r#"
        :root {{
            --tv-scale: {scale};
        }}
        "#
    );

    if let Ok(bytes) = gtk::gio::resources_lookup_data(
        "/moe/tsuna/tsukimi/tv.css",
        gtk::gio::ResourceLookupFlags::NONE,
    ) && let Ok(extra) = std::str::from_utf8(&bytes)
    {
        css.push_str(extra);
    }

    css
}

pub fn sync_tv_style() {
    let display = Display::default().expect("Could not connect to a display.");
    TV_PROVIDER.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.is_none() {
            *slot = Some(CssProvider::new());
        }
        let provider = slot.as_ref().expect("TV CSS provider");
        provider.load_from_string(&build_tv_css());
        if !TV_CSS_ATTACHED.swap(true, Ordering::Relaxed) {
            gtk::style_context_add_provider_for_display(
                &display,
                provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
            );
        }
    });
}

pub fn apply_to_window(window: &Window, cli_fullscreen: bool) {
    set_tv_mode_active(true);
    if !SETTINGS.gamepad_enabled() {
        let _ = SETTINGS.set_gamepad_enabled(true);
    }
    sync_tv_style();
    window.enable_tv_mode_ui(cli_fullscreen);
}

pub fn remove_from_window(window: &Window) {
    set_tv_mode_active(false);
    window.disable_tv_mode_ui();
}

pub fn apply_tv_startup(window: &Window, cli_fullscreen: bool) {
    if !is_tv_mode_active() {
        return;
    }

    apply_to_window(window, cli_fullscreen);
}

pub fn tv_scale_factor() -> f64 {
    if is_tv_mode_active() {
        SETTINGS.tv_ui_scale()
    } else {
        1.0
    }
}

pub fn scale_i32(value: i32) -> i32 {
    (value as f64 * tv_scale_factor()).round() as i32
}

pub fn scale_pair((w, h): (i32, i32)) -> (i32, i32) {
    (scale_i32(w), scale_i32(h))
}
