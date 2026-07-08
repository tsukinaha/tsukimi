use adw::prelude::*;
use gtk::glib::subclass::types::ObjectSubclassIsExt;

use super::{
    actions::InputAction,
    settings_navigator::SettingsNavigator,
};
use crate::{
    Window,
    tv::set_tv_focused,
};

thread_local! {
    static NAVIGATOR: std::cell::RefCell<SettingsNavigator> =
        std::cell::RefCell::new(SettingsNavigator::default());
}

pub fn handle(window: &Window, action: InputAction) -> bool {
    if window
        .imp()
        .active_settings
        .borrow()
        .as_ref()
        .is_some_and(|settings| settings.is_visible())
        || window
            .imp()
            .active_account_dialog
            .borrow()
            .as_ref()
            .is_some_and(|dialog| dialog.is_visible())
    {
        return false;
    }

    if let Some(buttons) = find_alert_buttons(window) {
        return handle_buttons(&buttons, action, || {});
    }

    let Some(dialog) = find_visible_dialog(window) else {
        return false;
    };

    if let Some(grid) = find_visible_grid_in_dialog(&dialog) {
        return handle_grid_selection(&grid, action, || {
            let _ = dialog.close();
        });
    }

    if let Some(listbox) = find_visible_listbox_in_dialog(&dialog) {
        return handle_listbox(&listbox, action, || {
            let _ = dialog.close();
        });
    }

    let root = dialog_content_root(&dialog);
    let widgets = collect_focusable_widgets(&root);
    NAVIGATOR.with(|nav| {
        nav.borrow().handle_widgets(&widgets, action, || {
            let _ = dialog.close();
        })
    })
}

fn dialog_content_root(dialog: &adw::Dialog) -> gtk::Widget {
    let Some(child) = dialog.child() else {
        return dialog.upcast_ref::<gtk::Widget>().clone();
    };
    if let Some(nav) = find_descendant::<adw::NavigationView>(child.upcast_ref())
        && let Some(page) = nav.visible_page()
    {
        return page.upcast::<gtk::Widget>();
    }
    child
}

fn find_visible_dialog(window: &Window) -> Option<adw::Dialog> {
    let mut stack = vec![window.upcast_ref::<gtk::Widget>().clone()];
    let mut found = None;
    while let Some(widget) = stack.pop() {
        if let Ok(dialog) = widget.clone().downcast::<adw::Dialog>()
            && dialog.is_visible()
        {
            found = Some(dialog);
        }
        let mut child = widget.first_child();
        while let Some(next) = child {
            stack.push(next.clone());
            child = next.next_sibling();
        }
    }
    found
}

fn find_alert_buttons(window: &Window) -> Option<Vec<gtk::Button>> {
    let buttons = collect_alert_buttons(window.upcast_ref());
    if !buttons.is_empty() {
        Some(buttons)
    } else {
        None
    }
}

fn collect_alert_buttons(root: &gtk::Widget) -> Vec<gtk::Button> {
    let mut buttons = Vec::new();
    let mut stack = vec![root.clone()];
    while let Some(widget) = stack.pop() {
        if let Ok(button) = widget.clone().downcast::<gtk::Button>()
            && button.is_visible()
            && button.is_sensitive()
            && button.parent().and_then(|p| p.parent()).is_some_and(|gp| {
                gp.has_css_class("dialog")
                    || gp.type_().name().contains("Alert")
                    || gp.type_().name().contains("Message")
            })
        {
            buttons.push(button);
        }
        let mut child = widget.first_child();
        while let Some(next) = child {
            stack.push(next.clone());
            child = next.next_sibling();
        }
    }
    buttons
}

fn handle_buttons(buttons: &[gtk::Button], action: InputAction, on_back: impl FnOnce()) -> bool {
    if buttons.is_empty() {
        return false;
    }
    let focused = buttons
        .iter()
        .position(|button| button.has_css_class("tv-focused"))
        .unwrap_or(0) as i32;
    match action {
        InputAction::NavigateRight | InputAction::NavigateDown => {
            let next = (focused + 1).min(buttons.len() as i32 - 1) as usize;
            apply_button_focus(buttons, next);
            true
        }
        InputAction::NavigateLeft | InputAction::NavigateUp => {
            let next = focused.saturating_sub(1) as usize;
            apply_button_focus(buttons, next);
            true
        }
        InputAction::Activate => {
            let index = buttons
                .iter()
                .position(|button| button.has_css_class("tv-focused"))
                .unwrap_or(0);
            buttons[index].emit_clicked();
            true
        }
        InputAction::Back => {
            on_back();
            buttons.first().map(gtk::Button::emit_clicked);
            true
        }
        _ => false,
    }
}

fn apply_button_focus(buttons: &[gtk::Button], index: usize) {
    for button in buttons {
        set_tv_focused(button, false);
    }
    if let Some(button) = buttons.get(index) {
        set_tv_focused(button, true);
        button.grab_focus();
    }
}

fn find_visible_grid_in_dialog(dialog: &adw::Dialog) -> Option<gtk::GridView> {
    find_descendant(&dialog_content_root(dialog))
}

fn find_visible_listbox_in_dialog(dialog: &adw::Dialog) -> Option<gtk::ListBox> {
    find_descendant(&dialog_content_root(dialog))
}

fn find_descendant<T: IsA<gtk::Widget>>(root: &gtk::Widget) -> Option<T> {
    let mut stack = vec![root.clone()];
    while let Some(widget) = stack.pop() {
        if let Ok(found) = widget.clone().downcast::<T>() {
            return Some(found);
        }
        let mut child = widget.first_child();
        while let Some(next) = child {
            stack.push(next.clone());
            child = next.next_sibling();
        }
    }
    None
}

fn handle_grid_selection(
    grid: &gtk::GridView, action: InputAction, on_back: impl FnOnce(),
) -> bool {
    let Some(selection) = grid
        .model()
        .and_then(|model| model.downcast::<gtk::SingleSelection>().ok())
    else {
        return false;
    };
    let count = selection.n_items() as i32;
    if count == 0 {
        return false;
    }
    let current = if selection.selected() == gtk::INVALID_LIST_POSITION {
        0
    } else {
        selection.selected() as i32
    };
    let cols = estimate_grid_columns(grid);
    let (row, col) = (current / cols, current % cols);
    match action {
        InputAction::NavigateLeft => {
            let next = (current - 1).clamp(0, count - 1) as u32;
            selection.set_selected(next);
            grid.scroll_to(next, gtk::ListScrollFlags::NONE, None);
            true
        }
        InputAction::NavigateRight => {
            let next = (current + 1).clamp(0, count - 1) as u32;
            selection.set_selected(next);
            grid.scroll_to(next, gtk::ListScrollFlags::NONE, None);
            true
        }
        InputAction::NavigateUp => {
            let next = ((row - 1) * cols + col).clamp(0, count - 1) as u32;
            selection.set_selected(next);
            grid.scroll_to(next, gtk::ListScrollFlags::NONE, None);
            true
        }
        InputAction::NavigateDown => {
            let next = ((row + 1) * cols + col).clamp(0, count - 1) as u32;
            selection.set_selected(next);
            grid.scroll_to(next, gtk::ListScrollFlags::NONE, None);
            true
        }
        InputAction::Activate => {
            let position = selection.selected();
            if position != gtk::INVALID_LIST_POSITION {
                grid.emit_by_name::<()>("activate", &[&position]);
            }
            true
        }
        InputAction::Back => {
            on_back();
            true
        }
        _ => false,
    }
}

fn estimate_grid_columns(grid: &gtk::GridView) -> i32 {
    let width = grid.width().max(1);
    let max_cols = grid.max_columns().max(1) as i32;
    let item_width = (width / max_cols).max(1);
    (width / item_width).max(1)
}

fn handle_listbox(listbox: &gtk::ListBox, action: InputAction, on_back: impl FnOnce()) -> bool {
    let rows: Vec<gtk::ListBoxRow> = listbox
        .observe_children()
        .into_iter()
        .filter_map(|child| child.ok())
        .filter_map(|child| child.downcast::<gtk::ListBoxRow>().ok())
        .collect();
    if rows.is_empty() {
        return false;
    }
    let current = listbox
        .selected_row()
        .and_then(|row| rows.iter().position(|r| r == &row))
        .unwrap_or(0) as i32;
    match action {
        InputAction::NavigateDown | InputAction::NavigateRight => {
            let next = (current + 1).min(rows.len() as i32 - 1) as usize;
            apply_listbox_focus(listbox, &rows, next);
            true
        }
        InputAction::NavigateUp | InputAction::NavigateLeft => {
            let next = current.saturating_sub(1) as usize;
            apply_listbox_focus(listbox, &rows, next);
            true
        }
        InputAction::Activate => {
            if let Some(row) = listbox.selected_row() {
                row.activate();
            }
            true
        }
        InputAction::Back => {
            on_back();
            true
        }
        _ => false,
    }
}

fn apply_listbox_focus(listbox: &gtk::ListBox, rows: &[gtk::ListBoxRow], index: usize) {
    let Some(row) = rows.get(index) else {
        return;
    };
    for row in rows {
        set_tv_focused(row, false);
    }
    listbox.select_row(Some(row));
    set_tv_focused(row, true);
    row.grab_focus();
}

pub fn collect_focusable_widgets(root: &gtk::Widget) -> Vec<gtk::Widget> {
    let mut rows = Vec::new();
    let mut stack = vec![root.clone()];
    while let Some(widget) = stack.pop() {
        if widget.downcast_ref::<adw::PreferencesRow>().is_some()
            || widget.downcast_ref::<adw::ActionRow>().is_some()
            || widget.downcast_ref::<adw::ButtonRow>().is_some()
            || widget.downcast_ref::<adw::ComboRow>().is_some()
            || widget.downcast_ref::<adw::EntryRow>().is_some()
            || widget.downcast_ref::<adw::PasswordEntryRow>().is_some()
            || widget.downcast_ref::<adw::SpinRow>().is_some()
            || widget.downcast_ref::<adw::SwitchRow>().is_some()
            || widget.downcast_ref::<gtk::CheckButton>().is_some()
            || (widget.downcast_ref::<gtk::Button>().is_some()
                && widget.parent().is_some_and(|p| !p.is_ancestor(root)))
        {
            if widget.is_visible() && widget.is_sensitive() {
                rows.push(widget);
            }
            continue;
        }
        let mut child = widget.first_child();
        while let Some(c) = child {
            stack.push(c.clone());
            child = c.next_sibling();
        }
    }
    rows
}
