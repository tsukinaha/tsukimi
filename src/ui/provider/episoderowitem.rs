mod imp {
    use glib::Properties;
    use gtk::glib;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use std::cell::RefCell;

    use super::TaskData;

    // ANCHOR: struct_and_subclass
    // Object holding the state
    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::EpisodeObject)]
    pub struct EpisodeObject {
        #[property(name = "imageid", get, set, type = String, member = imageid)]
        #[property(name = "label", get, set, type = String, member = label)]
        pub data: RefCell<TaskData>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for EpisodeObject {
        const NAME: &'static str = "EpisodeObject";
        type Type = super::EpisodeObject;
    }

    // Trait shared by all GObjects
    #[glib::derived_properties]
    impl ObjectImpl for EpisodeObject {}
    // ANCHOR_END: struct_and_subclass
}

use glib::Object;
use gtk::glib;

// ANCHOR: glib_wrapper_and_new
glib::wrapper! {
    pub struct EpisodeObject(ObjectSubclass<imp::EpisodeObject>);
}

impl EpisodeObject {
    pub fn new(imageid: bool, label: String) -> Self {
        Object::builder()
            .property("imageid", imageid)
            .property("label", label)
            .build()
    }
}
// ANCHOR_END: glib_wrapper_and_new

// ANCHOR: task_data
#[derive(Default)]
pub struct TaskData {
    pub imageid: String,
    pub label: String,
}
// ANCHOR: task_data
