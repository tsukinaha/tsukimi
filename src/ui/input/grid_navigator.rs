use gtk::glib::subclass::types::ObjectSubclassIsExt;

use super::actions::InputAction;
use crate::{
    Window,
    ui::widgets::single_grid::SingleGrid,
};

pub struct GridNavigator;

impl GridNavigator {
    pub fn handle(&self, window: &Window, grid: &SingleGrid, action: InputAction) -> bool {
        let scrolled = grid.tuview_scrolled();

        match action {
            InputAction::NavigateLeft => {
                scrolled.move_grid_selection(0, -1);
                true
            }
            InputAction::NavigateRight => {
                scrolled.move_grid_selection(0, 1);
                true
            }
            InputAction::NavigateUp => {
                scrolled.move_grid_selection(-1, 0);
                true
            }
            InputAction::NavigateDown => {
                scrolled.move_grid_selection(1, 0);
                true
            }
            InputAction::Activate => {
                scrolled.activate_selected(window);
                true
            }
            InputAction::Back => {
                window.on_pop();
                true
            }
            InputAction::Menu => {
                window.set_sidebar_panel_visible(
                    !window.tv_sidebar_collapsed()
                        || !window.imp().split_view.get().shows_sidebar(),
                );
                true
            }
            InputAction::PageScrollLeft => {
                scrolled.move_grid_selection(0, -scrolled.grid_column_count());
                true
            }
            InputAction::PageScrollRight => {
                scrolled.move_grid_selection(0, scrolled.grid_column_count());
                true
            }
            _ => false,
        }
    }
}

impl Default for GridNavigator {
    fn default() -> Self {
        Self
    }
}
