use std::cell::RefCell;

use gtk::prelude::*;

use super::actions::InputAction;
use crate::{
    Window,
    tv::set_tv_focused,
    ui::widgets::server_action_row::ServerActionRow,
};

pub struct PlaceholderNavigator {
    selected_index: RefCell<u32>,
    selected_column: RefCell<u32>,
    last_row: RefCell<Option<gtk::ListBoxRow>>,
    add_server_button: RefCell<Option<gtk::Button>>,
}

impl Default for PlaceholderNavigator {
    fn default() -> Self {
        Self {
            selected_index: RefCell::new(0),
            selected_column: RefCell::new(0),
            last_row: RefCell::new(None),
            add_server_button: RefCell::new(None),
        }
    }
}

impl PlaceholderNavigator {
    pub fn reset(&self) {
        *self.selected_index.borrow_mut() = 0;
        *self.selected_column.borrow_mut() = 0;
        self.clear_row_focus();
        self.clear_add_server_focus();
    }

    pub fn select_initial(&self, listbox: &gtk::ListBox) {
        if listbox.observe_children().n_items() > 0 {
            select_row(self, listbox, 0);
        }
    }

    pub fn focus_add_server(&self, button: &gtk::Button) {
        self.clear_row_focus();
        *self.add_server_button.borrow_mut() = Some(button.clone());
        set_tv_focused(button, true);
        button.grab_focus();
    }

    pub fn handle(
        &self, window: &Window, login_stack: &gtk::Stack, listbox: &gtk::ListBox,
        action: InputAction,
    ) -> bool {
        if login_stack.visible_child_name().as_deref() == Some("no-server") {
            return self.handle_no_server(window, action);
        }

        let count = listbox.observe_children().n_items();
        if count == 0 {
            return match action {
                InputAction::Activate => {
                    window.new_account();
                    true
                }
                _ => false,
            };
        }

        let mut index = *self.selected_index.borrow();
        if index >= count {
            index = count.saturating_sub(1);
        }
        let mut column = *self.selected_column.borrow();

        match action {
            InputAction::NavigateDown => {
                index = (index + 1).min(count - 1);
                column = 0;
                *self.selected_index.borrow_mut() = index;
                *self.selected_column.borrow_mut() = column;
                select_row(self, listbox, index);
                true
            }
            InputAction::NavigateUp => {
                index = index.saturating_sub(1);
                column = 0;
                *self.selected_index.borrow_mut() = index;
                *self.selected_column.borrow_mut() = column;
                select_row(self, listbox, index);
                true
            }
            InputAction::NavigateRight => {
                if let Some(row) = listbox.row_at_index(index as i32)
                    && let Ok(server_row) = row.downcast::<ServerActionRow>()
                {
                    let max_col = server_row.tv_focus_widgets().len().saturating_sub(1) as u32;
                    column = (column + 1).min(max_col);
                    *self.selected_column.borrow_mut() = column;
                    server_row.set_tv_column_focus(column as usize);
                    return true;
                }
                false
            }
            InputAction::NavigateLeft => {
                if column > 0 {
                    column = column.saturating_sub(1);
                    *self.selected_column.borrow_mut() = column;
                    if let Some(row) = listbox.row_at_index(index as i32)
                        && let Ok(server_row) = row.downcast::<ServerActionRow>()
                    {
                        server_row.set_tv_column_focus(column as usize);
                    }
                    return true;
                }
                false
            }
            InputAction::Activate => {
                if let Some(row) = listbox.row_at_index(index as i32) {
                    if let Ok(server_row) = row.clone().downcast::<ServerActionRow>() {
                        server_row.activate_column(column);
                    } else {
                        row.activate();
                    }
                }
                true
            }
            _ => false,
        }
    }

    fn handle_no_server(&self, window: &Window, action: InputAction) -> bool {
        match action {
            InputAction::Activate => {
                window.new_account();
                true
            }
            _ => false,
        }
    }
}

fn select_row(navigator: &PlaceholderNavigator, listbox: &gtk::ListBox, index: u32) {
    navigator.clear_row_focus();
    navigator.clear_add_server_focus();
    *navigator.selected_column.borrow_mut() = 0;
    if let Some(row) = listbox.row_at_index(index as i32) {
        listbox.select_row(Some(&row));
        row.grab_focus();
        if let Ok(server_row) = row.clone().downcast::<ServerActionRow>() {
            server_row.set_tv_column_focus(0);
        } else {
            set_tv_focused(&row, true);
        }
        *navigator.last_row.borrow_mut() = Some(row);
    }
}

impl PlaceholderNavigator {
    fn clear_row_focus(&self) {
        if let Some(row) = self.last_row.borrow_mut().take() {
            if let Ok(server_row) = row.clone().downcast::<ServerActionRow>() {
                server_row.clear_tv_column_focus();
            } else {
                set_tv_focused(&row, false);
            }
        }
    }

    fn clear_add_server_focus(&self) {
        if let Some(button) = self.add_server_button.borrow_mut().take() {
            set_tv_focused(&button, false);
        }
    }
}
