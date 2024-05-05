use adw::prelude::*;
use glib::Object;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use super::window::Window;
use crate::client::{network::*, structs::*};
use crate::utils::{
    get_data_with_cache, spawn, spawn_tokio, tu_list_item_factory, tu_list_view_connect_activate,
};

mod imp {

    use glib::subclass::InitializingObject;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate};
    use std::cell::{OnceCell, RefCell};

    use crate::utils::spawn_g_timeout;

    // Object holding the state
    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsukimi/singlelist.ui")]
    #[properties(wrapper_type = super::SingleListPage)]
    pub struct SingleListPage {
        #[property(get, set, construct_only)]
        pub id: OnceCell<String>,
        #[property(get, set, nullable)]
        pub parentid: RefCell<Option<String>>,
        #[property(get, set, construct_only)]
        pub collectiontype: OnceCell<String>,
        #[property(get, set, construct_only)]
        pub listtype: OnceCell<String>,
        #[template_child]
        pub listgrid: TemplateChild<gtk::GridView>,
        #[template_child]
        pub spinner: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub listrevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub count: TemplateChild<gtk::Label>,
        #[template_child]
        pub listscrolled: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub postmenu: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub dropdown: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub adbutton: TemplateChild<gtk::Box>,
        #[template_child]
        pub status: TemplateChild<adw::StatusPage>,
        pub selection: gtk::SingleSelection,
        pub popovermenu: RefCell<Option<gtk::PopoverMenu>>,
        pub sortorder: RefCell<String>,
        pub sortby: RefCell<String>,
        pub lock: RefCell<bool>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for SingleListPage {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "SingleListPage";
        type Type = super::SingleListPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
            klass.install_action_async("poster", None, |window, _action, _parameter| async move {
                window.poster("poster").await;
            });
            klass.install_action_async(
                "backdrop",
                None,
                |window, _action, _parameter| async move {
                    window.poster("backdrop").await;
                },
            );
            klass.install_action_async("banner", None, |window, _action, _parameter| async move {
                window.poster("banner").await;
            });
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // Trait shared by all GObjects
    #[glib::derived_properties]
    impl ObjectImpl for SingleListPage {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            spawn_g_timeout(glib::clone!(@weak obj => async move {
                obj.imp().sortorder.replace("Descending".to_string());
                obj.imp().sortby.replace("SortName".to_string());
                obj.handle_type().await;
                obj.set_up_dropdown();
                obj.set_factory().await;
            }));
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for SingleListPage {}

    // Trait shared by all windows
    impl WindowImpl for SingleListPage {}

    // Trait shared by all application windows
    impl ApplicationWindowImpl for SingleListPage {}

    impl adw::subclass::navigation_page::NavigationPageImpl for SingleListPage {}
}

glib::wrapper! {
    pub struct SingleListPage(ObjectSubclass<imp::SingleListPage>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget ,adw::NavigationPage,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

#[gtk::template_callbacks]
impl SingleListPage {
    pub fn new(
        id: String,
        collection_type: String,
        listtype: &str,
        parentid: Option<String>,
    ) -> Self {
        Object::builder()
            .property("id", id)
            .property("collectiontype", collection_type)
            .property("listtype", listtype)
            .property("parentid", parentid)
            .build()
    }

    #[template_callback]
    async fn sort_order_ascending_cb(&self, _btn: &gtk::ToggleButton) {
        self.imp().sortorder.replace("Ascending".to_string());
        self.sortorder().await;
    }

    #[template_callback]
    async fn sort_order_descending_cb(&self, _btn: &gtk::ToggleButton) {
        self.imp().sortorder.replace("Descending".to_string());
        self.sortorder().await;
    }

    #[template_callback]
    fn filter_panel_cb(&self, _btn: &gtk::Button) {
        let dialog = adw::Dialog::builder()
            .title("Filter")
            .presentation_mode(adw::DialogPresentationMode::BottomSheet)
            .build();
        dialog.present(self);
    }

    async fn handle_type(&self) {
        let imp = self.imp();
        let listtype = imp.listtype.get().unwrap();
        match listtype.as_str() {
            "all" => {}
            "resume" => {
                imp.postmenu.set_visible(false);
                imp.dropdown.set_visible(false);
                imp.adbutton.set_visible(false);
            }
            "boxset" => {
                imp.postmenu.set_visible(false);
            }
            "tags" => {
                imp.postmenu.set_visible(false);
                imp.dropdown.set_visible(false);
                imp.adbutton.set_visible(false);
            }
            "genres" => {
                imp.postmenu.set_visible(false);
                imp.dropdown.set_visible(false);
                imp.adbutton.set_visible(false);
            }
            "liked" => {
                imp.postmenu.set_visible(false);
            }
            _ => {
                imp.postmenu.set_visible(false);
            }
        }
    }

    async fn set_factory(&self) {
        let order = self.imp().sortorder.borrow().clone();
        let update_order = order.clone();
        let imp = self.imp();
        let spinner = imp.spinner.get();
        let listrevealer = imp.listrevealer.get();
        let count = imp.count.get();
        let id = imp.id.get().expect("id not set").clone();
        let include_item_types = self.get_include_item_types().to_owned();
        let listtype = imp.listtype.get().unwrap().clone();
        spinner.set_visible(true);
        let parentid = imp.parentid.borrow().clone();
        let sortby = imp.sortby.borrow().clone();
        let list_results = get_data_with_cache(
            id.to_string(),
            &format!("{}{}", listtype.clone(), include_item_types),
            async move {
                if let Some(parentid) = parentid {
                    get_inlist(
                        parentid.to_string(),
                        0.to_string(),
                        &listtype,
                        &id,
                        &order,
                        &sortby,
                    )
                    .await
                } else {
                    get_list(
                        id.to_string(),
                        0.to_string(),
                        &include_item_types,
                        &listtype,
                        &order,
                        &sortby,
                    )
                    .await
                }
            },
        )
        .await
        .unwrap();
        if list_results.items.is_empty() {
            self.imp().status.set_visible(true);
            self.imp().listrevealer.set_visible(false);
        };
        let store = gio::ListStore::new::<glib::BoxedAnyObject>();
        spawn(glib::clone!(@weak store=> async move {
                spinner.set_visible(false);
                listrevealer.set_reveal_child(true);
                count.set_text(&format!("{} Items",list_results.total_record_count));
                for result in list_results.items {
                    let object = glib::BoxedAnyObject::new(result);
                    store.append(&object);
                    gtk::glib::timeout_future(std::time::Duration::from_millis(30)).await;
                }
        }));
        imp.selection.set_model(Some(&store));
        let listtype = imp.listtype.get().unwrap().clone();
        let factory = tu_list_item_factory(listtype);
        imp.listgrid.set_factory(Some(&factory));
        imp.listgrid.set_model(Some(&imp.selection));
        imp.listgrid.set_min_columns(1);
        imp.listgrid.set_max_columns(13);
        imp.listgrid.connect_activate(
            glib::clone!(@weak self as obj => move |gridview, position| {
                let model = gridview.model().unwrap();
                let item = model.item(position).and_downcast::<glib::BoxedAnyObject>().unwrap();
                let result: std::cell::Ref<SimpleListItem> = item.borrow();
                let window = obj.root().and_downcast::<Window>().unwrap();
                tu_list_view_connect_activate(window,&result,obj.imp().id.get().cloned())
            }),
        );
        let listtype = imp.listtype.get().unwrap().clone();
        if listtype != "resume" {
            self.update(&update_order).await;
        }
    }

    pub async fn update(&self, order: &str) {
        let order = order.to_owned();
        let scrolled = self.imp().listscrolled.get();
        let include_item_types = self.get_include_item_types().to_owned();
        let is_running = Arc::new(AtomicBool::new(false));
        scrolled.connect_edge_reached(glib::clone!(@weak self as obj => move |_, pos| {
            if pos == gtk::PositionType::Bottom {
                let is_running = Arc::clone(&is_running);
                if is_running.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
                    return;
                }
                let order = order.clone();
                let spinner = obj.imp().spinner.get();
                spinner.set_visible(true);
                let store = obj.imp().selection.model().unwrap().downcast::<gio::ListStore>().unwrap();
                let id = obj.imp().id.get().expect("id not set").clone();
                let offset = obj.imp().selection.model().unwrap().n_items();
                let listtype = obj.imp().listtype.get().unwrap().clone();
                spinner.set_visible(true);
                let parentid = obj.imp().parentid.borrow().clone();
                let include_item_types = include_item_types.clone();
                let sortby = obj.imp().sortby.borrow().clone();
                let list_results = spawn_tokio(async move {
                    if let Some(parentid) = parentid {
                        get_inlist(parentid.to_string(), offset.to_string(), &listtype, &id, &order, &sortby).await.unwrap()
                    } else {
                        get_list(id.to_string(), offset.to_string(), &include_item_types, &listtype, &order, &sortby).await.unwrap()
                    }
                });
                spawn(glib::clone!(@weak store=> async move {
                    let list_results = list_results.await;
                    spinner.set_visible(false);
                    for result in list_results.items {
                        let object = glib::BoxedAnyObject::new(result);
                        store.append(&object);
                        gtk::glib::timeout_future(std::time::Duration::from_millis(30)).await;
                    }
                    is_running.store(false, Ordering::SeqCst);
                }));
            }
        }));
    }

    pub async fn sortorder(&self) {
        let order = self.imp().sortorder.borrow().clone();
        let spinner = self.imp().spinner.get();
        let store = self
            .imp()
            .selection
            .model()
            .unwrap()
            .downcast::<gio::ListStore>()
            .unwrap();
        let id = self.imp().id.get().expect("id not set").clone();
        let listtype = self.imp().listtype.get().unwrap().clone();
        spinner.set_visible(true);
        let parentid = self.imp().parentid.borrow().clone();
        let include_item_types = self.get_include_item_types().to_owned();
        let sortby = self.imp().sortby.borrow().clone();
        let list_results = spawn_tokio(async move {
            if let Some(parentid) = parentid {
                get_inlist(
                    parentid.to_string(),
                    0.to_string(),
                    &listtype,
                    &id,
                    &order,
                    &sortby,
                )
                .await
            } else {
                get_list(
                    id.to_string(),
                    0.to_string(),
                    &include_item_types,
                    &listtype,
                    &order,
                    &sortby,
                )
                .await
            }
        })
        .await
        .unwrap();
        spawn(glib::clone!(@weak store,@weak self as obj=> async move {
                store.remove_all();
                spinner.set_visible(false);
                for result in list_results.items {
                    let object = glib::BoxedAnyObject::new(result);
                    store.append(&object);
                    gtk::glib::timeout_future(std::time::Duration::from_millis(30)).await;
                }
        }));
    }

    pub fn get_include_item_types(&self) -> &str {
        let c = self.imp().collectiontype.get().unwrap();
        match c.as_str() {
            "movies" => "Movie",
            "tvshows" => "Series",
            "music" => "MusicAlbum",
            _ => "Movie, Series",
        }
    }

    pub fn set_up_dropdown(&self) {
        let imp = self.imp();
        let dropdown = imp.dropdown.get();
        dropdown.connect_selected_item_notify(glib::clone!(@weak self as obj => move |_| {
            spawn(glib::clone!(@weak obj=> async move {
                obj.set_dropdown_selected();
                obj.sortorder().await;
            }));
        }));
    }

    pub fn set_dropdown_selected(&self) {
        let imp = self.imp();
        let dropdown = imp.dropdown.get();
        let selected = dropdown.selected();
        let sortby = match selected {
            0 => "SortName",
            1 => "TotalBitrate,SortName",
            2 => "DateCreated,SortName",
            3 => "CommunityRating,SortName",
            4 => "CriticRating,SortName",
            5 => "ProductionYear,PremiereDate,SortName",
            6 => "OfficialRating,SortName",
            7 => "ProductionYear,SortName",
            8 => "DatePlayed,SortName",
            9 => "Runtime,SortName",
            _ => "SortName",
        };
        imp.sortby.replace(sortby.to_string());
    }

    pub async fn poster(&self, poster: &str) {
        let imp = self.imp();
        let listgrid = imp.listgrid.get();
        let listtype = imp.listtype.get().unwrap().clone();
        let poster = poster.to_string();
        let factory = gtk::SignalListItemFactory::new();
        factory.connect_bind(move |_, item| {
            let list_item = item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem");
            let entry = item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem")
                .item()
                .and_downcast::<glib::BoxedAnyObject>()
                .expect("Needs to be BoxedAnyObject");
            let latest: std::cell::Ref<SimpleListItem> = entry.borrow();
            if list_item.child().is_none() {
                super::tu_list_item::tu_list_poster(&latest, list_item, &listtype, &poster);
            }
        });
        listgrid.set_factory(Some(&factory));
    }
}
