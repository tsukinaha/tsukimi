use gtk::glib;
use gtk::glib::prelude::*;
use gtk::glib::subclass::prelude::*;
use std::cell::RefCell;

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

impl ImageTags {
    pub fn new() -> ImageTags {
        glib::object::Object::new()
    }
}
