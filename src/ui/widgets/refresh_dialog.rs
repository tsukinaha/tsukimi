use adw::{
    prelude::*,
    subclass::prelude::*,
};
use gettextrs::gettext;
use gtk::{
    glib,
    template_callbacks,
};

use crate::{
    client::{
        error::UserFacingError,
        jellyfin_client::JELLYFIN_CLIENT,
    },
    utils::spawn_tokio,
};

use super::utils::GlobalToast;

mod imp {
    use std::cell::OnceCell;

    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        glib,
    };

    use super::*;

    #[derive(Debug, Default, CompositeTemplate, glib::Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/refresh_dialog.ui")]
    #[properties(wrapper_type = super::RefreshDialog)]
    pub struct RefreshDialog {
        #[property(get, set, construct_only)]
        pub id: OnceCell<String>,
        #[template_child]
        pub metadata_check: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub image_check: TemplateChild<gtk::CheckButton>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RefreshDialog {
        const NAME: &'static str = "RefreshDialog";
        type Type = super::RefreshDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for RefreshDialog {}

    impl WidgetImpl for RefreshDialog {}
    impl AdwDialogImpl for RefreshDialog {}
}

glib::wrapper! {

    pub struct RefreshDialog(ObjectSubclass<imp::RefreshDialog>)
        @extends gtk::Widget, adw::Dialog, adw::PreferencesDialog, @implements gtk::Accessible, gtk::Root;
}

#[template_callbacks]
impl RefreshDialog {
    pub fn new(id: &str) -> Self {
        glib::Object::builder().property("id", id).build()
    }

    #[template_callback]
    async fn on_refresh(&self) {
        let id = self.id();
        let imp = self.imp();
        let metadata = imp.metadata_check.is_active();
        let image = imp.image_check.is_active();

        match spawn_tokio(async move {
            JELLYFIN_CLIENT
                .fullscan(&id, &metadata.to_string(), &image.to_string())
                .await
        })
        .await
        {
            Ok(_) => {
                self.toast(gettext("Scanning..."));
            }
            Err(e) => {
                self.toast(e.to_user_facing());
            }
        }

        self.close();
    }
}
