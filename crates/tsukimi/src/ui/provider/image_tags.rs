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

impl ImageTags {
    pub fn new(
        image_tags: Option<crate::client::structs::ImageTags>,
        backdrop_image_tags: Option<Vec<String>>,
    ) -> Option<Self> {
        let backdrop = backdrop_image_tags.and_then(|tags| tags.into_iter().next());
        if image_tags.is_none() && backdrop.is_none() {
            return None;
        }
        let provider: Self = glib::object::Object::new();
        provider.set_backdrop(backdrop);
        if let Some(image_tags) = image_tags {
            provider.set_primary(image_tags.primary);
            provider.set_thumb(image_tags.thumb);
            provider.set_banner(image_tags.banner);
        }
        Some(provider)
    }
}
