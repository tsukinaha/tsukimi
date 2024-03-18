use glib::Object;
// use gtk::prelude::*;
// use gtk::subclass::prelude::*;
use gtk::{gio, glib};

mod imp {
    // use std::cell::RefCell;

    use glib::subclass::InitializingObject;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate, Label, Picture};
    // use gtk::{gio,Entry};

    // Object holding the state
    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/moe/tsukimi/settings.ui")]
    pub struct HorizonItem {
        #[template_child]
        pub picture: TemplateChild<Picture>,
        #[template_child(id = "line1")]
        pub line1: TemplateChild<Label>,
        #[template_child(id = "line2")]
        pub line2: TemplateChild<Label>,
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
    impl ObjectImpl for HorizonItem {}

    // Trait shared by all widgets
    impl WidgetImpl for HorizonItem {}

    // Trait shared by all windows
    impl WindowImpl for HorizonItem {}

    // Trait shared by all application windows
    impl ApplicationWindowImpl for HorizonItem {}
}

glib::wrapper! {
    pub struct HorizonItem(ObjectSubclass<imp::HorizonItem>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl Default for HorizonItem {
    fn default() -> Self {
        Self::new()
    }
}

impl HorizonItem {
    pub fn new() -> Self {
        Object::builder().build()
    }
}
