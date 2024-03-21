use gtk::{gio, glib};
use glib::Object;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

mod imp{
    use std::cell::{OnceCell, RefCell};

    use glib::subclass::InitializingObject;
    use gtk::subclass::prelude::*;
    use gtk::{gio, glib, CompositeTemplate, Entry, Label, Picture};

    // Object holding the state
    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/moe/tsukimi/item.ui")]
    pub struct ItemPage {
        id: OnceCell<String>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for ItemPage {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "ItemPage";
        type Type = super::ItemPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // Trait shared by all GObjects
    impl ObjectImpl for ItemPage {
        fn constructed(&self) {

        }

    }

    // Trait shared by all widgets
    impl WidgetImpl for ItemPage {}

    // Trait shared by all windows
    impl WindowImpl for ItemPage {}

    // Trait shared by all application windows
    impl ApplicationWindowImpl for ItemPage {}

    impl adw::subclass::navigation_page::NavigationPageImpl for ItemPage {}
}

glib::wrapper! {
    pub struct ItemPage(ObjectSubclass<imp::ItemPage>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget ,adw::NavigationPage,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl ItemPage {
    pub fn new(id:String) -> Self {
        Object::builder().build()
    }
}