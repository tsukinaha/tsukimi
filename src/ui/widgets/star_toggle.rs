use gettextrs::gettext;
use gtk::{
    glib,
    prelude::*,
    subclass::prelude::*,
};

pub(crate) mod imp {
    use super::*;

    #[derive(Default)]
    pub struct StarToggle {}

    #[glib::object_subclass]
    impl ObjectSubclass for StarToggle {
        const NAME: &'static str = "StarToggle";
        type Type = super::StarToggle;
        type ParentType = gtk::ToggleButton;
    }

    impl ObjectImpl for StarToggle {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().set_up();
            self.obj().update();
        }
    }
    impl WidgetImpl for StarToggle {}

    impl ToggleButtonImpl for StarToggle {
        fn toggled(&self) {
            self.obj().update();
        }
    }

    impl ButtonImpl for StarToggle {}
}

glib::wrapper! {

    pub struct StarToggle(ObjectSubclass<imp::StarToggle>)
        @extends gtk::Widget, gtk::ToggleButton, gtk::Button;
}

impl Default for StarToggle {
    fn default() -> Self {
        Self::new()
    }
}

impl StarToggle {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn set_up(&self) {
        self.add_css_class("star");
        self.add_css_class("circular");
    }

    fn update(&self) {
        let starred = self.is_active();
        self.set_icon_name(if starred {
            "starred-symbolic"
        } else {
            "non-starred-symbolic"
        });
        let rm_text = gettext("Remove from favorites");
        let add_text = gettext("Add to favorites");
        self.set_tooltip_text(if starred {
            Some(&rm_text)
        } else {
            Some(&add_text)
        });

        if starred {
            self.add_css_class("starred")
        } else {
            self.remove_css_class("starred")
        }

        self.add_css_class("interacted")
    }
}
