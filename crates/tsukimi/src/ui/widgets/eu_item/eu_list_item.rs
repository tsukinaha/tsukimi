use adw::subclass::prelude::*;
use gtk::{
    CompositeTemplate,
    glib,
};

use gtk::template_callbacks;

use super::EuItem;

mod imp {
    use std::cell::RefCell;

    use glib::{
        Properties,
        subclass::InitializingObject,
    };
    use gtk::prelude::*;

    use crate::ui::{
        provider::tu_item::{
            image_type::{
                ART,
                BACKDROP,
                BANNER,
                DISC,
                LOGO,
                THUMB,
            },
            item_type::{
                EPISODE,
                MUSIC_ALBUM,
                MUSIC_ARTIST,
            },
        },
        widgets::{
            picture_loader::PictureLoader,
            tu_item::CardShape,
        },
    };

    use super::*;

    #[derive(Debug, Default, CompositeTemplate, Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/eu_item.ui")]
    #[properties(wrapper_type = super::EuListItem)]
    pub struct EuListItem {
        #[property(get, set = Self::set_item)]
        pub item: RefCell<EuItem>,

        #[template_child]
        pub label1: TemplateChild<gtk::Label>,
        #[template_child]
        pub label2: TemplateChild<gtk::Label>,
        #[template_child]
        pub label3: TemplateChild<gtk::Label>,

        #[template_child]
        pub picture_container: TemplateChild<gtk::Box>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EuListItem {
        const NAME: &'static str = "EuListItem";
        type Type = super::EuListItem;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for EuListItem {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for EuListItem {}

    impl BinImpl for EuListItem {}

    impl EuListItem {
        pub fn set_item(&self, item: EuItem) {
            self.label1.set_text(&item.line1().unwrap_or_default());
            if let Some(line2) = item.line2() {
                self.label2.set_text(&line2);
            }
            if let Some(line3) = item.line3() {
                self.label3.set_text(&line3);
                self.label3.set_visible(true);
            }
            if let Some(url) = item.image_url().or(item.image_original_url()) {
                let shape = match item.image_type().as_deref() {
                    Some(BACKDROP | ART | THUMB | LOGO) => CardShape::Backdrop,
                    Some(BANNER) => CardShape::Banner,
                    Some(DISC) => CardShape::Square,
                    _ => match item.item_type().as_deref() {
                        Some(EPISODE) => CardShape::Backdrop,
                        Some(MUSIC_ALBUM | MUSIC_ARTIST) => CardShape::Square,
                        _ => CardShape::Portrait,
                    },
                };
                let size = shape.size();
                let picture_loader = PictureLoader::new_for_url(&url);
                picture_loader.set_size_request(size.0, size.1);
                self.picture_container.append(&picture_loader);
            }
            self.item.replace(item);
        }
    }
}

glib::wrapper! {
    pub struct EuListItem(ObjectSubclass<imp::EuListItem>)
        @extends gtk::Widget, adw::Bin, @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for EuListItem {
    fn default() -> Self {
        glib::Object::new()
    }
}

#[template_callbacks]
impl EuListItem {
    pub fn new(item: &EuItem) -> Self {
        glib::Object::builder().property("item", item).build()
    }
}
