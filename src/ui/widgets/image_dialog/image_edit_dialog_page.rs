use adw::subclass::prelude::*;
use gtk::{
    glib,
    CompositeTemplate,
};

mod imp {
    use std::cell::Cell;

    use glib::{
        subclass::InitializingObject,
        Properties,
    };
    use gtk::prelude::*;

    use crate::ui::widgets::{
        check_row::CheckRow,
        image_dialog::ImageDropRow,
    };

    use super::*;

    #[derive(Debug, Default, CompositeTemplate, Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/image_dialog_edit_page.ui")]
    #[properties(wrapper_type = super::ImageDialogEditPage)]
    pub struct ImageDialogEditPage {
        #[template_child]
        pub url_check_button: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub image_drop_row: TemplateChild<ImageDropRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageDialogEditPage {
        const NAME: &'static str = "ImageDialogEditPage";
        type Type = super::ImageDialogEditPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            ImageDropRow::ensure_type();
            klass.bind_template();
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

impl Default for ImageDialogEditPage {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageDialogEditPage {
    pub fn new() -> Self {
        glib::Object::new()
    }
}
