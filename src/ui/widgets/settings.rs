use adw::prelude::*;
use dirs::home_dir;
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
        #[template_child]
        pub resumecontrol: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub themecontrol: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub proxyentry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub toast: TemplateChild<adw::ToastOverlay>,
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
            klass.install_action("win.proxy", None, move |set, _action, _parameter| {
                set.proxy();
            });
            klass.install_action("win.proxyclear", None, move |set, _action, _parameter| {
                set.proxyclear();
            });
            klass.install_action("setting.clear", None, move |set, _action, _parameter| {
                set.cacheclear();
            });
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
            obj.set_resume();
            obj.set_proxy();
            obj.set_theme();
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
        let imp = self.imp();
        let settings = gio::Settings::new(APP_ID);
        imp.sidebarcontrol.set_active(settings.boolean("is-overlay"));
        imp.sidebarcontrol.connect_active_notify(glib::clone!(@weak self as obj =>move |control| {
            let window = obj.root().unwrap().downcast::<super::window::Window>().unwrap();
            window.overlay_sidebar(control.is_active());
            settings.set_boolean("is-overlay", control.is_active()).unwrap();
        }));
    }

    pub fn set_back(&self) {
        let imp = self.imp();
        let settings = gio::Settings::new(APP_ID);
        imp.backcontrol.set_active(settings.boolean("is-progress-enabled"));
        imp.backcontrol.connect_active_notify(move |control| {
            settings.set_boolean("is-progress-enabled", control.is_active()).unwrap();
        });
    }

    pub fn set_spin(&self) {
        let imp = self.imp();
        let settings = gio::Settings::new(APP_ID);
        imp.spinrow.set_value(settings.int("background-height").into());
        imp.spinrow.connect_value_notify(move |control| {
            settings.set_int("background-height", control.value() as i32).unwrap();
        });
    }

    pub fn set_fullscreen(&self) {
        let imp = self.imp();
        let settings = gio::Settings::new(APP_ID);
        imp.autofullscreencontrol.set_active(settings.boolean("is-fullscreen"));
        imp.autofullscreencontrol.connect_active_notify(move |control| {
            settings.set_boolean("is-fullscreen", control.is_active()).unwrap();
        });
    }

    pub fn set_forcewindow(&self) {
        let imp = self.imp();
        let settings = gio::Settings::new(APP_ID);
        imp.forcewindowcontrol.set_active(settings.boolean("is-force-window"));
        imp.forcewindowcontrol.connect_active_notify(move |control| {
            settings.set_boolean("is-force-window", control.is_active()).unwrap();
        });
    }

    pub fn set_resume(&self) {
        let imp = self.imp();
        let settings = gio::Settings::new(APP_ID);
        imp.resumecontrol.set_active(settings.boolean("is-resume"));
        imp.resumecontrol.connect_active_notify(move |control| {
            settings.set_boolean("is-resume", control.is_active()).unwrap();
        });
    }
    
    pub fn proxy(&self) {
        let imp = self.imp();
        let settings = gio::Settings::new(APP_ID);
        settings.set_string("proxy", &imp.proxyentry.text()).unwrap();
    }

    pub fn set_proxy(&self) {
        let imp = self.imp();
        let settings = gio::Settings::new(APP_ID);
        imp.proxyentry.set_text(&settings.string("proxy"));
    }

    pub fn proxyclear(&self) {
        let imp = self.imp();
        let settings = gio::Settings::new(APP_ID);
        settings.set_string("proxy", "").unwrap();
        imp.proxyentry.set_text("");
    }

    pub fn cacheclear(&self) {
        let path = format!("{}/.local/share/tsukimi", home_dir().expect("can not find home").display());
        std::fs::remove_dir_all(path).unwrap();
        let toast = 
            adw::Toast::builder()
                        .title(format!("Cache Cleared"))
                        .timeout(3)
                        .build();
        let imp = self.imp();
        imp.toast.add_toast(toast);
    }

    pub fn set_theme(&self) {
        let imp = self.imp();
        let settings = gio::Settings::new(APP_ID);
        let theme = settings.string("theme");
        let mut pos = 0;
        match theme.as_str() {
            "default" => pos = 0,
            "Adwaita" => pos = 1,
            "Adwaita Dark" => pos = 2,
            "Catppuccino Latte" => pos = 3,
            "Tokyo Night Dark" => pos = 4,
            "Solarized Dark" => pos = 5,
            "Alpha Dark" => pos = 6,
            _ => (),
        }
        imp.themecontrol.set_selected(pos);
        imp.themecontrol.connect_selected_item_notify(glib::clone!(@weak self as obj =>move |control| {
            let theme = control.selected_item().and_then(|item| {
                item.downcast::<gtk::StringObject>().ok().map(|item| item.string())
            }).unwrap();
            match theme.as_str() {
                "System Default" => settings.set_string("theme", "default").unwrap(),
                "Adwaita" => settings.set_string("theme", "Adwaita").unwrap(),
                "Adwaita Dark" => settings.set_string("theme", "Adwaita Dark").unwrap(),
                "Catppuccin Latte" => settings.set_string("theme", "Catppuccino Latte").unwrap(),
                "Tokyo Night Dark" => settings.set_string("theme", "Tokyo Night Dark").unwrap(),
                "Solarized Dark" => settings.set_string("theme", "Solarized Dark").unwrap(),
                "Alpha Dark" => settings.set_string("theme", "Alpha Dark").unwrap(),
                _ => (),
            }
        }));
    }
}
