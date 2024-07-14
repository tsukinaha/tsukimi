use crate::{
    client::client::EMBY_CLIENT,
    toast,
    ui::models::{emby_cache_path, SETTINGS},
    utils::spawn_tokio,
};
use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gdk::RGBA, gio, glib, template_callbacks, CompositeTemplate};

use super::window::Window;

mod imp {
    use super::*;
    use glib::subclass::InitializingObject;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/moe/tsukimi/account_settings.ui")]
    pub struct AccountSettings {
        #[template_child]
        pub password_entry: TemplateChild<adw::PasswordEntryRow>,
        #[template_child]
        pub password_second_entry: TemplateChild<adw::PasswordEntryRow>,
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
        pub backgroundcontrol: TemplateChild<gtk::Switch>,
        #[template_child]
        pub fontspinrow: TemplateChild<adw::SpinRow>,
        #[template_child]
        pub font: TemplateChild<gtk::FontDialogButton>,
        #[template_child]
        pub dailyrecommendcontrol: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub mpvcontrol: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub ytdlcontrol: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub color: TemplateChild<gtk::ColorDialogButton>,
        #[template_child]
        pub fg_color: TemplateChild<gtk::ColorDialogButton>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AccountSettings {
        const NAME: &'static str = "AccountSettings";
        type Type = super::AccountSettings;
        type ParentType = adw::PreferencesDialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
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
            klass.install_action(
                "setting.fontclear",
                None,
                move |set, _action, _parameter| {
                    set.clear_font();
                },
            );
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AccountSettings {
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
            obj.set_auto_select_server();
            obj.set_fontsize();
            obj.set_font();
            obj.set_daily_recommend();
            obj.set_mpvcontrol();
            obj.set_ytdlcontrol();
            obj.set_color();
        }
    }

    impl WidgetImpl for AccountSettings {}
    impl AdwDialogImpl for AccountSettings {}
    impl PreferencesDialogImpl for AccountSettings {}
}

glib::wrapper! {
    /// Preference Window to display and update room details.
    pub struct AccountSettings(ObjectSubclass<imp::AccountSettings>)
        @extends gtk::Widget, adw::Dialog, adw::PreferencesDialog, @implements gtk::Accessible;
}

impl Default for AccountSettings {
    fn default() -> Self {
        Self::new()
    }
}

#[template_callbacks]
impl AccountSettings {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    #[template_callback]
    async fn on_change_password(&self, _button: gtk::Button) {
        let new_password = self.imp().password_entry.text();
        let new_password_second = self.imp().password_second_entry.text();
        if new_password.is_empty() || new_password_second.is_empty() {
            toast!(self, "Password cannot be empty!");
            return;
        }
        if new_password != new_password_second {
            toast!(self, "Passwords do not match!");
            return;
        }
        match spawn_tokio(async move { EMBY_CLIENT.change_password(&new_password).await }).await {
            Ok(_) => {
                toast!(self, "Password changed successfully! Please login again.");
            }
            Err(e) => {
                toast!(self, &format!("Failed to change password: {}", e));
            }
        };
    }

    pub fn set_sidebar(&self) {
        let imp = self.imp();
        imp.sidebarcontrol.set_active(SETTINGS.overlay());
        imp.sidebarcontrol.connect_active_notify(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            move |control| {
                let window = obj
                    .root()
                    .unwrap()
                    .downcast::<super::window::Window>()
                    .unwrap();
                window.overlay_sidebar(control.is_active());
                SETTINGS.set_overlay(control.is_active()).unwrap();
            }
        ));
    }

    pub fn set_back(&self) {
        let imp = self.imp();
        imp.backcontrol.set_active(SETTINGS.progress());
        imp.backcontrol.connect_active_notify(move |control| {
            SETTINGS.set_progress(control.is_active()).unwrap();
        });
    }

    pub fn set_color(&self) {
        let imp = self.imp();
        use std::str::FromStr;
        imp.color
            .set_rgba(&RGBA::from_str(&SETTINGS.accent_color_code()).unwrap());
        imp.color.connect_rgba_notify(move |control| {
            SETTINGS
                .set_accent_color_code(&control.rgba().to_string())
                .unwrap();
        });
        imp.fg_color
            .set_rgba(&RGBA::from_str(&SETTINGS.accent_fg_color_code()).unwrap());
        imp.fg_color.connect_rgba_notify(move |control| {
            SETTINGS
                .set_accent_fg_color_code(&control.rgba().to_string())
                .unwrap();
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
            imp.fontspinrow
                .set_value((settings.property::<i32>("gtk-xft-dpi") / 1024).into());
        } else {
            imp.fontspinrow.set_value(SETTINGS.font_size().into());
        }
        imp.fontspinrow.connect_value_notify(move |control| {
            settings.set_property("gtk-xft-dpi", control.value() as i32 * 1024);
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
        let path = emby_cache_path();
        if path.exists() {
            std::fs::remove_dir_all(path).unwrap();
        }
        toast!(self, "Cache Cleared")
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
            "Alpha Dark" => pos = 4,
            "???" => pos = 5,
            _ => (),
        }
        imp.themecontrol.set_selected(pos);
        imp.themecontrol
            .connect_selected_item_notify(move |control| {
                let theme = control
                    .selected_item()
                    .and_then(|item| {
                        item.downcast::<gtk::StringObject>()
                            .ok()
                            .map(|item| item.string())
                    })
                    .unwrap();
                SETTINGS.set_theme(&theme).unwrap();
            });
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
        imp.backgroundspinrow.connect_value_notify(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            move |control| {
                SETTINGS.set_pic_opacity(control.value() as i32).unwrap();
                let window = obj
                    .root()
                    .unwrap()
                    .downcast::<super::window::Window>()
                    .unwrap();
                window.set_picopacity(control.value() as i32);
            }
        ));
    }

    pub fn set_pic(&self) {
        let imp = self.imp();
        imp.backgroundcontrol
            .set_active(SETTINGS.background_enabled());
        imp.backgroundcontrol.connect_active_notify(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            move |control| {
                SETTINGS
                    .set_background_enabled(control.is_active())
                    .unwrap();
                if !control.is_active() {
                    let window = obj
                        .root()
                        .unwrap()
                        .downcast::<super::window::Window>()
                        .unwrap();
                    window.clear_pic();
                }
            }
        ));
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
        glib::spawn_future_local(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                let window = obj
                    .root()
                    .unwrap()
                    .downcast::<super::window::Window>()
                    .unwrap();
                window.clear_pic();
            }
        ));
        SETTINGS.set_root_pic("").unwrap();
    }

    pub fn set_font(&self) {
        let imp = self.imp();
        let settings = self.settings();
        imp.font
            .set_font_desc(&gtk::pango::FontDescription::from_string(
                &SETTINGS.font_name(),
            ));
        imp.font.connect_font_desc_notify(move |font| {
            let font_desc = font.font_desc().unwrap();
            let font_string = gtk::pango::FontDescription::to_string(&font_desc);
            settings.set_gtk_font_name(Some(&font_string));
            SETTINGS.set_font_name(&font_string).unwrap();
        });
    }

    pub fn clear_font(&self) {
        SETTINGS.set_font_name("").unwrap();
        toast!(self, "Font Cleared, Restart to take effect.");
    }

    pub fn set_daily_recommend(&self) {
        let imp = self.imp();
        imp.dailyrecommendcontrol
            .set_active(SETTINGS.daily_recommend());
        imp.dailyrecommendcontrol
            .connect_active_notify(move |control| {
                SETTINGS.set_daily_recommend(control.is_active()).unwrap();
            });
    }

    pub fn set_mpvcontrol(&self) {
        let imp = self.imp();
        imp.mpvcontrol.set_active(SETTINGS.mpv());
        imp.mpvcontrol.connect_active_notify(move |control| {
            SETTINGS.set_mpv(control.is_active()).unwrap();
        });
    }

    pub fn set_ytdlcontrol(&self) {
        let imp = self.imp();
        imp.ytdlcontrol.set_active(SETTINGS.ytdl());
        imp.ytdlcontrol.connect_active_notify(move |control| {
            SETTINGS.set_ytdl(control.is_active()).unwrap();
        });
    }
}
