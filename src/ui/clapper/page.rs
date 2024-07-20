use crate::client::client::EMBY_CLIENT;
use crate::client::network::RUNTIME;
use crate::client::structs::Back;
use crate::toast;
use crate::ui::widgets::song_widget::format_duration;
use crate::ui::widgets::window::Window;
use glib::Object;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

mod imp {

    use std::cell::RefCell;

    use glib::subclass::InitializingObject;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate};

    use crate::client::structs::Back;
    use crate::ui::clapper::mpvglarea::MPVGLArea;

    // Object holding the state
    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsukimi/clapperpage.ui")]
    #[properties(wrapper_type = super::ClapperPage)]
    pub struct ClapperPage {
        #[property(get, set, nullable)]
        pub url: RefCell<Option<String>>,
        #[template_child]
        pub video: TemplateChild<MPVGLArea>,
        pub timeout: RefCell<Option<glib::source::SourceId>>,
        pub back: RefCell<Option<Back>>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for ClapperPage {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "ClapperPage";
        type Type = super::ClapperPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            MPVGLArea::ensure_type();
            klass.bind_template();
            klass.bind_template_instance_callbacks();
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

impl Default for ClapperPage {
    fn default() -> Self {
        Self::new()
    }
}

#[gtk::template_callbacks]
impl ClapperPage {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn play(&self) {
        self.imp().video.play();
    }
}
