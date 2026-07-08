#![allow(deprecated)]

use adw::prelude::*;

use super::{
    actions::InputAction,
    popover_navigator,
};
use crate::{
    tv::{
        osk,
        set_tv_focused,
    },
    ui::widgets::{
        account_add::AccountWindow,
        account_settings::AccountSettings,
    },
};

pub struct SettingsNavigator {
    row_index: std::cell::Cell<u32>,
    tab_index: std::cell::Cell<u32>,
    row_child_index: std::cell::Cell<Option<u32>>,
}

impl Default for SettingsNavigator {
    fn default() -> Self {
        Self {
            row_index: std::cell::Cell::new(0),
            tab_index: std::cell::Cell::new(0),
            row_child_index: std::cell::Cell::new(None),
        }
    }
}

impl SettingsNavigator {
    pub fn reset_for_preferences(&self, settings: &AccountSettings) {
        self.row_index.set(0);
        self.row_child_index.set(None);
        let pages = settings.preference_pages();
        let tab_index = pages
            .iter()
            .position(|page| {
                settings
                    .visible_page()
                    .is_some_and(|visible| visible.as_ptr() == page.as_ptr())
            })
            .unwrap_or(0) as u32;
        self.tab_index.set(tab_index);
        let rows = rows_for_page(settings.upcast_ref(), &pages, tab_index);
        if !rows.is_empty() {
            self.apply_focus(&rows, 0);
        }
    }

    pub fn handle_window(&self, settings: &AccountSettings, action: InputAction) -> bool {
        if popover_navigator::handle_widget_tree(settings.upcast_ref(), action) {
            return true;
        }

        let pages = settings.preference_pages();
        if pages.is_empty() {
            return self.handle_window_inner(settings.upcast_ref(), action);
        }
        self.handle_preferences(settings.upcast_ref(), &pages, action)
    }

    fn handle_window_inner(&self, window: &adw::Window, action: InputAction) -> bool {
        let rows = collect_focusable_rows_from_root(&window.clone().upcast::<gtk::Widget>());
        self.handle_rows_only(&rows, action, false, || window.close())
    }

    fn handle_preferences(
        &self, prefs: &adw::PreferencesWindow, pages: &[adw::PreferencesPage], action: InputAction,
    ) -> bool {
        let prefs_widget = prefs.clone().upcast::<gtk::Widget>();
        let tab_count = pages.len() as u32;
        let mut tab_index = self.tab_index.get().min(tab_count.saturating_sub(1));

        if let Some(page) = pages.get(tab_index as usize) {
            prefs.set_visible_page(page);
        }

        let rows = rows_for_page(prefs, pages, tab_index);

        if self.handle_row_child_navigation(&rows, action) {
            return true;
        }

        match action {
            InputAction::NavigateLeft => {
                tab_index = tab_index.saturating_sub(1);
                self.switch_tab(prefs, pages, tab_index);
                true
            }
            InputAction::NavigateRight => {
                tab_index = (tab_index + 1).min(tab_count - 1);
                self.switch_tab(prefs, pages, tab_index);
                true
            }
            InputAction::NavigateDown => {
                if rows.is_empty() {
                    return true;
                }
                let count = rows.len() as u32;
                let index = (self.row_index.get() + 1).min(count - 1);
                self.row_index.set(index);
                self.apply_focus(&rows, index);
                true
            }
            InputAction::NavigateUp => {
                if rows.is_empty() {
                    return true;
                }
                let index = self.row_index.get().saturating_sub(1);
                self.row_index.set(index);
                self.apply_focus(&rows, index);
                true
            }
            InputAction::Activate => {
                if let Some(row) = rows.get(self.row_index.get() as usize) {
                    self.activate_row_widget(row);
                }
                true
            }
            InputAction::Back => {
                if self.row_child_index.get().is_some() {
                    self.row_child_index.set(None);
                    self.apply_focus(&rows, self.row_index.get());
                    true
                } else if popover_navigator::popdown_visible_popover(&prefs_widget) {
                    true
                } else {
                    prefs.close();
                    true
                }
            }
            _ => false,
        }
    }

    fn switch_tab(
        &self, prefs: &adw::PreferencesWindow, pages: &[adw::PreferencesPage], tab_index: u32,
    ) {
        popover_navigator::popdown_visible_popover(&prefs.clone().upcast::<gtk::Widget>());
        self.tab_index.set(tab_index);
        self.row_index.set(0);
        self.row_child_index.set(None);
        if let Some(page) = pages.get(tab_index as usize) {
            prefs.set_visible_page(page);
        }
        let rows = rows_for_page(prefs, pages, tab_index);
        if rows.is_empty() {
            return;
        }
        self.apply_focus(&rows, 0);
    }

    fn handle_rows_only(
        &self, rows: &[gtk::Widget], action: InputAction, allow_tab_switch: bool,
        on_back: impl FnOnce(),
    ) -> bool {
        if rows.is_empty() {
            return false;
        }
        let count = rows.len() as u32;
        let mut index = self.row_index.get().min(count - 1);

        if self.handle_row_child_navigation(rows, action) {
            return true;
        }

        match action {
            InputAction::NavigateDown => {
                index = (index + 1).min(count - 1);
                self.row_index.set(index);
                self.apply_focus(rows, index);
                true
            }
            InputAction::NavigateUp => {
                index = index.saturating_sub(1);
                self.row_index.set(index);
                self.apply_focus(rows, index);
                true
            }
            InputAction::NavigateLeft | InputAction::NavigateRight if allow_tab_switch => false,
            InputAction::Activate => {
                if let Some(row) = rows.get(index as usize) {
                    self.activate_row_widget(row);
                }
                true
            }
            InputAction::Back => {
                if self.row_child_index.get().is_some() {
                    self.row_child_index.set(None);
                    self.apply_focus(rows, index);
                    true
                } else {
                    on_back();
                    true
                }
            }
            _ => false,
        }
    }

    fn handle_row_child_navigation(&self, rows: &[gtk::Widget], action: InputAction) -> bool {
        let Some(child_index) = self.row_child_index.get() else {
            return false;
        };
        let row_index = self.row_index.get();
        let Some(row) = rows.get(row_index as usize) else {
            return false;
        };
        let buttons = row_suffix_buttons(row);
        if buttons.is_empty() {
            self.row_child_index.set(None);
            return false;
        }

        match action {
            InputAction::NavigateLeft => {
                let next = child_index.saturating_sub(1);
                self.row_child_index.set(Some(next));
                self.apply_child_focus(row, next, &buttons);
                true
            }
            InputAction::NavigateRight => {
                let next = (child_index + 1).min(buttons.len() as u32 - 1);
                self.row_child_index.set(Some(next));
                self.apply_child_focus(row, next, &buttons);
                true
            }
            InputAction::Activate => {
                if let Some(button) = buttons.get(child_index as usize) {
                    button.emit_clicked();
                }
                true
            }
            _ => false,
        }
    }

    pub fn handle_account_window(&self, account: &AccountWindow, action: InputAction) -> bool {
        if popover_navigator::handle_widget_tree(account.upcast_ref(), action) {
            return true;
        }
        let widgets = account.focus_widgets();
        let count = widgets.len() as u32;
        if matches!(action, InputAction::Activate) {
            let index = self.row_index.get().min(count.saturating_sub(1));
            if index == count.saturating_sub(1) {
                let account = account.clone();
                crate::utils::spawn(async move {
                    account.add().await;
                });
                return true;
            }
        }
        self.handle_widgets(&widgets, action, || {
            adw::prelude::AdwDialogExt::close(account);
        })
    }

    pub fn handle_widgets(
        &self, widgets: &[gtk::Widget], action: InputAction, on_back: impl FnOnce(),
    ) -> bool {
        self.handle_rows_only(widgets, action, false, on_back)
    }

    fn activate_row_widget(&self, row: &gtk::Widget) {
        if self.row_child_index.get().is_some() {
            let buttons = row_suffix_buttons(row);
            if let Some(child_index) = self.row_child_index.get()
                && let Some(button) = buttons.get(child_index as usize)
            {
                button.emit_clicked();
            }
            return;
        }

        let buttons = row_suffix_buttons(row);
        if buttons.len() == 1 {
            buttons[0].emit_clicked();
            return;
        }
        if buttons.len() > 1 {
            self.row_child_index.set(Some(0));
            self.apply_child_focus(row, 0, &buttons);
            return;
        }

        if let Ok(switch) = row.clone().downcast::<adw::SwitchRow>() {
            switch.set_active(!switch.is_active());
        } else if let Ok(switch) = row.clone().downcast::<gtk::Switch>() {
            switch.set_active(!switch.is_active());
        } else if let Ok(spin) = row.clone().downcast::<adw::SpinRow>() {
            let adjustment = spin.adjustment();
            adjustment.set_value(adjustment.value() + adjustment.step_increment());
        } else if let Ok(row) = row.clone().downcast::<adw::ComboRow>() {
            adw::prelude::ActionRowExt::activate(&row);
        } else if let Ok(row) = row.clone().downcast::<adw::PasswordEntryRow>() {
            row.grab_focus();
            osk::show_for_widget(&row);
        } else if let Ok(row) = row.clone().downcast::<adw::EntryRow>() {
            row.grab_focus();
            osk::show_for_widget(&row);
        } else if let Ok(row) = row.clone().downcast::<adw::ActionRow>() {
            adw::prelude::ActionRowExt::activate(&row);
        } else if let Ok(button) = row.clone().downcast::<gtk::Button>() {
            button.emit_clicked();
        } else {
            row.activate();
        }
    }

    fn apply_focus(&self, rows: &[gtk::Widget], index: u32) {
        self.row_child_index.set(None);
        for row in rows {
            set_tv_focused(row, false);
            clear_child_focus(row);
        }
        if let Some(row) = rows.get(index as usize) {
            set_tv_focused(row, true);
            if should_grab_row_focus(row) {
                row.grab_focus();
            }
        }
    }

    fn apply_child_focus(&self, row: &gtk::Widget, child_index: u32, buttons: &[gtk::Button]) {
        set_tv_focused(row, true);
        clear_child_focus(row);
        if let Some(button) = buttons.get(child_index as usize) {
            set_tv_focused(button, true);
        }
    }
}

fn should_grab_row_focus(row: &gtk::Widget) -> bool {
    !row.is::<adw::EntryRow>() && !row.is::<adw::PasswordEntryRow>() && !row.is::<adw::SpinRow>()
}

fn row_suffix_buttons(row: &gtk::Widget) -> Vec<gtk::Button> {
    let mut buttons = Vec::new();
    collect_buttons(row, &mut buttons);
    buttons
}

fn collect_buttons(widget: &gtk::Widget, buttons: &mut Vec<gtk::Button>) {
    if let Ok(button) = widget.clone().downcast::<gtk::Button>()
        && button.is_visible()
        && button.is_sensitive()
    {
        buttons.push(button);
    }
    let mut child = widget.first_child();
    while let Some(next) = child {
        collect_buttons(&next, buttons);
        child = next.next_sibling();
    }
}

fn clear_child_focus(row: &gtk::Widget) {
    for button in row_suffix_buttons(row) {
        set_tv_focused(&button, false);
    }
}

pub fn find_view_stack_pages(prefs: &adw::PreferencesWindow) -> Vec<adw::PreferencesPage> {
    let Some(stack) = find_view_stack(&prefs.clone().upcast::<gtk::Widget>()) else {
        return Vec::new();
    };
    let mut pages = Vec::new();
    let mut child = stack.first_child();
    while let Some(page_widget) = child {
        if let Ok(page) = page_widget.clone().downcast::<adw::PreferencesPage>() {
            pages.push(page);
        }
        child = page_widget.next_sibling();
    }
    pages
}

fn find_view_stack(widget: &gtk::Widget) -> Option<adw::ViewStack> {
    if let Ok(stack) = widget.clone().downcast::<adw::ViewStack>() {
        return Some(stack);
    }
    let mut child = widget.first_child();
    while let Some(next) = child {
        if let Some(stack) = find_view_stack(&next) {
            return Some(stack);
        }
        child = next.next_sibling();
    }
    None
}

fn rows_for_page(
    prefs: &adw::PreferencesWindow, pages: &[adw::PreferencesPage], tab_index: u32,
) -> Vec<gtk::Widget> {
    let page = prefs
        .visible_page()
        .or_else(|| pages.get(tab_index as usize).cloned());
    let Some(page) = page else {
        return Vec::new();
    };
    collect_focusable_rows_from_root(&page.upcast::<gtk::Widget>())
}

fn collect_focusable_rows_from_root(root: &gtk::Widget) -> Vec<gtk::Widget> {
    let mut rows = Vec::new();
    collect_focusable_rows_in_order(root, &mut rows);
    rows
}

fn collect_focusable_rows_in_order(widget: &gtk::Widget, rows: &mut Vec<gtk::Widget>) {
    if is_focusable_preferences_row(widget) {
        if widget.is_visible() && widget.is_sensitive() {
            rows.push(widget.clone());
        }
        return;
    }
    let mut child = widget.first_child();
    while let Some(next) = child {
        collect_focusable_rows_in_order(&next, rows);
        child = next.next_sibling();
    }
}

fn is_focusable_preferences_row(widget: &gtk::Widget) -> bool {
    widget.downcast_ref::<adw::PreferencesRow>().is_some()
        || widget.downcast_ref::<adw::ActionRow>().is_some()
        || widget.downcast_ref::<adw::ButtonRow>().is_some()
        || widget.downcast_ref::<adw::ComboRow>().is_some()
        || widget.downcast_ref::<adw::EntryRow>().is_some()
        || widget.downcast_ref::<adw::PasswordEntryRow>().is_some()
        || widget.downcast_ref::<adw::SpinRow>().is_some()
        || widget.downcast_ref::<adw::SwitchRow>().is_some()
        || widget.downcast_ref::<gtk::Button>().is_some()
}
