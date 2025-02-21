use adw::subclass::prelude::*;
use gtk::{
    CompositeTemplate,
    glib,
    prelude::*,
    template_callbacks,
};

use crate::client::structs::FilterItem;

use super::FiltersRow;

mod imp {
    use std::cell::RefCell;

    use glib::subclass::InitializingObject;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate, glib::Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/filter_label.ui")]
    #[properties(wrapper_type = super::FilterLabel)]
    pub struct FilterLabel {
        #[property(get, set, nullable)]
        pub label: RefCell<Option<String>>,
        #[property(get, set)]
        pub name: RefCell<String>,
        #[property(get, set, nullable)]
        pub id: RefCell<Option<String>>,
        #[property(get, set, nullable)]
        pub icon_name: RefCell<Option<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FilterLabel {
        const NAME: &'static str = "FilterLabel";
        type Type = super::FilterLabel;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for FilterLabel {
        fn constructed(&self) {
            self.parent_constructed();

            self.obj()
                .add_css_class(&format!("color{}", rand::random::<u32>() % 4 + 1));
        }
    }

    impl WidgetImpl for FilterLabel {}

    impl BinImpl for FilterLabel {}
}

glib::wrapper! {
    pub struct FilterLabel(ObjectSubclass<imp::FilterLabel>)
        @extends gtk::Widget, gtk::Button, @implements gtk::Actionable, gtk::Accessible;
}

impl Default for FilterLabel {
    fn default() -> Self {
        Self::new()
    }
}

#[template_callbacks]
impl FilterLabel {
    pub fn new() -> Self {
        glib::Object::new()
    }

    #[template_callback]
    fn on_delete_button_clicked(&self) {
        let Some(fillter_row) = self
            .ancestor(FiltersRow::static_type())
            .and_downcast::<FiltersRow>()
        else {
            return;
        };

        fillter_row.remove_filter(self.filter_item());
    }

    pub fn filter_item(&self) -> FilterItem {
        FilterItem {
            name: self.name(),
            id: self.id(),
        }
    }
}
