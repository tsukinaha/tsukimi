use gtk::{
    glib::subclass::types::ObjectSubclassIsExt,
    prelude::*,
};

use super::actions::InputAction;
use crate::{
    Window,
    ui::{
        models::SETTINGS,
        mpv::page::MPVPage,
    },
};

pub struct MpvNavigator {
    playlist_index: std::cell::Cell<u32>,
}

impl Default for MpvNavigator {
    fn default() -> Self {
        Self {
            playlist_index: std::cell::Cell::new(0),
        }
    }
}

impl MpvNavigator {
    pub fn handle(&self, window: &Window, mpv: &MPVPage, action: InputAction) -> bool {
        let sidebar = window.imp().mpv_view.get();
        let sidebar_visible = sidebar.shows_sidebar();

        if sidebar_visible {
            return self.handle_sidebar(window, action);
        }

        match action {
            InputAction::Back => {
                mpv.on_stop_clicked();
                true
            }
            InputAction::PlayPause => {
                mpv.on_play_pause_clicked();
                true
            }
            InputAction::NavigateLeft => {
                mpv.seek_relative(-(SETTINGS.mpv_seek_backward_step() as f64));
                true
            }
            InputAction::NavigateRight => {
                mpv.seek_relative(SETTINGS.mpv_seek_forward_step() as f64);
                true
            }
            InputAction::NavigateUp => {
                mpv.adjust_volume(5);
                true
            }
            InputAction::NavigateDown => {
                mpv.adjust_volume(-5);
                true
            }
            InputAction::Menu => {
                sidebar.set_show_sidebar(!sidebar.shows_sidebar());
                true
            }
            InputAction::Search => {
                mpv.open_subtitle_search();
                true
            }
            _ => false,
        }
    }

    fn handle_sidebar(&self, window: &Window, action: InputAction) -> bool {
        let selection = &window.imp().mpv_playlist_selection;
        let count = selection.n_items();
        if count == 0 {
            return false;
        }
        let mut index = self.playlist_index.get().min(count - 1);

        match action {
            InputAction::NavigateDown | InputAction::NavigateRight => {
                index = (index + 1).min(count - 1);
                self.playlist_index.set(index);
                selection.set_selected(index);
                self.highlight_playlist_row(window, index);
                true
            }
            InputAction::NavigateUp | InputAction::NavigateLeft => {
                index = index.saturating_sub(1);
                self.playlist_index.set(index);
                selection.set_selected(index);
                self.highlight_playlist_row(window, index);
                true
            }
            InputAction::Activate => {
                selection.set_selected(index);
                if let Some(item) = selection
                    .item(index)
                    .and_downcast::<crate::ui::provider::tu_object::TuObject>()
                {
                    item.activate(&window.imp().mpv_playlist.get());
                }
                true
            }
            InputAction::Back | InputAction::Menu => {
                window.imp().mpv_view.get().set_show_sidebar(false);
                true
            }
            _ => false,
        }
    }

    fn highlight_playlist_row(&self, window: &Window, index: u32) {
        let imp = window.imp();
        imp.mpv_playlist_selection.set_selected(index);
        imp.mpv_playlist
            .scroll_to(index, gtk::ListScrollFlags::NONE, None);
    }
}
