use glib::Object;
use gtk::{gio, glib};


mod imp {

    use std::cell::RefCell;
    use gtk::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate};

    // Object holding the state
    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsukimi/clapperpage.ui")]
    #[properties(wrapper_type = super::ClapperPage)]
    pub struct ClapperPage {
        #[property(get, set, nullable)]
        pub url: RefCell<Option<String>>,
        pub buffering: RefCell<Option<glib::SignalHandlerId>>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for ClapperPage {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "ClapperPage";
        type Type = super::ClapperPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // Trait shared by all GObjects
    #[glib::derived_properties]
    impl ObjectImpl for ClapperPage {
        fn constructed(&self) {
            self.parent_constructed();

        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for ClapperPage {}

    // Trait shared by all windows
    impl WindowImpl for ClapperPage {}

    // Trait shared by all application windows
    impl ApplicationWindowImpl for ClapperPage {}

    impl adw::subclass::navigation_page::NavigationPageImpl for ClapperPage {}
}

glib::wrapper! {
    pub struct ClapperPage(ObjectSubclass<imp::ClapperPage>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget ,adw::NavigationPage,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl ClapperPage {
    pub fn new() -> Self {
        Object::builder().build()
    }
}
