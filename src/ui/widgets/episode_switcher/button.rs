use gtk::{
    glib,
    prelude::*,
    subclass::prelude::*,
};

pub(crate) mod imp {
    use std::cell::OnceCell;

    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::EpisodeButton)]
    pub struct EpisodeButton {
        #[property(get, set, construct_only)]
        pub start_index: OnceCell<u32>,
        #[property(get, set, construct_only)]
        pub length: OnceCell<u32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EpisodeButton {
        const NAME: &'static str = "EpisodeButton";
        type Type = super::EpisodeButton;
        type ParentType = gtk::Button;
    }

    #[glib::derived_properties]
    impl ObjectImpl for EpisodeButton {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            let start_index = obj.start_index();
            let length = obj.length();

            obj.add_css_class("flat");
            obj.set_label(&format!("{} - {}", start_index + 1, start_index + length));
        }
    }

    impl WidgetImpl for EpisodeButton {}

    impl ButtonImpl for EpisodeButton {}
}

glib::wrapper! {

    pub struct EpisodeButton(ObjectSubclass<imp::EpisodeButton>)
        @extends gtk::Widget, gtk::Button;
}

impl EpisodeButton {
    pub fn new(start_index: u32, length: u32) -> Self {
        glib::Object::builder()
            .property("start-index", start_index)
            .property("length", length)
            .build()
    }
}
