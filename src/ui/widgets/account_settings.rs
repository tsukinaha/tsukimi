use adw::subclass::prelude::*;
use gtk::{glib, CompositeTemplate};



mod imp {
    use glib::subclass::InitializingObject;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/moe/tsukimi/account_settings.ui")]
    pub struct AccountSettings {

    }

    #[glib::object_subclass]
    impl ObjectSubclass for AccountSettings {
        const NAME: &'static str = "AccountSettings";
        type Type = super::AccountSettings;
        type ParentType = adw::PreferencesDialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
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

impl AccountSettings {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }
}
