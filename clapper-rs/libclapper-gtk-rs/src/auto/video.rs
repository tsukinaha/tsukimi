// This file was generated by gir (https://github.com/gtk-rs/gir)
// from
// from ..\gir-files-gstreamer
// from ..\gir-files-gtk
// from ..\libclapper-rs
// DO NOT EDIT

use glib::{
    prelude::*,
    signal::{connect_raw, SignalHandlerId},
    translate::*,
};
use std::boxed::Box as Box_;

glib::wrapper! {
    #[doc(alias = "ClapperGtkVideo")]
    pub struct Video(Object<ffi::ClapperGtkVideo, ffi::ClapperGtkVideoClass>) @extends gtk::Widget, @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;

    match fn {
        type_ => || ffi::clapper_gtk_video_get_type(),
    }
}

impl Video {
    #[doc(alias = "clapper_gtk_video_new")]
    pub fn new() -> Video {
        assert_initialized_main_thread!();
        unsafe { gtk::Widget::from_glib_none(ffi::clapper_gtk_video_new()).unsafe_cast() }
    }

    // rustdoc-stripper-ignore-next
    /// Creates a new builder-pattern struct instance to construct [`Video`] objects.
    ///
    /// This method returns an instance of [`VideoBuilder`](crate::builders::VideoBuilder) which can be used to create [`Video`] objects.
    pub fn builder() -> VideoBuilder {
        VideoBuilder::new()
    }

    #[doc(alias = "clapper_gtk_video_add_fading_overlay")]
    pub fn add_fading_overlay(&self, widget: &impl IsA<gtk::Widget>) {
        unsafe {
            ffi::clapper_gtk_video_add_fading_overlay(
                self.to_glib_none().0,
                widget.as_ref().to_glib_none().0,
            );
        }
    }

    #[doc(alias = "clapper_gtk_video_add_overlay")]
    pub fn add_overlay(&self, widget: &impl IsA<gtk::Widget>) {
        unsafe {
            ffi::clapper_gtk_video_add_overlay(
                self.to_glib_none().0,
                widget.as_ref().to_glib_none().0,
            );
        }
    }

    #[doc(alias = "clapper_gtk_video_get_auto_inhibit")]
    #[doc(alias = "get_auto_inhibit")]
    #[doc(alias = "auto-inhibit")]
    pub fn is_auto_inhibit(&self) -> bool {
        unsafe {
            from_glib(ffi::clapper_gtk_video_get_auto_inhibit(
                self.to_glib_none().0,
            ))
        }
    }

    #[doc(alias = "clapper_gtk_video_get_fade_delay")]
    #[doc(alias = "get_fade_delay")]
    #[doc(alias = "fade-delay")]
    pub fn fade_delay(&self) -> u32 {
        unsafe { ffi::clapper_gtk_video_get_fade_delay(self.to_glib_none().0) }
    }

    #[doc(alias = "clapper_gtk_video_get_inhibited")]
    #[doc(alias = "get_inhibited")]
    #[doc(alias = "inhibited")]
    pub fn is_inhibited(&self) -> bool {
        unsafe { from_glib(ffi::clapper_gtk_video_get_inhibited(self.to_glib_none().0)) }
    }

    #[doc(alias = "clapper_gtk_video_get_player")]
    #[doc(alias = "get_player")]
    pub fn player(&self) -> Option<clapper::Player> {
        unsafe { from_glib_none(ffi::clapper_gtk_video_get_player(self.to_glib_none().0)) }
    }

    #[doc(alias = "clapper_gtk_video_get_touch_fade_delay")]
    #[doc(alias = "get_touch_fade_delay")]
    #[doc(alias = "touch-fade-delay")]
    pub fn touch_fade_delay(&self) -> u32 {
        unsafe { ffi::clapper_gtk_video_get_touch_fade_delay(self.to_glib_none().0) }
    }

    #[doc(alias = "clapper_gtk_video_set_auto_inhibit")]
    #[doc(alias = "auto-inhibit")]
    pub fn set_auto_inhibit(&self, inhibit: bool) {
        unsafe {
            ffi::clapper_gtk_video_set_auto_inhibit(self.to_glib_none().0, inhibit.into_glib());
        }
    }

    #[doc(alias = "clapper_gtk_video_set_fade_delay")]
    pub fn set_fade_delay(&self, delay: u32) {
        unsafe {
            ffi::clapper_gtk_video_set_fade_delay(self.to_glib_none().0, delay);
        }
    }

    #[doc(alias = "clapper_gtk_video_set_touch_fade_delay")]
    pub fn set_touch_fade_delay(&self, delay: u32) {
        unsafe {
            ffi::clapper_gtk_video_set_touch_fade_delay(self.to_glib_none().0, delay);
        }
    }

    #[doc(alias = "seek-request")]
    pub fn connect_seek_request<F: Fn(&Self, bool) + 'static>(&self, f: F) -> SignalHandlerId {
        unsafe extern "C" fn seek_request_trampoline<F: Fn(&Video, bool) + 'static>(
            this: *mut ffi::ClapperGtkVideo,
            forward: glib::ffi::gboolean,
            f: glib::ffi::gpointer,
        ) {
            let f: &F = &*(f as *const F);
            f(&from_glib_borrow(this), from_glib(forward))
        }
        unsafe {
            let f: Box_<F> = Box_::new(f);
            connect_raw(
                self.as_ptr() as *mut _,
                b"seek-request\0".as_ptr() as *const _,
                Some(std::mem::transmute::<*const (), unsafe extern "C" fn()>(
                    seek_request_trampoline::<F> as *const (),
                )),
                Box_::into_raw(f),
            )
        }
    }

    #[doc(alias = "toggle-fullscreen")]
    pub fn connect_toggle_fullscreen<F: Fn(&Self) + 'static>(&self, f: F) -> SignalHandlerId {
        unsafe extern "C" fn toggle_fullscreen_trampoline<F: Fn(&Video) + 'static>(
            this: *mut ffi::ClapperGtkVideo,
            f: glib::ffi::gpointer,
        ) {
            let f: &F = &*(f as *const F);
            f(&from_glib_borrow(this))
        }
        unsafe {
            let f: Box_<F> = Box_::new(f);
            connect_raw(
                self.as_ptr() as *mut _,
                b"toggle-fullscreen\0".as_ptr() as *const _,
                Some(std::mem::transmute::<*const (), unsafe extern "C" fn()>(
                    toggle_fullscreen_trampoline::<F> as *const (),
                )),
                Box_::into_raw(f),
            )
        }
    }

    #[doc(alias = "auto-inhibit")]
    pub fn connect_auto_inhibit_notify<F: Fn(&Self) + 'static>(&self, f: F) -> SignalHandlerId {
        unsafe extern "C" fn notify_auto_inhibit_trampoline<F: Fn(&Video) + 'static>(
            this: *mut ffi::ClapperGtkVideo,
            _param_spec: glib::ffi::gpointer,
            f: glib::ffi::gpointer,
        ) {
            let f: &F = &*(f as *const F);
            f(&from_glib_borrow(this))
        }
        unsafe {
            let f: Box_<F> = Box_::new(f);
            connect_raw(
                self.as_ptr() as *mut _,
                b"notify::auto-inhibit\0".as_ptr() as *const _,
                Some(std::mem::transmute::<*const (), unsafe extern "C" fn()>(
                    notify_auto_inhibit_trampoline::<F> as *const (),
                )),
                Box_::into_raw(f),
            )
        }
    }

    #[doc(alias = "fade-delay")]
    pub fn connect_fade_delay_notify<F: Fn(&Self) + 'static>(&self, f: F) -> SignalHandlerId {
        unsafe extern "C" fn notify_fade_delay_trampoline<F: Fn(&Video) + 'static>(
            this: *mut ffi::ClapperGtkVideo,
            _param_spec: glib::ffi::gpointer,
            f: glib::ffi::gpointer,
        ) {
            let f: &F = &*(f as *const F);
            f(&from_glib_borrow(this))
        }
        unsafe {
            let f: Box_<F> = Box_::new(f);
            connect_raw(
                self.as_ptr() as *mut _,
                b"notify::fade-delay\0".as_ptr() as *const _,
                Some(std::mem::transmute::<*const (), unsafe extern "C" fn()>(
                    notify_fade_delay_trampoline::<F> as *const (),
                )),
                Box_::into_raw(f),
            )
        }
    }

    #[doc(alias = "inhibited")]
    pub fn connect_inhibited_notify<F: Fn(&Self) + 'static>(&self, f: F) -> SignalHandlerId {
        unsafe extern "C" fn notify_inhibited_trampoline<F: Fn(&Video) + 'static>(
            this: *mut ffi::ClapperGtkVideo,
            _param_spec: glib::ffi::gpointer,
            f: glib::ffi::gpointer,
        ) {
            let f: &F = &*(f as *const F);
            f(&from_glib_borrow(this))
        }
        unsafe {
            let f: Box_<F> = Box_::new(f);
            connect_raw(
                self.as_ptr() as *mut _,
                b"notify::inhibited\0".as_ptr() as *const _,
                Some(std::mem::transmute::<*const (), unsafe extern "C" fn()>(
                    notify_inhibited_trampoline::<F> as *const (),
                )),
                Box_::into_raw(f),
            )
        }
    }

    #[doc(alias = "player")]
    pub fn connect_player_notify<F: Fn(&Self) + 'static>(&self, f: F) -> SignalHandlerId {
        unsafe extern "C" fn notify_player_trampoline<F: Fn(&Video) + 'static>(
            this: *mut ffi::ClapperGtkVideo,
            _param_spec: glib::ffi::gpointer,
            f: glib::ffi::gpointer,
        ) {
            let f: &F = &*(f as *const F);
            f(&from_glib_borrow(this))
        }
        unsafe {
            let f: Box_<F> = Box_::new(f);
            connect_raw(
                self.as_ptr() as *mut _,
                b"notify::player\0".as_ptr() as *const _,
                Some(std::mem::transmute::<*const (), unsafe extern "C" fn()>(
                    notify_player_trampoline::<F> as *const (),
                )),
                Box_::into_raw(f),
            )
        }
    }

    #[doc(alias = "touch-fade-delay")]
    pub fn connect_touch_fade_delay_notify<F: Fn(&Self) + 'static>(&self, f: F) -> SignalHandlerId {
        unsafe extern "C" fn notify_touch_fade_delay_trampoline<F: Fn(&Video) + 'static>(
            this: *mut ffi::ClapperGtkVideo,
            _param_spec: glib::ffi::gpointer,
            f: glib::ffi::gpointer,
        ) {
            let f: &F = &*(f as *const F);
            f(&from_glib_borrow(this))
        }
        unsafe {
            let f: Box_<F> = Box_::new(f);
            connect_raw(
                self.as_ptr() as *mut _,
                b"notify::touch-fade-delay\0".as_ptr() as *const _,
                Some(std::mem::transmute::<*const (), unsafe extern "C" fn()>(
                    notify_touch_fade_delay_trampoline::<F> as *const (),
                )),
                Box_::into_raw(f),
            )
        }
    }
}

impl Default for Video {
    fn default() -> Self {
        Self::new()
    }
}

// rustdoc-stripper-ignore-next
/// A [builder-pattern] type to construct [`Video`] objects.
///
/// [builder-pattern]: https://doc.rust-lang.org/1.0.0/style/ownership/builders.html
#[must_use = "The builder must be built to be used"]
pub struct VideoBuilder {
    builder: glib::object::ObjectBuilder<'static, Video>,
}

impl VideoBuilder {
    fn new() -> Self {
        Self {
            builder: glib::object::Object::builder(),
        }
    }

    pub fn auto_inhibit(self, auto_inhibit: bool) -> Self {
        Self {
            builder: self.builder.property("auto-inhibit", auto_inhibit),
        }
    }

    pub fn can_focus(self, can_focus: bool) -> Self {
        Self {
            builder: self.builder.property("can-focus", can_focus),
        }
    }

    pub fn can_target(self, can_target: bool) -> Self {
        Self {
            builder: self.builder.property("can-target", can_target),
        }
    }

    pub fn css_classes(self, css_classes: impl Into<glib::StrV>) -> Self {
        Self {
            builder: self.builder.property("css-classes", css_classes.into()),
        }
    }

    pub fn css_name(self, css_name: impl Into<glib::GString>) -> Self {
        Self {
            builder: self.builder.property("css-name", css_name.into()),
        }
    }

    pub fn cursor(self, cursor: &gdk::Cursor) -> Self {
        Self {
            builder: self.builder.property("cursor", cursor.clone()),
        }
    }

    pub fn focus_on_click(self, focus_on_click: bool) -> Self {
        Self {
            builder: self.builder.property("focus-on-click", focus_on_click),
        }
    }

    pub fn focusable(self, focusable: bool) -> Self {
        Self {
            builder: self.builder.property("focusable", focusable),
        }
    }

    pub fn halign(self, halign: gtk::Align) -> Self {
        Self {
            builder: self.builder.property("halign", halign),
        }
    }

    pub fn has_tooltip(self, has_tooltip: bool) -> Self {
        Self {
            builder: self.builder.property("has-tooltip", has_tooltip),
        }
    }

    pub fn height_request(self, height_request: i32) -> Self {
        Self {
            builder: self.builder.property("height-request", height_request),
        }
    }

    pub fn hexpand(self, hexpand: bool) -> Self {
        Self {
            builder: self.builder.property("hexpand", hexpand),
        }
    }

    pub fn hexpand_set(self, hexpand_set: bool) -> Self {
        Self {
            builder: self.builder.property("hexpand-set", hexpand_set),
        }
    }

    pub fn layout_manager(self, layout_manager: &impl IsA<gtk::LayoutManager>) -> Self {
        Self {
            builder: self
                .builder
                .property("layout-manager", layout_manager.clone().upcast()),
        }
    }

    pub fn margin_bottom(self, margin_bottom: i32) -> Self {
        Self {
            builder: self.builder.property("margin-bottom", margin_bottom),
        }
    }

    pub fn margin_end(self, margin_end: i32) -> Self {
        Self {
            builder: self.builder.property("margin-end", margin_end),
        }
    }

    pub fn margin_start(self, margin_start: i32) -> Self {
        Self {
            builder: self.builder.property("margin-start", margin_start),
        }
    }

    pub fn margin_top(self, margin_top: i32) -> Self {
        Self {
            builder: self.builder.property("margin-top", margin_top),
        }
    }

    pub fn name(self, name: impl Into<glib::GString>) -> Self {
        Self {
            builder: self.builder.property("name", name.into()),
        }
    }

    pub fn opacity(self, opacity: f64) -> Self {
        Self {
            builder: self.builder.property("opacity", opacity),
        }
    }

    pub fn overflow(self, overflow: gtk::Overflow) -> Self {
        Self {
            builder: self.builder.property("overflow", overflow),
        }
    }

    pub fn receives_default(self, receives_default: bool) -> Self {
        Self {
            builder: self.builder.property("receives-default", receives_default),
        }
    }

    pub fn sensitive(self, sensitive: bool) -> Self {
        Self {
            builder: self.builder.property("sensitive", sensitive),
        }
    }

    pub fn tooltip_markup(self, tooltip_markup: impl Into<glib::GString>) -> Self {
        Self {
            builder: self
                .builder
                .property("tooltip-markup", tooltip_markup.into()),
        }
    }

    pub fn tooltip_text(self, tooltip_text: impl Into<glib::GString>) -> Self {
        Self {
            builder: self.builder.property("tooltip-text", tooltip_text.into()),
        }
    }

    pub fn valign(self, valign: gtk::Align) -> Self {
        Self {
            builder: self.builder.property("valign", valign),
        }
    }

    pub fn vexpand(self, vexpand: bool) -> Self {
        Self {
            builder: self.builder.property("vexpand", vexpand),
        }
    }

    pub fn vexpand_set(self, vexpand_set: bool) -> Self {
        Self {
            builder: self.builder.property("vexpand-set", vexpand_set),
        }
    }

    pub fn visible(self, visible: bool) -> Self {
        Self {
            builder: self.builder.property("visible", visible),
        }
    }

    pub fn width_request(self, width_request: i32) -> Self {
        Self {
            builder: self.builder.property("width-request", width_request),
        }
    }

    pub fn accessible_role(self, accessible_role: gtk::AccessibleRole) -> Self {
        Self {
            builder: self.builder.property("accessible-role", accessible_role),
        }
    }

    // rustdoc-stripper-ignore-next
    /// Build the [`Video`].
    #[must_use = "Building the object from the builder is usually expensive and is not expected to have side effects"]
    pub fn build(self) -> Video {
        self.builder.build()
    }
}
