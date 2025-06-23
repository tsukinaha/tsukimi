use std::io::Read;

use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::{
    CompositeTemplate,
    glib,
    prelude::*,
};

use gtk::template_callbacks;
use reqwest::Response;

use crate::{
    client::jellyfin_client::JELLYFIN_CLIENT,
    ui::GlobalToast,
    utils::{
        spawn,
        spawn_tokio,
    },
};

use super::ImageDialogNavigtion;

mod imp {
    use std::cell::OnceCell;

    use glib::{
        Properties,
        subclass::InitializingObject,
    };
    use gtk::prelude::*;

    use crate::ui::widgets::image_dialog::ImageDropRow;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate, Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/image_dialog_edit_page.ui")]
    #[properties(wrapper_type = super::ImageDialogEditPage)]
    pub struct ImageDialogEditPage {
        #[property(get, set, construct_only)]
        pub id: OnceCell<String>,
        #[property(get, set, construct_only)]
        pub image_type: OnceCell<String>,
        #[property(get, set, construct_only)]
        pub image_tag: OnceCell<u8>,

        #[template_child]
        pub url_check_button: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub image_drop_row: TemplateChild<ImageDropRow>,

        #[template_child]
        pub entry: TemplateChild<adw::EntryRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageDialogEditPage {
        const NAME: &'static str = "ImageDialogEditPage";
        type Type = super::ImageDialogEditPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            ImageDropRow::ensure_type();
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ImageDialogEditPage {
        fn constructed(&self) {
            self.parent_constructed();

            self.url_check_button
                .set_group(Some(&self.image_drop_row.imp().upload_check_button.get()));
        }
    }

    impl WidgetImpl for ImageDialogEditPage {}

    impl NavigationPageImpl for ImageDialogEditPage {}
}

glib::wrapper! {
    pub struct ImageDialogEditPage(ObjectSubclass<imp::ImageDialogEditPage>)
        @extends gtk::Widget, adw::NavigationPage, @implements gtk::Accessible;
}

use anyhow::{
    Result,
    anyhow,
};

#[template_callbacks]
impl ImageDialogEditPage {
    pub fn new(id: &str, image_type: &str, image_tag: u8) -> Self {
        glib::Object::builder()
            .property("id", id)
            .property("image-type", image_type)
            .property("image-tag", image_tag)
            .build()
    }

    async fn post_image(&self, id: String, image_type: String) -> Result<Response> {
        let file = match self.imp().image_drop_row.imp().image_file.upgrade() {
            Some(file) => file,
            None => return Err(anyhow!("No file found")),
        };

        let Ok((bytes, _)) = file.load_bytes_future().await else {
            return Err(anyhow!("Failed to load file"));
        };

        let Ok(bytes) = bytes.bytes().collect::<Result<Vec<u8>, _>>() else {
            return Err(anyhow!("Failed to read file"));
        };

        let content_type = if let Some(mime) = file
            .query_info_future(
                "standard::content-type",
                gtk::gio::FileQueryInfoFlags::NONE,
                glib::Priority::LOW,
            )
            .await
            .ok()
            .and_then(|info| info.content_type())
        {
            mime.to_string()
        } else {
            "image/jpeg".to_string()
        };

        use base64::{
            Engine as _,
            engine::general_purpose::STANDARD,
        };
        let bytes = STANDARD.encode(bytes);

        spawn_tokio(async move {
            JELLYFIN_CLIENT
                .post_image(&id, &image_type, bytes, &content_type)
                .await
        })
        .await
    }

    #[template_callback]
    async fn on_save_button_clicked(&self) {
        let Some(dialog) = self.image_dialog() else {
            return;
        };

        dialog.loading_page();

        let id = self.id();
        let image_type = self.image_type();

        let result = if self.imp().url_check_button.is_active() {
            let url = self.imp().entry.text().to_string();
            let image_tag = self.image_tag();

            spawn_tokio(async move {
                JELLYFIN_CLIENT
                    .post_image_url(&id, &image_type, image_tag, &url)
                    .await
            })
            .await
        } else {
            self.post_image(id, image_type).await
        };

        match result {
            Ok(_) => {
                spawn(async move {
                    dialog.pop_page();
                    dialog.set_image_items().await;
                    dialog.view_page();
                    dialog.toast(gettext("Image saved"));
                });
            }
            Err(_) => {
                dialog.view_page();
                dialog.toast(gettext("Failed to load image"));
            }
        }
    }
}
