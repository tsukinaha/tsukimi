use adw::subclass::prelude::*;
use gtk::{glib, CompositeTemplate};

mod imp {
    use super::*;
    use glib::subclass::InitializingObject;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/moe/tsukimi/mpv_control_sidebar.ui")]
    pub struct MPVControlSidebar {
        
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MPVControlSidebar {
        const NAME: &'static str = "MPVControlSidebar";
        type Type = super::MPVControlSidebar;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MPVControlSidebar {}

    impl WidgetImpl for MPVControlSidebar {}
    impl NavigationPageImpl for MPVControlSidebar {}
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

impl MPVControlSidebar {
    pub fn new() -> Self {
        glib::Object::new()
    }
}
