// This file was generated by gir (https://github.com/gtk-rs/gir)
// from
// from ..\gir-files-gstreamer
// from ..\gir-files-gtk
// from ..\libclapper-rs
// DO NOT EDIT

use glib::{prelude::*, translate::*};

glib::wrapper! {
    #[doc(alias = "ClapperGtkPreviousItemButton")]
    pub struct PreviousItemButton(Object<ffi::ClapperGtkPreviousItemButton, ffi::ClapperGtkPreviousItemButtonClass>) @extends gtk::Button, gtk::Widget, @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Actionable;

    match fn {
        type_ => || ffi::clapper_gtk_previous_item_button_get_type(),
    }
}

impl PreviousItemButton {
    #[doc(alias = "clapper_gtk_previous_item_button_new")]
    pub fn new() -> PreviousItemButton {
        assert_initialized_main_thread!();
        unsafe {
            gtk::Widget::from_glib_none(ffi::clapper_gtk_previous_item_button_new()).unsafe_cast()
        }
    }
}

impl Default for PreviousItemButton {
    fn default() -> Self {
        Self::new()
    }
}
