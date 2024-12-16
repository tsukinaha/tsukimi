use std::cell::RefCell;

use gtk::{
    glib,
    glib::{
        prelude::*,
        subclass::prelude::*,
    },
};

use super::EuItem;

pub mod imp {
    use gtk::glib::Properties;

    use super::*;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::EuObject)]
    pub struct EuObject {
        #[property(get, set, nullable)]
        item: RefCell<Option<EuItem>>,
    }

    #[glib::derived_properties]
    impl ObjectImpl for EuObject {}

    #[glib::object_subclass]
    impl ObjectSubclass for EuObject {
        const NAME: &'static str = "EuObject";
        type Type = super::EuObject;
    }
}

glib::wrapper! {
    pub struct EuObject(ObjectSubclass<imp::EuObject>);
}

impl EuObject {
    pub fn new(item: &EuItem) -> Self {
        glib::Object::builder().property("item", item).build()
    }
}
