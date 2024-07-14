use adw::subclass::prelude::*;
use gtk::glib;
use gtk::prelude::*;
use gtk::template_callbacks;

use crate::{
    client::{client::EMBY_CLIENT, error::UserFacingError},
    toast,
    utils::spawn_tokio,
};

mod imp {
    use super::*;
    use crate::{
        client::structs::ImageItem,
        ui::{provider::IS_ADMIN, widgets::image_infocard::ImageInfoCard},
        utils::spawn,
    };
    use adw::prelude::*;
    use glib::subclass::InitializingObject;

    use gtk::{glib, CompositeTemplate};
    use std::cell::OnceCell;

    #[derive(Debug, Default, CompositeTemplate, glib::Properties)]
    #[template(resource = "/moe/tsukimi/images_dialog.ui")]
    #[properties(wrapper_type = super::ImagesDialog)]
    pub struct ImagesDialog {
        #[property(get, set, construct_only)]
        pub id: OnceCell<String>,

        #[template_child]
        pub hint: TemplateChild<adw::ActionRow>,

        #[template_child]
        pub page: TemplateChild<adw::NavigationPage>,

        #[template_child]
        pub primary: TemplateChild<ImageInfoCard>,
        #[template_child]
        pub logo: TemplateChild<ImageInfoCard>,
        #[template_child]
        pub thumb: TemplateChild<ImageInfoCard>,
        #[template_child]
        pub banner: TemplateChild<ImageInfoCard>,
        #[template_child]
        pub disc: TemplateChild<ImageInfoCard>,
        #[template_child]
        pub art: TemplateChild<ImageInfoCard>,

        #[template_child]
        pub flowbox: TemplateChild<gtk::FlowBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImagesDialog {
        const NAME: &'static str = "ImagesDialog";
        type Type = super::ImagesDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            ImageInfoCard::ensure_type();
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ImagesDialog {
        fn constructed(&self) {
            self.parent_constructed();
            self.init();
        }
    }

    impl WidgetImpl for ImagesDialog {}
    impl AdwDialogImpl for ImagesDialog {}

    impl ImagesDialog {
        fn init(&self) {
            if IS_ADMIN.load(std::sync::atomic::Ordering::Relaxed) {
                self.page.set_title("View Images");
                self.hint
                    .set_subtitle("This page is READ-ONLY, because it is not finished yet.");
            }

            let obj = self.obj();
            spawn(glib::clone!(
                #[weak]
                obj,
                async move {
                    obj.set_image_items().await;
                }
            ));
        }

        pub fn set_card(&self, card: &ImageInfoCard, item: &ImageItem) {
            card.set_loading_visible();
            card.set_size(&item.width, &item.height, &item.size);
            card.set_picture(&item.image_type, &self.obj().id(), &None);
        }

        pub fn add_backdrop(&self, item: &ImageItem) {
            let card = ImageInfoCard::new("Backdrop");
            card.set_loading_visible();
            card.set_size(&item.width, &item.height, &item.size);
            card.set_picture(&item.image_type, &self.obj().id(), &item.image_index);
            self.flowbox.append(&card);
        }

        pub fn set_item(&self, item: &ImageItem) {
            match item.image_type.as_str() {
                "Primary" => {
                    self.set_card(&self.primary, item);
                }
                "Logo" => {
                    self.set_card(&self.logo, item);
                }
                "Thumb" => {
                    self.set_card(&self.thumb, item);
                }
                "Banner" => {
                    self.set_card(&self.banner, item);
                }
                "Disc" => {
                    self.set_card(&self.disc, item);
                }
                "Art" => {
                    self.set_card(&self.art, item);
                }
                "Backdrop" => {
                    self.add_backdrop(item);
                }
                _ => {}
            }
        }
    }
}

glib::wrapper! {
    /// Preference Window to display and update room details.
    pub struct ImagesDialog(ObjectSubclass<imp::ImagesDialog>)
        @extends gtk::Widget, adw::Dialog, adw::PreferencesDialog, @implements gtk::Accessible, gtk::Root;
}

#[template_callbacks]
impl ImagesDialog {
    pub fn new(id: &str) -> Self {
        glib::Object::builder().property("id", id).build()
    }

    pub async fn set_image_items(&self) {
        let id = self.id();
        match spawn_tokio(async move { EMBY_CLIENT.get_image_items(&id).await }).await {
            Ok(items) => {
                for item in items {
                    self.imp().set_item(&item);
                }
            }
            Err(e) => {
                toast!(self, e.to_user_facing());
            }
        }
    }
}
