use std::cell::RefCell;

use adw::prelude::*;
use gtk::glib::{
    self,
    subclass::prelude::*,
};

pub mod imp {

    use gtk::glib::Properties;

    use super::*;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::EuItem)]
    pub struct EuItem {
        #[property(get, set, nullable)]
        image_url: RefCell<Option<String>>,
        #[property(get, set, nullable)]
        image_original_url: RefCell<Option<String>>,
        #[property(get, set, nullable)]
        line1: RefCell<Option<String>>,
        #[property(get, set, nullable)]
        line2: RefCell<Option<String>>,
        #[property(get, set, nullable)]
        line3: RefCell<Option<String>>,
    }

    #[glib::derived_properties]
    impl ObjectImpl for EuItem {}

    #[glib::object_subclass]
    impl ObjectSubclass for EuItem {
        const NAME: &'static str = "EuItem";
        type Type = super::EuItem;
    }
}

glib::wrapper! {
    pub struct EuItem(ObjectSubclass<imp::EuItem>);
}

impl Default for EuItem {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl EuItem {
    pub fn new(
        image_url: Option<String>, image_original_url: Option<String>, line1: Option<String>,
        line2: Option<String>, line3: Option<String>,
    ) -> Self {
        glib::Object::builder()
            .property("image-url", &image_url)
            .property("image-original-url", &image_original_url)
            .property("line1", &line1)
            .property("line2", &line2)
            .property("line3", &line3)
            .build()
    }
}
