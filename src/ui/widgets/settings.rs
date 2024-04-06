use adw::prelude::*;
use glib::Object;
use gtk::{
    gio,
    glib,
    subclass::prelude::*,
};

use crate::APP_ID;

mod imp {

    use glib::subclass::InitializingObject;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate};

    // Object holding the state
    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/moe/tsukimi/settings.ui")]
    pub struct SettingsPage {
        #[template_child]
        pub backcontrol: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub sidebarcontrol: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub autofullscreencontrol: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub spinrow: TemplateChild<adw::SpinRow>,
        #[template_child]
        pub forcewindowcontrol: TemplateChild<adw::SwitchRow>,
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
            obj.set_spin();
            obj.set_fullscreen();
            obj.set_forcewindow();
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

    pub fn set_spin(&self) {
        let imp = imp::SettingsPage::from_obj(self);
        let settings = gio::Settings::new(APP_ID);
        imp.spinrow.set_value(settings.int("background-height").into());
        imp.spinrow.connect_value_notify(glib::clone!(@weak self as obj =>move |control| {
            settings.set_int("background-height", control.value() as i32).unwrap();
        }));
    }

    pub fn set_fullscreen(&self) {
        let imp = imp::SettingsPage::from_obj(self);
        let settings = gio::Settings::new(APP_ID);
        imp.autofullscreencontrol.set_active(settings.boolean("is-fullscreen"));
        imp.autofullscreencontrol.connect_active_notify(glib::clone!(@weak self as obj =>move |control| {
            settings.set_boolean("is-fullscreen", control.is_active()).unwrap();
        }));
    }

    pub fn set_forcewindow(&self) {
        let imp = imp::SettingsPage::from_obj(self);
        let settings = gio::Settings::new(APP_ID);
        imp.forcewindowcontrol.set_active(settings.boolean("is-force-window"));
        imp.forcewindowcontrol.connect_active_notify(glib::clone!(@weak self as obj =>move |control| {
            settings.set_boolean("is-force-window", control.is_active()).unwrap();
        }));
    }
    
}
