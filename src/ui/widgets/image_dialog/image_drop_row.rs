use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::{
    glib, prelude::WidgetExt, template_callbacks, CompositeTemplate
};

use crate::toast;

mod imp {
    use std::cell::Cell;

    use glib::{
        subclass::InitializingObject,
        Properties, WeakRef,
    };
    use gtk::prelude::*;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate, Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/image_drop_row.ui")]
    #[properties(wrapper_type = super::ImageDropRow)]
    pub struct ImageDropRow {
        #[template_child]
        pub upload_check_button: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub file_dialog: TemplateChild<gtk::FileDialog>,

        #[template_child]
        pub image: TemplateChild<gtk::Picture>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,

        #[template_child]
        pub frame: TemplateChild<gtk::Frame>,

        #[template_child]
        pub drop_target: TemplateChild<gtk::DropTarget>,

        pub image_file: WeakRef<gtk::gio::File>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageDropRow {
        const NAME: &'static str = "ImageDropRow";
        type Type = super::ImageDropRow;
        type ParentType = adw::PreferencesRow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ImageDropRow {
        fn constructed(&self) {
            self.parent_constructed();

            self.drop_target.set_types(&[gtk::gio::File::static_type()]);
        }
    }

    impl WidgetImpl for ImageDropRow {}
    impl ListBoxRowImpl for ImageDropRow {}
    impl PreferencesRowImpl for ImageDropRow {}
}

glib::wrapper! {
    pub struct ImageDropRow(ObjectSubclass<imp::ImageDropRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::ActionRow, adw::PreferencesRow, @implements gtk::Actionable, gtk::Accessible;
}

impl Default for ImageDropRow {
    fn default() -> Self {
        Self::new()
    }
}

#[template_callbacks]
impl ImageDropRow {
    pub fn new() -> Self {
        glib::Object::new()
    }

    #[template_callback]
    async fn on_upload_button_clicked(&self) {
        if let Ok(file) = self
            .imp()
            .file_dialog
            .open_future(None::<&gtk::Window>)
            .await
        {
            self.imp().stack.set_visible_child_name("image-page");
            self.imp().image_file.set(Some(&file));
            self.imp().image.set_file(Some(&file));
        }
    }

    #[template_callback]
    fn drop_cb(&self, value: glib::Value, _x: f64, _y: f64) -> bool {
        let Ok(file) = value.get::<gtk::gio::File>() else {
            return false;
        };
        self.imp().stack.set_visible_child_name("image-page");
        self.imp().image_file.set(Some(&file));
        self.imp().image.set_file(Some(&file));
        true
    }

    #[template_callback]
    fn enter_cb(&self, _x: f64, _y: f64) -> gtk::gdk::DragAction {
        self.imp().frame.add_css_class("hovered-drop");
        gtk::gdk::DragAction::MOVE | gtk::gdk::DragAction::COPY
    }

    #[template_callback]
    fn leave_cb(&self) {
        self.imp().frame.remove_css_class("hovered-drop");
    }
}
