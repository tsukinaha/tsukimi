use adw::subclass::prelude::*;
use gtk::{
    glib,
    prelude::*,
    template_callbacks,
};

use crate::{
    client::{
        error::UserFacingError,
        jellyfin_client::JELLYFIN_CLIENT,
    },
    ui::GlobalToast,
    utils::spawn_tokio,
};

mod imp {
    use std::cell::OnceCell;

    use adw::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        glib,
    };

    use super::*;
    use crate::{
        client::structs::ImageItem,
        ui::{
            provider::IS_ADMIN,
            widgets::image_dialog::ImageInfoCard,
        },
        utils::spawn,
    };

    #[derive(Debug, Default, CompositeTemplate, glib::Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/images_dialog.ui")]
    #[properties(wrapper_type = super::ImagesDialog)]
    pub struct ImagesDialog {
        #[property(get, set, construct_only)]
        pub id: OnceCell<String>,

        #[template_child]
        pub hint: TemplateChild<adw::ActionRow>,

        #[template_child]
        pub page: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub view: TemplateChild<adw::NavigationView>,

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
        pub wrapbox: TemplateChild<adw::WrapBox>,

        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,

        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,

        #[template_child]
        pub size_group: TemplateChild<gtk::SizeGroup>,
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

            let id = self.obj().id();

            self.primary.set_imgid(id.to_owned());
            self.logo.set_imgid(id.to_owned());
            self.thumb.set_imgid(id.to_owned());
            self.banner.set_imgid(id.to_owned());
            self.disc.set_imgid(id.to_owned());
            self.art.set_imgid(id.to_owned());

            self.size_group.add_widget(&self.primary.imp().stack.get());
            self.size_group.add_widget(&self.logo.imp().stack.get());
            self.size_group.add_widget(&self.thumb.imp().stack.get());
            self.size_group.add_widget(&self.banner.imp().stack.get());
            self.size_group.add_widget(&self.disc.imp().stack.get());
            self.size_group.add_widget(&self.art.imp().stack.get());

            self.init();
        }
    }

    impl WidgetImpl for ImagesDialog {}
    impl AdwDialogImpl for ImagesDialog {}

    impl ImagesDialog {
        fn init(&self) {
            if IS_ADMIN.load(std::sync::atomic::Ordering::Relaxed) {
                self.page.set_title(&gettextrs::gettext("View Images"));
                self.hint.set_visible(false);
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

        pub async fn set_card(&self, card: &ImageInfoCard, item: &ImageItem) {
            card.set_loading_visible();
            card.set_size(&item.width, &item.height, &item.size);
            card.set_picture(&item.image_type, &self.obj().id(), &None)
                .await;
        }

        pub async fn add_backdrop(&self, item: &ImageItem) {
            let card = ImageInfoCard::new("Backdrop", &self.obj().id());
            card.set_loading_visible();
            card.set_size(&item.width, &item.height, &item.size);
            card.set_picture(&item.image_type, &self.obj().id(), &item.image_index)
                .await;
            self.size_group.add_widget(&card.imp().stack.get());
            self.wrapbox.append(&card);
        }

        pub async fn set_item(&self, item: &ImageItem) {
            match item.image_type.as_str() {
                "Primary" => {
                    self.set_card(&self.primary, item).await;
                }
                "Logo" => {
                    self.set_card(&self.logo, item).await;
                }
                "Thumb" => {
                    self.set_card(&self.thumb, item).await;
                }
                "Banner" => {
                    self.set_card(&self.banner, item).await;
                }
                "Disc" => {
                    self.set_card(&self.disc, item).await;
                }
                "Art" => {
                    self.set_card(&self.art, item).await;
                }
                "Backdrop" => {
                    self.add_backdrop(item).await;
                }
                _ => {}
            }
        }
    }
}

glib::wrapper! {

    pub struct ImagesDialog(ObjectSubclass<imp::ImagesDialog>)
        @extends gtk::Widget, adw::Dialog, adw::PreferencesDialog, @implements gtk::Accessible, gtk::Root;
}

#[template_callbacks]
impl ImagesDialog {
    const LOADING_STACK_PAGE: &'static str = "loading";
    const VIEW_STACK_PAGE: &'static str = "view";

    pub fn new(id: &str) -> Self {
        glib::Object::builder().property("id", id).build()
    }

    pub fn loading_page(&self) {
        self.imp()
            .stack
            .set_visible_child_name(Self::LOADING_STACK_PAGE);
    }

    pub fn view_page(&self) {
        self.imp()
            .stack
            .set_visible_child_name(Self::VIEW_STACK_PAGE);
    }

    pub fn add_toast(&self, toast: adw::Toast) {
        self.imp().toast_overlay.add_toast(toast);
    }

    pub async fn set_image_items(&self) {
        let id = self.id();
        match spawn_tokio(async move { JELLYFIN_CLIENT.get_image_items(&id).await }).await {
            Ok(items) => {
                while let Some(item) = self.imp().wrapbox.first_child() {
                    self.imp().wrapbox.remove(&item);
                }

                for item in items {
                    self.imp().set_item(&item).await;
                }
            }
            Err(e) => {
                self.toast(e.to_user_facing());
            }
        }

        self.view_page();
    }

    pub fn pop_page(&self) {
        self.imp().view.pop();
    }

    #[template_callback]
    fn on_backdrop_search_clicked(&self) {
        let page = super::ImageDialogSearchPage::new(&self.id(), "Backdrop");
        self.imp().view.push(&page);
    }

    #[template_callback]
    fn on_backdrop_add_clicked(&self) {
        let page = super::ImageDialogEditPage::new(&self.id(), "Backdrop", 0);
        self.imp().view.push(&page);
    }
}
