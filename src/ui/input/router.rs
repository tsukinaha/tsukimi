use gtk::{
    gdk::Key,
    glib::translate::FromGlib,
};

use super::actions::InputAction;

pub fn key_to_action(keyval: u32) -> Option<InputAction> {
    let key = unsafe { Key::from_glib(keyval) };
    use InputAction::*;
    match key {
        Key::Left | Key::KP_Left => Some(NavigateLeft),
        Key::Right | Key::KP_Right => Some(NavigateRight),
        Key::Up | Key::KP_Up => Some(NavigateUp),
        Key::Down | Key::KP_Down => Some(NavigateDown),
        Key::Return | Key::KP_Enter | Key::space => Some(Activate),
        Key::Escape | Key::BackSpace => Some(Back),
        Key::Home => Some(Home),
        Key::Menu | Key::F10 => Some(Menu),
        Key::question | Key::f | Key::F => Some(Search),
        Key::Page_Up | Key::KP_Page_Up => Some(PageScrollLeft),
        Key::Page_Down | Key::KP_Page_Down => Some(PageScrollRight),
        Key::AudioPlay | Key::AudioPause | Key::Pause => Some(PlayPause),
        k if Key::from_name("Guide") == Some(k) => Some(ToggleHints),
        _ => None,
    }
}
