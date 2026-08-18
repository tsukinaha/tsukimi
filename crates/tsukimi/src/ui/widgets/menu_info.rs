use std::cell::RefCell;

use adw::subclass::prelude::*;
use gtk::{
    CompositeTemplate,
    glib::{
        self,
        Properties,
    },
};

mod imp {

    use glib::subclass::InitializingObject;
    use gtk::prelude::*;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate, Properties)]
    #[properties(wrapper_type = super::MenuInfo)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/right_menu_info.ui")]
    pub struct MenuInfo {
        #[property(get, set)]
        pub title: RefCell<String>,
        #[property(get, set = Self::set_subtitle)]
        pub subtitle: RefCell<String>,
        #[property(get, set = Self::set_rating)]
        pub rating: RefCell<String>,

        #[template_child]
        pub subtitle_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub star_box: TemplateChild<gtk::Box>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MenuInfo {
        const NAME: &'static str = "MenuInfo";
        type Type = super::MenuInfo;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for MenuInfo {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for MenuInfo {}

    impl BinImpl for MenuInfo {}

    impl MenuInfo {
        pub fn set_subtitle(&self, subtitle: String) {
            if subtitle.is_empty() {
                return;
            }

            self.subtitle_label.set_visible(true);
            self.subtitle.replace(subtitle);
        }

        pub fn set_rating(&self, rating: String) {
            if rating.is_empty() {
                return;
            }

            self.star_box.set_visible(true);
            self.rating.replace(rating);
        }
    }
}

glib::wrapper! {
    /// A widget displaying a `MenuInfo`.
    pub struct MenuInfo(ObjectSubclass<imp::MenuInfo>)
        @extends gtk::Widget, adw::Bin, @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl MenuInfo {
    pub fn new() -> Self {
        glib::Object::new()
    }
}

impl Default for MenuInfo {
    fn default() -> Self {
        Self::new()
    }
}
