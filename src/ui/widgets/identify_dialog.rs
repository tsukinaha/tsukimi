use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;
use gtk::template_callbacks;

use crate::client::structs::ExternalIdInfo;
use crate::utils::spawn;
use crate::{
    client::{client::EMBY_CLIENT, error::UserFacingError},
    toast,
    utils::spawn_tokio,
};

mod imp {
    use super::*;

    use glib::subclass::InitializingObject;

    use gtk::{glib, CompositeTemplate};
    use std::cell::OnceCell;

    #[derive(Debug, Default, CompositeTemplate, glib::Properties)]
    #[template(resource = "/moe/tsukimi/identify_dialog.ui")]
    #[properties(wrapper_type = super::IdentifyDialog)]
    pub struct IdentifyDialog {
        #[property(get, set, construct_only)]
        pub id: OnceCell<String>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub vbox: TemplateChild<gtk::Box>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for IdentifyDialog {
        const NAME: &'static str = "IdentifyDialog";
        type Type = super::IdentifyDialog;
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
    impl ObjectImpl for IdentifyDialog {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().init();
        }
    }

    impl WidgetImpl for IdentifyDialog {}
    impl AdwDialogImpl for IdentifyDialog {}

}

glib::wrapper! {
    /// Preference Window to display and update room details.
    pub struct IdentifyDialog(ObjectSubclass<imp::IdentifyDialog>)
        @extends gtk::Widget, adw::Dialog, adw::PreferencesDialog, @implements gtk::Accessible, gtk::Root;
}

#[template_callbacks]
impl IdentifyDialog {
    pub fn new(id: &str) -> Self {
        glib::Object::builder().property("id", id).build()
    }

    pub fn init(&self) {
        spawn(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            async move { obj.get_data().await }
        ));
    }

    async fn get_data(&self) {
        let id = self.id();
        match spawn_tokio(async move { EMBY_CLIENT.get_external_id_info(&id).await }).await {
            Ok(data) => {
                self.imp().stack.set_visible_child_name("page");
                self.load_data(data);
            }
            Err(e) => {
                toast!(self, e.to_user_facing());
            }
        }
    }

    fn load_data(&self, data: Vec<ExternalIdInfo>) {
        for info in data {
            let entry = adw::EntryRow::builder()
                .title(&info.name)
                .build();
            self.imp().vbox.append(&entry);
        }
    }

    #[template_callback]
    fn on_search(&self) {

    }
}
