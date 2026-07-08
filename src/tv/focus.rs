use gtk::prelude::*;

use crate::ui::SETTINGS;

pub fn tv_focus_enabled() -> bool {
    crate::tv::is_tv_mode_active() || SETTINGS.gamepad_enabled()
}

/// Whether gamepad/keyboard should drive TV-style navigation instead of GTK defaults.
pub fn controller_navigation_enabled() -> bool {
    tv_focus_enabled()
}

pub fn set_tv_focused(widget: &impl IsA<gtk::Widget>, focused: bool) {
    if !tv_focus_enabled() {
        widget.remove_css_class("tv-focused");
        return;
    }
    if focused {
        widget.add_css_class("tv-focused");
    } else {
        widget.remove_css_class("tv-focused");
    }
}
