use adw::subclass::prelude::*;
use gtk::{glib, template_callbacks, CompositeTemplate};
use libmpv2::SetData;

mod imp {
    use crate::ui::mpv::mpvglarea::MPVGLArea;
    use gtk::{glib, prelude::*, subclass::prelude::*};
    use super::*;
    use glib::subclass::InitializingObject;

    #[derive(Debug, Default, CompositeTemplate, glib::Properties)]
    #[template(resource = "/moe/tsukimi/mpv_control_sidebar.ui")]
    #[properties(wrapper_type = super::MPVControlSidebar)]
    pub struct MPVControlSidebar {
        #[property(get, set = Self::set_player, explicit_notify, nullable)]
        pub player: glib::WeakRef<MPVGLArea>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MPVControlSidebar {
        const NAME: &'static str = "MPVControlSidebar";
        type Type = super::MPVControlSidebar;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for MPVControlSidebar {}

    impl WidgetImpl for MPVControlSidebar {}
    impl NavigationPageImpl for MPVControlSidebar {}

    impl MPVControlSidebar {
        fn set_player(&self, player: Option<MPVGLArea>) {
            if self.player.upgrade() == player {
                return;
            }
            self.player.set(player.as_ref());
        }
    }
}

glib::wrapper! {
    pub struct MPVControlSidebar(ObjectSubclass<imp::MPVControlSidebar>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::ActionRow, adw::PreferencesRow, @implements gtk::Accessible;
}

impl Default for MPVControlSidebar {
    fn default() -> Self {
        Self::new()
    }
}

#[template_callbacks]
impl MPVControlSidebar {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_mpv_property<V>(&self, property: &str, value: V)
    where
        V: SetData,
    {
        self.player().map(|player| player.set_property(property, value));
    }

    #[template_callback]
    pub fn on_brightness_spinrow(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.set_mpv_property("brightness", spin.value());
    }

    #[template_callback]
    pub fn on_contrast_spinrow(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.set_mpv_property("contrast", spin.value());
    }

    #[template_callback]
    pub fn on_gamma_spinrow(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.set_mpv_property("gamma", spin.value());
    }

    #[template_callback]
    pub fn on_hue_spinrow(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.set_mpv_property("hue", spin.value());
    }

    #[template_callback]
    pub fn on_saturation_spinrow(&self, _param: glib::ParamSpec, spin: adw::SpinRow) {
        self.set_mpv_property("saturation", spin.value());
    }
}
