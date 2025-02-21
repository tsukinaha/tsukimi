use adw::subclass::prelude::*;
use gtk::{
    CompositeTemplate,
    glib,
};

mod imp {

    use glib::subclass::InitializingObject;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/mpv_menu_actions.ui")]
    pub struct MenuActions {
        #[template_child]
        pub play_pause_button: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MenuActions {
        const NAME: &'static str = "MenuActions";
        type Type = super::MenuActions;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MenuActions {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for MenuActions {}

    impl BinImpl for MenuActions {}
}

glib::wrapper! {
    /// A widget displaying a `MenuActions`.
    pub struct MenuActions(ObjectSubclass<imp::MenuActions>)
        @extends gtk::Widget, adw::Bin, @implements gtk::Accessible;
}

impl MenuActions {
    pub fn new() -> Self {
        glib::Object::new()
    }
}

impl Default for MenuActions {
    fn default() -> Self {
        Self::new()
    }
}
