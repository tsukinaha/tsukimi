use adw::{
    prelude::*,
    subclass::prelude::*,
};
use gtk::{
    CompositeTemplate,
    glib,
};

use gtk::template_callbacks;

use crate::{
    client::{
        error::UserFacingError,
        jellyfin_client::JELLYFIN_CLIENT,
        structs::FilterItem,
    },
    ui::GlobalToast,
    utils::{
        spawn,
        spawn_tokio,
    },
};

mod imp {
    use std::{
        cell::RefCell,
        sync::{
            LazyLock,
            Mutex,
        },
    };

    use glib::{
        WeakRef,
        subclass::{
            InitializingObject,
            Signal,
        },
    };

    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/filter_search_page.ui")]
    pub struct FilterDialogSearchPage {
        pub filter_list: Mutex<Vec<FilterItem>>,

        #[template_child]
        pub listbox: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,

        pub filter_row_ref: WeakRef<FiltersRow>,
        pub active_filter_rows: RefCell<Vec<WeakRef<FilterRow>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FilterDialogSearchPage {
        const NAME: &'static str = "FilterDialogSearchPage";
        type Type = super::FilterDialogSearchPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for FilterDialogSearchPage {
        fn signals() -> &'static [Signal] {
            static SIGNALS: LazyLock<Vec<Signal>> =
                LazyLock::new(|| vec![Signal::builder("filters-changed").build()]);
            SIGNALS.as_ref()
        }

        fn constructed(&self) {
            self.parent_constructed();
            self.obj().connect_filters_changed(|obj| {
                obj.listbox_retain_filters();
            });
            self.listbox.set_filter_func(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                #[upgrade_or]
                true,
                move |row| {
                    let Some(filter) = row.downcast_ref::<FilterRow>() else {
                        return true;
                    };
                    filter
                        .name()
                        .to_lowercase()
                        .contains(&imp.search_entry.text().to_lowercase())
                }
            ));
        }
    }

    impl WidgetImpl for FilterDialogSearchPage {}

    impl NavigationPageImpl for FilterDialogSearchPage {}
}

glib::wrapper! {
    pub struct FilterDialogSearchPage(ObjectSubclass<imp::FilterDialogSearchPage>)
        @extends gtk::Widget, adw::NavigationPage, @implements gtk::Accessible;
}

use super::{
    FilterPanelDialog,
    FilterRow,
    FiltersRow,
};

impl Default for FilterDialogSearchPage {
    fn default() -> Self {
        Self::new()
    }
}

#[template_callbacks]
impl FilterDialogSearchPage {
    pub fn new() -> Self {
        glib::Object::new()
    }

    #[template_callback]
    fn on_search_changed(&self, _: &gtk::SearchEntry) {
        self.imp().listbox.invalidate_filter();
    }

    pub fn push_filter(&self, filter_type: String) {
        spawn(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            #[strong]
            filter_type,
            async move {
                let binding = obj.ancestor(FilterPanelDialog::static_type());
                let Some(dialog) = binding.and_downcast_ref::<FilterPanelDialog>() else {
                    return;
                };

                dialog.loading_page();

                let filters =
                    match spawn_tokio(async move { JELLYFIN_CLIENT.filters(&filter_type).await })
                        .await
                    {
                        Ok(filters) => filters,
                        Err(e) => {
                            obj.toast(e.to_user_facing());
                            return;
                        }
                    };

                dialog.view_page();

                let listbox = &obj.imp().listbox;
                filters.items.iter().for_each(|filter| {
                    let filter_row = FilterRow::new(&filter.name, filter.id.to_owned());
                    filter_row.set_title(&filter.name.to_owned().replace("&", "&amp;"));

                    listbox.append(&filter_row);
                });
            }
        ));
    }

    pub fn set_filter_row(&self, filter_row: &FiltersRow) {
        self.imp().filter_row_ref.set(Some(filter_row));
    }

    pub fn clear_filters(&self) {
        let _ = self
            .imp()
            .filter_list
            .lock()
            .map(|mut filters| filters.clear());
        self.emit_by_name::<()>("filters-changed", &[]);
    }

    pub fn filters(&self) -> Option<Vec<FilterItem>> {
        self.imp()
            .filter_list
            .lock()
            .ok()
            .map(|filters| filters.to_owned())
    }

    pub fn add_filter(&self, filter: FilterItem) {
        let _ = self
            .imp()
            .filter_list
            .lock()
            .map(|mut filters| filters.push(filter));
        self.emit_by_name::<()>("filters-changed", &[]);
    }

    pub fn remove_filter(&self, filter: FilterItem) {
        let _ = self
            .imp()
            .filter_list
            .lock()
            .map(|mut filters| filters.retain(|f| *f != filter));
        self.emit_by_name::<()>("filters-changed", &[]);
    }

    pub fn remove_active_rows(&self, row: &FilterRow) {
        self.imp().active_filter_rows.borrow_mut().retain(|c| {
            if let Some(c) = c.upgrade() {
                c != *row
            } else {
                false
            }
        });
    }

    pub fn clear_active_rows(&self) {
        self.imp().active_filter_rows.borrow_mut().clear();
    }

    pub fn add_active_rows(&self, row: &FilterRow) {
        self.imp()
            .active_filter_rows
            .borrow_mut()
            .push(row.downgrade());
    }

    pub fn listbox_retain_filters(&self) {
        for row in self.imp().active_filter_rows.borrow().iter() {
            let Some(filter_row) = row.upgrade() else {
                continue;
            };

            let filter = FilterItem {
                id: filter_row.id(),
                name: filter_row.name(),
            };

            let Ok(filters) = self.imp().filter_list.lock() else {
                return;
            };

            let is_active = filters.contains(&filter);
            filter_row.set_active(is_active);
        }
    }

    pub fn connect_filters_changed<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_closure(
            "filters-changed",
            true,
            glib::closure_local!(move |obj: Self| {
                f(&obj);
            }),
        )
    }
}
