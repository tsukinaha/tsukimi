use std::sync::atomic::Ordering;
use std::sync::Arc;

use super::window::Window;
use crate::client::client::EMBY_CLIENT;
use crate::client::error::UserFacingError;
use crate::client::structs::*;
use crate::ui::models::SETTINGS;
use crate::utils::{
    req_cache, spawn, spawn_tokio, tu_list_item_factory, tu_list_view_connect_activate,
};
use crate::{fraction, fraction_reset, toast};
use adw::prelude::*;
use glib::Object;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};
mod imp {

    use std::cell::{OnceCell, RefCell};
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    use glib::subclass::InitializingObject;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate};

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
        pub isinlist: OnceCell<bool>,
        #[property(get, set, construct_only)]
        pub listtype: OnceCell<String>,
        #[template_child]
        pub listgrid: TemplateChild<gtk::GridView>,
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
        pub lock: Arc<AtomicBool>,
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

            let store = gtk::gio::ListStore::new::<glib::BoxedAnyObject>();
            self.selection.set_model(Some(&store));

            spawn_g_timeout(glib::clone!(@weak obj => async move {
                obj.set_up().await;
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
        is_inlist: bool,
    ) -> Self {
        Object::builder()
            .property("id", id)
            .property("collectiontype", collection_type)
            .property("listtype", listtype)
            .property("parentid", parentid)
            .property("isinlist", is_inlist)
            .build()
    }

    async fn set_up(&self) {
        fraction_reset!(self);
        self.imp().sortorder.replace("Descending".to_string());
        self.imp().sortby.replace("SortName".to_string());
        self.handle_type().await;
        self.set_up_dropdown();
        self.set_factory().await;
        fraction!(self);
    }

    #[template_callback]
    async fn sort_order_ascending_cb(&self, _btn: &gtk::ToggleButton) {
        self.imp().sortorder.replace("Ascending".to_string());
        let store = self
            .imp()
            .selection
            .model()
            .unwrap()
            .downcast::<gio::ListStore>()
            .unwrap();
        store.remove_all();
        self.update_view("0").await;
    }

    #[template_callback]
    async fn sort_order_descending_cb(&self, _btn: &gtk::ToggleButton) {
        self.imp().sortorder.replace("Descending".to_string());
        let store = self
            .imp()
            .selection
            .model()
            .unwrap()
            .downcast::<gio::ListStore>()
            .unwrap();
        store.remove_all();
        self.update_view("0").await;
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
        let imp = self.imp();
        let listrevealer = imp.listrevealer.get();
        let count = imp.count.get();
        let id = imp.id.get().expect("id not set").clone();
        let include_item_types = self.get_include_item_types().to_owned();
        let listtype = imp.listtype.get().unwrap().clone();
        let parentid = imp.parentid.borrow().clone();
        let sortby = imp.sortby.borrow().clone();

        let is_inlist = *imp.isinlist.get().unwrap();

        let list_results = match req_cache(
            &format!("{}_{}_{}", id, listtype.clone(), include_item_types),
            async move {
                if is_inlist {
                    EMBY_CLIENT
                        .get_inlist(parentid, "0", &listtype, &id, &order, &sortby)
                        .await
                } else {
                    EMBY_CLIENT
                        .get_list(
                            id.to_string(),
                            "0",
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
        {
            Ok(list_results) => list_results,
            Err(e) => {
                toast!(self, e.to_user_facing());
                return;
            }
        };
        if list_results.items.is_empty() {
            self.imp().status.set_visible(true);
            self.imp().listrevealer.set_visible(false);
        };
        let store = gio::ListStore::new::<glib::BoxedAnyObject>();
        spawn(glib::clone!(@weak store=> async move {
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
    }

    #[template_callback]
    pub async fn edge_reached_cb(&self, pos: gtk::PositionType, _: gtk::ScrolledWindow) {
        let listtype = self.imp().listtype.get().unwrap();

        if listtype == "resume" {
            return;
        }

        if pos == gtk::PositionType::Bottom {
            let is_running = Arc::clone(&self.imp().lock);

            if is_running
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                .is_err()
            {
                return;
            }

            let offset = self.imp().selection.model().unwrap().n_items();

            self.update_view(&offset.to_string()).await;

            is_running.store(false, Ordering::SeqCst);
        }
    }

    pub async fn update_view(&self, pos: &str) {
        fraction_reset!(self);
        self.update_view_cb(pos).await;
        fraction!(self);
    }

    pub async fn update_view_cb(&self, pos: &str) {
        let pos = pos.to_owned();
        let order = self.imp().sortorder.borrow().clone();
        let store = self
            .imp()
            .selection
            .model()
            .unwrap()
            .downcast::<gio::ListStore>()
            .unwrap();

        let id = self.imp().id.get().unwrap().clone();
        let listtype = self.imp().listtype.get().unwrap().clone();
        let parentid = self.imp().parentid.borrow().clone();
        let include_item_types = self.get_include_item_types().to_owned();
        let sortby = self.imp().sortby.borrow().clone();

        let is_inlist = *self.imp().isinlist.get().unwrap();

        let list_results = match spawn_tokio(async move {
            if is_inlist {
                EMBY_CLIENT
                    .get_inlist(parentid, &pos, &listtype, &id, &order, &sortby)
                    .await
            } else {
                EMBY_CLIENT
                    .get_list(
                        id.to_string(),
                        &pos,
                        &include_item_types,
                        &listtype,
                        &order,
                        &sortby,
                    )
                    .await
            }
        })
        .await
        {
            Ok(list_results) => list_results,
            Err(e) => {
                toast!(self, e.to_user_facing());
                return;
            }
        };

        for result in list_results.items {
            let object = glib::BoxedAnyObject::new(result);
            store.append(&object);
        }
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

        let sort = SETTINGS.list_sort();
        if sort >= 0 {
            dropdown.set_selected(sort as u32);
        }
        dropdown.connect_selected_item_notify(glib::clone!(@weak self as obj => move |_| {
            spawn(glib::clone!(@weak obj=> async move {
                obj.set_dropdown_selected();
                let store = obj
                    .imp()
                    .selection
                    .model()
                    .unwrap()
                    .downcast::<gio::ListStore>()
                    .unwrap();
                store.remove_all();
                obj.update_view("0").await;
            }));
        }));
    }

    pub fn set_dropdown_selected(&self) {
        let imp = self.imp();
        let dropdown = imp.dropdown.get();
        let selected = dropdown.selected();
        SETTINGS.set_list_sort(&selected).unwrap();
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
                super::tu_list_item::tu_list_poster(
                    &latest,
                    list_item,
                    &listtype == "resume",
                    &poster,
                );
            }
        });
        listgrid.set_factory(Some(&factory));
    }
}
