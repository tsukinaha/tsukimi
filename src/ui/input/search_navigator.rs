use gtk::{
    glib::subclass::types::ObjectSubclassIsExt,
    prelude::*,
};

use super::actions::InputAction;
use crate::{
    Window,
    tv::set_tv_focused,
    ui::widgets::search::SearchPage,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum SearchZone {
    Entry,
    Filters,
    Results,
}

pub struct SearchNavigator {
    zone: std::cell::RefCell<SearchZone>,
    filter_index: std::cell::RefCell<usize>,
}

impl Default for SearchNavigator {
    fn default() -> Self {
        Self {
            zone: std::cell::RefCell::new(SearchZone::Results),
            filter_index: std::cell::RefCell::new(0),
        }
    }
}

impl SearchNavigator {
    pub fn handle(&self, window: &Window, page: &SearchPage, action: InputAction) -> bool {
        match action {
            InputAction::Search => {
                *self.zone.borrow_mut() = SearchZone::Entry;
                page.search_entry().grab_focus();
                true
            }
            InputAction::Menu => {
                window.set_sidebar_panel_visible(
                    !window.tv_sidebar_collapsed()
                        || !window.imp().split_view.get().shows_sidebar(),
                );
                true
            }
            InputAction::Back => {
                window.homepage();
                true
            }
            InputAction::NavigateDown => self.navigate(page, 1),
            InputAction::NavigateUp => self.navigate(page, -1),
            InputAction::NavigateLeft => self.navigate_horizontal(page, -1),
            InputAction::NavigateRight => self.navigate_horizontal(page, 1),
            InputAction::Activate => self.activate(window, page),
            _ => false,
        }
    }

    fn navigate(&self, page: &SearchPage, delta: i32) -> bool {
        let zones = self.visible_zones(page);
        if zones.is_empty() {
            return false;
        }
        let current = *self.zone.borrow();
        let pos = zones.iter().position(|z| *z == current).unwrap_or(0) as i32;
        let next = (pos + delta).clamp(0, zones.len() as i32 - 1) as usize;
        *self.zone.borrow_mut() = zones[next];
        self.apply_zone_focus(page);
        true
    }

    fn navigate_results(&self, page: &SearchPage, delta: i32) -> bool {
        if *self.zone.borrow() != SearchZone::Results {
            return false;
        }
        page.search_scrolled().move_selection(delta);
        true
    }

    fn navigate_filters(&self, page: &SearchPage, delta: i32) -> bool {
        let rows = page.filter_rows();
        if rows.is_empty() {
            return false;
        }
        let current = *self.filter_index.borrow() as i32;
        let next = (current + delta).clamp(0, rows.len() as i32 - 1) as usize;
        *self.filter_index.borrow_mut() = next;
        for row in &rows {
            set_tv_focused(row, false);
        }
        if let Some(row) = rows.get(next) {
            set_tv_focused(row, true);
        }
        true
    }

    fn navigate_horizontal(&self, page: &SearchPage, delta: i32) -> bool {
        match *self.zone.borrow() {
            SearchZone::Filters => self.navigate_filters(page, delta),
            SearchZone::Results => self.navigate_results(page, delta),
            _ => false,
        }
    }

    fn activate(&self, window: &Window, page: &SearchPage) -> bool {
        match *self.zone.borrow() {
            SearchZone::Entry => {
                page.trigger_search();
                true
            }
            SearchZone::Filters => {
                let rows = page.filter_rows();
                let idx = *self.filter_index.borrow();
                if let Some(row) = rows.get(idx) {
                    row.set_active(!row.is_active());
                }
                true
            }
            SearchZone::Results => {
                page.search_scrolled().activate_selected(window);
                true
            }
        }
    }

    fn visible_zones(&self, page: &SearchPage) -> Vec<SearchZone> {
        let mut zones = vec![SearchZone::Entry, SearchZone::Filters];
        if page.search_scrolled().n_items() > 0 {
            zones.push(SearchZone::Results);
        }
        zones
    }

    fn apply_zone_focus(&self, page: &SearchPage) {
        let entry = page.search_entry();
        set_tv_focused(&entry, false);
        for row in page.filter_rows() {
            set_tv_focused(&row, false);
        }
        page.search_scrolled().clear_tv_focus();

        match *self.zone.borrow() {
            SearchZone::Entry => set_tv_focused(&entry, true),
            SearchZone::Filters => {
                let idx = *self.filter_index.borrow();
                if let Some(row) = page.filter_rows().get(idx) {
                    set_tv_focused(row, true);
                }
            }
            SearchZone::Results => page.search_scrolled().ensure_selection(),
        }
    }
}
