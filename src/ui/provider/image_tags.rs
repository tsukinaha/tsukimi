use std::cell::RefCell;

use gtk::{
    glib,
    glib::{
        prelude::*,
        subclass::prelude::*,
    },
};

pub mod imp {
    use gtk::glib::Properties;

    use super::*;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::ImageTags)]
    pub struct ImageTags {
        #[property(get, set, nullable)]
        pub backdrop: RefCell<Option<String>>,
        #[property(get, set, nullable)]
        pub primary: RefCell<Option<String>>,
        #[property(get, set, nullable)]
        pub thumb: RefCell<Option<String>>,
        #[property(get, set, nullable)]
        pub banner: RefCell<Option<String>>,
    }

    #[glib::derived_properties]
    impl ObjectImpl for ImageTags {}

    #[glib::object_subclass]
    impl ObjectSubclass for ImageTags {
        const NAME: &'static str = "ImageTags";
        type Type = super::ImageTags;
    }
}

glib::wrapper! {
    pub struct ImageTags(ObjectSubclass<imp::ImageTags>);
}

impl Default for ImageTags {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageTags {
    pub fn new() -> ImageTags {
        glib::object::Object::new()
    }

    pub fn all_none(&self) -> bool {
        let imp = self.imp();
        imp.backdrop.borrow().is_none()
            && imp.primary.borrow().is_none()
            && imp.thumb.borrow().is_none()
            && imp.banner.borrow().is_none()
    }
}
