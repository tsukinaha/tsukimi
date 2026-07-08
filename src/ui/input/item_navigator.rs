use gtk::{
    glib::subclass::types::ObjectSubclassIsExt,
    prelude::*,
};

use super::actions::InputAction;
use crate::{
    Window,
    ui::widgets::{
        horbu_scrolled::HorbuScrolled,
        hortu_scrolled::HortuScrolled,
        item::ItemPage,
    },
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum ItemZone {
    TopBar,
    Hortu(usize),
    Horbu(usize),
    Episodes,
    MediaInfo,
}

pub struct ItemPageNavigator {
    zone: std::cell::Cell<ItemZone>,
    hortu_rows: std::cell::RefCell<Vec<HortuScrolled>>,
    horbu_rows: std::cell::RefCell<Vec<HorbuScrolled>>,
}

impl Default for ItemPageNavigator {
    fn default() -> Self {
        Self {
            zone: std::cell::Cell::new(ItemZone::TopBar),
            hortu_rows: std::cell::RefCell::new(Vec::new()),
            horbu_rows: std::cell::RefCell::new(Vec::new()),
        }
    }
}

impl ItemPageNavigator {
    pub fn handle(&self, window: &Window, page: &ItemPage, action: InputAction) -> bool {
        *self.hortu_rows.borrow_mut() = page.focus_hortu_rows();
        *self.horbu_rows.borrow_mut() = page.focus_horbu_rows();

        match action {
            InputAction::NavigateDown => {
                if self.zone.get() == ItemZone::TopBar && page.navigate_top_bar_spatial(0, 1) {
                    return true;
                }
                if self.zone.get() == ItemZone::Episodes && page.is_episode_toolbar_focused() {
                    page.focus_episode_list();
                    return true;
                }
                self.navigate_zone(page, 1)
            }
            InputAction::NavigateUp => {
                if self.zone.get() == ItemZone::TopBar && page.navigate_top_bar_spatial(0, -1) {
                    return true;
                }
                if self.zone.get() == ItemZone::Episodes && !page.is_episode_toolbar_focused() {
                    page.focus_episode_toolbar(0);
                    return true;
                }
                self.navigate_zone(page, -1)
            }
            InputAction::NavigateLeft => {
                if self.zone.get() == ItemZone::TopBar && page.navigate_top_bar_spatial(-1, 0) {
                    return true;
                }
                if self.zone.get() == ItemZone::Episodes && page.is_episode_toolbar_focused() {
                    page.navigate_episode_toolbar(-1);
                    return true;
                }
                self.navigate_horizontal(page, -1)
            }
            InputAction::NavigateRight => {
                if self.zone.get() == ItemZone::TopBar && page.navigate_top_bar_spatial(1, 0) {
                    return true;
                }
                if self.zone.get() == ItemZone::Episodes && page.is_episode_toolbar_focused() {
                    page.navigate_episode_toolbar(1);
                    return true;
                }
                self.navigate_horizontal(page, 1)
            }
            InputAction::Activate => self.activate(window, page),
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
            InputAction::Search => {
                page.open_subtitle_search();
                true
            }
            _ => false,
        }
    }

    fn visible_zones(&self, page: &ItemPage) -> Vec<ItemZone> {
        let mut zones = vec![ItemZone::TopBar];
        if page.has_episode_list() {
            zones.push(ItemZone::Episodes);
        }
        for (i, row) in self.hortu_rows.borrow().iter().enumerate() {
            if row.is_visible() && row.item_count() > 0 {
                zones.push(ItemZone::Hortu(i));
            }
        }
        for (i, row) in self.horbu_rows.borrow().iter().enumerate() {
            if row.is_visible() && row.button_count() > 0 {
                zones.push(ItemZone::Horbu(i));
            }
        }
        if page.mediainfo_card_count() > 0 {
            zones.push(ItemZone::MediaInfo);
        }
        zones
    }

    fn navigate_zone(&self, page: &ItemPage, delta: i32) -> bool {
        let zones = self.visible_zones(page);
        if zones.is_empty() {
            return false;
        }
        let current = self.zone.get();
        let pos = zones.iter().position(|z| *z == current).unwrap_or(0) as i32;
        let next = (pos + delta).clamp(0, zones.len() as i32 - 1) as usize;
        if zones[next] == current {
            return true;
        }
        self.clear_zone_focus(page);
        self.zone.set(zones[next]);
        self.apply_zone_focus(page);
        true
    }

    fn navigate_horizontal(&self, page: &ItemPage, delta: i32) -> bool {
        match self.zone.get() {
            ItemZone::Hortu(idx) => {
                if let Some(row) = self.hortu_rows.borrow().get(idx) {
                    row.move_selection(delta);
                }
                true
            }
            ItemZone::Horbu(idx) => {
                if let Some(row) = self.horbu_rows.borrow().get(idx) {
                    row.move_selection(delta);
                }
                true
            }
            ItemZone::Episodes => {
                page.navigate_episodes(delta);
                true
            }
            ItemZone::MediaInfo => {
                page.move_mediainfo_selection(delta);
                true
            }
            ItemZone::TopBar => false,
        }
    }

    fn activate(&self, window: &Window, page: &ItemPage) -> bool {
        match self.zone.get() {
            ItemZone::TopBar => {
                page.activate_focused_top_bar();
                true
            }
            ItemZone::Hortu(idx) => {
                if let Some(row) = self.hortu_rows.borrow().get(idx) {
                    row.activate_selected(window);
                }
                true
            }
            ItemZone::Horbu(idx) => {
                if let Some(row) = self.horbu_rows.borrow().get(idx) {
                    row.activate_selected();
                }
                true
            }
            ItemZone::Episodes => {
                if page.is_episode_toolbar_focused() {
                    page.activate_episode_toolbar();
                } else {
                    page.activate_focused_episode();
                }
                true
            }
            ItemZone::MediaInfo => true,
        }
    }

    fn clear_zone_focus(&self, page: &ItemPage) {
        page.clear_top_bar_focus();
        page.clear_episode_toolbar_focus();
        for row in self.hortu_rows.borrow().iter() {
            row.clear_selection();
        }
        for row in self.horbu_rows.borrow().iter() {
            row.clear_selection();
        }
        page.clear_episode_focus();
        page.clear_mediainfo_focus();
    }

    fn apply_zone_focus(&self, page: &ItemPage) {
        match self.zone.get() {
            ItemZone::TopBar => {
                page.scroll_to_hero_page();
                page.focus_default_top_bar_spatial();
            }
            ItemZone::Episodes => {
                page.scroll_to_hero_page();
                if page.has_episode_toolbar() {
                    page.focus_episode_toolbar(0);
                } else {
                    page.focus_episode_list();
                }
            }
            ItemZone::Hortu(idx) => {
                if let Some(row) = self.hortu_rows.borrow().get(idx).cloned() {
                    page.focus_details_row(move || {
                        row.ensure_selection();
                        row.scroll_into_parent_viewport();
                    });
                }
            }
            ItemZone::Horbu(idx) => {
                if let Some(row) = self.horbu_rows.borrow().get(idx).cloned() {
                    page.focus_details_row(move || {
                        row.ensure_selection();
                        row.scroll_into_parent_viewport();
                    });
                }
            }
            ItemZone::MediaInfo => page.scroll_mediainfo_into_view(),
        }
    }
}
