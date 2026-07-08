use std::cell::Cell;

use adw::prelude::*;
use gtk::{
    gio,
    gio::prelude::MenuModelExt,
    glib,
    glib::subclass::types::ObjectSubclassIsExt,
};

use super::actions::InputAction;
use crate::{
    Window,
    tv::set_tv_focused,
};

thread_local! {
    static MENU_POPOVER: Cell<usize> = const { Cell::new(0) };
    static MENU_INDEX: Cell<u32> = const { Cell::new(0) };
}

struct FlatMenuEntry {
    action: glib::GString,
}

pub fn handle(window: &Window, action: InputAction) -> bool {
    if let Some(settings) = window.imp().active_settings.borrow().clone()
        && settings.is_visible()
        && handle_widget_tree(settings.upcast_ref(), action)
    {
        return true;
    }
    if let Some(account) = window.imp().active_account_dialog.borrow().clone()
        && account.is_visible()
        && handle_widget_tree(account.upcast_ref(), action)
    {
        return true;
    }
    if handle_widget_tree(window.upcast_ref(), action) {
        return true;
    }
    if let Some(root) = window.root() {
        let root_widget = root.upcast_ref::<gtk::Widget>();
        if root_widget != window.upcast_ref::<gtk::Widget>() {
            return handle_widget_tree(root_widget, action);
        }
    }
    false
}

pub fn handle_widget_tree(root: &gtk::Widget, action: InputAction) -> bool {
    if let Some(popover) = find_open_menu_popover(root) {
        return handle_popover(&popover, action);
    }
    let Some(popover) = find_visible_popover(root) else {
        return false;
    };
    handle_popover(&popover, action)
}

pub fn popdown_visible_popover(root: &gtk::Widget) -> bool {
    if let Some(popover) = find_open_menu_popover(root).or_else(|| find_visible_popover(root)) {
        popover.popdown();
        return true;
    }
    false
}

fn find_open_menu_popover(root: &gtk::Widget) -> Option<gtk::Popover> {
    let mut stack = vec![root.clone()];
    let mut menu_popover = None;
    while let Some(widget) = stack.pop() {
        if let Ok(menu) = widget.clone().downcast::<gtk::PopoverMenu>()
            && menu.is_visible()
        {
            menu_popover = Some(menu.upcast());
        }
        if let Ok(button) = widget.clone().downcast::<gtk::MenuButton>()
            && button.is_active()
            && let Some(popover) = button.popover()
            && popover.is_visible()
        {
            return Some(popover);
        }
        let mut child = widget.first_child();
        while let Some(next) = child {
            stack.push(next.clone());
            child = next.next_sibling();
        }
    }
    menu_popover
}

fn activate_menu_action(anchor: &gtk::Widget, action: &str) {
    let mut current = Some(anchor.clone());
    while let Some(widget) = current {
        if gtk::prelude::WidgetExt::activate_action(&widget, action, None).is_ok() {
            return;
        }
        current = widget.parent();
    }
}

fn find_visible_popover(root: &gtk::Widget) -> Option<gtk::Popover> {
    let mut stack = vec![root.clone()];
    let mut found = None;
    while let Some(widget) = stack.pop() {
        if let Ok(popover) = widget.clone().downcast::<gtk::Popover>()
            && popover.is_visible()
        {
            found = Some(popover);
        }
        let mut child = widget.first_child();
        while let Some(next) = child {
            stack.push(next.clone());
            child = next.next_sibling();
        }
    }
    found
}

fn find_parent_popover(widget: &gtk::Widget) -> Option<gtk::Popover> {
    let mut parent = widget.parent();
    while let Some(w) = parent {
        if let Ok(popover) = w.clone().downcast::<gtk::Popover>() {
            return Some(popover);
        }
        parent = w.parent();
    }
    None
}

fn find_parent_dropdown(widget: &gtk::Widget) -> Option<gtk::DropDown> {
    let mut parent = widget.parent();
    while let Some(w) = parent {
        if let Ok(dropdown) = w.clone().downcast::<gtk::DropDown>() {
            return Some(dropdown);
        }
        parent = w.parent();
    }
    None
}

fn find_parent_combo_row(widget: &gtk::Widget) -> Option<adw::ComboRow> {
    let mut parent = widget.parent();
    while let Some(w) = parent {
        if let Ok(row) = w.clone().downcast::<adw::ComboRow>() {
            return Some(row);
        }
        parent = w.parent();
    }
    None
}

fn menu_anchor(popover: &gtk::Popover) -> gtk::Widget {
    popover
        .parent()
        .unwrap_or_else(|| popover.upcast_ref::<gtk::Widget>().clone())
}

fn append_menu_entries(model: &gio::MenuModel, entries: &mut Vec<FlatMenuEntry>) {
    for i in 0..model.n_items() {
        if let Some(submodel) = model.item_link(i, gio::MENU_LINK_SECTION) {
            append_menu_entries(&submodel, entries);
            continue;
        }
        if let Some(submodel) = model.item_link(i, gio::MENU_LINK_SUBMENU) {
            append_menu_entries(&submodel, entries);
            continue;
        }
        let Some(action) = model
            .item_attribute_value(i, gio::MENU_ATTRIBUTE_ACTION, Some(glib::VariantTy::STRING))
            .and_then(|value| value.get::<String>())
            .filter(|action| !action.is_empty())
        else {
            continue;
        };
        entries.push(FlatMenuEntry {
            action: action.into(),
        });
    }
}

fn collect_menu_entries(model: &gio::MenuModel) -> Vec<FlatMenuEntry> {
    let mut entries = Vec::new();
    append_menu_entries(model, &mut entries);
    entries
}

fn sync_menu_listview(popover: &gtk::Popover, index: u32) {
    let content = popover
        .child()
        .unwrap_or_else(|| popover.upcast_ref::<gtk::Widget>().clone());
    let Some(listview) = find_popover_listview(&content) else {
        return;
    };
    if let Some(selection) = listview
        .model()
        .and_then(|model| model.downcast::<gtk::SingleSelection>().ok())
    {
        selection.set_selected(index);
        listview.scroll_to(index, gtk::ListScrollFlags::NONE, None);
    }
    highlight_listview_row(&listview, index);
}

fn highlight_listview_row(listview: &gtk::ListView, index: u32) {
    clear_listview_tv_focus(listview);
    let mut pos = 0u32;
    let mut child = listview.first_child();
    while let Some(widget) = child {
        if pos == index {
            set_tv_focused(&widget, true);
            return;
        }
        pos += 1;
        child = widget.next_sibling();
    }
}

fn clear_listview_tv_focus(listview: &gtk::ListView) {
    let mut child = listview.first_child();
    while let Some(widget) = child {
        set_tv_focused(&widget, false);
        child = widget.next_sibling();
    }
}

fn handle_gmenu_popover(
    popover: &gtk::PopoverMenu, action: InputAction, on_back: impl FnOnce(),
) -> bool {
    let Some(model) = popover.menu_model() else {
        return false;
    };
    let anchor = menu_anchor(popover.upcast_ref());
    let entries = collect_menu_entries(&model);
    if entries.is_empty() {
        return false;
    }

    let popover_id = popover.as_ptr() as usize;
    if MENU_POPOVER.get() != popover_id {
        MENU_POPOVER.set(popover_id);
        MENU_INDEX.set(0);
    }

    let count = entries.len() as u32;
    let mut index = MENU_INDEX.get().min(count.saturating_sub(1));
    sync_menu_listview(popover.upcast_ref(), index);

    match action {
        InputAction::NavigateDown | InputAction::NavigateRight => {
            index = (index + 1).min(count - 1);
            MENU_INDEX.set(index);
            sync_menu_listview(popover.upcast_ref(), index);
            true
        }
        InputAction::NavigateUp | InputAction::NavigateLeft => {
            index = index.saturating_sub(1);
            MENU_INDEX.set(index);
            sync_menu_listview(popover.upcast_ref(), index);
            true
        }
        InputAction::Activate => {
            if let Some(entry) = entries.get(index as usize) {
                activate_menu_action(&anchor, &entry.action);
            }
            popover.popdown();
            true
        }
        InputAction::Back => {
            on_back();
            true
        }
        _ => false,
    }
}

fn handle_popover(popover: &gtk::Popover, action: InputAction) -> bool {
    if let Ok(menu_popover) = popover.clone().downcast::<gtk::PopoverMenu>()
        && menu_popover.menu_model().is_some()
    {
        return handle_gmenu_popover(&menu_popover, action, || popover.popdown());
    }

    let content = popover
        .child()
        .unwrap_or_else(|| popover.upcast_ref::<gtk::Widget>().clone());

    if let Some(listview) = find_popover_listview(&content) {
        return handle_listview(listview, action, || popover.popdown());
    }

    if let Some(listbox) = find_popover_listbox(&content) {
        return handle_listbox(&listbox, action, || popover.popdown());
    }

    let widgets = super::dialog_navigator::collect_focusable_widgets(&content);
    super::settings_navigator::SettingsNavigator::default()
        .handle_widgets(&widgets, action, || popover.popdown())
}

fn find_popover_listview(root: &gtk::Widget) -> Option<gtk::ListView> {
    let mut stack = vec![root.clone()];
    let mut found = None;
    while let Some(widget) = stack.pop() {
        if let Ok(listview) = widget.clone().downcast::<gtk::ListView>()
            && listview
                .model()
                .and_then(|model| model.downcast::<gtk::SingleSelection>().ok())
                .is_some_and(|sel| sel.n_items() > 0)
        {
            found = Some(listview);
        }
        let mut child = widget.first_child();
        while let Some(next) = child {
            stack.push(next.clone());
            child = next.next_sibling();
        }
    }
    found
}

fn find_popover_listbox(root: &gtk::Widget) -> Option<gtk::ListBox> {
    let mut stack = vec![root.clone()];
    let mut found = None;
    while let Some(widget) = stack.pop() {
        if let Ok(listbox) = widget.clone().downcast::<gtk::ListBox>()
            && !listbox_rows(&listbox).is_empty()
        {
            found = Some(listbox);
        }
        let mut child = widget.first_child();
        while let Some(next) = child {
            stack.push(next.clone());
            child = next.next_sibling();
        }
    }
    found
}

fn listbox_rows(listbox: &gtk::ListBox) -> Vec<gtk::ListBoxRow> {
    let mut rows = Vec::new();
    let mut child = listbox.first_child();
    while let Some(widget) = child {
        let next = widget.next_sibling();
        if let Ok(row) = widget.downcast::<gtk::ListBoxRow>()
            && row.is_visible()
            && row.is_sensitive()
            && row.parent().as_ref() == Some(listbox.upcast_ref())
        {
            rows.push(row);
        }
        child = next;
    }
    rows
}

fn handle_listbox(listbox: &gtk::ListBox, action: InputAction, on_back: impl FnOnce()) -> bool {
    let rows = listbox_rows(listbox);
    if rows.is_empty() {
        return false;
    }

    let mut current = listbox
        .selected_row()
        .and_then(|row| rows.iter().position(|r| r == &row))
        .unwrap_or(0) as i32;

    if listbox.selected_row().is_none() {
        select_listbox_row(listbox, &rows, 0);
        current = 0;
    }

    match action {
        InputAction::NavigateDown | InputAction::NavigateRight => {
            let next = (current + 1).min(rows.len() as i32 - 1) as usize;
            select_listbox_row(listbox, &rows, next);
            true
        }
        InputAction::NavigateUp | InputAction::NavigateLeft => {
            let next = current.saturating_sub(1) as usize;
            select_listbox_row(listbox, &rows, next);
            true
        }
        InputAction::Activate => {
            let index = current.clamp(0, rows.len() as i32 - 1) as usize;
            if let Some(row) = rows.get(index) {
                listbox.select_row(Some(row));
                row.activate();
            }
            if let Some(popover) = find_parent_popover(listbox.upcast_ref()) {
                popover.popdown();
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

fn select_listbox_row(listbox: &gtk::ListBox, rows: &[gtk::ListBoxRow], index: usize) {
    let Some(row) = rows.get(index) else {
        return;
    };
    for row in rows {
        set_tv_focused(row, false);
    }
    listbox.select_row(Some(row));
    set_tv_focused(row, true);
}

fn handle_listview(listview: gtk::ListView, action: InputAction, on_back: impl FnOnce()) -> bool {
    let Some(selection) = listview
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
    if selection.selected() == gtk::INVALID_LIST_POSITION {
        selection.set_selected(0);
        listview.scroll_to(0, gtk::ListScrollFlags::NONE, None);
    }

    match action {
        InputAction::NavigateDown | InputAction::NavigateRight => {
            let next = (current + 1).clamp(0, count - 1) as u32;
            selection.set_selected(next);
            listview.scroll_to(next, gtk::ListScrollFlags::NONE, None);
            highlight_listview_row(&listview, next);
            true
        }
        InputAction::NavigateUp | InputAction::NavigateLeft => {
            let next = (current - 1).clamp(0, count - 1) as u32;
            selection.set_selected(next);
            listview.scroll_to(next, gtk::ListScrollFlags::NONE, None);
            highlight_listview_row(&listview, next);
            true
        }
        InputAction::Activate => {
            let index = selection.selected();
            let widget = listview.upcast_ref::<gtk::Widget>();
            if let Some(combo) = find_parent_combo_row(widget) {
                if index != gtk::INVALID_LIST_POSITION {
                    combo.set_selected(index);
                }
                if let Some(popover) = find_parent_popover(widget) {
                    popover.popdown();
                }
            } else if let Some(dropdown) = find_parent_dropdown(widget) {
                if index != gtk::INVALID_LIST_POSITION {
                    dropdown.set_selected(index);
                }
                if let Some(popover) = find_parent_popover(widget) {
                    popover.popdown();
                }
            } else {
                listview.activate();
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
