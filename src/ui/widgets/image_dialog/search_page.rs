use adw::subclass::prelude::*;
use gtk::{
    glib,
    CompositeTemplate,
};

use gtk::template_callbacks;

mod imp {
    use std::cell::OnceCell;

    use glib::{
        subclass::InitializingObject,
        Properties,
    };
    use gtk::prelude::*;

    use crate::ui::widgets::image_dialog::ImageDropRow;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate, Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/image_dialog_search_page.ui")]
    #[properties(wrapper_type = super::ImageDialogSearchPage)]
    pub struct ImageDialogSearchPage {
        #[property(get, set, construct_only)]
        pub id: OnceCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageDialogSearchPage {
        const NAME: &'static str = "ImageDialogSearchPage";
        type Type = super::ImageDialogSearchPage;
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
    impl ObjectImpl for ImageDialogSearchPage {
        fn constructed(&self) {
            self.parent_constructed();
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
    pub fn new(id: &str) -> Self {
        glib::Object::builder()
            .property("id", id)
            .build()
    }
}
