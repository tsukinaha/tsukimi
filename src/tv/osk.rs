use std::{
    cell::RefCell,
    rc::Rc,
};

use gtk::{
    glib::object::IsA,
    prelude::*,
};

use crate::{
    tv::{
        controller_navigation_enabled,
        is_tv_mode_active,
        set_tv_focused,
    },
    ui::input::InputAction,
};

static LAST_GAMEPAD_MS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static LAST_DIRECT_MS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

thread_local! {
    static OSK: RefCell<Option<Rc<OnScreenKeyboard>>> = const { RefCell::new(None) };
}

const ROW_LENGTHS: [usize; 5] = [10, 10, 9, 9, 3];

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Call when the user moves or clicks with a mouse/trackpad.
pub fn mark_pointer_input() {
    LAST_DIRECT_MS.store(now_ms(), std::sync::atomic::Ordering::Relaxed);
    super::cursor::on_pointer_activity();
}

/// Call when the user types on a physical keyboard.
pub fn mark_keyboard_input() {
    LAST_DIRECT_MS.store(now_ms(), std::sync::atomic::Ordering::Relaxed);
    hide_keyboard();
}

/// Call when a gamepad button or stick produces navigation input.
pub fn mark_gamepad_input() {
    LAST_GAMEPAD_MS.store(now_ms(), std::sync::atomic::Ordering::Relaxed);
    if crate::tv::cursor::suppress_pointer_hover() {
        crate::ui::widgets::hover_scale::request_pointer_targeting_sync();
    }
}

/// True when gamepad input is more recent than mouse/keyboard direct input.
pub fn gamepad_owns_direct_input() -> bool {
    LAST_GAMEPAD_MS.load(std::sync::atomic::Ordering::Relaxed)
        > LAST_DIRECT_MS.load(std::sync::atomic::Ordering::Relaxed)
}

fn should_offer_osk() -> bool {
    !crate::steam::is_steam_big_picture()
        && (is_tv_mode_active() || controller_navigation_enabled())
}

#[allow(dead_code)]
pub fn is_visible() -> bool {
    OSK.with(|slot| {
        slot.borrow()
            .as_ref()
            .is_some_and(|osk| osk.revealer.reveals_child())
    })
}

pub fn handle_input(action: InputAction) -> bool {
    OSK.with(|slot| {
        let Some(osk) = slot.borrow().clone() else {
            return false;
        };
        if !osk.revealer.reveals_child() {
            return false;
        }
        osk.handle_input(action)
    })
}

struct OnScreenKeyboard {
    revealer: gtk::Revealer,
    target: RefCell<Option<gtk::Widget>>,
    shifted: RefCell<bool>,
    buttons: RefCell<Vec<gtk::Button>>,
    focused_index: RefCell<usize>,
}

impl OnScreenKeyboard {
    fn ensure() -> Rc<Self> {
        OSK.with(|slot| {
            if let Some(existing) = slot.borrow().clone() {
                return existing;
            }

            let revealer = gtk::Revealer::builder()
                .valign(gtk::Align::End)
                .transition_type(gtk::RevealerTransitionType::SlideUp)
                .reveal_child(false)
                .build();
            revealer.add_css_class("tv-osk");

            let keyboard = Rc::new(Self {
                revealer: revealer.clone(),
                target: RefCell::new(None),
                shifted: RefCell::new(false),
                buttons: RefCell::new(Vec::new()),
                focused_index: RefCell::new(0),
            });

            let grid = gtk::Grid::builder()
                .row_spacing(6)
                .column_spacing(6)
                .margin_top(8)
                .margin_bottom(8)
                .margin_start(12)
                .margin_end(12)
                .build();

            let rows: &[&[&str]] = &[
                &["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"],
                &["q", "w", "e", "r", "t", "y", "u", "i", "o", "p"],
                &["a", "s", "d", "f", "g", "h", "j", "k", "l"],
                &["⇧", "z", "x", "c", "v", "b", "n", "m", "⌫"],
                &["␣", "↵", "✕"],
            ];

            for (row_idx, row) in rows.iter().enumerate() {
                for (col_idx, key) in row.iter().enumerate() {
                    let button = gtk::Button::with_label(key);
                    button.add_css_class("flat");
                    button.set_hexpand(*key == "␣");
                    if *key == "␣" {
                        grid.attach(&button, col_idx as i32, row_idx as i32, 3, 1);
                    } else {
                        grid.attach(&button, col_idx as i32, row_idx as i32, 1, 1);
                    }

                    keyboard.buttons.borrow_mut().push(button.clone());

                    let keyboard = keyboard.clone();
                    let label = (*key).to_string();
                    button.connect_clicked(move |_| keyboard.on_key_pressed(&label));
                }
            }

            let frame = gtk::Frame::new(None);
            frame.set_child(Some(&grid));
            revealer.set_child(Some(&frame));

            *slot.borrow_mut() = Some(keyboard.clone());
            keyboard
        })
    }

    fn attach_to_window(&self, widget: &gtk::Widget) {
        let Some(root) = widget.root() else {
            return;
        };
        if self.revealer.parent().is_some() {
            return;
        }
        let root_widget = root.upcast_ref::<gtk::Widget>();
        if let Some(overlay) = find_window_overlay(root_widget) {
            overlay.add_overlay(&self.revealer);
        }
    }

    fn show_for(&self, widget: &gtk::Widget) {
        self.attach_to_window(widget);
        *self.target.borrow_mut() = Some(widget.clone());
        widget.grab_focus();
        *self.focused_index.borrow_mut() = 0;
        self.apply_key_focus();
        self.revealer.set_reveal_child(true);
    }

    fn hide(&self) {
        self.clear_key_focus();
        self.revealer.set_reveal_child(false);
        *self.target.borrow_mut() = None;
        *self.shifted.borrow_mut() = false;
    }

    fn handle_input(&self, action: InputAction) -> bool {
        match action {
            InputAction::NavigateLeft => self.move_focus(-1, 0),
            InputAction::NavigateRight => self.move_focus(1, 0),
            InputAction::NavigateUp => self.move_focus(0, -1),
            InputAction::NavigateDown => self.move_focus(0, 1),
            InputAction::Activate => {
                self.activate_focused_key();
                true
            }
            InputAction::Back => {
                self.hide();
                true
            }
            _ => false,
        }
    }

    fn move_focus(&self, delta_col: i32, delta_row: i32) -> bool {
        let buttons = self.buttons.borrow();
        if buttons.is_empty() {
            return false;
        }
        let (row, col) = self.index_to_row_col(*self.focused_index.borrow());
        let (next_row, next_col) = if delta_row != 0 {
            let target_row =
                (row as i32 + delta_row).clamp(0, ROW_LENGTHS.len() as i32 - 1) as usize;
            let max_col = ROW_LENGTHS[target_row].saturating_sub(1) as i32;
            let target_col = col.min(max_col as usize);
            (target_row, target_col)
        } else {
            let max_col = ROW_LENGTHS[row].saturating_sub(1) as i32;
            let target_col = (col as i32 + delta_col).clamp(0, max_col) as usize;
            (row, target_col)
        };
        let next_index = self.row_col_to_index(next_row, next_col);
        *self.focused_index.borrow_mut() = next_index;
        self.apply_key_focus();
        true
    }

    fn row_col_to_index(&self, row: usize, col: usize) -> usize {
        ROW_LENGTHS.iter().take(row).sum::<usize>() + col.min(ROW_LENGTHS[row].saturating_sub(1))
    }

    fn index_to_row_col(&self, index: usize) -> (usize, usize) {
        let mut remaining = index;
        for (row, len) in ROW_LENGTHS.iter().enumerate() {
            if remaining < *len {
                return (row, remaining);
            }
            remaining -= len;
        }
        (
            ROW_LENGTHS.len() - 1,
            ROW_LENGTHS.last().copied().unwrap_or(1) - 1,
        )
    }

    fn apply_key_focus(&self) {
        let buttons = self.buttons.borrow();
        let index = *self.focused_index.borrow();
        for (idx, button) in buttons.iter().enumerate() {
            set_tv_focused(button, idx == index);
        }
        if let Some(button) = buttons.get(index) {
            button.grab_focus();
        }
    }

    fn clear_key_focus(&self) {
        for button in self.buttons.borrow().iter() {
            set_tv_focused(button, false);
        }
    }

    fn activate_focused_key(&self) {
        let buttons = self.buttons.borrow();
        let index = *self.focused_index.borrow();
        if let Some(button) = buttons.get(index) {
            button.emit_clicked();
        }
    }

    fn on_key_pressed(&self, key: &str) {
        let Some(target) = self.target.borrow().clone() else {
            return;
        };

        match key {
            "✕" => self.hide(),
            "⇧" => {
                let mut shifted = self.shifted.borrow_mut();
                *shifted = !*shifted;
            }
            "⌫" => {
                if let Some(editable) = editable_from_widget(&target) {
                    let start = editable.selection_bound();
                    let end = editable.position();
                    if start != end {
                        editable.delete_selection();
                    } else if start > 0 {
                        editable.delete_text(start - 1, start);
                    }
                }
            }
            "␣" => insert_text(&target, " ", *self.shifted.borrow()),
            "↵" => {
                if let Ok(entry) = target.clone().downcast::<gtk::Entry>() {
                    entry.activate();
                } else if let Ok(entry) = target.clone().downcast::<gtk::SearchEntry>() {
                    entry.activate();
                } else if let Ok(row) = target.clone().downcast::<adw::EntryRow>() {
                    row.activate();
                } else if let Ok(row) = target.clone().downcast::<adw::PasswordEntryRow>() {
                    row.activate();
                }
                self.hide();
            }
            _ => insert_text(&target, key, *self.shifted.borrow()),
        }
    }
}

fn editable_from_widget(widget: &gtk::Widget) -> Option<gtk::Editable> {
    widget.clone().downcast::<gtk::Editable>().ok()
}

fn insert_text(widget: &gtk::Widget, text: &str, shifted: bool) {
    let Some(editable) = editable_from_widget(widget) else {
        return;
    };
    let text = if shifted {
        text.to_uppercase()
    } else {
        text.to_string()
    };
    let mut pos = editable.position();
    editable.insert_text(&text, &mut pos);
    if shifted && let Some(osk) = OSK.with(|slot| slot.borrow().clone()) {
        *osk.shifted.borrow_mut() = false;
    }
}

fn show_keyboard_for(widget: &gtk::Widget) {
    if !should_offer_osk() {
        return;
    }
    if now_ms().saturating_sub(LAST_DIRECT_MS.load(std::sync::atomic::Ordering::Relaxed)) < 500 {
        return;
    }
    OnScreenKeyboard::ensure().show_for(widget);
}

/// Show the on-screen keyboard for a text entry (e.g. after controller Activate).
pub fn show_for_widget(widget: &impl IsA<gtk::Widget>) {
    show_keyboard_for(widget.upcast_ref());
}

pub fn hide_keyboard() {
    OSK.with(|slot| {
        if let Some(osk) = slot.borrow().as_ref() {
            osk.hide();
        }
    });
}

fn find_window_overlay(root: &gtk::Widget) -> Option<gtk::Overlay> {
    let mut stack = vec![root.clone()];
    while let Some(widget) = stack.pop() {
        if let Some(overlay) = widget.downcast_ref::<gtk::Overlay>() {
            return Some(overlay.clone());
        }
        let mut child = widget.first_child();
        while let Some(next) = child {
            stack.push(next.clone());
            child = next.next_sibling();
        }
    }
    None
}

pub fn attach_on_screen_keyboard(widget: &impl IsA<gtk::Widget>) {
    let widget = widget.upcast_ref::<gtk::Widget>().clone();

    let focus = gtk::EventControllerFocus::new();
    focus.connect_enter({
        let widget = widget.clone();
        move |_| show_keyboard_for(&widget)
    });
    focus.connect_leave({
        let widget = widget.clone();
        move |_| {
            let hide = OSK.with(|slot| {
                slot.borrow()
                    .as_ref()
                    .and_then(|osk| osk.target.borrow().clone())
                    .is_some_and(|target| target == widget)
            });
            if hide {
                hide_keyboard();
            }
        }
    });
    widget.add_controller(focus);

    let click = gtk::GestureClick::new();
    click.connect_pressed({
        let widget = widget.clone();
        move |_, _, _, _| show_keyboard_for(&widget)
    });
    widget.add_controller(click);
}
