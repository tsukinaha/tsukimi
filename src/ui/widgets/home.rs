use std::collections::HashSet;

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
        CacheEvent,
        CachePolicy,
        CacheSource,
        fetch_with_cache,
        spawn,
        spawn_g_timeout,
    },
};
mod imp {

    use std::{
        cell::RefCell,
        collections::HashMap,
    };

    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        glib::{
            self,
            WeakRef,
        },
        prelude::StaticTypeExt,
        subclass::prelude::*,
    };

    use crate::ui::widgets::hortu_scrolled::HortuScrolled;

    // Object holding the state
    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/home.ui")]
    pub struct HomePage {
        #[template_child]
        pub root: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub libsbox: TemplateChild<gtk::Box>,
        #[template_child]
        pub hishortu: TemplateChild<HortuScrolled>,
        #[template_child]
        pub nextuphortu: TemplateChild<HortuScrolled>,
        #[template_child]
        pub libhortu: TemplateChild<HortuScrolled>,
        pub selection: gtk::SingleSelection,

        pub libs_hortu: RefCell<HashMap<String, WeakRef<HortuScrolled>>>,
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
        futures_util::join!(
            self.setup_history(enable_cache),
            self.setup_next_up(enable_cache),
            self.setup_library(enable_cache)
        );
        fraction!(self);
    }

    pub async fn setup_history(&self, enable_cache: bool) {
        let hortu = self.imp().hishortu.get();

        let mut events = fetch_with_cache(
            "history",
            if enable_cache {
                CachePolicy::ReadCacheAndRefresh
            } else {
                CachePolicy::RefreshIfChanged
            },
            async { JELLYFIN_CLIENT.get_resume().await },
        )
        .await;

        while let Some(event) = events.recv().await {
            match event {
                CacheEvent::Data { data, .. } => {
                    hortu.set_items(data.items);
                }
                CacheEvent::Error(e) => {
                    self.toast(e.to_user_facing());
                    return;
                }
            }
        }
    }

    pub async fn setup_next_up(&self, enable_cache: bool) {
        let hortu = self.imp().nextuphortu.get();

        if !JELLYFIN_CLIENT.is_jellyfin() {
            hortu.set_visible(false);
            return;
        }

        let mut events = fetch_with_cache(
            "next_up",
            if enable_cache {
                CachePolicy::ReadCacheAndRefresh
            } else {
                CachePolicy::RefreshIfChanged
            },
            async { JELLYFIN_CLIENT.get_next_up().await },
        )
        .await;

        while let Some(event) = events.recv().await {
            match event {
                CacheEvent::Data { data, .. } => {
                    hortu.set_items(data.items);
                }
                CacheEvent::Error(e) => {
                    self.toast(e.to_user_facing());
                    return;
                }
            }
        }
    }

    pub async fn setup_library(&self, enable_cache: bool) {
        let hortu = self.imp().libhortu.get();

        let mut events = fetch_with_cache(
            "library",
            if enable_cache {
                CachePolicy::ReadCacheAndRefresh
            } else {
                CachePolicy::RefreshAndEmitLatest
            },
            async { JELLYFIN_CLIENT.get_library().await },
        )
        .await;

        while let Some(event) = events.recv().await {
            match event {
                CacheEvent::Data { data, source } => {
                    if enable_cache || matches!(source, CacheSource::Network) {
                        hortu.set_items(data.items.clone());
                    }
                    self.setup_libsview(data.items, enable_cache);
                }
                CacheEvent::Error(e) => {
                    self.toast(e.to_user_facing());
                    return;
                }
            }
        }
    }

    pub fn setup_libsview(&self, items: Vec<SimpleListItem>, enable_cache: bool) {
        let current_ids = items
            .iter()
            .map(|view| view.id.as_str())
            .collect::<HashSet<_>>();
        let removed_ids = self
            .imp()
            .libs_hortu
            .borrow()
            .keys()
            .filter(|id| !current_ids.contains(id.as_str()))
            .cloned()
            .collect::<Vec<_>>();

        for id in removed_ids {
            if let Some(hortu) = self.imp().libs_hortu.borrow_mut().remove(&id) {
                if let Some(hortu) = hortu.upgrade() {
                    self.imp().libsbox.remove(&hortu);
                }
            }
        }

        for view in items {
            spawn(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                async move {
                    obj.setup_libview(view, enable_cache).await;
                }
            ));
        }
    }

    fn setup_hortu(&self, ac_view: SimpleListItem) -> HortuScrolled {
        let hortu = HortuScrolled::new();

        hortu.set_moreview(true);

        hortu.set_unify_size(UnifySize::Majority);

        hortu.set_prefer_poster(PreferPoster::ParentPost);

        hortu.set_title(format!("{} {}", gettext("Latest"), ac_view.name));

        let ac_view_id = ac_view.id.to_owned();

        hortu.connect_morebutton(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            move |_| {
                let list_item = TuItem::default();
                list_item.set_id(ac_view.id.to_owned());
                list_item.set_name(ac_view.name.to_owned());
                list_item.set_item_type(ac_view.item_type.to_owned());
                list_item.set_collection_type(ac_view.collection_type.to_owned());
                list_item.activate(&obj);
            }
        ));

        self.imp().libsbox.append(&hortu);

        self.imp()
            .libs_hortu
            .borrow_mut()
            .insert(ac_view_id, hortu.downgrade());

        hortu
    }

    async fn setup_libview(&self, view: SimpleListItem, enable_cache: bool) {
        let hortu = self
            .imp()
            .libs_hortu
            .borrow()
            .get(&view.id)
            .and_then(|w| w.upgrade());

        let view_id = view.id.clone();
        let collection_type = view.collection_type.clone();

        let hortu = hortu.unwrap_or_else(|| self.setup_hortu(view));

        let mut events = fetch_with_cache(
            &format!("library_{}", view_id),
            if enable_cache {
                CachePolicy::ReadCacheAndRefresh
            } else {
                CachePolicy::RefreshIfChanged
            },
            async move {
                if collection_type.as_deref() == Some("livetv") {
                    JELLYFIN_CLIENT.get_channels().await.map(|x| x.items)
                } else {
                    JELLYFIN_CLIENT.get_latest(&view_id).await
                }
            },
        )
        .await;

        while let Some(event) = events.recv().await {
            match event {
                CacheEvent::Data { data, .. } => {
                    hortu.set_items(data);
                }
                CacheEvent::Error(e) => {
                    self.toast(e.to_user_facing());
                    return;
                }
            }
        }
    }
}
