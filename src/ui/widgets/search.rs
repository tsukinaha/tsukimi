use glib::Object;
use gtk::{
    gio,
    glib,
    prelude::*,
    subclass::prelude::*,
    template_callbacks,
};

use crate::{
    client::{
        emby_client::EMBY_CLIENT,
        error::UserFacingError,
        structs::*,
    },
    fraction,
    fraction_reset,
    toast,
    ui::provider::tu_item::TuItem,
    utils::{
        spawn,
        spawn_tokio,
    },
};

mod imp {

    use std::sync::atomic::Ordering;

    use glib::subclass::InitializingObject;
    use gst::prelude::StaticTypeExt;
    use gtk::{
        glib,
        subclass::prelude::*,
        CompositeTemplate,
    };

    use crate::{
        ui::widgets::tuview_scrolled::TuViewScrolled,
        utils::spawn,
    };

    // Object holding the state
    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/search.ui")]
    pub struct SearchPage {
        #[template_child]
        pub searchentry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub searchscrolled: TemplateChild<TuViewScrolled>,
        #[template_child]
        pub recommendbox: TemplateChild<gtk::Box>,
        #[template_child]
        pub movie: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub series: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub boxset: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub person: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub music: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub audio: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub video: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub episode: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        pub selection: gtk::SingleSelection,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SearchPage {
        const NAME: &'static str = "SearchPage";
        type Type = super::SearchPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            TuViewScrolled::ensure_type();
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SearchPage {
        fn constructed(&self) {
            let obj = self.obj();
            self.parent_constructed();
            self.searchscrolled.connect_end_edge_reached(glib::clone!(
                #[weak]
                obj,
                move |scrolled, lock| {
                    spawn(glib::clone!(
                        #[weak]
                        obj,
                        #[weak]
                        scrolled,
                        async move {
                            let search_results = obj.get_search_results::<true>().await;

                            scrolled.set_store::<false>(search_results.items, false);

                            lock.store(false, Ordering::SeqCst);
                        },
                    ))
                }
            ));
            obj.update();
        }
    }

    impl WidgetImpl for SearchPage {}

    impl WindowImpl for SearchPage {}

    impl ApplicationWindowImpl for SearchPage {}

    impl adw::subclass::navigation_page::NavigationPageImpl for SearchPage {}
}

glib::wrapper! {
    pub struct SearchPage(ObjectSubclass<imp::SearchPage>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget ,adw::NavigationPage,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl Default for SearchPage {
    fn default() -> Self {
        Self::new()
    }
}

#[template_callbacks]
impl SearchPage {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn update(&self) {
        spawn(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                obj.setup_recommend().await;
            }
        ));
    }

    pub async fn setup_recommend(&self) {
        let recommend = match spawn_tokio(async { EMBY_CLIENT.get_search_recommend().await }).await
        {
            Ok(list) => list,
            Err(e) => {
                toast!(self, e.to_user_facing());
                List::default()
            }
        };

        let imp = self.imp();
        let recommendbox = imp.recommendbox.get();
        for _ in 0..recommendbox.observe_children().n_items() {
            recommendbox.remove(&recommendbox.last_child().unwrap());
        }

        for item in recommend.items {
            let button = gtk::Button::new();
            let buttoncontent = adw::ButtonContent::builder()
                .label(&item.name)
                .icon_name(if item.item_type == "Movie" {
                    "video-display-symbolic"
                } else {
                    "video-reel-symbolic"
                })
                .build();
            button.set_halign(gtk::Align::Center);
            button.set_child(Some(&buttoncontent));
            button.connect_clicked(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move |_| {
                    let tu_item = TuItem::from_simple(&item, None);
                    tu_item.activate(&obj, None);
                }
            ));
            recommendbox.append(&button);
        }

        imp.stack.set_visible_child_name("recommend");
    }

    #[template_callback]
    async fn on_search_activate(&self) {
        let imp = self.imp();

        let search_results = self.get_search_results::<false>().await;

        if search_results.items.is_empty() {
            imp.stack.set_visible_child_name("fallback");
            return;
        };

        imp.searchscrolled
            .set_store::<true>(search_results.items, false);

        imp.stack.set_visible_child_name("result");
    }

    pub async fn get_search_results<const F: bool>(&self) -> List {
        let imp = self.imp();

        let search_content = imp.searchentry.text().to_string();
        let search_filter = {
            let mut filter = Vec::new();
            if imp.movie.is_active() {
                filter.push("Movie");
            }
            if imp.series.is_active() {
                filter.push("Series");
            }
            if imp.boxset.is_active() {
                filter.push("BoxSet");
            }
            if imp.person.is_active() {
                filter.push("Person");
            }
            if imp.music.is_active() {
                filter.push("MusicAlbum");
            }
            if imp.audio.is_active() {
                filter.push("Audio");
            }
            if imp.video.is_active() {
                filter.push("Video");
            }
            if imp.episode.is_active() {
                filter.push("Episode");
            }
            if filter.is_empty() {
                return List::default();
            }
            filter
        };
        let n_items = if F {
            fraction_reset!(self);
            imp.searchscrolled.n_items()
        } else {
            imp.stack.set_visible_child_name("loading");
            0
        };

        let search_results = match spawn_tokio(async move {
            EMBY_CLIENT
                .search(&search_content, &search_filter, &n_items.to_string())
                .await
        })
        .await
        {
            Ok(list) => list,
            Err(e) => {
                toast!(self, e.to_user_facing());
                List::default()
            }
        };

        if F {
            fraction!(self)
        }

        search_results
    }
}
