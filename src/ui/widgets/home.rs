use gettextrs::gettext;
use glib::Object;
use gtk::{
    gio,
    glib,
    prelude::*,
    subclass::prelude::*,
    template_callbacks,
};

use super::{
    hortu_scrolled::{
        HortuScrolled,
        UnifySize,
    },
    utils::GlobalToast,
};
use crate::{
    client::{
        error::UserFacingError,
        jellyfin_client::JELLYFIN_CLIENT,
        structs::*,
    },
    fraction,
    fraction_reset,
    ui::provider::tu_item::{
        PreferPoster,
        TuItem,
    },
    utils::{
        CachePolicy,
        fetch_with_cache,
        spawn,
        spawn_g_timeout,
    },
};
mod imp {

    use std::collections::HashMap;

    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        glib,
        prelude::StaticTypeExt,
        subclass::prelude::*,
    };

    use crate::ui::widgets::hortu_scrolled::HortuScrolled;

    // Object holding the state
    #[derive(CompositeTemplate)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/home.ui")]
    pub struct HomePage {
        #[template_child]
        pub root: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub libsbox: TemplateChild<gtk::Box>,
        #[template_child]
        pub hishortu: TemplateChild<HortuScrolled>,
        #[template_child]
        pub libhortu: TemplateChild<HortuScrolled>,
        pub selection: gtk::SingleSelection,
        /// Keeps track of HortuScrolled widgets in libsbox keyed by library view ID.
        pub libs_hortu_map: std::cell::RefCell<HashMap<String, HortuScrolled>>,
    }

    impl Default for HomePage {
        fn default() -> Self {
            Self {
                root: TemplateChild::default(),
                libsbox: TemplateChild::default(),
                hishortu: TemplateChild::default(),
                libhortu: TemplateChild::default(),
                selection: gtk::SingleSelection::default(),
                libs_hortu_map: std::cell::RefCell::new(HashMap::new()),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for HomePage {
        const NAME: &'static str = "HomePage";
        type Type = super::HomePage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            HortuScrolled::ensure_type();
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for HomePage {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.init_load();
        }
    }

    impl WidgetImpl for HomePage {}

    impl WindowImpl for HomePage {}

    impl ApplicationWindowImpl for HomePage {}

    impl adw::subclass::navigation_page::NavigationPageImpl for HomePage {}
}

glib::wrapper! {
    pub struct HomePage(ObjectSubclass<imp::HomePage>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget ,adw::NavigationPage,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl Default for HomePage {
    fn default() -> Self {
        Self::new()
    }
}

#[template_callbacks]
impl HomePage {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn init_load(&self) {
        spawn(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                obj.setup(true).await;
            }
        ));
    }

    pub fn update(&self, enable_cache: bool) {
        spawn_g_timeout(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                obj.setup(enable_cache).await;
            }
        ));
    }

    pub async fn setup(&self, enable_cache: bool) {
        fraction_reset!(self);
        self.setup_history(enable_cache).await;
        self.setup_library(enable_cache).await;
        fraction!(self);
    }

    pub async fn setup_history(&self, enable_cache: bool) {
        let hortu = self.imp().hishortu.get();

        let (cache_policy, on_refresh): (CachePolicy, Option<_>) = if enable_cache {
            let hortu_ref = hortu.clone();
            (
                CachePolicy::ReadCacheAndRefresh,
                Some(move |data: List| {
                    hortu_ref.set_items(&data.items);
                }),
            )
        } else {
            (CachePolicy::RefreshCache, None)
        };

        let results = match fetch_with_cache(
            "history",
            cache_policy,
            async { JELLYFIN_CLIENT.get_resume().await },
            on_refresh,
        )
        .await
        {
            Ok(history) => history,
            Err(e) => {
                self.toast(e.to_user_facing());
                return;
            }
        };

        hortu.set_items(&results.items);
    }

    pub async fn setup_library(&self, enable_cache: bool) {
        let hortu = self.imp().libhortu.get();

        let (cache_policy, on_refresh): (CachePolicy, Option<_>) = if enable_cache {
            let hortu_ref = hortu.clone();
            (
                CachePolicy::ReadCacheAndRefresh,
                Some(move |data: List| {
                    hortu_ref.set_items(&data.items);
                }),
            )
        } else {
            (CachePolicy::RefreshCache, None)
        };

        let results = match fetch_with_cache(
            "library",
            cache_policy,
            async { JELLYFIN_CLIENT.get_library().await },
            on_refresh,
        )
        .await
        {
            Ok(history) => history,
            Err(e) => {
                self.toast(e.to_user_facing());
                return;
            }
        };

        let results = results.items;

        hortu.set_items(&results);

        self.setup_libsview(results, enable_cache).await;
    }

    pub async fn setup_libsview(&self, items: Vec<SimpleListItem>, enable_cache: bool) {
        let imp = self.imp();
        let libsbox = &imp.libsbox;

        // Collect new view IDs for tracking what to keep
        let new_ids: std::collections::HashSet<String> =
            items.iter().map(|v| v.id.clone()).collect();

        // Remove HortuScrolled widgets whose view ID is no longer present
        let mut map = imp.libs_hortu_map.borrow_mut();
        map.retain(|id, hortu| {
            if new_ids.contains(id) {
                true
            } else {
                libsbox.remove(hortu);
                false
            }
        });
        drop(map);

        for view in items {
            let view_id = view.id.clone();

            // Reuse existing HortuScrolled if available, otherwise create new
            if let Some(hortu) = imp.libs_hortu_map.borrow().get(&view_id) {
                let hortu = hortu.clone();
                // Update title in case the view name changed
                hortu.set_title(format!("{} {}", gettext("Latest"), view.name));

                spawn(glib::clone!(
                    #[weak(rename_to = obj)]
                    self,
                    async move {
                        obj.refresh_libview(hortu, view, enable_cache).await;
                    }
                ));
            } else {
                spawn(glib::clone!(
                    #[weak(rename_to = obj)]
                    self,
                    async move {
                        obj.setup_libview(view, enable_cache).await;
                    }
                ));
            }
        }
    }

    async fn setup_libview(&self, view: SimpleListItem, enable_cache: bool) {
        let ac_view = view.to_owned();
        let view_id = view.id.clone();

        let hortu = HortuScrolled::new();

        hortu.set_moreview(true);
        hortu.set_unify_size(UnifySize::Majority);
        hortu.set_prefer_poster(PreferPoster::ParentPost);
        hortu.set_title(format!("{} {}", gettext("Latest"), view.name));

        hortu.connect_morebutton(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            move |_| {
                let list_item = TuItem::default();
                list_item.set_id(ac_view.id.to_owned());
                list_item.set_name(ac_view.name.to_owned());
                list_item.set_item_type(ac_view.item_type.to_owned());
                list_item.set_collection_type(ac_view.collection_type.to_owned());
                list_item.activate(&obj, None);
            }
        ));

        self.imp().libsbox.append(&hortu);

        // Register so subsequent refreshes can reuse this widget
        self.imp()
            .libs_hortu_map
            .borrow_mut()
            .insert(view_id.clone(), hortu.clone());

        self.fetch_and_fill_libview(&hortu, view, enable_cache)
            .await;
    }

    /// Refreshes an existing HortuScrolled with new data (warm path – diff animations).
    async fn refresh_libview(
        &self, hortu: HortuScrolled, view: SimpleListItem, enable_cache: bool,
    ) {
        self.fetch_and_fill_libview(&hortu, view, enable_cache)
            .await;
    }

    /// Shared helper: fetches data for a library view and calls set_items.
    async fn fetch_and_fill_libview(
        &self, hortu: &HortuScrolled, view: SimpleListItem, enable_cache: bool,
    ) {
        let view_id = view.id.clone();
        let collection_type = view.collection_type.clone();

        let (cache_policy, on_refresh): (CachePolicy, Option<_>) = if enable_cache {
            let hortu_ref = hortu.clone();
            (
                CachePolicy::ReadCacheAndRefresh,
                Some(move |data: Vec<SimpleListItem>| {
                    hortu_ref.set_items(&data);
                }),
            )
        } else {
            (CachePolicy::RefreshCache, None)
        };

        let results = match fetch_with_cache(
            &format!("library_{}", view_id),
            cache_policy,
            async move {
                if collection_type.as_deref() == Some("livetv") {
                    JELLYFIN_CLIENT.get_channels().await.map(|x| x.items)
                } else {
                    JELLYFIN_CLIENT.get_latest(&view_id).await
                }
            },
            on_refresh,
        )
        .await
        {
            Ok(history) => history,
            Err(e) => {
                self.toast(e.to_user_facing());
                return;
            }
        };

        hortu.set_items(&results);
    }
}
