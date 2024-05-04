use adw::subclass::prelude::*;
use gtk::{glib, CompositeTemplate};
use gtk::prelude::*;

use crate::client::network::get_included;
use crate::client::structs::SimpleListItem;
use crate::utils::{get_data_with_cache, spawn, tu_list_item_factory, tu_list_view_connect_activate};

use super::window::Window;

mod imp {
    use std::cell::OnceCell;

    use glib::subclass::InitializingObject;
    use gtk::glib::clone;

    use crate::utils::spawn;

    use super::*;

    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsukimi/included.ui")]
    #[properties(wrapper_type = super::IncludedDialog)]
    pub struct IncludedDialog {
        #[property(get, set, construct_only)]
        pub id: OnceCell<String>,
        #[template_child]
        pub spinner: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub listview: TemplateChild<gtk::ListView>,
        #[template_child]
        pub status: TemplateChild<adw::StatusPage>,
        pub selection: gtk::SingleSelection,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for IncludedDialog {
        const NAME: &'static str = "IncludedDialog";
        type Type = super::IncludedDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for IncludedDialog {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            spawn(clone!(@weak obj => async move {
                obj.setup_list().await;
            }));
        }
    }

    impl WidgetImpl for IncludedDialog {}
    impl AdwDialogImpl for IncludedDialog {}
    impl PreferencesDialogImpl for IncludedDialog {}
}

glib::wrapper! {
    /// Preference Window to display and update room details.
    pub struct IncludedDialog(ObjectSubclass<imp::IncludedDialog>)
        @extends gtk::Widget, adw::Dialog, adw::PreferencesDialog, @implements gtk::Accessible;
}

impl IncludedDialog {
    pub fn new(id: &str) -> Self {
        glib::Object::builder().property("id", id).build()
    }

    async fn setup_list(&self) {
        let imp = self.imp();

        let factory = tu_list_item_factory("".to_string());
        imp.listview.set_factory(Some(&factory));

        let store = gtk::gio::ListStore::new::<glib::BoxedAnyObject>();
        imp.selection.set_model(Some(&store));
        imp.listview.set_model(Some(&imp.selection));

        imp.listview.connect_activate(
            glib::clone!(@weak self as obj => move |gridview, position| {
                let model = gridview.model().unwrap();
                let item = model.item(position).and_downcast::<glib::BoxedAnyObject>().unwrap();
                let result: std::cell::Ref<SimpleListItem> = item.borrow();
                let window = obj.root().and_downcast::<Window>().unwrap();
                tu_list_view_connect_activate(window,&result,obj.imp().id.get().cloned())
            }),
        );

        let store = self
            .imp()
            .selection
            .model()
            .unwrap()
            .downcast::<gtk::gio::ListStore>()
            .unwrap();
        let boxset_list = self.get_boxset_list().await;
        spawn(glib::clone!(@weak store,@weak self as obj=> async move {
            obj.imp().spinner.set_visible(false);
            if boxset_list.is_empty() {
                obj.imp().status.set_visible(true);
                return;
            }
            for result in boxset_list {
                let object = glib::BoxedAnyObject::new(result);
                store.append(&object);
                gtk::glib::timeout_future(std::time::Duration::from_millis(30)).await;
            }
        }));
    }

    async fn get_boxset_list(&self) -> Vec<SimpleListItem> {
        let imp = self.imp();
        let id = imp.id.get().unwrap().clone();
        let list = get_data_with_cache(id.to_string(), "include", async move {
            get_included(&id).await
        }).await.unwrap();
        list.items
    }
}
