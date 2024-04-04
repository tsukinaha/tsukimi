use adw::prelude::*;
use glib::Object;
use gtk::{
    gio,
    glib,
    subclass::prelude::*,
};
use gtk::prelude::*;

use crate::APP_ID;

mod imp {

    use glib::subclass::InitializingObject;
    use gtk::subclass::prelude::*;
    use gtk::prelude::*;
    use gtk::{glib, CompositeTemplate};

    // Object holding the state
    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/moe/tsukimi/settings.ui")]
    pub struct SettingsPage {
        #[template_child]
        pub backcontrol: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub sidebarcontrol: TemplateChild<adw::SwitchRow>,

    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for SettingsPage {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "SettingsPage";
        type Type = super::SettingsPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // Trait shared by all GObjects
    impl ObjectImpl for SettingsPage {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.set_back();
            obj.set_sidebar();
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for SettingsPage {}

    // Trait shared by all windows
    impl WindowImpl for SettingsPage {}

    // Trait shared by all application windows
    impl ApplicationWindowImpl for SettingsPage {}

    impl adw::subclass::navigation_page::NavigationPageImpl for SettingsPage {}
}

glib::wrapper! {
    pub struct SettingsPage(ObjectSubclass<imp::SettingsPage>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget ,adw::NavigationPage,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl Default for SettingsPage {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsPage {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn set_sidebar(&self) {
        let imp = imp::SettingsPage::from_obj(self);
        let settings = gio::Settings::new(APP_ID);
        imp.sidebarcontrol.set_active(settings.boolean("is-overlay"));
        imp.sidebarcontrol.connect_active_notify(glib::clone!(@weak self as obj =>move |control| {
            let window = obj.root().unwrap().downcast::<super::window::Window>().unwrap();
            window.overlay_sidebar(control.is_active());
            settings.set_boolean("is-overlay", control.is_active()).unwrap();
        }));
    }

    pub fn set_back(&self) {
        let imp = imp::SettingsPage::from_obj(self);
        let settings = gio::Settings::new(APP_ID);
        imp.backcontrol.set_active(settings.boolean("is-progress-enabled"));
        imp.backcontrol.connect_active_notify(glib::clone!(@weak self as obj =>move |control| {
            settings.set_boolean("is-progress-enabled", control.is_active()).unwrap();
        }));
    }
}
