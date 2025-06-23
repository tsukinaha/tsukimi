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
    hortu_scrolled::HortuScrolled,
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
    ui::provider::tu_item::TuItem,
    utils::{
        CachePolicy,
        fetch_with_cache,
        spawn,
    },
};
mod imp {

    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        glib,
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
        pub libhortu: TemplateChild<HortuScrolled>,
        pub selection: gtk::SingleSelection,
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
                gtk::glib::timeout_future_seconds(1).await;
                obj.setup_history(false).await;
            }
        ));
    }

    pub fn update(&self, enable_cache: bool) {
        spawn(glib::clone!(
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
        self.setup_library().await;
        fraction!(self);
    }

    pub async fn setup_history(&self, enable_cache: bool) {
        let hortu = self.imp().hishortu.get();

        let cache_policy = if enable_cache {
            CachePolicy::UseCacheIfAvailable
        } else {
            CachePolicy::RefreshCache
        };

        let results = match fetch_with_cache("history", cache_policy, async {
            JELLYFIN_CLIENT.get_resume().await
        })
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

    pub async fn setup_library(&self) {
        let hortu = self.imp().libhortu.get();

        let results = match fetch_with_cache("library", CachePolicy::ReadCacheAndRefresh, async {
            JELLYFIN_CLIENT.get_library().await
        })
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

        self.setup_libsview(results).await;
    }

    pub async fn setup_libsview(&self, items: Vec<SimpleListItem>) {
        let libsbox = &self.imp().libsbox;
        for _ in 0..libsbox.observe_children().n_items() {
            libsbox.remove(&libsbox.last_child().unwrap());
        }

        for view in items {
            spawn(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                async move {
                    obj.setup_libview(view).await;
                }
            ));
        }
    }

    async fn setup_libview(&self, view: SimpleListItem) {
        let ac_view = view.to_owned();

        let results = match fetch_with_cache(
            &format!("library_{}", view.id),
            CachePolicy::ReadCacheAndRefresh,
            async move {
                if view.collection_type.as_deref() == Some("livetv") {
                    JELLYFIN_CLIENT.get_channels().await.map(|x| x.items)
                } else {
                    JELLYFIN_CLIENT.get_latest(&view.id).await
                }
            },
        )
        .await
        {
            Ok(history) => history,
            Err(e) => {
                self.toast(e.to_user_facing());
                return;
            }
        };

        let hortu = HortuScrolled::new();

        hortu.set_moreview(true);

        hortu.set_title(format!("{} {}", gettext("Latest"), view.name));

        hortu.set_items(&results);

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
    }
}
