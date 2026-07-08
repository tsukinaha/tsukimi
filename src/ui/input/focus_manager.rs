use std::cell::RefCell;

use gtk::{
    glib::{
        WeakRef,
        subclass::types::ObjectSubclassIsExt,
    },
    prelude::*,
};

use super::actions::InputAction;
use crate::{
    Window,
    ui::widgets::{
        home::HomePage,
        hortu_scrolled::HortuScrolled,
    },
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum FocusArea {
    Sidebar,
    Content,
}

#[derive(Clone, Copy, Debug)]
pub struct HomeFocusSnapshot {
    pub content_index: usize,
    pub sidebar_focused: bool,
}

pub struct FocusManager {
    area: RefCell<FocusArea>,
    content_rows: RefCell<Vec<WeakRef<HortuScrolled>>>,
    content_index: RefCell<usize>,
}

impl Default for FocusManager {
    fn default() -> Self {
        Self {
            area: RefCell::new(FocusArea::Content),
            content_rows: RefCell::new(Vec::new()),
            content_index: RefCell::new(0),
        }
    }
}

impl FocusManager {
    pub fn register_home(&self, home: &HomePage) {
        self.set_rows(home.focus_hortu_rows());
        *self.content_index.borrow_mut() = 0;
        *self.area.borrow_mut() = FocusArea::Content;
        self.clear_all_row_selections();
        self.focus_current_row();
    }

    pub fn refresh_home_rows(&self, home: &HomePage) {
        self.set_rows(home.focus_hortu_rows());
        let rows_len = self.content_rows.borrow().len();
        if rows_len == 0 {
            return;
        }
        let idx = (*self.content_index.borrow()).min(rows_len - 1);
        *self.content_index.borrow_mut() = idx;
        self.focus_current_row();
    }

    pub fn register_rows(&self, rows: Vec<HortuScrolled>) {
        self.set_rows(rows);
        *self.content_index.borrow_mut() = 0;
        *self.area.borrow_mut() = FocusArea::Content;
        self.clear_all_row_selections();
        self.focus_current_row();
    }

    pub fn snapshot_home_focus(&self) -> HomeFocusSnapshot {
        HomeFocusSnapshot {
            content_index: *self.content_index.borrow(),
            sidebar_focused: *self.area.borrow() == FocusArea::Sidebar,
        }
    }

    pub fn restore_home_focus(&self, home: &HomePage, snapshot: HomeFocusSnapshot) {
        self.set_rows(home.focus_hortu_rows());
        let rows_len = self.content_rows.borrow().len();
        let idx = if rows_len == 0 {
            0
        } else {
            snapshot.content_index.min(rows_len - 1)
        };
        *self.content_index.borrow_mut() = idx;
        *self.area.borrow_mut() = FocusArea::Content;
        self.clear_all_row_selections();
        self.focus_current_row();
    }

    pub fn clear_all_row_selections(&self) {
        for weak in self.content_rows.borrow().iter() {
            if let Some(row) = weak.upgrade() {
                row.clear_selection();
                row.clear_keyboard_focus();
            }
        }
    }

    pub fn clear(&self) {
        self.clear_all_row_selections();
        self.content_rows.borrow_mut().clear();
        *self.content_index.borrow_mut() = 0;
    }

    fn set_rows(&self, rows: Vec<HortuScrolled>) {
        *self.content_rows.borrow_mut() = rows.into_iter().map(|r| r.downgrade()).collect();
    }

    pub fn handle(&self, window: &Window, action: InputAction) -> bool {
        if *self.area.borrow() == FocusArea::Sidebar {
            return self.handle_sidebar(window, action);
        }

        match action {
            InputAction::NavigateLeft => self.navigate_horizontal(window, -1),
            InputAction::NavigateRight => self.navigate_horizontal(window, 1),
            InputAction::NavigateUp => self.navigate_vertical(window, -1),
            InputAction::NavigateDown => self.navigate_vertical(window, 1),
            InputAction::Activate => self.activate(window),
            InputAction::Back => {
                window.on_pop();
                true
            }
            InputAction::Menu => self.toggle_sidebar(window),
            InputAction::Home => {
                window.homepage();
                true
            }
            InputAction::Search => {
                window.searchpage();
                true
            }
            InputAction::PageScrollLeft => self.page_scroll(-1),
            InputAction::PageScrollRight => self.page_scroll(1),
            _ => false,
        }
    }

    pub fn handle_rows_only(&self, window: &Window, action: InputAction) -> bool {
        match action {
            InputAction::NavigateLeft => self.navigate_horizontal(window, -1),
            InputAction::NavigateRight => self.navigate_horizontal(window, 1),
            InputAction::NavigateUp => self.navigate_vertical_no_sidebar(window, -1),
            InputAction::NavigateDown => self.navigate_vertical_no_sidebar(window, 1),
            InputAction::Activate => self.activate(window),
            InputAction::Back => {
                window.on_pop();
                true
            }
            InputAction::Menu => self.toggle_sidebar(window),
            InputAction::Home => {
                window.homepage();
                true
            }
            InputAction::Search => {
                window.searchpage();
                true
            }
            InputAction::PageScrollLeft => self.page_scroll(-1),
            InputAction::PageScrollRight => self.page_scroll(1),
            _ => false,
        }
    }

    fn is_navigable_row(row: &HortuScrolled) -> bool {
        row.is_visible() && row.item_count() > 0
    }

    fn visible_row_at(&self, index: usize) -> Option<(usize, HortuScrolled)> {
        let rows = self.content_rows.borrow();
        for (offset, weak) in rows.iter().enumerate().skip(index) {
            if let Some(row) = weak.upgrade()
                && Self::is_navigable_row(&row)
            {
                return Some((offset, row));
            }
        }
        None
    }

    fn focus_current_row(&self) {
        if *self.area.borrow() == FocusArea::Sidebar {
            return;
        }
        let index = *self.content_index.borrow();
        if let Some((_, row)) = self.visible_row_at(index) {
            row.ensure_selection();
            row.show_scroll_controls_for_focus();
            row.scroll_into_parent_viewport();
        }
    }

    fn navigate_horizontal(&self, window: &Window, delta: i32) -> bool {
        if *self.area.borrow() == FocusArea::Sidebar {
            return false;
        }
        let index = *self.content_index.borrow();
        let Some((_, row)) = self.visible_row_at(index) else {
            return false;
        };
        if delta < 0
            && !row.is_header_focused()
            && row.selection_at_start()
            && !window.tv_sidebar_collapsed()
        {
            self.enter_sidebar(window);
            return true;
        }
        row.move_selection(delta);
        true
    }

    fn navigate_vertical(&self, window: &Window, delta: i32) -> bool {
        if *self.area.borrow() == FocusArea::Sidebar {
            return self.move_sidebar_selection(window, delta);
        }
        self.navigate_vertical_no_sidebar(window, delta)
    }

    fn navigate_vertical_no_sidebar(&self, window: &Window, delta: i32) -> bool {
        let rows = self.content_rows.borrow();
        let mut idx = *self.content_index.borrow();
        let step = if delta > 0 { 1 } else { -1 };

        loop {
            if step > 0 {
                idx += 1;
            } else if idx == 0 {
                if window.tv_sidebar_collapsed() {
                    return true;
                }
                self.enter_sidebar(window);
                return true;
            } else {
                idx -= 1;
            }

            if idx >= rows.len() {
                return true;
            }

            if let Some(row) = rows[idx].upgrade()
                && Self::is_navigable_row(&row)
            {
                if let Some(current) = self.visible_row_at(*self.content_index.borrow()) {
                    current.1.clear_selection();
                }
                *self.content_index.borrow_mut() = idx;
                self.focus_current_row();
                return true;
            }
        }
    }

    fn activate(&self, window: &Window) -> bool {
        if *self.area.borrow() == FocusArea::Sidebar {
            window.activate_sidebar_selection();
            return true;
        }
        let index = *self.content_index.borrow();
        let Some((_, row)) = self.visible_row_at(index) else {
            return false;
        };
        row.activate_selected(window);
        true
    }

    fn page_scroll(&self, direction: i32) -> bool {
        if *self.area.borrow() == FocusArea::Sidebar {
            return false;
        }
        let index = *self.content_index.borrow();
        let Some((_, row)) = self.visible_row_at(index) else {
            return false;
        };
        if direction < 0 {
            row.scroll_page_left();
        } else {
            row.scroll_page_right();
        }
        true
    }

    fn handle_sidebar(&self, window: &Window, action: InputAction) -> bool {
        match action {
            InputAction::NavigateRight => {
                self.enter_content(window);
                true
            }
            InputAction::NavigateDown => self.move_sidebar_selection(window, 1),
            InputAction::NavigateUp => self.move_sidebar_selection(window, -1),
            InputAction::NavigateLeft => true,
            InputAction::Activate => {
                window.activate_sidebar_selection();
                self.enter_content(window);
                true
            }
            InputAction::Back | InputAction::Menu => {
                self.enter_content(window);
                true
            }
            _ => false,
        }
    }

    fn enter_sidebar(&self, window: &Window) {
        window.set_sidebar_panel_visible(true);
        *self.area.borrow_mut() = FocusArea::Sidebar;
        window.imp().selectlist.get().grab_focus();
    }

    fn enter_content(&self, window: &Window) {
        window.set_sidebar_panel_visible(false);
        *self.area.borrow_mut() = FocusArea::Content;
        self.focus_current_row();
    }

    fn toggle_sidebar(&self, window: &Window) -> bool {
        if window.tv_sidebar_collapsed() {
            let split_view = window.imp().split_view.get();
            if split_view.shows_sidebar() {
                self.enter_content(window);
            } else {
                self.enter_sidebar(window);
            }
            return true;
        }
        gtk::prelude::ActionGroupExt::activate_action(window, "win.sidebar", None);
        true
    }

    fn move_sidebar_selection(&self, window: &Window, delta: i32) -> bool {
        let sidebar = window.imp().selectlist.get();
        let mut index = sidebar.selected() as i32;
        let step = if delta > 0 { 1 } else { -1 };

        index += step;
        if index < 0 {
            return true;
        }
        if sidebar.item(index as u32).is_none() {
            return true;
        }
        sidebar.set_selected(index as u32);
        true
    }
}
