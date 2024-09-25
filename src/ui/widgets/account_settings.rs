#![allow(deprecated)]

use crate::{
    client::client::EMBY_CLIENT,
    toast,
    ui::models::{emby_cache_path, SETTINGS},
    utils::spawn_tokio,
};
use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::{gdk::RGBA, gio, glib, template_callbacks, CompositeTemplate};

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
        pub sidebarcontrol: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub backgroundspinrow: TemplateChild<adw::SpinRow>,
        #[template_child]
        pub threadspinrow: TemplateChild<adw::SpinRow>,
        #[template_child]
        pub selectlastcontrol: TemplateChild<adw::SwitchRow>,
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
        pub color: TemplateChild<gtk::ColorDialogButton>,
        #[template_child]
        pub fg_color: TemplateChild<gtk::ColorDialogButton>,
        #[template_child]
        pub estimate_control: TemplateChild<gtk::Switch>,
        #[template_child]
        pub estimate_spinrow: TemplateChild<adw::SpinRow>,
        #[template_child]
        pub seek_forward_spinrow: TemplateChild<adw::SpinRow>,
        #[template_child]
        pub seek_backward_spinrow: TemplateChild<adw::SpinRow>,
        #[template_child]
        pub config_switchrow: TemplateChild<adw::SwitchRow>,

        #[template_child]
        pub buffer_switchrow: TemplateChild<adw::SwitchRow>,

        #[template_child]
        pub cachesize_spinrow: TemplateChild<adw::SpinRow>,

        #[template_child]
        pub stereo_switchrow: TemplateChild<adw::SwitchRow>,

        #[template_child]
        pub volume_spinrow: TemplateChild<adw::SpinRow>,

        #[template_child]
        pub mpv_sub_font_button: TemplateChild<gtk::FontDialogButton>,
        #[template_child]
        pub mpv_sub_size_spinrow: TemplateChild<adw::SpinRow>,

        #[template_child]
        pub preferred_audio_language_comborow: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub preferred_subtitle_language_comborow: TemplateChild<adw::ComboRow>,

        #[template_child]
        pub video_subpage: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub preferred_version_subpage: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub add_version_preferences_dialog: TemplateChild<adw::Dialog>,
    
        #[template_child]
        pub descriptor_string_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub descriptor_regex_label: TemplateChild<gtk::Label>,

        #[template_child]
        pub descriptor_type_comborow: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub descriptor_entryrow: TemplateChild<adw::EntryRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AccountSettings {
        const NAME: &'static str = "AccountSettings";
        type Type = super::AccountSettings;
        type ParentType = adw::PreferencesWindow;

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
            klass.install_action("version.add-perfer", None, move |set, _action, _parameter| {
                set.add_preferred_version();
            });
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AccountSettings {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.set_sidebar();
            obj.set_proxy();
            obj.set_picopactiy();
            obj.set_pic();
            obj.set_fontsize();
            obj.set_font();
            obj.set_color();
            obj.set_estimate();
        }
    }

    impl WidgetImpl for AccountSettings {}
    impl WindowImpl for AccountSettings {}
    impl AdwWindowImpl for AccountSettings {}
    impl PreferencesWindowImpl for AccountSettings {}
}

glib::wrapper! {
    /// Preference Window to display preferences.
    pub struct AccountSettings(ObjectSubclass<imp::AccountSettings>)
    @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget, adw::PreferencesWindow,
    @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
        gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
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
            toast!(self, gettext("Password cannot be empty!"));
            return;
        }
        if new_password != new_password_second {
            toast!(self, gettext("Passwords do not match!"));
            return;
        }
        match spawn_tokio(async move { EMBY_CLIENT.change_password(&new_password).await }).await {
            Ok(_) => {
                toast!(
                    self,
                    gettext("Password changed successfully! Please login again.")
                );
            }
            Err(e) => {
                toast!(
                    self,
                    &format!("{}: {}", gettext("Failed to change password"), e)
                );
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
                let window = obj.window();
                window.overlay_sidebar(control.is_active());
                SETTINGS.set_overlay(control.is_active()).unwrap();
            }
        ));
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
        toast!(self, gettext("Cache Cleared"))
    }

    pub async fn set_rootpic(&self) {
        let images_filter = gtk::FileFilter::new();
        images_filter.set_name(Some("Image"));
        images_filter.add_pixbuf_formats();
        let model = gio::ListStore::new::<gtk::FileFilter>();
        model.append(&images_filter);
        let window = self.window();
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
            Err(_) => toast!(self, gettext("No file selected")),
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
                let window = obj.window();
                window.set_picopacity(control.value() as i32);
            }
        ));
    }

    fn window(&self) -> super::window::Window {
        let windows = self.application().unwrap().windows();
        let window = windows
            .into_iter()
            .find(|w| w.is::<super::window::Window>())
            .unwrap();
        window.downcast::<super::window::Window>().unwrap()
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
                    let window = obj.window();
                    window.clear_pic();
                }
            }
        ));
    }

    pub fn clearpic(&self) {
        glib::spawn_future_local(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                let window = obj.window();
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
        toast!(self, gettext("Font Cleared, Restart to take effect."));
    }

    pub fn set_estimate(&self) {
        let imp = self.imp();
        SETTINGS.bind("is-blurenabled", &imp.backgroundblurcontrol.get(), "active").build();
        SETTINGS.bind("pic-blur", &imp.backgroundblurspinrow.get(), "value").build();
        SETTINGS.bind("is-daily-recommend", &imp.dailyrecommendcontrol.get(), "active").build();
        SETTINGS.bind("mpv-estimate", &imp.estimate_control.get(), "active").build();
        SETTINGS.bind("mpv-estimate-target-frame", &imp.estimate_spinrow.get(), "value").build();
        SETTINGS.bind("mpv-seek-backward-step", &imp.seek_backward_spinrow.get(), "value").build();
        SETTINGS.bind("mpv-seek-forward-step", &imp.seek_forward_spinrow.get(), "value").build();
        SETTINGS.bind("mpv-config", &imp.config_switchrow.get(), "active").build();
        SETTINGS.bind("mpv-show-buffer-speed", &imp.buffer_switchrow.get(), "active").build();
        SETTINGS.bind("mpv-force-stereo", &imp.stereo_switchrow.get(), "active").build();
        SETTINGS.bind("mpv-default-volume", &imp.volume_spinrow.get(), "value").build();
        SETTINGS.bind("mpv-cache-size", &imp.cachesize_spinrow.get(), "value").build();
        SETTINGS.bind("mpv-subtitle-size", &imp.mpv_sub_size_spinrow.get(), "value").build();
        SETTINGS.bind("mpv-audio-preferred-lang", &imp.preferred_audio_language_comborow.get(), "selected").build();
        SETTINGS.bind("mpv-subtitle-preferred-lang", &imp.preferred_subtitle_language_comborow.get(), "selected").build();
        SETTINGS.bind("is-auto-select-server", &imp.selectlastcontrol.get(), "active").build();
        SETTINGS.bind("threads", &imp.threadspinrow.get(), "value").build();

        imp.mpv_sub_font_button
            .set_font_desc(&gtk::pango::FontDescription::from_string(
                &SETTINGS.mpv_subtitle_font(),
            ));

        let action_group = gio::SimpleActionGroup::new();

        let action_video_end = gio::ActionEntry::builder("video-end")
            .parameter_type(Some(&i32::static_variant_type()))
            .state(SETTINGS.mpv_action_after_video_end().to_variant())
            .activate(move |_, action, parameter| {
                let parameter = parameter
                    .expect("Could not get parameter.")
                    .get::<i32>()
                    .expect("The variant needs to be of type `i32`.");

                SETTINGS.set_mpv_action_after_video_end(parameter).unwrap();

                action.set_state(&parameter.to_variant());
            })
            .build();

        let action_vo = gio::ActionEntry::builder("video-output")
            .parameter_type(Some(&i32::static_variant_type()))
            .state(SETTINGS.mpv_video_output().to_variant())
            .activate(move |_, action, parameter| {
                let parameter = parameter
                    .expect("Could not get parameter.")
                    .get::<i32>()
                    .expect("The variant needs to be of type `i32`.");

                SETTINGS.set_mpv_video_output(parameter).unwrap();

                action.set_state(&parameter.to_variant());
            })
            .build();

        let action_hwdec = gio::ActionEntry::builder("hwdec")
            .parameter_type(Some(&i32::static_variant_type()))
            .state(SETTINGS.mpv_hwdec().to_variant())
            .activate(move |_, action, parameter| {
                let parameter = parameter
                    .expect("Could not get parameter.")
                    .get::<i32>()
                    .expect("The variant needs to be of type `i32`.");

                SETTINGS.set_mpv_hwdec(parameter).unwrap();

                action.set_state(&parameter.to_variant());
            })
            .build();

        action_group.add_action_entries([action_video_end, action_vo, action_hwdec]);
        self.insert_action_group("setting", Some(&action_group));
    }

    #[template_callback]
    fn subpage_activated_cb(&self) {
        let subpage = self.imp().video_subpage.get();
        self.push_subpage(&subpage);
    }

    #[template_callback]
    fn preferred_subpage_activated_cb(&self) {
        let subpage = self.imp().preferred_version_subpage.get();
        self.push_subpage(&subpage);
    }

    #[template_callback]
    fn preferred_add_button_cb(&self) {
        let dialog = self.imp().add_version_preferences_dialog.get();
        dialog.present(Some(self));
    }

    #[template_callback]
    pub fn on_mpvsub_font_dialog_button(
        &self,
        _param: glib::ParamSpec,
        button: gtk::FontDialogButton,
    ) {
        let font_desc = button.font_desc().unwrap();
        SETTINGS
            .set_mpv_subtitle_font(gtk::pango::FontDescription::to_string(&font_desc))
            .unwrap();
    }

    #[template_callback]
    fn on_descriptor_type_changed_comborow(&self, _param: glib::ParamSpec, combo: adw::ComboRow) {
        match combo.selected() {
            0 => {
                self.imp().descriptor_string_label.set_visible(true);
                self.imp().descriptor_regex_label.set_visible(false);
            }
            1 => {
                self.imp().descriptor_string_label.set_visible(false);
                self.imp().descriptor_regex_label.set_visible(true);
            }
            _ => unreachable!(), 
        }
    }

    pub fn add_preferred_version(&self) {
        let imp = self.imp();
        match imp.descriptor_type_comborow.selected() {
            0 => {
                let descriptor = imp.descriptor_entryrow.text();
                if descriptor.is_empty() {
                    toast!(self, gettext("Descriptor cannot be empty!"));
                    return;
                }
                
            }
            1 => {
                let descriptor = imp.descriptor_entryrow.text();
                if descriptor.is_empty() {
                    toast!(self, gettext("Descriptor cannot be empty!"));
                    return;
                }
                match regex::Regex::new(&descriptor) {
                    Ok(_) => {},
                    Err(e) => toast!(self, &format!("{}: {}", gettext("Invalid regex"), e)),
                }
            }
            _ => unreachable!(),
        }
    }
}
