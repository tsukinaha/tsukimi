use adw::subclass::prelude::*;
use gtk::{
    glib,
    CompositeTemplate,
};

use gtk::template_callbacks;

use super::EuItem;


mod imp {
    use std::cell::OnceCell;

    use glib::{
        subclass::InitializingObject,
        Properties,
    };
    use gtk::prelude::*;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate, Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/eu_item.ui")]
    #[properties(wrapper_type = super::EuListItem)]
    pub struct EuListItem {
        #[property(get, set, construct_only)]
        pub item: OnceCell<EuItem>,

        #[template_child]
        pub label1: TemplateChild<gtk::Label>,
        #[template_child]
        pub label2: TemplateChild<gtk::Label>,
        #[template_child]
        pub label3: TemplateChild<gtk::Label>,
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
}

glib::wrapper! {
    pub struct EuListItem(ObjectSubclass<imp::EuListItem>)
        @extends gtk::Widget, @implements gtk::Accessible;
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

    pub async fn init() {}
}
