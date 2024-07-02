use adw::subclass::prelude::*;
use gtk::{glib, CompositeTemplate};

mod imp {
    use super::*;
    use glib::subclass::InitializingObject;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/moe/tsukimi/metadata_dialog.ui")]
    pub struct MetadataDialog {}

    #[glib::object_subclass]
    impl ObjectSubclass for MetadataDialog {
        const NAME: &'static str = "MetadataDialog";
        type Type = super::MetadataDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MetadataDialog {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for MetadataDialog {}
    impl AdwDialogImpl for MetadataDialog {}
    impl PreferencesDialogImpl for MetadataDialog {}
}

glib::wrapper! {
    /// Preference Window to display and update room details.
    pub struct MetadataDialog(ObjectSubclass<imp::MetadataDialog>)
        @extends gtk::Widget, adw::Dialog, adw::PreferencesDialog, @implements gtk::Accessible;
}

impl Default for MetadataDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl MetadataDialog {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }
}
