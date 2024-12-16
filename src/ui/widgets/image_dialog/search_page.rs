use adw::subclass::prelude::*;
use gtk::{
    glib,
    CompositeTemplate,
};

use gtk::{
    prelude::*,
    template_callbacks,
};

use crate::{
    client::{
        emby_client::EMBY_CLIENT,
        error::UserFacingError,
    },
    toast,
    utils::{
        spawn,
        spawn_tokio,
    },
};

use super::ImageDialogNavigtion;

mod imp {
    use std::cell::OnceCell;

    use glib::{
        subclass::InitializingObject,
        Properties,
    };
    use gtk::prelude::*;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate, Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/image_dialog_search_page.ui")]
    #[properties(wrapper_type = super::ImageDialogSearchPage)]
    pub struct ImageDialogSearchPage {
        #[property(get, set, construct_only)]
        pub id: OnceCell<String>,
        #[property(get, set, construct_only)]
        pub image_type: OnceCell<String>,

        #[template_child]
        pub items_count_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub dropdown_string_list: TemplateChild<gtk::StringList>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageDialogSearchPage {
        const NAME: &'static str = "ImageDialogSearchPage";
        type Type = super::ImageDialogSearchPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ImageDialogSearchPage {
        fn constructed(&self) {
            self.parent_constructed();

            spawn(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                async move {
                    imp.obj().init().await;
                }
            ));
        }
    }

    impl WidgetImpl for ImageDialogSearchPage {}

    impl NavigationPageImpl for ImageDialogSearchPage {}
}

glib::wrapper! {
    pub struct ImageDialogSearchPage(ObjectSubclass<imp::ImageDialogSearchPage>)
        @extends gtk::Widget, adw::NavigationPage, @implements gtk::Accessible;
}

#[template_callbacks]
impl ImageDialogSearchPage {
    pub fn new(id: &str, image_type: &str) -> Self {
        glib::Object::builder()
            .property("id", id)
            .property("image-type", image_type)
            .build()
    }

    pub async fn init(&self) {
        let id = self.id();
        let type_ = self.image_type();

        let Some(dialog) = self.image_dialog() else {
            return;
        };

        dialog.loading_page();

        let remote_image_list = match spawn_tokio(async move {
            EMBY_CLIENT
                .get_remote_image_list(&id, 0, false, &type_, "")
                .await
        })
        .await
        {
            Ok(remote_image_list) => remote_image_list,
            Err(e) => {
                toast!(self, e.to_user_facing());
                return;
            }
        };

        dialog.view_page();

        self.imp()
            .items_count_label
            .set_text(&remote_image_list.total_record_count.to_string());

        for provider in remote_image_list.providers {
            self.imp().dropdown_string_list.append(&provider);
        }
    }
}
