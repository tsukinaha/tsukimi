use std::{
    cell::RefCell,
    sync::atomic::{
        AtomicBool,
        AtomicU64,
        Ordering,
    },
};

use gtk::{
    gdk,
    glib::WeakRef,
    prelude::*,
};

use super::focus::controller_navigation_enabled;

fn cursor_auto_hide_enabled() -> bool {
    controller_navigation_enabled() && crate::ui::SETTINGS.gamepad_enabled()
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

static POINTER_HIDDEN: AtomicBool = AtomicBool::new(false);
static LAST_GAMEPAD_CURSOR_MS: AtomicU64 = AtomicU64::new(0);

/// Ignore mouse motion briefly after gamepad input (scroll/focus can spuriously wake the pointer).
const POINTER_SUPPRESS_AFTER_GAMEPAD_MS: u64 = 500;

thread_local! {
    static WINDOW: RefCell<Option<WeakRef<crate::Window>>> = const { RefCell::new(None) };
    static BLANK_CURSOR: RefCell<Option<gdk::Cursor>> = const { RefCell::new(None) };
}

pub fn register_window(window: &crate::Window) {
    WINDOW.with(|slot| *slot.borrow_mut() = Some(window.downgrade()));
}

pub fn on_pointer_activity() {
    if !cursor_auto_hide_enabled() {
        restore();
        return;
    }
    if now_ms().saturating_sub(LAST_GAMEPAD_CURSOR_MS.load(Ordering::Relaxed))
        < POINTER_SUPPRESS_AFTER_GAMEPAD_MS
    {
        return;
    }
    show();
    crate::ui::widgets::hover_scale::request_pointer_targeting_sync();
}

pub fn on_gamepad_activity() {
    if !cursor_auto_hide_enabled() {
        return;
    }
    LAST_GAMEPAD_CURSOR_MS.store(now_ms(), Ordering::Relaxed);
    hide();
    crate::ui::widgets::hover_scale::request_pointer_targeting_sync();
}

/// While gamepad is driving the UI, ignore stale pointer hover under the last mouse
/// position (including newly bound widgets after pagination or page changes).
pub fn suppress_pointer_hover() -> bool {
    if !cursor_auto_hide_enabled() {
        return false;
    }
    POINTER_HIDDEN.load(Ordering::Relaxed) || super::osk::gamepad_owns_direct_input()
}

pub fn restore() {
    show();
}

fn hide() {
    with_surface(|surface| {
        if let Some(cursor) = blank_cursor() {
            surface.set_cursor(Some(&cursor));
            POINTER_HIDDEN.store(true, Ordering::Relaxed);
        }
    });
}

fn show() {
    with_surface(|surface| {
        if !POINTER_HIDDEN.load(Ordering::Relaxed) {
            return;
        }
        surface.set_cursor(gdk::Cursor::from_name("default", None).as_ref());
        POINTER_HIDDEN.store(false, Ordering::Relaxed);
    });
}

fn with_surface(f: impl FnOnce(&gdk::Surface)) {
    WINDOW.with(|slot| {
        let Some(window) = slot.borrow().as_ref().and_then(|weak| weak.upgrade()) else {
            return;
        };
        let Some(surface) = window.native().and_then(|native| native.surface()) else {
            return;
        };
        f(&surface);
    });
}

fn blank_cursor() -> Option<gdk::Cursor> {
    BLANK_CURSOR.with(|slot| {
        if slot.borrow().is_none() {
            let cursor = if let Some(cursor) = gdk::Cursor::from_name("none", None) {
                cursor
            } else {
                let bytes = gtk::glib::Bytes::from_static(&[0, 0, 0, 0]);
                let texture = gdk::MemoryTextureBuilder::new()
                    .set_width(1)
                    .set_height(1)
                    .set_format(gdk::MemoryFormat::R8g8b8a8)
                    .set_stride(4)
                    .set_bytes(Some(&bytes))
                    .build();
                gdk::Cursor::from_texture(&texture, 0, 0, None)
            };
            *slot.borrow_mut() = Some(cursor);
        }
        slot.borrow().clone()
    })
}
