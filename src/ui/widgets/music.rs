use glib::Object;
use gtk::{gio, glib};

mod imp {
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::prelude::*;
    use gtk::{glib, CompositeTemplate};
    use std::cell::OnceCell;

    use crate::utils::spawn_g_timeout;
    // Object holding the state
    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsukimi/music.ui")]
    #[properties(wrapper_type = super::MusicPage)]
    pub struct MusicPage {
        #[property(get, set, construct_only)]
        pub id: OnceCell<String>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for MusicPage {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "MusicPage";
        type Type = super::MusicPage;
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
    impl ObjectImpl for MusicPage {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            spawn_g_timeout(glib::clone!(@weak obj => async move {

            }));
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for MusicPage {}

    // Trait shared by all windows
    impl WindowImpl for MusicPage {}

    // Trait shared by all application windows
    impl ApplicationWindowImpl for MusicPage {}

    impl adw::subclass::navigation_page::NavigationPageImpl for MusicPage {}
}

glib::wrapper! {
    pub struct MusicPage(ObjectSubclass<imp::MusicPage>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget ,adw::NavigationPage,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl MusicPage {
    pub fn new(id: &str) -> Self {
        Object::builder().property("id", id).build()
    }
}
