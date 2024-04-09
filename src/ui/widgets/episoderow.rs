use glib::Object;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use crate::ui::provider::episoderowitem::EpisodeObject;

mod imp {
    use std::cell::RefCell;

    use glib::Binding;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate, Label, Picture};

    // Object holding the state
    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/moe/tsukimi/episoderow.ui")]
    pub struct EpisodeRow {
        #[template_child]
        pub image: TemplateChild<Picture>,
        #[template_child]
        pub content_label: TemplateChild<Label>,
        // Vector holding the bindings to properties of `TaskObject`
        pub bindings: RefCell<Vec<Binding>>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for EpisodeRow {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "EpisodeRow";
        type Type = super::EpisodeRow;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // Trait shared by all GObjects
    impl ObjectImpl for EpisodeRow {}

    // Trait shared by all widgets
    impl WidgetImpl for EpisodeRow {}

    // Trait shared by all boxes
    impl BoxImpl for EpisodeRow {}
}

glib::wrapper! {
    pub struct EpisodeRow(ObjectSubclass<imp::EpisodeRow>)
    @extends gtk::Box, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl Default for EpisodeRow {
    fn default() -> Self {
        Self::new()
    }
}

impl EpisodeRow {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn bind(&self, episode_object: &EpisodeObject) {
        // Get state
        let image = self.imp().image.get();
        let content_label = self.imp().content_label.get();
        let mut bindings = self.imp().bindings.borrow_mut();

        // Bind `task_object.completed` to `task_row.completed_button.active`
        let image_binding = episode_object
            .bind_property("completed", &image, "active")
            .bidirectional()
            .sync_create()
            .build();
        // Save binding
        bindings.push(image_binding);

        // Bind `task_object.content` to `task_row.content_label.label`
        let content_label_binding = episode_object
            .bind_property("content", &content_label, "label")
            .sync_create()
            .build();
        // Save binding
        bindings.push(content_label_binding);
    }

    pub fn unbind(&self) {
        // Unbind all stored bindings
        for binding in self.imp().bindings.borrow_mut().drain(..) {
            binding.unbind();
        }
    }
}
