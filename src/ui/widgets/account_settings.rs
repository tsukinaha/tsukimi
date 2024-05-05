use crate::{client::network::change_password, utils::spawn_tokio};
use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{glib, template_callbacks, CompositeTemplate};

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
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AccountSettings {
        const NAME: &'static str = "AccountSettings";
        type Type = super::AccountSettings;
        type ParentType = adw::PreferencesDialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AccountSettings {}

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
            let window = self.root().and_downcast::<super::window::Window>().unwrap();
            window.toast("Password cannot be empty!");
            return;
        }
        if new_password != new_password_second {
            let window = self.root().and_downcast::<super::window::Window>().unwrap();
            window.toast("Passwords do not match!");
            return;
        }
        match spawn_tokio(async move { change_password(&new_password).await }).await {
            Ok(_) => {
                let window = self.root().and_downcast::<super::window::Window>().unwrap();
                window.toast("Password changed successfully! Please login again.");
            }
            Err(e) => {
                let window = self.root().and_downcast::<super::window::Window>().unwrap();
                window.toast(&format!("Failed to change password: {}", e));
            }
        };
    }
}
