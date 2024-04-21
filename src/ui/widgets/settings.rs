use adw::prelude::*;
use glib::Object;
use gtk::{gio, glib, subclass::prelude::*};
use std::env;

use crate::config::get_cache_dir;
use crate::ui::models::SETTINGS;

use super::window::Window;

mod imp {

    use glib::subclass::InitializingObject;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate};

    use crate::utils::spawn_g_timeout;

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
        pub selectlastcontrol: TemplateChild<adw::SwitchRow>,
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
        #[template_child]
        pub fontspinrow: TemplateChild<adw::SpinRow>,
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
            spawn_g_timeout(glib::clone!(@weak obj=> async move {
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
                obj.set_auto_select_server();
                obj.set_fontsize();
            }));
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
        imp.sidebarcontrol.set_active(SETTINGS.overlay());
        imp.sidebarcontrol
            .connect_active_notify(glib::clone!(@weak self as obj =>move |control| {
                let window = obj.root().unwrap().downcast::<super::window::Window>().unwrap();
                window.overlay_sidebar(control.is_active());
                SETTINGS.set_overlay(control.is_active()).unwrap();
            }));
    }

    pub fn set_back(&self) {
        let imp = self.imp();
        imp.backcontrol.set_active(SETTINGS.progress());
        imp.backcontrol.connect_active_notify(move |control| {
            SETTINGS.set_progress(control.is_active()).unwrap();
        });
    }

    pub fn set_auto_select_server(&self) {
        let imp = self.imp();
        imp.selectlastcontrol
            .set_active(SETTINGS.auto_select_server());
        imp.selectlastcontrol.connect_active_notify(move |control| {
            SETTINGS
                .set_auto_select_server(control.is_active())
                .unwrap();
        });
    }

    pub fn set_spin(&self) {
        let imp = self.imp();
        imp.spinrow.set_value(SETTINGS.background_height().into());
        imp.spinrow.connect_value_notify(move |control| {
            SETTINGS
                .set_background_height(control.value() as i32)
                .unwrap();
        });
    }

    pub fn set_fontsize(&self) {
        let imp = self.imp();
        let settings = gtk::Settings::default().unwrap();
        if SETTINGS.font_size() == -1 {
            imp.fontspinrow.set_value((settings.property::<i32>("gtk-xft-dpi") / 1024).into());
        } else {
            imp.fontspinrow.set_value(SETTINGS.font_size().into());
        }
        imp.fontspinrow.connect_value_notify(move |control| {
            settings.set_property("gtk-xft-dpi",control.value() as i32 * 1024);
            SETTINGS.set_font_size(control.value() as i32).unwrap();
        });
    }

    pub fn set_fullscreen(&self) {
        let imp = self.imp();
        imp.autofullscreencontrol.set_active(SETTINGS.fullscreen());
        imp.autofullscreencontrol
            .connect_active_notify(move |control| {
                SETTINGS.set_fullscreen(control.is_active()).unwrap();
            });
    }

    pub fn set_forcewindow(&self) {
        let imp = self.imp();
        imp.forcewindowcontrol.set_active(SETTINGS.forcewindow());
        imp.forcewindowcontrol
            .connect_active_notify(move |control| {
                SETTINGS.set_forcewindow(control.is_active()).unwrap();
            });
    }

    pub fn set_resume(&self) {
        let imp = self.imp();
        imp.resumecontrol.set_active(SETTINGS.resume());
        imp.resumecontrol.connect_active_notify(move |control| {
            SETTINGS.set_resume(control.is_active()).unwrap();
        });
    }

    pub fn proxy(&self) {
        let imp = self.imp();
        SETTINGS.set_proxy(&imp.proxyentry.text()).unwrap();
    }

    pub fn set_proxy(&self) {
        let imp = self.imp();
        imp.proxyentry.set_text(&SETTINGS.proxy());
    }

    pub fn proxyclear(&self) {
        let imp = self.imp();
        SETTINGS.set_proxy("").unwrap();
        imp.proxyentry.set_text("");
    }

    pub fn cacheclear(&self) {
        let path = get_cache_dir(env::var("EMBY_NAME").unwrap());
        #[cfg(unix)]
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
        #[cfg(windows)]
        remove_file(std::path::PathBuf::from(path.parent().unwrap())).unwrap();
        let toast = adw::Toast::builder()
            .title("Cache Cleared".to_string())
            .timeout(3)
            .build();
        let imp = self.imp();
        imp.toast.add_toast(toast);
    }

    pub fn set_theme(&self) {
        let imp = self.imp();
        let theme = SETTINGS.theme();
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
        imp.themecontrol.connect_selected_item_notify(
            glib::clone!(@weak self as obj =>move |control| {
                let theme = control.selected_item().and_then(|item| {
                    item.downcast::<gtk::StringObject>().ok().map(|item| item.string())
                }).unwrap();
                SETTINGS.set_theme(&theme).unwrap();
            }),
        );
    }

    pub fn set_thread(&self) {
        let imp = self.imp();
        imp.threadspinrow.set_value(SETTINGS.threads().into());
        imp.threadspinrow.connect_value_notify(move |control| {
            SETTINGS.set_threads(control.value() as i32).unwrap();
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
                SETTINGS.set_root_pic(&file_path).unwrap();
                window.set_rootpic(file);
            }
            Err(_) => window.toast("Failed to set root picture."),
        };
    }

    pub fn set_picopactiy(&self) {
        let imp = self.imp();
        imp.backgroundspinrow
            .set_value(SETTINGS.pic_opacity().into());
        imp.backgroundspinrow.connect_value_notify(
            glib::clone!(@weak self as obj =>move |control| {
                SETTINGS.set_pic_opacity(control.value() as i32).unwrap();
                let window = obj.root().unwrap().downcast::<super::window::Window>().unwrap();
                window.set_picopacity(control.value() as i32);
            }),
        );
    }

    pub fn set_pic(&self) {
        let imp = self.imp();
        imp.backgroundcontrol
            .set_active(SETTINGS.background_enabled());
        imp.backgroundcontrol.connect_active_notify(
            glib::clone!(@weak self as obj =>move |control| {
                SETTINGS.set_background_enabled(control.is_active()).unwrap();
                if !control.is_active() {
                    let window = obj.root().unwrap().downcast::<super::window::Window>().unwrap();
                    window.clear_pic();
                }
            }),
        );
    }

    pub fn set_picblur(&self) {
        let imp = self.imp();
        imp.backgroundblurcontrol
            .set_active(SETTINGS.is_blur_enabled());
        imp.backgroundblurcontrol
            .connect_active_notify(move |control| {
                SETTINGS.set_blur_enabled(control.is_active()).unwrap();
            });
    }

    pub fn change_picblur(&self) {
        let imp = self.imp();
        imp.backgroundblurspinrow
            .set_value(SETTINGS.pic_blur().into());
        imp.backgroundblurspinrow
            .connect_value_notify(move |control| {
                SETTINGS.set_pic_blur(control.value() as i32).unwrap();
            });
    }

    pub fn clearpic(&self) {
        glib::spawn_future_local(glib::clone!(@weak self as obj => async move {
            let window = obj.root().unwrap().downcast::<super::window::Window>().unwrap();
            window.clear_pic();
        }));
        SETTINGS.set_root_pic("").unwrap();
    }
}

/// for Scoop users cache is persist folder created by Scoop, removing it would fail.
#[cfg(windows)]
fn remove_file(path: std::path::PathBuf) -> std::io::Result<()> {
    let entries = std::fs::read_dir(path)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            std::fs::remove_file(path)?;
        } else if path.is_dir() {
            // remove files recursively
            std::fs::remove_dir_all(path)?;
        }
    }

    Ok(())
}
