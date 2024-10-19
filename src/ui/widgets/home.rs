use crate::client::client::EMBY_CLIENT;
use crate::client::error::UserFacingError;
use crate::client::structs::*;
use crate::ui::provider::tu_item::TuItem;
use crate::utils::{fetch_with_cache, spawn, CachePolicy};
use crate::{fraction, fraction_reset, toast};
use gettextrs::gettext;
use glib::Object;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};
use gtk::{prelude::*, template_callbacks};

use super::hortu_scrolled::HortuScrolled;

mod imp {

    use glib::subclass::InitializingObject;
    use gtk::prelude::StaticTypeExt;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate};

    use crate::ui::widgets::hortu_scrolled::HortuScrolled;

    // Object holding the state
    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/com/github/inaha/tsukimi/ui/home.ui")]
    pub struct HomePage {
        #[template_child]
        pub root: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub libsbox: TemplateChild<gtk::Box>,
        #[template_child]
        pub toast: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub hishortu: TemplateChild<HortuScrolled>,
        #[template_child]
        pub libhortu: TemplateChild<HortuScrolled>,
        pub selection: gtk::SingleSelection,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for HomePage {
        // `NAME` needs to match `class` attribute of template
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

    // Trait shared by all GObjects
    impl ObjectImpl for HomePage {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.init_load();
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for HomePage {}

    // Trait shared by all windows
    impl WindowImpl for HomePage {}

    // Trait shared by all application windows
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
            EMBY_CLIENT.get_resume().await
        })
        .await
        {
            Ok(history) => history,
            Err(e) => {
                toast!(self, e.to_user_facing());
                return;
            }
        };

        hortu.set_title(&gettext("Continue Watching"));

        hortu.set_items(&results.items);
    }

    pub async fn setup_library(&self) {
        let hortu = self.imp().libhortu.get();

        let results = match fetch_with_cache("library", CachePolicy::ReadCacheAndRefresh, async {
            EMBY_CLIENT.get_library().await
        })
        .await
        {
            Ok(history) => history,
            Err(e) => {
                toast!(self, e.to_user_facing());
                return;
            }
        };

        let results = results.items;

        hortu.set_title(&gettext("Library"));

        hortu.set_items(&results);

        self.setup_libsview(results).await;
    }

    pub async fn setup_libsview(&self, items: Vec<SimpleListItem>) {
        let libsbox = &self.imp().libsbox;
        for _ in 0..libsbox.observe_children().n_items() {
            libsbox.remove(&libsbox.last_child().unwrap());
        }

        for view in items {
            let ac_view = view.clone();

            let Some(collection_type) = view.collection_type else {
                continue;
            };

            let results = match fetch_with_cache(
                &format!("library_{}", view.id),
                CachePolicy::ReadCacheAndRefresh,
                async move {
                    if collection_type == "livetv" {
                        EMBY_CLIENT.get_channels().await.map(|x| x.items)
                    } else {
                        EMBY_CLIENT.get_latest(&view.id).await
                    }
                },
            )
            .await
            {
                Ok(history) => history,
                Err(e) => {
                    toast!(self, e.to_user_facing());
                    return;
                }
            };

            let hortu = HortuScrolled::new(false);

            hortu.set_moreview(true);

            hortu.set_title(&format!("{} {}", gettext("Latest"), view.name));

            hortu.set_items(&results);

            hortu.connect_morebutton(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move |_| {
                    let list_item = TuItem::default();
                    list_item.set_id(ac_view.id.clone());
                    list_item.set_name(ac_view.name.clone());
                    list_item.set_item_type(ac_view.item_type.clone());
                    list_item.set_collection_type(ac_view.collection_type.clone());
                    list_item.activate(&obj, None);
                }
            ));

            libsbox.append(&hortu);
        }
    }
}
