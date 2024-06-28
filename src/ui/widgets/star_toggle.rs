use gtk::{glib, prelude::*, subclass::prelude::*};

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
    /// Preference Window to display and update room details.
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
        self.set_tooltip_text(if starred {
            Some("Remove from favorites")
        } else {
            Some("Add to favorites")
        });

        if starred {
            self.add_css_class("starred")
        } else {
            self.remove_css_class("starred")
        }

        self.add_css_class("interacted")
    }
}
