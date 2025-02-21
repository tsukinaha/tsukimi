use adw::subclass::prelude::*;
use gst::prelude::{
    CastNone,
    StaticType,
};
use gtk::{
    CompositeTemplate,
    glib,
    prelude::*,
    template_callbacks,
};

use crate::client::structs::FilterItem;

use super::{
    FilterDialogSearchPage,
    FilterPanelDialog,
};

mod imp {
    use std::cell::{
        OnceCell,
        RefCell,
    };

    use glib::subclass::InitializingObject;
    use gtk::prelude::*;

    use crate::ui::widgets::filter_panel::FilterDialogSearchPage;

    use super::*;

    #[derive(Default, CompositeTemplate, glib::Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/filters_row.ui")]
    #[properties(wrapper_type = super::FiltersRow)]
    pub struct FiltersRow {
        #[property(get, set, nullable)]
        pub title: RefCell<Option<String>>,
        #[property(get, set, nullable)]
        pub filter_type: RefCell<Option<String>>,
        #[property(get, set, nullable)]
        pub icon_name: RefCell<Option<String>>,

        #[template_child]
        pub flowbox: TemplateChild<gtk::FlowBox>,

        pub search_page: OnceCell<FilterDialogSearchPage>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FiltersRow {
        const NAME: &'static str = "FiltersRow";
        type Type = super::FiltersRow;
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
    impl ObjectImpl for FiltersRow {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for FiltersRow {}

    impl ListBoxRowImpl for FiltersRow {}

    impl PreferencesRowImpl for FiltersRow {}
}

glib::wrapper! {
    pub struct FiltersRow(ObjectSubclass<imp::FiltersRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::ActionRow, adw::PreferencesRow, @implements gtk::Actionable, gtk::Accessible;
}

impl Default for FiltersRow {
    fn default() -> Self {
        Self::new()
    }
}

#[template_callbacks]
impl FiltersRow {
    pub fn new() -> Self {
        glib::Object::new()
    }

    #[template_callback]
    fn on_add_button_clicked(&self) {
        let filter_type = self.filter_type();

        let page = self.imp().search_page.get_or_init(|| {
            let page = FilterDialogSearchPage::new();
            page.set_filter_row(self);
            page.push_filter(filter_type.unwrap_or_default());
            page.connect_filters_changed(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move |_| {
                    obj.on_filter_changed();
                }
            ));
            page
        });

        if let Some(dialog) = self
            .ancestor(FilterPanelDialog::static_type())
            .and_downcast_ref::<FilterPanelDialog>()
        {
            dialog.push_page(page)
        }
    }

    pub fn on_filter_changed(&self) {
        let flowbox = self.imp().flowbox.get();

        flowbox.remove_all();

        let Some(search_page) = self.imp().search_page.get() else {
            return;
        };

        let Ok(filter_list) = search_page.imp().filter_list.lock() else {
            return;
        };

        for filter in filter_list.iter() {
            let label = super::FilterLabel::new();
            label.set_label(Some(filter.name.clone().replace("&", "&amp;")));
            label.set_name(filter.name.clone());
            label.set_id(filter.id.clone());
            label.set_icon_name(self.icon_name());
            flowbox.append(&label);
        }
    }

    pub fn remove_filter(&self, filter: FilterItem) {
        let Some(search_page) = self.imp().search_page.get() else {
            return;
        };

        search_page.remove_filter(filter);
    }

    pub fn clear_filters(&self) {
        let Some(search_page) = self.imp().search_page.get() else {
            return;
        };

        search_page.clear_filters();
    }

    pub fn filters(&self) -> Option<Vec<FilterItem>> {
        let search_page = self.imp().search_page.get()?;

        search_page.filters()
    }
}
