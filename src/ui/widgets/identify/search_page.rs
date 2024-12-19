use adw::subclass::prelude::*;
use gtk::{
    gio,
    glib,
    CompositeTemplate,
};

use gtk::{
    prelude::*,
    template_callbacks,
};

use crate::{
    client::structs::RemoteSearchResult,
    ui::widgets::eu_item::{
        self,
        EuItem,
        EuObject,
    },
};

mod imp {

    use eu_item::EuListItemExt;
    use glib::subclass::InitializingObject;
    use gtk::gio;

    use crate::ui::widgets::eu_item::EuObject;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/identify_dialog_search_page.ui")]
    pub struct IdentifyDialogSearchPage {
        #[template_child]
        pub grid: TemplateChild<gtk::GridView>,

        pub selection: gtk::SingleSelection,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for IdentifyDialogSearchPage {
        const NAME: &'static str = "IdentifyDialogSearchPage";
        type Type = super::IdentifyDialogSearchPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for IdentifyDialogSearchPage {
        fn constructed(&self) {
            self.parent_constructed();
            let store = gio::ListStore::new::<EuObject>();
            self.selection.set_model(Some(&store));
            self.grid
                .set_factory(Some(gtk::SignalListItemFactory::new().eu_item()));
            self.grid.set_model(Some(&self.selection));
        }
    }

    impl WidgetImpl for IdentifyDialogSearchPage {}

    impl NavigationPageImpl for IdentifyDialogSearchPage {}
}

glib::wrapper! {
    pub struct IdentifyDialogSearchPage(ObjectSubclass<imp::IdentifyDialogSearchPage>)
        @extends gtk::Widget, adw::NavigationPage, @implements gtk::Accessible;
}

impl Default for IdentifyDialogSearchPage {
    fn default() -> Self {
        Self::new()
    }
}

#[template_callbacks]
impl IdentifyDialogSearchPage {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn extend_item(&self, items: Vec<RemoteSearchResult>, item_type: String) {
        let Some(store) = self
            .imp()
            .selection
            .model()
            .and_downcast::<gio::ListStore>()
        else {
            return;
        };
        for item in items {
            let eu_item = EuItem::new(
                item.image_url,
                None,
                Some(item.name),
                item.production_year.map(|p| p.to_string()),
                None,
                Some(item_type.to_owned()),
            );
            let eu_object = EuObject::new(&eu_item);
            store.append(&eu_object);
        }
    }
}
