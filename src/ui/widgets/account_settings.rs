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

    use crate::Window;

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
        pub selectlastcontrol: TemplateChild<adw::SwitchRow>,
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
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AccountSettings {
        const NAME: &'static str = "AccountSettings";
        type Type = super::AccountSettings;
        type ParentType = adw::PreferencesWindow;

        fn class_init(klass: &mut Self::Class) {
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
            obj.refersh_descriptors();
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
                SETTINGS.set_root_pic(&file_path).unwrap();
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
                SETTINGS.set_pic_opacity(control.value() as i32).unwrap();
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
            .bind("mpv-config-path", &imp.folder_button_content.get(), "label")
            .build();
        SETTINGS
            .bind("threads", &imp.threadspinrow.get(), "value")
            .build();
        SETTINGS
            .bind("is-refresh", &imp.refresh_control.get(), "active")
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

                SETTINGS.set_mpv_video_output(parameter).unwrap();

                action.set_state(&parameter.to_variant());
            })
            .build();

        action_group.add_action_entries([action_vo]);
        self.insert_action_group("setting", Some(&action_group));

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
        SETTINGS
            .set_mpv_subtitle_font(gtk::pango::FontDescription::to_string(&font_desc))
            .unwrap();
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
                    SETTINGS
                        .set_preferred_version_descriptors(descriptors)
                        .expect("Failed to set descriptors");
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
