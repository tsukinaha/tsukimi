use adw::subclass::prelude::*;
use gtk::{
    CompositeTemplate,
    glib,
    prelude::*,
};

use crate::client::structs::FilterItem;

use super::FilterDialogSearchPage;

mod imp {
    use std::cell::RefCell;

    use glib::{
        Properties,
        subclass::InitializingObject,
    };
    use gtk::prelude::*;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate, Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/filter_row.ui")]
    #[properties(wrapper_type = super::FilterRow)]
    pub struct FilterRow {
        #[property(get, set)]
        pub name: RefCell<String>,
        #[property(get, set, nullable)]
        pub id: RefCell<Option<String>>,

        #[template_child]
        pub check: TemplateChild<gtk::CheckButton>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FilterRow {
        const NAME: &'static str = "FilterRow";
        type Type = super::FilterRow;
        type ParentType = adw::ActionRow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for FilterRow {}

    impl WidgetImpl for FilterRow {}
    impl ListBoxRowImpl for FilterRow {}
    impl PreferencesRowImpl for FilterRow {}
    impl ActionRowImpl for FilterRow {}
}

glib::wrapper! {
    pub struct FilterRow(ObjectSubclass<imp::FilterRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::ActionRow, adw::PreferencesRow, @implements gtk::Accessible;
}

#[gtk::template_callbacks]
impl FilterRow {
    pub fn new(name: &str, id: Option<String>) -> Self {
        glib::Object::builder()
            .property("name", name)
            .property("id", id)
            .build()
    }

    #[template_callback]
    fn on_check_toggled(&self, check_button: &gtk::CheckButton) {
        let binding = self.ancestor(FilterDialogSearchPage::static_type());
        let Some(search_page) = binding.and_downcast_ref::<FilterDialogSearchPage>() else {
            return;
        };

        let filter = FilterItem {
            id: self.id(),
            name: self.name(),
        };
        match check_button.is_active() {
            true => {
                search_page.add_active_rows(self);
                search_page.add_filter(filter)
            }
            false => {
                search_page.remove_active_rows(self);
                search_page.remove_filter(filter);
            }
        }
    }

    pub fn set_active(&self, active: bool) {
        self.imp().check.set_active(active);
    }
}
