use gtk::{
    glib::subclass::types::ObjectSubclassIsExt,
    prelude::*,
};

use super::{
    actions::InputAction,
    focus_manager::FocusManager,
    grid_navigator::GridNavigator,
};
use crate::{
    Window,
    tv::set_tv_focused,
    ui::widgets::{
        list::ListPage,
        media_viewer::MediaViewer,
        music_album::AlbumPage,
        other::OtherPage,
        server_panel::ServerPanel,
    },
};
#[derive(Clone, Copy, PartialEq, Eq)]
enum ListZone {
    Tabs,
    Toolbar,
    Grid,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AlbumZone {
    Actions,
    Songs,
    Rows,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OtherZone {
    Actions,
    Carousel,
    Rows,
    Episodes,
}

pub struct PushedNavigator {
    album_focus: FocusManager,
    other_focus: FocusManager,
    grid_navigator: GridNavigator,
    list_zone: std::cell::Cell<ListZone>,
    list_toolbar_index: std::cell::Cell<usize>,
    album_zone: std::cell::RefCell<AlbumZone>,
    other_zone: std::cell::RefCell<OtherZone>,
    server_group_index: std::cell::RefCell<usize>,
    server_row_index: std::cell::RefCell<usize>,
}

impl Default for PushedNavigator {
    fn default() -> Self {
        Self {
            album_focus: FocusManager::default(),
            other_focus: FocusManager::default(),
            grid_navigator: GridNavigator,
            list_zone: std::cell::Cell::new(ListZone::Grid),
            list_toolbar_index: std::cell::Cell::new(0),
            album_zone: std::cell::RefCell::new(AlbumZone::Actions),
            other_zone: std::cell::RefCell::new(OtherZone::Actions),
            server_group_index: std::cell::RefCell::new(0),
            server_row_index: std::cell::RefCell::new(0),
        }
    }
}

impl PushedNavigator {
    pub fn handle(&self, window: &Window, action: InputAction) -> bool {
        let Some(page) = window.imp().mainview.visible_page() else {
            return self.handle_back(window, action);
        };

        if let Some(album) = page.downcast_ref::<AlbumPage>() {
            return self.handle_album(window, album, action);
        }
        if let Some(list) = page.downcast_ref::<ListPage>() {
            return self.handle_list(window, list, action);
        }
        if let Some(other) = page.downcast_ref::<OtherPage>() {
            return self.handle_other(window, other, action);
        }
        if let Some(server) = page.downcast_ref::<ServerPanel>() {
            return self.handle_server(window, server, action);
        }

        self.handle_back(window, action)
    }

    fn handle_back(&self, window: &Window, action: InputAction) -> bool {
        match action {
            InputAction::Back => {
                window.on_pop();
                true
            }
            _ => false,
        }
    }

    fn handle_album(&self, window: &Window, page: &AlbumPage, action: InputAction) -> bool {
        self.album_focus.register_rows(page.focus_hortu_rows());

        match action {
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
            InputAction::NavigateDown => self.navigate_album_zone(page, 1),
            InputAction::NavigateUp => self.navigate_album_zone(page, -1),
            InputAction::NavigateLeft => self.navigate_album_horizontal(window, page, -1),
            InputAction::NavigateRight => self.navigate_album_horizontal(window, page, 1),
            InputAction::Activate => self.activate_album(window, page),
            _ => false,
        }
    }

    fn visible_album_zones(&self, page: &AlbumPage) -> Vec<AlbumZone> {
        let mut zones = vec![AlbumZone::Actions];
        if !page.song_widgets().is_empty() {
            zones.push(AlbumZone::Songs);
        }
        if page
            .focus_hortu_rows()
            .iter()
            .any(|row| row.is_visible() && row.item_count() > 0)
        {
            zones.push(AlbumZone::Rows);
        }
        zones
    }

    fn navigate_album_zone(&self, page: &AlbumPage, delta: i32) -> bool {
        let zones = self.visible_album_zones(page);
        let current = *self.album_zone.borrow();
        let pos = zones.iter().position(|z| *z == current).unwrap_or(0) as i32;
        let next = (pos + delta).clamp(0, zones.len() as i32 - 1) as usize;
        self.clear_album_focus(page);
        *self.album_zone.borrow_mut() = zones[next];
        self.apply_album_focus(page);
        true
    }

    fn navigate_album_horizontal(&self, window: &Window, page: &AlbumPage, delta: i32) -> bool {
        match *self.album_zone.borrow() {
            AlbumZone::Actions => {
                page.navigate_actions(delta);
                true
            }
            AlbumZone::Songs => {
                page.navigate_songs(delta);
                true
            }
            AlbumZone::Rows => self.album_focus.handle_rows_only(
                window,
                if delta < 0 {
                    InputAction::NavigateLeft
                } else {
                    InputAction::NavigateRight
                },
            ),
        }
    }

    fn activate_album(&self, window: &Window, page: &AlbumPage) -> bool {
        match *self.album_zone.borrow() {
            AlbumZone::Actions => {
                page.activate_focused_action();
                true
            }
            AlbumZone::Songs => {
                page.activate_focused_song();
                true
            }
            AlbumZone::Rows => self
                .album_focus
                .handle_rows_only(window, InputAction::Activate),
        }
    }

    fn clear_album_focus(&self, page: &AlbumPage) {
        page.clear_action_focus();
        page.clear_song_focus();
        self.album_focus.clear_all_row_selections();
    }

    fn apply_album_focus(&self, page: &AlbumPage) {
        match *self.album_zone.borrow() {
            AlbumZone::Actions => page.focus_default_action(),
            AlbumZone::Songs => page.focus_default_song(),
            AlbumZone::Rows => self.album_focus.register_rows(page.focus_hortu_rows()),
        }
    }

    fn handle_list(&self, window: &Window, page: &ListPage, action: InputAction) -> bool {
        if self.list_zone.get() == ListZone::Grid
            && let Some(grid) = page.visible_grid()
        {
            grid.tuview_scrolled().ensure_selection();
        }

        match action {
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
            InputAction::NavigateUp => {
                if self.list_zone.get() == ListZone::Grid
                    && let Some(grid) = page.visible_grid()
                {
                    if grid.tuview_scrolled().is_at_top_row() {
                        return self.navigate_list_zone(page, -1);
                    }
                    return self.grid_navigator.handle(window, &grid, action);
                }
                self.navigate_list_zone(page, -1)
            }
            InputAction::NavigateDown => {
                let zone = self.list_zone.get();
                match zone {
                    ListZone::Grid => {
                        if let Some(grid) = page.visible_grid() {
                            self.grid_navigator.handle(window, &grid, action)
                        } else {
                            false
                        }
                    }
                    _ => self.navigate_list_zone(page, 1),
                }
            }
            InputAction::NavigateLeft => self.navigate_list_horizontal(page, window, -1),
            InputAction::NavigateRight => self.navigate_list_horizontal(page, window, 1),
            InputAction::Activate => self.activate_list(window, page),
            _ => false,
        }
    }

    fn list_zones(&self, page: &ListPage) -> Vec<ListZone> {
        let has_tabs = page.tab_count() > 1;
        if has_tabs {
            vec![ListZone::Tabs, ListZone::Toolbar, ListZone::Grid]
        } else {
            vec![ListZone::Toolbar, ListZone::Grid]
        }
    }

    fn navigate_list_zone(&self, page: &ListPage, delta: i32) -> bool {
        let zones = self.list_zones(page);
        let current = self.list_zone.get();
        let pos = zones.iter().position(|z| *z == current).unwrap_or(0) as i32;
        let next = (pos + delta).clamp(0, zones.len() as i32 - 1) as usize;
        self.list_zone.set(zones[next]);
        if zones[next] == ListZone::Toolbar {
            self.list_toolbar_index.set(0);
        }
        self.apply_list_focus(page);
        true
    }

    fn navigate_list_horizontal(&self, page: &ListPage, window: &Window, delta: i32) -> bool {
        match self.list_zone.get() {
            ListZone::Tabs => {
                page.switch_tab(delta);
                true
            }
            ListZone::Toolbar => {
                if let Some(grid) = page.visible_grid() {
                    let count = grid.toolbar_widget_count();
                    if count == 0 {
                        return false;
                    }
                    let current = self.list_toolbar_index.get() as i32;
                    let next = (current + delta).clamp(0, count as i32 - 1) as usize;
                    self.list_toolbar_index.set(next);
                    grid.focus_toolbar_index(next);
                    true
                } else {
                    false
                }
            }
            ListZone::Grid => {
                if let Some(grid) = page.visible_grid() {
                    self.grid_navigator.handle(
                        window,
                        &grid,
                        if delta < 0 {
                            InputAction::NavigateLeft
                        } else {
                            InputAction::NavigateRight
                        },
                    )
                } else {
                    false
                }
            }
        }
    }

    fn activate_list(&self, window: &Window, page: &ListPage) -> bool {
        match self.list_zone.get() {
            ListZone::Tabs => {
                self.list_zone.set(ListZone::Toolbar);
                self.list_toolbar_index.set(0);
                self.apply_list_focus(page);
                true
            }
            ListZone::Toolbar => page
                .visible_grid()
                .is_some_and(|grid| grid.activate_toolbar_index(self.list_toolbar_index.get())),
            ListZone::Grid => page.visible_grid().is_some_and(|grid| {
                self.grid_navigator
                    .handle(window, &grid, InputAction::Activate)
            }),
        }
    }

    fn apply_list_focus(&self, page: &ListPage) {
        match self.list_zone.get() {
            ListZone::Tabs => {
                page.focus_current_tab();
                if let Some(grid) = page.visible_grid() {
                    grid.tuview_scrolled().clear_selection();
                    grid.clear_toolbar_focus();
                }
            }
            ListZone::Toolbar => {
                if let Some(grid) = page.visible_grid() {
                    grid.tuview_scrolled().clear_selection();
                    grid.focus_toolbar_index(self.list_toolbar_index.get());
                }
            }
            ListZone::Grid => {
                if let Some(grid) = page.visible_grid() {
                    grid.clear_toolbar_focus();
                    grid.tuview_scrolled().ensure_selection();
                }
            }
        }
    }

    fn handle_other(&self, window: &Window, page: &OtherPage, action: InputAction) -> bool {
        self.other_focus.register_rows(page.focus_hortu_rows());

        match action {
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
            InputAction::NavigateDown => self.navigate_other_zone(page, 1),
            InputAction::NavigateUp => self.navigate_other_zone(page, -1),
            InputAction::NavigateLeft => self.navigate_other_horizontal(page, window, -1),
            InputAction::NavigateRight => self.navigate_other_horizontal(page, window, 1),
            InputAction::Activate => self.activate_other(window, page),
            _ => false,
        }
    }

    fn visible_other_zones(&self, page: &OtherPage) -> Vec<OtherZone> {
        let mut zones = vec![OtherZone::Actions];
        if page.carousel_page_count() > 1 {
            zones.push(OtherZone::Carousel);
        }
        if page
            .focus_hortu_rows()
            .iter()
            .any(|row| row.is_visible() && row.item_count() > 0)
        {
            zones.push(OtherZone::Rows);
        }
        if page.has_episode_list() {
            zones.push(OtherZone::Episodes);
        }
        zones
    }

    fn navigate_other_zone(&self, page: &OtherPage, delta: i32) -> bool {
        let zones = self.visible_other_zones(page);
        let current = *self.other_zone.borrow();
        let pos = zones.iter().position(|z| *z == current).unwrap_or(0) as i32;
        let next = (pos + delta).clamp(0, zones.len() as i32 - 1) as usize;
        self.clear_other_focus(page);
        *self.other_zone.borrow_mut() = zones[next];
        self.apply_other_focus(page);
        true
    }

    fn navigate_other_horizontal(&self, page: &OtherPage, window: &Window, delta: i32) -> bool {
        match *self.other_zone.borrow() {
            OtherZone::Actions => {
                page.navigate_actions(delta);
                true
            }
            OtherZone::Carousel => {
                page.navigate_carousel(delta);
                true
            }
            OtherZone::Rows => self.other_focus.handle_rows_only(
                window,
                if delta < 0 {
                    InputAction::NavigateLeft
                } else {
                    InputAction::NavigateRight
                },
            ),
            OtherZone::Episodes => {
                page.navigate_episodes(delta);
                true
            }
        }
    }

    fn activate_other(&self, window: &Window, page: &OtherPage) -> bool {
        match *self.other_zone.borrow() {
            OtherZone::Actions => {
                page.activate_focused_action();
                true
            }
            OtherZone::Carousel => {
                if page.has_episode_list() {
                    self.clear_other_focus(page);
                    *self.other_zone.borrow_mut() = OtherZone::Episodes;
                    self.apply_other_focus(page);
                }
                true
            }
            OtherZone::Rows => self
                .other_focus
                .handle_rows_only(window, InputAction::Activate),
            OtherZone::Episodes => {
                page.activate_focused_episode();
                true
            }
        }
    }

    fn clear_other_focus(&self, page: &OtherPage) {
        page.clear_action_focus();
        self.other_focus.clear_all_row_selections();
        page.clear_episode_focus();
    }

    fn apply_other_focus(&self, page: &OtherPage) {
        match *self.other_zone.borrow() {
            OtherZone::Actions => page.focus_default_action(),
            OtherZone::Carousel => {}
            OtherZone::Rows => self.other_focus.register_rows(page.focus_hortu_rows()),
            OtherZone::Episodes => page.focus_default_episode(),
        }
    }

    fn handle_server(&self, window: &Window, page: &ServerPanel, action: InputAction) -> bool {
        let groups = page.focus_groups();
        if groups.is_empty() {
            return self.handle_back(window, action);
        }

        match action {
            InputAction::Back => {
                window.on_pop();
                true
            }
            InputAction::NavigateDown => self.navigate_server(&groups, 1),
            InputAction::NavigateUp => self.navigate_server(&groups, -1),
            InputAction::Activate => page.activate_focused_row(
                &groups,
                *self.server_group_index.borrow(),
                *self.server_row_index.borrow(),
            ),
            _ => false,
        }
    }

    fn navigate_server(
        &self, groups: &[crate::ui::widgets::server_panel::ServerFocusGroup], delta: i32,
    ) -> bool {
        let group_idx = *self.server_group_index.borrow();
        let row_idx = *self.server_row_index.borrow();
        let Some(group) = groups.get(group_idx) else {
            return false;
        };

        if delta > 0 {
            if row_idx + 1 < group.rows.len() {
                *self.server_row_index.borrow_mut() = row_idx + 1;
            } else if group_idx + 1 < groups.len() {
                *self.server_group_index.borrow_mut() = group_idx + 1;
                *self.server_row_index.borrow_mut() = 0;
            }
        } else if row_idx > 0 {
            *self.server_row_index.borrow_mut() = row_idx - 1;
        } else if group_idx > 0 {
            let prev = group_idx - 1;
            *self.server_group_index.borrow_mut() = prev;
            *self.server_row_index.borrow_mut() = groups[prev].rows.len().saturating_sub(1);
        }

        self.apply_server_focus(groups);
        true
    }

    fn apply_server_focus(&self, groups: &[crate::ui::widgets::server_panel::ServerFocusGroup]) {
        for (gidx, group) in groups.iter().enumerate() {
            for (ridx, row) in group.rows.iter().enumerate() {
                let focused = gidx == *self.server_group_index.borrow()
                    && ridx == *self.server_row_index.borrow();
                set_tv_focused(row, focused);
            }
        }
    }
}

#[derive(Default)]
pub struct MediaViewerNavigator;

impl MediaViewerNavigator {
    pub fn handle(&self, _window: &Window, viewer: &MediaViewer, action: InputAction) -> bool {
        match action {
            InputAction::Back => {
                viewer.dismiss();
                true
            }
            InputAction::Activate => {
                viewer.activate_action("media-viewer.close", None).ok();
                true
            }
            InputAction::Menu => {
                viewer.focus_menu();
                true
            }
            InputAction::NavigateUp => {
                viewer.reveal_header_for_tv(true);
                true
            }
            InputAction::NavigateDown => {
                viewer.reveal_header_for_tv(false);
                true
            }
            _ => false,
        }
    }
}
