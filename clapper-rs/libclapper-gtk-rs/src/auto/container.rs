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
    #[doc(alias = "ClapperGtkContainer")]
    pub struct Container(Object<ffi::ClapperGtkContainer, ffi::ClapperGtkContainerClass>) @extends gtk::Widget, @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;

    match fn {
        type_ => || ffi::clapper_gtk_container_get_type(),
    }
}

impl Container {
    pub const NONE: Option<&'static Container> = None;

    #[doc(alias = "clapper_gtk_container_new")]
    pub fn new() -> Container {
        assert_initialized_main_thread!();
        unsafe { gtk::Widget::from_glib_none(ffi::clapper_gtk_container_new()).unsafe_cast() }
    }

    // rustdoc-stripper-ignore-next
    /// Creates a new builder-pattern struct instance to construct [`Container`] objects.
    ///
    /// This method returns an instance of [`ContainerBuilder`](crate::builders::ContainerBuilder) which can be used to create [`Container`] objects.
    pub fn builder() -> ContainerBuilder {
        ContainerBuilder::new()
    }
}

impl Default for Container {
    fn default() -> Self {
        Self::new()
    }
}

// rustdoc-stripper-ignore-next
/// A [builder-pattern] type to construct [`Container`] objects.
///
/// [builder-pattern]: https://doc.rust-lang.org/1.0.0/style/ownership/builders.html
#[must_use = "The builder must be built to be used"]
pub struct ContainerBuilder {
    builder: glib::object::ObjectBuilder<'static, Container>,
}

impl ContainerBuilder {
    fn new() -> Self {
        Self {
            builder: glib::object::Object::builder(),
        }
    }

    pub fn adaptive_height(self, adaptive_height: i32) -> Self {
        Self {
            builder: self.builder.property("adaptive-height", adaptive_height),
        }
    }

    pub fn adaptive_width(self, adaptive_width: i32) -> Self {
        Self {
            builder: self.builder.property("adaptive-width", adaptive_width),
        }
    }

    pub fn height_target(self, height_target: i32) -> Self {
        Self {
            builder: self.builder.property("height-target", height_target),
        }
    }

    pub fn width_target(self, width_target: i32) -> Self {
        Self {
            builder: self.builder.property("width-target", width_target),
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
    /// Build the [`Container`].
    #[must_use = "Building the object from the builder is usually expensive and is not expected to have side effects"]
    pub fn build(self) -> Container {
        self.builder.build()
    }
}

#[allow(dead_code)]
mod sealed {
    pub trait Sealed {}
    impl<T: super::IsA<super::Container>> Sealed for T {}
}

#[allow(dead_code)]
pub trait ContainerExt: IsA<Container> + sealed::Sealed + 'static {
    #[doc(alias = "clapper_gtk_container_get_adaptive_height")]
    #[doc(alias = "get_adaptive_height")]
    #[doc(alias = "adaptive-height")]
    fn adaptive_height(&self) -> i32 {
        unsafe { ffi::clapper_gtk_container_get_adaptive_height(self.as_ref().to_glib_none().0) }
    }

    #[doc(alias = "clapper_gtk_container_get_adaptive_width")]
    #[doc(alias = "get_adaptive_width")]
    #[doc(alias = "adaptive-width")]
    fn adaptive_width(&self) -> i32 {
        unsafe { ffi::clapper_gtk_container_get_adaptive_width(self.as_ref().to_glib_none().0) }
    }

    #[doc(alias = "clapper_gtk_container_get_child")]
    #[doc(alias = "get_child")]
    fn child(&self) -> Option<gtk::Widget> {
        unsafe {
            from_glib_none(ffi::clapper_gtk_container_get_child(
                self.as_ref().to_glib_none().0,
            ))
        }
    }

    #[doc(alias = "clapper_gtk_container_get_height_target")]
    #[doc(alias = "get_height_target")]
    #[doc(alias = "height-target")]
    fn height_target(&self) -> i32 {
        unsafe { ffi::clapper_gtk_container_get_height_target(self.as_ref().to_glib_none().0) }
    }

    #[doc(alias = "clapper_gtk_container_get_width_target")]
    #[doc(alias = "get_width_target")]
    #[doc(alias = "width-target")]
    fn width_target(&self) -> i32 {
        unsafe { ffi::clapper_gtk_container_get_width_target(self.as_ref().to_glib_none().0) }
    }

    #[doc(alias = "clapper_gtk_container_set_adaptive_height")]
    #[doc(alias = "adaptive-height")]
    fn set_adaptive_height(&self, height: i32) {
        unsafe {
            ffi::clapper_gtk_container_set_adaptive_height(self.as_ref().to_glib_none().0, height);
        }
    }

    #[doc(alias = "clapper_gtk_container_set_adaptive_width")]
    #[doc(alias = "adaptive-width")]
    fn set_adaptive_width(&self, width: i32) {
        unsafe {
            ffi::clapper_gtk_container_set_adaptive_width(self.as_ref().to_glib_none().0, width);
        }
    }

    #[doc(alias = "clapper_gtk_container_set_child")]
    fn set_child(&self, child: &impl IsA<gtk::Widget>) {
        unsafe {
            ffi::clapper_gtk_container_set_child(
                self.as_ref().to_glib_none().0,
                child.as_ref().to_glib_none().0,
            );
        }
    }

    #[doc(alias = "clapper_gtk_container_set_height_target")]
    #[doc(alias = "height-target")]
    fn set_height_target(&self, height: i32) {
        unsafe {
            ffi::clapper_gtk_container_set_height_target(self.as_ref().to_glib_none().0, height);
        }
    }

    #[doc(alias = "clapper_gtk_container_set_width_target")]
    #[doc(alias = "width-target")]
    fn set_width_target(&self, width: i32) {
        unsafe {
            ffi::clapper_gtk_container_set_width_target(self.as_ref().to_glib_none().0, width);
        }
    }

    #[doc(alias = "adapt")]
    fn connect_adapt<F: Fn(&Self, bool) + 'static>(&self, f: F) -> SignalHandlerId {
        unsafe extern "C" fn adapt_trampoline<P: IsA<Container>, F: Fn(&P, bool) + 'static>(
            this: *mut ffi::ClapperGtkContainer,
            adapt: glib::ffi::gboolean,
            f: glib::ffi::gpointer,
        ) {
            let f: &F = &*(f as *const F);
            f(
                Container::from_glib_borrow(this).unsafe_cast_ref(),
                from_glib(adapt),
            )
        }
        unsafe {
            let f: Box_<F> = Box_::new(f);
            connect_raw(
                self.as_ptr() as *mut _,
                b"adapt\0".as_ptr() as *const _,
                Some(std::mem::transmute::<*const (), unsafe extern "C" fn()>(
                    adapt_trampoline::<Self, F> as *const (),
                )),
                Box_::into_raw(f),
            )
        }
    }

    #[doc(alias = "adaptive-height")]
    fn connect_adaptive_height_notify<F: Fn(&Self) + 'static>(&self, f: F) -> SignalHandlerId {
        unsafe extern "C" fn notify_adaptive_height_trampoline<
            P: IsA<Container>,
            F: Fn(&P) + 'static,
        >(
            this: *mut ffi::ClapperGtkContainer,
            _param_spec: glib::ffi::gpointer,
            f: glib::ffi::gpointer,
        ) {
            let f: &F = &*(f as *const F);
            f(Container::from_glib_borrow(this).unsafe_cast_ref())
        }
        unsafe {
            let f: Box_<F> = Box_::new(f);
            connect_raw(
                self.as_ptr() as *mut _,
                b"notify::adaptive-height\0".as_ptr() as *const _,
                Some(std::mem::transmute::<*const (), unsafe extern "C" fn()>(
                    notify_adaptive_height_trampoline::<Self, F> as *const (),
                )),
                Box_::into_raw(f),
            )
        }
    }

    #[doc(alias = "adaptive-width")]
    fn connect_adaptive_width_notify<F: Fn(&Self) + 'static>(&self, f: F) -> SignalHandlerId {
        unsafe extern "C" fn notify_adaptive_width_trampoline<
            P: IsA<Container>,
            F: Fn(&P) + 'static,
        >(
            this: *mut ffi::ClapperGtkContainer,
            _param_spec: glib::ffi::gpointer,
            f: glib::ffi::gpointer,
        ) {
            let f: &F = &*(f as *const F);
            f(Container::from_glib_borrow(this).unsafe_cast_ref())
        }
        unsafe {
            let f: Box_<F> = Box_::new(f);
            connect_raw(
                self.as_ptr() as *mut _,
                b"notify::adaptive-width\0".as_ptr() as *const _,
                Some(std::mem::transmute::<*const (), unsafe extern "C" fn()>(
                    notify_adaptive_width_trampoline::<Self, F> as *const (),
                )),
                Box_::into_raw(f),
            )
        }
    }

    #[doc(alias = "height-target")]
    fn connect_height_target_notify<F: Fn(&Self) + 'static>(&self, f: F) -> SignalHandlerId {
        unsafe extern "C" fn notify_height_target_trampoline<
            P: IsA<Container>,
            F: Fn(&P) + 'static,
        >(
            this: *mut ffi::ClapperGtkContainer,
            _param_spec: glib::ffi::gpointer,
            f: glib::ffi::gpointer,
        ) {
            let f: &F = &*(f as *const F);
            f(Container::from_glib_borrow(this).unsafe_cast_ref())
        }
        unsafe {
            let f: Box_<F> = Box_::new(f);
            connect_raw(
                self.as_ptr() as *mut _,
                b"notify::height-target\0".as_ptr() as *const _,
                Some(std::mem::transmute::<*const (), unsafe extern "C" fn()>(
                    notify_height_target_trampoline::<Self, F> as *const (),
                )),
                Box_::into_raw(f),
            )
        }
    }

    #[doc(alias = "width-target")]
    fn connect_width_target_notify<F: Fn(&Self) + 'static>(&self, f: F) -> SignalHandlerId {
        unsafe extern "C" fn notify_width_target_trampoline<
            P: IsA<Container>,
            F: Fn(&P) + 'static,
        >(
            this: *mut ffi::ClapperGtkContainer,
            _param_spec: glib::ffi::gpointer,
            f: glib::ffi::gpointer,
        ) {
            let f: &F = &*(f as *const F);
            f(Container::from_glib_borrow(this).unsafe_cast_ref())
        }
        unsafe {
            let f: Box_<F> = Box_::new(f);
            connect_raw(
                self.as_ptr() as *mut _,
                b"notify::width-target\0".as_ptr() as *const _,
                Some(std::mem::transmute::<*const (), unsafe extern "C" fn()>(
                    notify_width_target_trampoline::<Self, F> as *const (),
                )),
                Box_::into_raw(f),
            )
        }
    }
}

impl<O: IsA<Container>> ContainerExt for O {}
