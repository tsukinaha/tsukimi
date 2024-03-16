use std::cell::RefCell;

use glib::subclass::InitializingObject;
use gtk::subclass::prelude::*;
use gtk::{gio, glib, CompositeTemplate, Entry, Label, Picture};

// Object holding the state
#[derive(CompositeTemplate, Default)]
#[template(resource = "/moe/tsukimi/horizon_item.ui")]
pub struct HorizonItem {
    #[template_child]
    pub picture: TemplateChild<Picture>,
    #[template_child]
    pub label1: TemplateChild<Label>,
    #[template_child]
    pub label2: TemplateChild<Label>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for HorizonItem {
    // `NAME` needs to match `class` attribute of template
    const NAME: &'static str = "HorizonItem";
    type Type = super::HorizonItem;
    type ParentType = gtk::ApplicationWindow;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

// Trait shared by all GObjects
impl ObjectImpl for HorizonItem {
    fn constructed(&self) {
        // Call "constructed" on parent
        self.parent_constructed();

        // Setup
        let obj = self.obj();
        obj.setup_tasks();
        obj.setup_callbacks();
        obj.setup_factory();
    }
}

// Trait shared by all widgets
impl WidgetImpl for HorizonItem {}

// Trait shared by all windows
impl WindowImpl for HorizonItem {}

// Trait shared by all application windows
impl ApplicationWindowImpl for HorizonItem {}