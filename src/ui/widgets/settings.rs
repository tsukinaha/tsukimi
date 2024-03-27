use glib::Object;
use gtk::{gio, glib};

mod imp {

    use glib::subclass::InitializingObject;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate};

    // Object holding the state
    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/moe/tsukimi/settings.ui")]
    pub struct SettingsPage {
        #[template_child]
        pub proxyentry: TemplateChild<adw::EntryRow>,
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
            klass.install_action("setting.proxy", None, move |window, _action, _parameter| {});
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // Trait shared by all GObjects
    impl ObjectImpl for SettingsPage {}

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
}
