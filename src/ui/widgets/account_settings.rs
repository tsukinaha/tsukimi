#![allow(deprecated)]

use super::utils::GlobalToast;
use crate::{
    client::jellyfin_client::JELLYFIN_CLIENT,
    ui::{
        models::{
            SETTINGS,
            jellyfin_cache_path,
        },
        provider::descriptor::{
            Descriptor,
            DescriptorType,
        },
    },
    utils::{
        spawn,
        spawn_tokio,
    },
};
use adw::{
    prelude::*,
    subclass::prelude::*,
};
use gettextrs::gettext;
use glib::translate::IntoGlib;
use gtk::{
    CompositeTemplate,
    gdk::{
        DragAction,
        RGBA,
    },
    gio,
    glib,
    template_callbacks,
};

mod imp {
    use std::cell::{
        Cell,
        OnceCell,
        RefCell,
    };

    use glib::subclass::InitializingObject;

    use crate::{
        Window,
        ui::widgets::action_row::AActionRow,
    };

    use super::*;

    #[derive(Debug, Default, CompositeTemplate, glib::Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/account_settings.ui")]
    #[properties(wrapper_type = super::AccountSettings)]
    pub struct AccountSettings {
        #[property(get, set, construct_only)]
        pub window: OnceCell<Window>,
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
        pub refresh_control: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub merge_resume_next_up_control: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub selectlastcontrol: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub text_display_group: TemplateChild<adw::ToggleGroup>,
        #[template_child]
        pub card_style_group: TemplateChild<adw::ToggleGroup>,
        #[template_child]
        pub custom_accent_color_control: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub backgroundblurspinrow: TemplateChild<adw::SpinRow>,
        #[template_child]
        pub backgroundblurcontrol: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub backgroundcontrol: TemplateChild<gtk::Switch>,
        #[template_child]
        pub color: TemplateChild<gtk::ColorDialogButton>,
        #[template_child]
        pub config_switchrow: TemplateChild<adw::SwitchRow>,

        #[template_child]
        pub preferred_audio_language_comborow: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub preferred_subtitle_language_comborow: TemplateChild<adw::ComboRow>,

        #[template_child]
        pub preferred_version_subpage: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub add_version_preferences_dialog: TemplateChild<adw::Dialog>,

        #[template_child]
        pub descriptor_string_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub descriptor_regex_label: TemplateChild<gtk::Label>,

        #[template_child]
        pub descriptor_string_label_edit: TemplateChild<gtk::Label>,
        #[template_child]
        pub descriptor_regex_label_edit: TemplateChild<gtk::Label>,

        #[template_child]
        pub descriptor_type_comborow: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub descriptor_entryrow: TemplateChild<adw::EntryRow>,

        #[template_child]
        pub descriptor_type_comborow_edit: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub descriptor_entryrow_edit: TemplateChild<adw::EntryRow>,

        #[template_child]
        pub descriptors_listbox: TemplateChild<gtk::ListBox>,

        #[template_child]
        pub preferred_version_list_stack: TemplateChild<gtk::Stack>,

        #[template_child]
        pub edit_descriptor_dialog: TemplateChild<adw::Dialog>,

        #[template_child]
        pub avatar: TemplateChild<adw::Avatar>,

        #[template_child]
        pub folder_dialog: TemplateChild<gtk::FileDialog>,

        #[template_child]
        pub folder_button_content: TemplateChild<adw::ButtonContent>,

        pub now_editing_descriptor: RefCell<Option<Descriptor>>,

        pub descriptor_grab_x: Cell<f64>,
        pub descriptor_grab_y: Cell<f64>,

        pub preference_pages: RefCell<Vec<adw::PreferencesPage>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AccountSettings {
        const NAME: &'static str = "AccountSettings";
        type Type = super::AccountSettings;
        type ParentType = adw::PreferencesWindow;

        fn class_init(klass: &mut Self::Class) {
            AActionRow::ensure_type();
            klass.bind_template();
            klass.bind_template_instance_callbacks();
            klass.install_action_async(
                "setting.clear",
                None,
                |set, _action, _parameter| async move {
                    set.cacheclear().await;
                },
            );
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
                "version.add-prefer",
                None,
                move |set, _action, _parameter| {
                    set.add_preferred_version();
                },
            );
            klass.install_action(
                "version.edit-prefer",
                None,
                move |set, _action, _parameter| {
                    set.edit_preferred_version();
                },
            );
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for AccountSettings {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.set_sidebar();
            obj.set_picopactiy();
            obj.set_pic();
            obj.set_color();
            obj.bind_settings();
            obj.setup_htpc_settings();
            obj.setup_playback_and_subtitle_settings();
            obj.setup_osk_entries();
            obj.refersh_descriptors();
            obj.cache_preference_pages();
            obj.setup_tv_controller_navigation();
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
    @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget, adw::PreferencesWindow, adw::Window,
    @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
        gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

#[template_callbacks]
impl AccountSettings {
    pub fn new(window: crate::Window) -> Self {
        glib::Object::builder().property("window", window).build()
    }

    pub fn preference_pages(&self) -> Vec<adw::PreferencesPage> {
        self.imp().preference_pages.borrow().clone()
    }

    fn cache_preference_pages(&self) {
        *self.imp().preference_pages.borrow_mut() =
            crate::ui::input::settings_navigator::find_view_stack_pages(
                self.upcast_ref::<adw::PreferencesWindow>(),
            );
    }

    fn setup_tv_controller_navigation(&self) {
        if !crate::tv::focus::tv_focus_enabled() {
            return;
        }
        let settings = self.clone();
        let parent = self.window();
        let keys = gtk::EventControllerKey::new();
        keys.connect_key_pressed(move |_, keyval, _, _| {
            if !crate::tv::controller_navigation_enabled() {
                return glib::Propagation::Proceed;
            }
            let Some(action) = crate::ui::input::key_to_action(keyval.into_glib()) else {
                return glib::Propagation::Proceed;
            };
            if parent
                .imp()
                .settings_navigator
                .borrow()
                .handle_window(&settings, action)
            {
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        self.add_controller(keys);

        let pointer = gtk::EventControllerLegacy::new();
        pointer.connect_event(move |_, event| {
            if event.event_type() == gtk::gdk::EventType::ButtonPress
                || event.event_type() == gtk::gdk::EventType::ButtonRelease
                || event.event_type() == gtk::gdk::EventType::MotionNotify
            {
                crate::tv::osk::mark_pointer_input();
            }
            glib::Propagation::Proceed
        });
        self.add_controller(pointer);
    }

    #[template_callback]
    async fn on_change_password(&self, _button: gtk::Button) {
        let new_password = self.imp().password_entry.text();
        let new_password_second = self.imp().password_second_entry.text();
        if new_password.is_empty() || new_password_second.is_empty() {
            self.toast(gettext("Password cannot be empty!"));
            return;
        }
        if new_password != new_password_second {
            self.toast(gettext("Passwords do not match!"));
            return;
        }
        match spawn_tokio(async move { JELLYFIN_CLIENT.change_password(&new_password).await }).await
        {
            Ok(_) => {
                self.toast(gettext(
                    "Password changed successfully! Please login again.",
                ));
            }
            Err(e) => {
                self.toast(format!("{}: {}", gettext("Failed to change password"), e));
            }
        };
    }

    pub fn setup_osk_entries(&self) {
        let imp = self.imp();
        crate::tv::osk::attach_on_screen_keyboard(&imp.password_entry.get());
        crate::tv::osk::attach_on_screen_keyboard(&imp.password_second_entry.get());
    }

    pub fn setup_htpc_settings(&self) {
        let page = adw::PreferencesPage::new();
        page.set_title(&gettext("TV / HTPC"));
        page.set_name(Some("tv-htpc"));
        page.set_icon_name(Some("preferences-desktop-remote-desktop-symbolic"));

        let controls = adw::PreferencesGroup::new();
        controls.set_title(&gettext("Controls"));

        let gamepad_row = adw::SwitchRow::new();
        gamepad_row.set_title(&gettext("Enable Gamepad"));
        gamepad_row.set_subtitle(&gettext(
            "Xbox, PlayStation, Steam Deck, and other controllers",
        ));
        gamepad_row.set_active(SETTINGS.gamepad_enabled());
        gamepad_row.connect_active_notify(|row| {
            let _ = SETTINGS.set_gamepad_enabled(row.is_active());
            if !row.is_active() {
                crate::tv::cursor::restore();
            }
        });
        controls.add(&gamepad_row);

        let tv_group = adw::PreferencesGroup::new();
        tv_group.set_title(&gettext("TV / HTPC"));

        let tv_mode_row = adw::SwitchRow::new();
        tv_mode_row.set_title(&gettext("TV Mode"));
        tv_mode_row.set_subtitle(&gettext("Larger UI scaled for couch viewing"));
        tv_mode_row.set_active(SETTINGS.tv_mode());
        tv_mode_row.connect_active_notify(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            move |row| {
                let active = row.is_active();
                let _ = SETTINGS.set_tv_mode(active);
                crate::tv::set_tv_mode_active(active);
                let window = obj.window();
                if active {
                    crate::tv::apply_to_window(&window, false);
                } else {
                    crate::tv::remove_from_window(&window);
                }
            }
        ));
        tv_group.add(&tv_mode_row);

        let tv_scale_adjustment =
            gtk::Adjustment::new(SETTINGS.tv_ui_scale(), 1.0, 2.0, 0.05, 0.1, 0.0);
        let tv_scale_row = adw::SpinRow::new(Some(&tv_scale_adjustment), 0.05, 2);
        tv_scale_row.set_title(&gettext("TV UI Scale"));
        tv_scale_row.set_subtitle(&gettext("Multiplier for poster and control sizes"));
        tv_scale_row.set_range(1.0, 2.0);
        tv_scale_row.set_value(SETTINGS.tv_ui_scale());
        tv_scale_row.connect_value_notify(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            move |row| {
                let _ = SETTINGS.set_tv_ui_scale(row.value());
                if crate::tv::is_tv_mode_active() {
                    crate::tv::sync_tv_style();
                    let window = obj.window();
                    window.enable_tv_mode_ui(false);
                }
            }
        ));
        tv_group.add(&tv_scale_row);

        let tv_fullscreen_row = adw::SwitchRow::new();
        tv_fullscreen_row.set_title(&gettext("Start Fullscreen in TV Mode"));
        tv_fullscreen_row.set_active(SETTINGS.tv_start_fullscreen());
        tv_fullscreen_row.connect_active_notify(|row| {
            let _ = SETTINGS.set_tv_start_fullscreen(row.is_active());
        });
        tv_group.add(&tv_fullscreen_row);

        let tv_sidebar_row = adw::SwitchRow::new();
        tv_sidebar_row.set_title(&gettext("Hide Sidebar in TV Mode"));
        tv_sidebar_row.set_active(SETTINGS.tv_hide_sidebar());
        tv_sidebar_row.connect_active_notify(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            move |row| {
                let _ = SETTINGS.set_tv_hide_sidebar(row.is_active());
                if crate::tv::is_tv_mode_active() {
                    let window = obj.window();
                    if row.is_active() {
                        window.set_sidebar_panel_visible(false);
                    } else {
                        window.overlay_sidebar(SETTINGS.overlay());
                        window.imp().split_view.set_show_sidebar(true);
                    }
                }
            }
        ));
        tv_group.add(&tv_sidebar_row);

        let tv_hints_row = adw::SwitchRow::new();
        tv_hints_row.set_title(&gettext("Show Controller Hints"));
        tv_hints_row.set_active(SETTINGS.tv_show_button_hints());
        tv_hints_row.connect_active_notify(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            move |row| {
                let _ = SETTINGS.set_tv_show_button_hints(row.is_active());
                obj.window().sync_tv_button_hints();
            }
        ));
        tv_group.add(&tv_hints_row);

        let steam_group = adw::PreferencesGroup::new();
        steam_group.set_title(&gettext("Steam"));

        let steam_row = adw::ActionRow::new();
        steam_row.set_title(&gettext("Add to Steam"));
        steam_row.set_subtitle(&gettext(
            "Add Tsukimi to your Steam library for Big Picture launch",
        ));
        let steam_button = gtk::Button::with_label(&gettext("Add"));
        steam_button.set_valign(gtk::Align::Center);
        steam_row.add_suffix(&steam_button);
        steam_button.connect_clicked(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            move |_| {
                let window = obj.window();
                match crate::steam::shortcuts::add_tsukimi_to_steam(&window) {
                    Ok(()) => {}
                    Err(e) => obj.toast(e),
                }
            }
        ));
        steam_group.add(&steam_row);

        page.add(&controls);
        page.add(&tv_group);
        page.add(&steam_group);
        self.add(&page);
    }

    pub fn setup_playback_and_subtitle_settings(&self) {
        use crate::{
            playback::{
                PlaybackRuleEditor,
                rules::{
                    LanguageCondition,
                    PlaybackRule,
                    PlaybackRulesConfig,
                    SubtitleOutcome,
                },
            },
            tv::osk,
        };

        fn rule_summary(rule: &PlaybackRule) -> String {
            let subtitles = match &rule.then.subtitles {
                SubtitleOutcome::Off => gettext("Subs: off"),
                SubtitleOutcome::Forced { language } if language.is_empty() => {
                    gettext("Subs: forced")
                }
                SubtitleOutcome::Forced { language } => {
                    format!("{} {language}", gettext("Subs: forced"))
                }
                SubtitleOutcome::Full { language } if language.is_empty() => gettext("Subs: full"),
                SubtitleOutcome::Full { language } => {
                    format!("{} {language}", gettext("Subs: full"))
                }
                SubtitleOutcome::PreferLanguage { language } => {
                    format!("{} {language}", gettext("Subs:"))
                }
            };
            let when = match &rule.when.audio_language {
                LanguageCondition::Any => gettext("When audio: any"),
                LanguageCondition::Equals(language) => {
                    format!("{} = {language}", gettext("When audio"))
                }
                LanguageCondition::NotEquals(language) => {
                    format!("{} ≠ {language}", gettext("When audio"))
                }
            };
            format!("{when} → {subtitles}")
        }

        let page = adw::PreferencesPage::new();
        page.set_title(&gettext("Playback Rules"));
        page.set_name(Some("playback-rules"));
        page.set_icon_name(Some("media-playlist-repeat-symbolic"));

        let rules_group = adw::PreferencesGroup::new();
        rules_group.set_title(&gettext("Conditional Tracks"));

        let enable_row = adw::SwitchRow::new();
        enable_row.set_title(&gettext("Enable Playback Rules"));
        enable_row.set_subtitle(&gettext(
            "Choose subtitle tracks based on the audio language",
        ));

        let rules_list = gtk::ListBox::new();
        rules_list.set_selection_mode(gtk::SelectionMode::None);
        rules_list.add_css_class("boxed-list");

        let config = SETTINGS.playback_conditional_rules();
        enable_row.set_active(config.enabled);

        let settings = self.clone();
        let refresh_rules_for_init = std::rc::Rc::new(std::cell::RefCell::new(
            None::<std::rc::Rc<dyn Fn(&PlaybackRulesConfig)>>,
        ));
        let rebuild = {
            let rules_list = rules_list.clone();
            let enable_row = enable_row.clone();
            let refresh_rules_for_init = refresh_rules_for_init.clone();
            let settings = settings.clone();
            std::rc::Rc::new(move |config: &PlaybackRulesConfig| {
                while let Some(child) = rules_list.first_child() {
                    rules_list.remove(&child);
                }
                let mut rules = config.rules.clone();
                rules.sort_by_key(|rule| rule.priority);
                for rule in rules {
                    let row = adw::ActionRow::new();
                    row.set_title(&rule_summary(&rule));
                    row.set_subtitle(&format!("{} {}", gettext("Priority"), rule.priority));
                    row.set_activatable(true);

                    let edit_button = gtk::Button::from_icon_name("document-edit-symbolic");
                    edit_button.set_valign(gtk::Align::Center);
                    edit_button.add_css_class("flat");
                    edit_button.set_tooltip_text(Some(&gettext("Edit")));

                    let remove_button = gtk::Button::from_icon_name("user-trash-symbolic");
                    remove_button.set_valign(gtk::Align::Center);
                    remove_button.add_css_class("flat");
                    remove_button.set_tooltip_text(Some(&gettext("Remove")));
                    row.add_suffix(&edit_button);
                    row.add_suffix(&remove_button);
                    rules_list.append(&row);

                    let priority = rule.priority;
                    let rule_snapshot = rule.clone();
                    let enable_row_for_edit = enable_row.clone();
                    let rebuild_for_edit = refresh_rules_for_init.borrow().clone();
                    edit_button.connect_clicked(glib::clone!(
                        #[weak]
                        settings,
                        #[strong]
                        rule_snapshot,
                        move |_| {
                            let editor = PlaybackRuleEditor::edit_rule_dialog(&rule_snapshot);
                            editor.present(Some(&settings));
                            let priority = rule_snapshot.priority;
                            let enable_row = enable_row_for_edit.clone();
                            let rebuild = rebuild_for_edit.clone();
                            editor.connect_save(move |updated| {
                                let mut config = SETTINGS.playback_conditional_rules();
                                if let Some(entry) = config
                                    .rules
                                    .iter_mut()
                                    .find(|entry| entry.priority == priority)
                                {
                                    *entry = updated;
                                }
                                let _ = SETTINGS.set_playback_conditional_rules(&config);
                                enable_row.set_active(config.enabled);
                                if let Some(rebuild) = rebuild.as_ref() {
                                    rebuild(&config);
                                }
                            });
                        }
                    ));

                    let enable_row_for_remove = enable_row.clone();
                    let rebuild_for_remove = refresh_rules_for_init.borrow().clone();
                    remove_button.connect_clicked(move |_| {
                        let mut updated = SETTINGS.playback_conditional_rules();
                        updated.rules.retain(|entry| entry.priority != priority);
                        let _ = SETTINGS.set_playback_conditional_rules(&updated);
                        enable_row_for_remove.set_active(updated.enabled);
                        if let Some(rebuild) = rebuild_for_remove.as_ref() {
                            rebuild(&updated);
                        }
                    });
                }
            })
        };
        *refresh_rules_for_init.borrow_mut() = Some(rebuild.clone());
        rebuild(&config);

        rules_group.add(&enable_row);
        rules_group.add(&rules_list);

        let add_row = adw::ButtonRow::new();
        add_row.set_title(&gettext("Add Rule"));
        let rebuild_for_add = rebuild.clone();
        let settings_for_add = self.clone();
        let settings_weak = settings_for_add.downgrade();
        let enable_weak = enable_row.downgrade();
        add_row.connect_activated(move |_| {
            let Some(settings) = settings_weak.upgrade() else {
                return;
            };
            let updated = SETTINGS.playback_conditional_rules();
            let next_priority = updated
                .rules
                .iter()
                .map(|rule| rule.priority)
                .max()
                .unwrap_or(0)
                + 1;
            let editor = PlaybackRuleEditor::new_rule_dialog(next_priority);
            editor.present(Some(&settings));
            let rebuild_for_add = rebuild_for_add.clone();
            let enable_weak = enable_weak.clone();
            editor.connect_save(move |rule| {
                let mut config = SETTINGS.playback_conditional_rules();
                config.rules.push(rule);
                let _ = SETTINGS.set_playback_conditional_rules(&config);
                if let Some(enable_row) = enable_weak.upgrade() {
                    enable_row.set_active(config.enabled);
                }
                rebuild_for_add(&config);
            });
        });
        rules_group.add(&add_row);

        let default_row = adw::ActionRow::new();
        default_row.set_title(&gettext("Edit Default Outcome"));
        default_row.set_subtitle(&gettext("Used when no rule matches"));
        default_row.set_activatable(true);
        let rebuild_for_default = rebuild.clone();
        let settings_for_default = self.clone();
        let settings_weak = settings_for_default.downgrade();
        default_row.connect_activated(move |_| {
            let Some(settings) = settings_weak.upgrade() else {
                return;
            };
            let config = SETTINGS.playback_conditional_rules();
            let editor = PlaybackRuleEditor::default_outcome_dialog(&config);
            editor.present(Some(&settings));
            let rebuild_for_default = rebuild_for_default.clone();
            editor.connect_save(move |rule| {
                let mut config = SETTINGS.playback_conditional_rules();
                config.default = rule.then;
                let _ = SETTINGS.set_playback_conditional_rules(&config);
                rebuild_for_default(&config);
            });
        });
        rules_group.add(&default_row);

        enable_row.connect_active_notify(|row| {
            let mut updated = SETTINGS.playback_conditional_rules();
            updated.enabled = row.is_active();
            let _ = SETTINGS.set_playback_conditional_rules(&updated);
        });

        let subtitle_page = adw::PreferencesPage::new();
        subtitle_page.set_title(&gettext("Subtitle Providers"));
        subtitle_page.set_name(Some("subtitle-providers"));
        subtitle_page.set_icon_name(Some("text-x-generic-symbolic"));

        let provider_group = adw::PreferencesGroup::new();
        provider_group.set_title(&gettext("Online Subtitles"));

        let opensubtitles_row = adw::PasswordEntryRow::new();
        opensubtitles_row.set_title(&gettext("OpenSubtitles API Key"));
        opensubtitles_row.set_text(&SETTINGS.opensubtitles_api_key());
        osk::attach_on_screen_keyboard(&opensubtitles_row);
        opensubtitles_row.connect_changed(|row| {
            let _ = SETTINGS.set_opensubtitles_api_key(row.text().as_ref());
        });
        provider_group.add(&opensubtitles_row);

        let subdl_row = adw::PasswordEntryRow::new();
        subdl_row.set_title(&gettext("SubDL API Key"));
        subdl_row.set_text(&SETTINGS.subdl_api_key());
        osk::attach_on_screen_keyboard(&subdl_row);
        subdl_row.connect_changed(|row| {
            let _ = SETTINGS.set_subdl_api_key(row.text().as_ref());
        });
        provider_group.add(&subdl_row);

        let opensubtitles_switch = adw::SwitchRow::new();
        opensubtitles_switch.set_title(&gettext("Enable OpenSubtitles"));
        opensubtitles_switch.set_active(SETTINGS.subtitle_provider_enabled("opensubtitles"));
        opensubtitles_switch.connect_active_notify(|row| {
            let mut providers = SETTINGS
                .string("subtitle-providers-enabled")
                .split(',')
                .map(str::trim)
                .filter(|entry| !entry.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>();
            if row.is_active() {
                if !providers.iter().any(|entry| entry == "opensubtitles") {
                    providers.push("opensubtitles".into());
                }
            } else {
                providers.retain(|entry| entry != "opensubtitles");
            }
            let _ = SETTINGS.set_string("subtitle-providers-enabled", &providers.join(","));
        });
        provider_group.add(&opensubtitles_switch);

        let subdl_switch = adw::SwitchRow::new();
        subdl_switch.set_title(&gettext("Enable SubDL"));
        subdl_switch.set_active(SETTINGS.subtitle_provider_enabled("subdl"));
        subdl_switch.connect_active_notify(|row| {
            let mut providers = SETTINGS
                .string("subtitle-providers-enabled")
                .split(',')
                .map(str::trim)
                .filter(|entry| !entry.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>();
            if row.is_active() {
                if !providers.iter().any(|entry| entry == "subdl") {
                    providers.push("subdl".into());
                }
            } else {
                providers.retain(|entry| entry != "subdl");
            }
            let _ = SETTINGS.set_string("subtitle-providers-enabled", &providers.join(","));
        });
        provider_group.add(&subdl_switch);

        page.add(&rules_group);
        subtitle_page.add(&provider_group);
        self.add(&page);
        self.add(&subtitle_page);
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
                let _ = SETTINGS.set_overlay(control.is_active());
            }
        ));
    }

    pub fn set_color(&self) {
        let imp = self.imp();
        use std::str::FromStr;
        imp.color
            .set_rgba(&RGBA::from_str(&SETTINGS.accent_color_code()).unwrap());
        imp.color.connect_rgba_notify(move |control| {
            let _ = SETTINGS.set_accent_color_code(&control.rgba().to_string());
        });
    }

    pub async fn cacheclear(&self) {
        let path = jellyfin_cache_path().await;
        if path.exists() {
            std::fs::remove_dir_all(path).unwrap();
        }
        self.toast(gettext("Cache Cleared"))
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
                let _ = SETTINGS.set_root_pic(&file_path);
                window.set_rootpic(file);
            }
            Err(_) => self.toast(gettext("No file selected")),
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
                let _ = SETTINGS.set_pic_opacity(control.value() as i32);
                let window = obj.window();
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
                let _ = SETTINGS.set_background_enabled(control.is_active());
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
        let _ = SETTINGS.set_root_pic("");
    }

    pub fn bind_settings(&self) {
        let imp = self.imp();
        SETTINGS
            .bind("is-blurenabled", &imp.backgroundblurcontrol.get(), "active")
            .build();
        SETTINGS
            .bind("pic-blur", &imp.backgroundblurspinrow.get(), "value")
            .build();
        SETTINGS
            .bind("mpv-config", &imp.config_switchrow.get(), "active")
            .build();
        SETTINGS
            .bind(
                "mpv-audio-preferred-lang",
                &imp.preferred_audio_language_comborow.get(),
                "selected",
            )
            .build();
        SETTINGS
            .bind(
                "mpv-subtitle-preferred-lang",
                &imp.preferred_subtitle_language_comborow.get(),
                "selected",
            )
            .build();
        SETTINGS
            .bind(
                "is-auto-select-server",
                &imp.selectlastcontrol.get(),
                "active",
            )
            .build();
        SETTINGS
            .bind(
                "use-custom-accent-color",
                &imp.custom_accent_color_control.get(),
                "active",
            )
            .build();
        imp.text_display_group
            .set_active_name(Some(SETTINGS.item_text_display().as_str()));
        imp.card_style_group
            .set_active_name(Some(SETTINGS.item_card_style().as_str()));
        imp.text_display_group.connect_active_name_notify(|group| {
            if let Some(active_name) = group.active_name() {
                let _ = SETTINGS.set_item_text_display(&active_name);
            }
        });
        imp.card_style_group.connect_active_name_notify(|group| {
            if let Some(active_name) = group.active_name() {
                let _ = SETTINGS.set_item_card_style(&active_name);
            }
        });
        SETTINGS
            .bind("use-custom-accent-color", &imp.color.get(), "sensitive")
            .build();
        SETTINGS
            .bind("mpv-config-path", &imp.folder_button_content.get(), "label")
            .build();
        SETTINGS
            .bind("threads", &imp.threadspinrow.get(), "value")
            .build();
        SETTINGS
            .bind("is-refresh", &imp.refresh_control.get(), "active")
            .build();
        SETTINGS
            .bind(
                "merge-resume-and-next-up",
                &imp.merge_resume_next_up_control.get(),
                "active",
            )
            .build();

        let action_group = gio::SimpleActionGroup::new();

        let action_vo = gio::ActionEntry::builder("video-output")
            .parameter_type(Some(&i32::static_variant_type()))
            .state(SETTINGS.mpv_video_output().to_variant())
            .activate(move |_, action, parameter| {
                let parameter = parameter
                    .expect("Could not get parameter.")
                    .get::<i32>()
                    .expect("The variant needs to be of type `i32`.");

                let _ = SETTINGS.set_mpv_video_output(parameter);

                action.set_state(&parameter.to_variant());
            })
            .build();

        action_group.add_action_entries([action_vo]);
        self.insert_action_group("setting", Some(&action_group));

        if JELLYFIN_CLIENT.session().account.user_id.is_empty() {
            return;
        }

        spawn(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                let avatar =
                    match spawn_tokio(async move { JELLYFIN_CLIENT.get_user_avatar().await }).await
                    {
                        Ok(avatar) => avatar,
                        Err(e) => {
                            obj.toast(e.to_string());
                            return;
                        }
                    };

                let Some(texture) = gtk::gdk::Texture::from_file(&gio::File::for_path(avatar)).ok()
                else {
                    return;
                };

                obj.imp().avatar.set_custom_image(Some(&texture));
            }
        ));
    }

    #[template_callback]
    fn preferred_subpage_activated_cb(&self) {
        let subpage = self.imp().preferred_version_subpage.get();
        self.push_subpage(&subpage);
    }

    #[template_callback]
    fn preferred_add_button_cb(&self) {
        let imp = self.imp();
        let dialog = imp.add_version_preferences_dialog.get();

        // Reset the dialog
        imp.descriptor_entryrow.set_text("");

        dialog.present(Some(self));
    }

    #[template_callback]
    pub fn on_mpvsub_font_dialog_button(
        &self, _param: glib::ParamSpec, button: gtk::FontDialogButton,
    ) {
        let font_desc = button.font_desc().unwrap();
        let _ = SETTINGS.set_mpv_subtitle_font(gtk::pango::FontDescription::to_string(&font_desc));
    }

    #[template_callback]
    async fn dir_cb(&self, _button: gtk::Button) {
        if let Ok(file) = self
            .imp()
            .folder_dialog
            .select_folder_future(Some(self))
            .await
        {
            self.imp().folder_button_content.set_label(
                &file
                    .path()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or("None".into()),
            );
        }
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

    #[template_callback]
    fn on_descriptor_type_changed_comborow_edit(
        &self, _param: glib::ParamSpec, combo: adw::ComboRow,
    ) {
        match combo.selected() {
            0 => {
                self.imp().descriptor_string_label_edit.set_visible(true);
                self.imp().descriptor_regex_label_edit.set_visible(false);
            }
            1 => {
                self.imp().descriptor_string_label_edit.set_visible(false);
                self.imp().descriptor_regex_label_edit.set_visible(true);
            }
            _ => unreachable!(),
        }
    }

    pub fn add_preferred_version(&self) {
        let imp = self.imp();
        let descriptor = match imp.descriptor_type_comborow.selected() {
            0 => {
                let descriptor_content = imp.descriptor_entryrow.text();
                if descriptor_content.is_empty() {
                    self.toast(gettext("Descriptor cannot be empty!"));
                    return;
                }

                Descriptor::new(descriptor_content.to_string(), DescriptorType::String)
            }
            1 => {
                let descriptor_content = imp.descriptor_entryrow.text();
                if descriptor_content.is_empty() {
                    self.toast(gettext("Descriptor cannot be empty!"));
                    return;
                }
                match regex::Regex::new(&descriptor_content) {
                    Ok(_) => {}
                    Err(e) => self.toast(format!("{}: {}", gettext("Invalid regex"), e)),
                }

                Descriptor::new(descriptor_content.to_string(), DescriptorType::Regex)
            }
            _ => unreachable!(),
        };

        SETTINGS
            .add_preferred_version_descriptor(descriptor)
            .expect("Failed to add descriptor");
        self.refersh_descriptors();

        imp.add_version_preferences_dialog.close();
    }

    pub fn edit_preferred_version(&self) {
        let imp = self.imp();

        let old_descriptor = imp
            .now_editing_descriptor
            .borrow()
            .to_owned()
            .expect("No descriptor to edit");

        let descriptor = match imp.descriptor_type_comborow_edit.selected() {
            0 => {
                let descriptor_content = imp.descriptor_entryrow_edit.text();
                if descriptor_content.is_empty() {
                    self.toast(gettext("Descriptor cannot be empty!"));
                    return;
                }

                Descriptor::new(descriptor_content.to_string(), DescriptorType::String)
            }
            1 => {
                let descriptor_content = imp.descriptor_entryrow_edit.text();
                if descriptor_content.is_empty() {
                    self.toast(gettext("Descriptor cannot be empty!"));
                    return;
                }
                match regex::Regex::new(&descriptor_content) {
                    Ok(_) => {}
                    Err(e) => self.toast(format!("{}: {}", gettext("Invalid regex"), e)),
                }

                Descriptor::new(descriptor_content.to_string(), DescriptorType::Regex)
            }
            _ => unreachable!(),
        };

        SETTINGS
            .edit_preferred_version_descriptor(old_descriptor, descriptor)
            .expect("Failed to edit descriptor");
        self.refersh_descriptors();

        imp.edit_descriptor_dialog.close();
    }

    fn refersh_descriptors(&self) {
        let imp = self.imp();
        let group = imp.descriptors_listbox.get();
        let descriptors = SETTINGS.preferred_version_descriptors();

        if descriptors.is_empty() {
            imp.preferred_version_list_stack
                .set_visible_child_name("empty");
            return;
        } else {
            imp.preferred_version_list_stack
                .set_visible_child_name("list");
        }

        group.remove_all();

        for (index, descriptor) in descriptors.iter().enumerate() {
            let row = adw::ActionRow::builder()
                .subtitle(descriptor.type_.to_string())
                .title(&descriptor.content)
                .activatable(true)
                .build();

            let edit_button = gtk::Button::builder()
                .icon_name("document-edit-symbolic")
                .valign(gtk::Align::Center)
                .css_classes(["flat"])
                .build();

            edit_button.connect_clicked(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                #[strong]
                descriptor,
                move |_| {
                    let dialog = obj.imp().edit_descriptor_dialog.get();
                    let imp = obj.imp();

                    imp.descriptor_entryrow_edit.set_text(&descriptor.content);
                    match descriptor.type_ {
                        DescriptorType::String => {
                            imp.descriptor_type_comborow_edit.set_selected(0);
                        }
                        DescriptorType::Regex => {
                            imp.descriptor_type_comborow_edit.set_selected(1);
                        }
                    }

                    imp.now_editing_descriptor
                        .replace(Some(descriptor.to_owned()));
                    dialog.present(Some(&obj));
                }
            ));

            let delete_button = gtk::Button::builder()
                .icon_name("user-trash-symbolic")
                .valign(gtk::Align::Center)
                .css_classes(["flat"])
                .build();

            delete_button.connect_clicked(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                #[strong]
                descriptor,
                move |_| {
                    SETTINGS
                        .remove_preferred_version_descriptor(descriptor.to_owned())
                        .expect("Failed to remove descriptor");
                    obj.refersh_descriptors();
                }
            ));

            let prefix_image = gtk::Image::builder()
                .icon_name("list-drag-handle-symbolic")
                .build();

            row.add_suffix(&edit_button);
            row.add_suffix(&delete_button);
            row.add_prefix(&prefix_image);

            let drag_source = gtk::DragSource::builder()
                .name("descriptor-drag-format")
                .actions(DragAction::MOVE)
                .build();

            drag_source.connect_prepare(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                #[weak(rename_to = widget)]
                row,
                #[strong]
                descriptor,
                #[upgrade_or]
                None,
                move |drag_context, _x, _y| {
                    obj.imp().descriptors_listbox.drag_highlight_row(&widget);
                    let icon = gtk::WidgetPaintable::new(Some(&widget));
                    drag_context.set_icon(Some(&icon), 0, 0);
                    let object = glib::BoxedAnyObject::new(descriptor.to_owned());
                    Some(gtk::gdk::ContentProvider::for_value(&object.to_value()))
                }
            ));

            let drop_target = gtk::DropTarget::builder()
                .name("descriptor-drag-format")
                .propagation_phase(gtk::PropagationPhase::Capture)
                .actions(gtk::gdk::DragAction::MOVE)
                .build();

            drop_target.set_types(&[glib::BoxedAnyObject::static_type()]);

            drop_target.connect_drop(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                #[strong]
                descriptor,
                #[upgrade_or]
                false,
                move |_drop_target, value, _y, _data| {
                    let lr_descriptor = value
                        .get::<glib::BoxedAnyObject>()
                        .expect("Failed to get descriptor from drop data");
                    let lr_descriptor: std::cell::Ref<Descriptor> = lr_descriptor.borrow();

                    if descriptor == *lr_descriptor {
                        return false;
                    }

                    let mut descriptors = SETTINGS.preferred_version_descriptors();
                    let lr_index = descriptors
                        .iter()
                        .position(|d| *d == *lr_descriptor)
                        .unwrap();
                    descriptors.remove(lr_index);
                    descriptors.insert(index, lr_descriptor.to_owned());
                    let _ = SETTINGS.set_preferred_version_descriptors(descriptors);
                    obj.refersh_descriptors();

                    true
                }
            ));

            row.add_controller(drag_source);
            row.add_controller(drop_target);

            group.append(&row);
        }
    }
}
