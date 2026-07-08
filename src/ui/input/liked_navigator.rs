use gtk::prelude::*;

use crate::{
    Window,
    ui::{
        input::{
            actions::InputAction,
            focus_manager::FocusManager,
        },
        widgets::liked::LikedPage,
    },
};

#[derive(Default)]
pub struct LikedNavigator {
    focus: FocusManager,
}

impl LikedNavigator {
    pub fn register(&self, liked: &LikedPage) {
        self.focus.register_rows(liked.focus_hortu_rows());
    }

    pub fn handle(&self, window: &Window, liked: &LikedPage, action: InputAction) -> bool {
        if self.focus.handle_rows_only(window, action) {
            return true;
        }
        // Re-register if rows were empty on first visit.
        if liked.focus_hortu_rows().iter().any(|r| r.is_visible()) {
            self.register(liked);
            return self.focus.handle_rows_only(window, action);
        }
        false
    }
}
