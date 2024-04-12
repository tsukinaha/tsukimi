use adw::prelude::*;
use dirs::home_dir;
use glib::Object;
use gtk::{gio, glib, subclass::prelude::*};

use crate::APP_ID;

use super::window::Window;

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
        pub backgroundspinrow: TemplateChild<adw::SpinRow>,
        #[template_child]
        pub threadspinrow: TemplateChild<adw::SpinRow>,
        #[template_child]
        pub forcewindowcontrol: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub resumecontrol: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub themecontrol: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub proxyentry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub backgroundblurspinrow: TemplateChild<adw::SpinRow>,
        #[template_child]
        pub backgroundblurcontrol: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub backgroundcontrol: TemplateChild<adw::SwitchRow>,
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
            klass.install_action_async(
                "setting.rootpic",
                None,
                |set, _action, _parameter| async move {
                    set.set_rootpic().await;
                },
            );
            klass.install_action(
                "setting.backgroundclear",
                None,
                move |set, _action, _parameter| {
                    set.clearpic();
                },
            );
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
            obj.set_thread();
            obj.set_picopactiy();
            obj.set_pic();
            obj.set_picblur();
            obj.change_picblur();
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
        imp.sidebarcontrol
            .set_active(settings.boolean("is-overlay"));
        imp.sidebarcontrol
            .connect_active_notify(glib::clone!(@weak self as obj =>move |control| {
                let window = obj.root().unwrap().downcast::<super::window::Window>().unwrap();
                window.overlay_sidebar(control.is_active());
                settings.set_boolean("is-overlay", control.is_active()).unwrap();
            }));
    }

    pub fn set_back(&self) {
        let imp = self.imp();
        let settings = gio::Settings::new(APP_ID);
        imp.backcontrol
            .set_active(settings.boolean("is-progress-enabled"));
        imp.backcontrol.connect_active_notify(move |control| {
            settings
                .set_boolean("is-progress-enabled", control.is_active())
                .unwrap();
        });
    }

    pub fn set_spin(&self) {
        let imp = self.imp();
        let settings = gio::Settings::new(APP_ID);
        imp.spinrow
            .set_value(settings.int("background-height").into());
        imp.spinrow.connect_value_notify(move |control| {
            settings
                .set_int("background-height", control.value() as i32)
                .unwrap();
        });
    }

    pub fn set_fullscreen(&self) {
        let imp = self.imp();
        let settings = gio::Settings::new(APP_ID);
        imp.autofullscreencontrol
            .set_active(settings.boolean("is-fullscreen"));
        imp.autofullscreencontrol
            .connect_active_notify(move |control| {
                settings
                    .set_boolean("is-fullscreen", control.is_active())
                    .unwrap();
            });
    }

    pub fn set_forcewindow(&self) {
        let imp = self.imp();
        let settings = gio::Settings::new(APP_ID);
        imp.forcewindowcontrol
            .set_active(settings.boolean("is-force-window"));
        imp.forcewindowcontrol
            .connect_active_notify(move |control| {
                settings
                    .set_boolean("is-force-window", control.is_active())
                    .unwrap();
            });
    }

    pub fn set_resume(&self) {
        let imp = self.imp();
        let settings = gio::Settings::new(APP_ID);
        imp.resumecontrol.set_active(settings.boolean("is-resume"));
        imp.resumecontrol.connect_active_notify(move |control| {
            settings
                .set_boolean("is-resume", control.is_active())
                .unwrap();
        });
    }

    pub fn proxy(&self) {
        let imp = self.imp();
        let settings = gio::Settings::new(APP_ID);
        settings
            .set_string("proxy", &imp.proxyentry.text())
            .unwrap();
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
        let path = format!(
            "{}/.local/share/tsukimi",
            home_dir().expect("can not find home").display()
        );
        std::fs::remove_dir_all(path).unwrap();
        let toast = adw::Toast::builder()
            .title("Cache Cleared".to_string())
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

    pub fn set_thread(&self) {
        let imp = self.imp();
        let settings = gio::Settings::new(APP_ID);
        imp.threadspinrow.set_value(settings.int("threads").into());
        imp.threadspinrow.connect_value_notify(move |control| {
            settings.set_int("threads", control.value() as i32).unwrap();
        });
    }

    pub async fn set_rootpic(&self) {
        let images_filter = gtk::FileFilter::new();
        images_filter.set_name(Some("Image"));
        images_filter.add_pixbuf_formats();
        let model = gio::ListStore::new::<gtk::FileFilter>();
        model.append(&images_filter);
        let window = self.root().and_downcast::<Window>().unwrap();
        let filedialog = gtk::FileDialog::builder()
            .modal(true)
            .title("Select a picture")
            .filters(&model)
            .build();
        match filedialog.open_future(Some(&window)).await {
            Ok(file) => {
                let file_path = file.path().unwrap().display().to_string();
                let settings = gio::Settings::new(APP_ID);
                settings.set_string("root-pic", &file_path).unwrap();
                window.set_rootpic(file);
            }
            Err(_) => window.toast("Failed to set root picture."),
        };
    }

    pub fn set_picopactiy(&self) {
        let imp = self.imp();
        let settings = gio::Settings::new(APP_ID);
        imp.backgroundspinrow
            .set_value(settings.int("pic-opacity").into());
        imp.backgroundspinrow.connect_value_notify(
            glib::clone!(@weak self as obj =>move |control| {
                settings
                    .set_int("pic-opacity", control.value() as i32)
                    .unwrap();
                let window = obj.root().unwrap().downcast::<super::window::Window>().unwrap();
                window.set_picopacity(control.value() as i32);
            }),
        );
    }

    pub fn set_pic(&self) {
        let imp = self.imp();
        let settings = gio::Settings::new(APP_ID);
        imp.backgroundcontrol
            .set_active(settings.boolean("is-backgroundenabled"));
        imp.backgroundcontrol.connect_active_notify(
            glib::clone!(@weak self as obj =>move |control| {
                settings
                    .set_boolean("is-backgroundenabled", control.is_active())
                    .unwrap();
                if !control.is_active() {
                    let window = obj.root().unwrap().downcast::<super::window::Window>().unwrap();
                    window.clear_pic();
                }
            }),
        );
    }

    pub fn set_picblur(&self) {
        let imp = self.imp();
        let settings = gio::Settings::new(APP_ID);
        imp.backgroundblurcontrol
            .set_active(settings.boolean("is-blurenabled"));
        imp.backgroundblurcontrol
            .connect_active_notify(move |control| {
                settings
                    .set_boolean("is-blurenabled", control.is_active())
                    .unwrap();
            });
    }

    pub fn change_picblur(&self) {
        let imp = self.imp();
        let settings = gio::Settings::new(APP_ID);
        imp.backgroundblurspinrow
            .set_value(settings.int("pic-blur").into());
        imp.backgroundblurspinrow
            .connect_value_notify(move |control| {
                settings
                    .set_int("pic-blur", control.value() as i32)
                    .unwrap();
            });
    }

    pub fn clearpic(&self) {
        glib::spawn_future_local(glib::clone!(@weak self as obj => async move {
            let window = obj.root().unwrap().downcast::<super::window::Window>().unwrap();
            window.clear_pic();
        }));
        let settings = gio::Settings::new(APP_ID);
        settings.set_string("root-pic", "").unwrap();
    }
}
