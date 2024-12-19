use adw::subclass::prelude::*;
use gtk::{
    glib,
    CompositeTemplate,
};

mod imp {

    use glib::subclass::InitializingObject;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/theme_switcher.ui")]
    pub struct ThemeSwitcher {}

    #[glib::object_subclass]
    impl ObjectSubclass for ThemeSwitcher {
        const NAME: &'static str = "ThemeSwitcher";
        type Type = super::ThemeSwitcher;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ThemeSwitcher {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for ThemeSwitcher {}

    impl BinImpl for ThemeSwitcher {}
}

glib::wrapper! {
    /// A widget displaying a `ThemeSwitcher`.
    pub struct ThemeSwitcher(ObjectSubclass<imp::ThemeSwitcher>)
        @extends gtk::Widget, adw::Bin, @implements gtk::Accessible;
}

impl ThemeSwitcher {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn theme_selected(&self) {
        let style_provider = adw::StyleManager::default();
        
    }
}

impl Default for ThemeSwitcher {
    fn default() -> Self {
        Self::new()
    }
}
