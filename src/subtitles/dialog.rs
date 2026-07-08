use std::{
    cell::RefCell,
    path::PathBuf,
    rc::Rc,
};

use adw::prelude::*;
use gettextrs::gettext;

use super::provider::{
    SubtitleProviderRegistry,
    SubtitleResult,
};
use crate::{
    playback::language_codes,
    tv::{
        osk,
        set_tv_focused,
    },
    ui::{
        SETTINGS,
        input::InputAction,
    },
    utils::spawn,
};

thread_local! {
    static ACTIVE_DIALOG: RefCell<Option<Rc<SubtitleSearchDialogInner>>> = const { RefCell::new(None) };
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SubtitleZone {
    Search,
    Language,
    SearchButton,
    Results,
    Download,
    Close,
}

type DownloadCallback = Box<dyn Fn(PathBuf)>;

struct SubtitleSearchDialogInner {
    dialog: adw::Dialog,
    search_entry: adw::EntryRow,
    language_combo: adw::ComboRow,
    results_list: gtk::ListBox,
    status_label: gtk::Label,
    search_button: gtk::Button,
    download_button: gtk::Button,
    close_button: gtk::Button,
    results: RefCell<Vec<SubtitleResult>>,
    selected_index: RefCell<Option<usize>>,
    on_downloaded: RefCell<Option<DownloadCallback>>,
    imdb_id: RefCell<Option<String>>,
    zone: RefCell<SubtitleZone>,
    result_index: RefCell<usize>,
}

pub struct SubtitleSearchDialog {
    inner: Rc<SubtitleSearchDialogInner>,
}

pub fn handle_active_input(action: InputAction) -> bool {
    ACTIVE_DIALOG.with(|slot| {
        let Some(inner) = slot.borrow().clone() else {
            return false;
        };
        if !inner.dialog.is_visible() {
            *slot.borrow_mut() = None;
            return false;
        }
        if inner.has_open_popover() {
            return false;
        }
        inner.handle_input(action)
    })
}

impl SubtitleSearchDialogInner {
    fn has_open_popover(&self) -> bool {
        let Some(content) = self.dialog.child() else {
            return false;
        };
        popover_open_in_tree(content.upcast_ref())
    }

    fn clear_focus(&self) {
        set_tv_focused(&self.search_entry, false);
        set_tv_focused(&self.language_combo, false);
        set_tv_focused(&self.search_button, false);
        set_tv_focused(&self.download_button, false);
        set_tv_focused(&self.close_button, false);
        let mut child = self.results_list.first_child();
        while let Some(row) = child {
            set_tv_focused(&row, false);
            child = row.next_sibling();
        }
    }

    fn apply_zone_focus(&self) {
        self.clear_focus();
        match *self.zone.borrow() {
            SubtitleZone::Search => set_tv_focused(&self.search_entry, true),
            SubtitleZone::Language => set_tv_focused(&self.language_combo, true),
            SubtitleZone::SearchButton => set_tv_focused(&self.search_button, true),
            SubtitleZone::Download => set_tv_focused(&self.download_button, true),
            SubtitleZone::Close => set_tv_focused(&self.close_button, true),
            SubtitleZone::Results => {
                let idx = *self.result_index.borrow();
                if let Some(row) = self.results_list.row_at_index(idx as i32) {
                    self.results_list.select_row(Some(&row));
                    set_tv_focused(&row, true);
                }
            }
        }
    }

    fn visible_zones(&self) -> Vec<SubtitleZone> {
        let mut zones = vec![
            SubtitleZone::Search,
            SubtitleZone::Language,
            SubtitleZone::SearchButton,
        ];
        if !self.results.borrow().is_empty() {
            zones.push(SubtitleZone::Results);
        }
        zones.push(SubtitleZone::Download);
        zones.push(SubtitleZone::Close);
        zones
    }

    fn selected_language_code(&self) -> String {
        let index = self.language_combo.selected();
        if index == 0 {
            return String::new();
        }
        language_codes::code_at_index(index - 1)
    }

    fn handle_input(&self, action: InputAction) -> bool {
        match action {
            InputAction::Back => {
                self.dialog.close();
                true
            }
            InputAction::NavigateUp | InputAction::NavigateDown => {
                let zones = self.visible_zones();
                let current = *self.zone.borrow();
                let pos = zones.iter().position(|z| *z == current).unwrap_or(0) as i32;
                let delta = if matches!(action, InputAction::NavigateDown) {
                    1
                } else {
                    -1
                };
                let next = (pos + delta).clamp(0, zones.len() as i32 - 1) as usize;
                *self.zone.borrow_mut() = zones[next];
                self.apply_zone_focus();
                true
            }
            InputAction::NavigateLeft | InputAction::NavigateRight => {
                if *self.zone.borrow() != SubtitleZone::Results {
                    return false;
                }
                let count = self.results.borrow().len();
                if count == 0 {
                    return false;
                }
                let delta = if matches!(action, InputAction::NavigateRight) {
                    1
                } else {
                    -1
                };
                let current = *self.result_index.borrow() as i32;
                let next = (current + delta).clamp(0, count as i32 - 1) as usize;
                *self.result_index.borrow_mut() = next;
                self.apply_zone_focus();
                true
            }
            InputAction::Activate => match *self.zone.borrow() {
                SubtitleZone::Search => {
                    self.search_entry.grab_focus();
                    true
                }
                SubtitleZone::Language => {
                    self.language_combo.grab_focus();
                    adw::prelude::ActionRowExt::activate(&self.language_combo);
                    true
                }
                SubtitleZone::SearchButton => {
                    self.search_button.emit_clicked();
                    true
                }
                SubtitleZone::Results => {
                    let idx = *self.result_index.borrow();
                    *self.selected_index.borrow_mut() = Some(idx);
                    self.download_button.set_sensitive(true);
                    if let Some(row) = self.results_list.row_at_index(idx as i32) {
                        self.results_list.select_row(Some(&row));
                    }
                    true
                }
                SubtitleZone::Download => {
                    self.download_button.emit_clicked();
                    true
                }
                SubtitleZone::Close => {
                    self.close_button.emit_clicked();
                    true
                }
            },
            _ => false,
        }
    }

    fn unregister(&self) {
        ACTIVE_DIALOG.with(|slot| {
            *slot.borrow_mut() = None;
        });
    }
}

impl SubtitleSearchDialog {
    pub fn new(query: &str, language: &str, imdb_id: Option<&str>) -> Self {
        let dialog = adw::Dialog::builder()
            .title(gettext("Download Subtitles"))
            .content_width(520)
            .content_height(480)
            .build();

        let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
        content.set_margin_top(12);
        content.set_margin_bottom(12);
        content.set_margin_start(12);
        content.set_margin_end(12);

        let search_entry = adw::EntryRow::new();
        search_entry.set_title(&gettext("Search"));
        search_entry.set_text(query);

        let language_labels: Vec<String> = std::iter::once(gettext("Any language"))
            .chain(language_codes::language_combo_labels())
            .collect();
        let language_model = gtk::StringList::new(
            &language_labels
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>(),
        );
        let language_combo = adw::ComboRow::new();
        language_combo.set_title(&gettext("Language"));
        language_combo.set_model(Some(&language_model));
        language_combo.set_selected(language_combo_index(language));

        let search_button = gtk::Button::with_label(&gettext("Search"));
        search_button.add_css_class("suggested-action");

        let results_scrolled = gtk::ScrolledWindow::builder()
            .vexpand(true)
            .min_content_height(240)
            .build();
        let results_list = gtk::ListBox::new();
        results_list.set_selection_mode(gtk::SelectionMode::Single);
        results_scrolled.set_child(Some(&results_list));

        let status_label = gtk::Label::new(None);
        status_label.add_css_class("dim-label");
        status_label.set_halign(gtk::Align::Start);

        let button_box = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        button_box.set_halign(gtk::Align::End);
        let download_button = gtk::Button::with_label(&gettext("Download"));
        download_button.set_sensitive(false);
        let close_button = gtk::Button::with_label(&gettext("Close"));
        button_box.append(&download_button);
        button_box.append(&close_button);

        content.append(&search_entry);
        content.append(&language_combo);
        content.append(&search_button);
        content.append(&results_scrolled);
        content.append(&status_label);
        content.append(&button_box);
        dialog.set_child(Some(&content));

        osk::attach_on_screen_keyboard(&search_entry);

        let inner = Rc::new(SubtitleSearchDialogInner {
            dialog: dialog.clone(),
            search_entry,
            language_combo,
            results_list,
            status_label,
            search_button,
            download_button,
            close_button: close_button.clone(),
            results: RefCell::new(Vec::new()),
            selected_index: RefCell::new(None),
            on_downloaded: RefCell::new(None),
            imdb_id: RefCell::new(imdb_id.map(str::to_string)),
            zone: RefCell::new(SubtitleZone::Search),
            result_index: RefCell::new(0),
        });

        close_button.connect_clicked({
            let inner = inner.clone();
            let dialog = dialog.clone();
            move |_| {
                inner.unregister();
                dialog.close();
            }
        });

        dialog.connect_closed({
            let inner = inner.clone();
            move |_| {
                inner.unregister();
            }
        });

        let this = Self {
            inner: inner.clone(),
        };
        this.wire_signals(inner);
        this
    }

    pub fn present(&self, parent: Option<&impl IsA<gtk::Widget>>) {
        *self.inner.zone.borrow_mut() = SubtitleZone::Search;
        *self.inner.result_index.borrow_mut() = 0;
        self.inner.apply_zone_focus();
        ACTIVE_DIALOG.with(|slot| {
            *slot.borrow_mut() = Some(self.inner.clone());
        });
        self.inner.dialog.present(parent);
    }

    pub fn connect_subtitle_downloaded<F>(&self, callback: F)
    where
        F: Fn(PathBuf) + 'static,
    {
        *self.inner.on_downloaded.borrow_mut() = Some(Box::new(callback));
    }

    fn wire_signals(&self, inner: Rc<SubtitleSearchDialogInner>) {
        {
            let inner = inner.clone();
            self.inner.search_button.connect_clicked(move |_| {
                let query = inner.search_entry.text().to_string();
                if query.len() < 2 {
                    inner
                        .status_label
                        .set_text(&gettext("Enter at least 2 characters"));
                    return;
                }
                let language = inner.selected_language_code();
                let registry = SubtitleProviderRegistry::new();
                if registry.searchable_providers().is_empty() {
                    inner
                        .status_label
                        .set_text(&gettext("Configure subtitle provider API keys in Settings"));
                    return;
                }
                inner.status_label.set_text(&gettext("Searching..."));
                inner.download_button.set_sensitive(false);
                while let Some(child) = inner.results_list.first_child() {
                    inner.results_list.remove(&child);
                }
                inner.results.borrow_mut().clear();
                *inner.selected_index.borrow_mut() = None;
                *inner.result_index.borrow_mut() = 0;

                let results_list = inner.results_list.clone();
                let status_label = inner.status_label.clone();
                let results = inner.results.clone();
                let inner_for_spawn = inner.clone();

                spawn(async move {
                    let registry = SubtitleProviderRegistry::new();
                    let imdb_id = inner_for_spawn.imdb_id.borrow().clone();
                    let found = registry
                        .search_all(&query, imdb_id.as_deref(), &language)
                        .await;

                    for result in found {
                        let row = adw::ActionRow::new();
                        row.set_title(&result.title);
                        row.set_subtitle(&format!("{} · {}", result.provider, result.language));
                        results_list.append(&row);
                        results.borrow_mut().push(result);
                    }

                    let count = results.borrow().len();
                    if count == 0 {
                        status_label.set_text(&gettext(
                            "No subtitles found (check provider API keys and rate limits)",
                        ));
                    } else if let Some(first) = results_list.row_at_index(0) {
                        results_list.select_row(Some(&first));
                        status_label.set_text(&format!("{count} {}", gettext("results")));
                        *inner_for_spawn.result_index.borrow_mut() = 0;
                        if *inner_for_spawn.zone.borrow() == SubtitleZone::Results {
                            inner_for_spawn.apply_zone_focus();
                        }
                    }
                });
            });
        }

        {
            let inner = inner.clone();
            self.inner
                .results_list
                .connect_row_selected(move |list, row| {
                    let index = row.and_then(|selected| {
                        let mut child = list.first_child();
                        let mut index = 0usize;
                        while let Some(row_widget) = child {
                            let next = row_widget.next_sibling();
                            if let Ok(list_row) = row_widget.downcast::<gtk::ListBoxRow>()
                                && list_row.eq(selected)
                            {
                                return Some(index);
                            }
                            index += 1;
                            child = next;
                        }
                        None
                    });
                    *inner.selected_index.borrow_mut() = index;
                    if let Some(index) = index {
                        *inner.result_index.borrow_mut() = index;
                    }
                    inner
                        .download_button
                        .set_sensitive(inner.selected_index.borrow().is_some());
                });
        }

        {
            let inner = inner.clone();
            self.inner.download_button.connect_clicked(move |_| {
                let Some(index) = *inner.selected_index.borrow() else {
                    return;
                };
                let Some(result) = inner.results.borrow().get(index).cloned() else {
                    return;
                };

                inner.status_label.set_text(&gettext("Downloading..."));
                inner.download_button.set_sensitive(false);

                let status_label = inner.status_label.clone();
                let download_button = inner.download_button.clone();
                let dialog = inner.dialog.clone();
                let inner = inner.clone();

                spawn(async move {
                    let registry = SubtitleProviderRegistry::new();
                    let provider = registry
                        .searchable_providers()
                        .into_iter()
                        .find(|provider| provider.id() == result.provider);
                    let Some(provider) = provider else {
                        status_label.set_text(&gettext("Provider unavailable"));
                        download_button.set_sensitive(true);
                        return;
                    };
                    match provider.download(&result).await {
                        Ok(path) => {
                            status_label.set_text(&gettext("Subtitle downloaded"));
                            if let Some(callback) = inner.on_downloaded.borrow().as_ref() {
                                callback(path);
                            }
                            inner.unregister();
                            dialog.close();
                        }
                        Err(err) => {
                            status_label
                                .set_text(&format!("{}: {err}", gettext("Download failed")));
                            download_button.set_sensitive(true);
                        }
                    }
                });
            });
        }
    }
}

pub fn preferred_subtitle_language_code() -> String {
    match SETTINGS.mpv_subtitle_preferred_lang() {
        1 => "en".into(),
        2 => "zh-CN".into(),
        3 => "ja".into(),
        4 => "zh-TW".into(),
        5 => "ar".into(),
        6 => "nb".into(),
        7 => "pt".into(),
        8 => "fr".into(),
        9 => "ru".into(),
        _ => String::new(),
    }
}

fn language_combo_index(language: &str) -> u32 {
    if language.trim().is_empty() {
        return 0;
    }
    let normalized = match language {
        "en" => "eng",
        "ja" => "jpn",
        "zh-CN" | "zh" => "zho",
        "zh-TW" => "zho",
        "fr" => "fra",
        "ar" => "ara",
        "nb" | "no" => "nor",
        "pt" => "por",
        "ru" => "rus",
        other => other,
    };
    language_codes::index_for_code(normalized) + 1
}

fn popover_open_in_tree(root: &gtk::Widget) -> bool {
    let mut stack = vec![root.clone()];
    while let Some(widget) = stack.pop() {
        if let Ok(popover) = widget.clone().downcast::<gtk::Popover>()
            && popover.is_visible()
        {
            return true;
        }
        let mut child = widget.first_child();
        while let Some(next) = child {
            stack.push(next.clone());
            child = next.next_sibling();
        }
    }
    false
}
