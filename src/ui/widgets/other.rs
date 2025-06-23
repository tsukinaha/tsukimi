use super::{
    horbu_scrolled::HorbuScrolled,
    item::dt,
    picture_loader::PictureLoader,
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
    ui::provider::{
        tu_item::TuItem,
        tu_object::TuObject,
    },
    utils::{
        CachePolicy,
        fetch_with_cache,
    },
};
use chrono::{
    DateTime,
    Utc,
};
use glib::Object;
use gtk::{
    gio,
    glib,
    prelude::*,
    subclass::prelude::*,
    template_callbacks,
};

pub(crate) mod imp {
    use std::cell::OnceCell;

    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        SignalListItemFactory,
        gio,
        glib,
        prelude::*,
    };

    use crate::{
        ui::{
            provider::{
                tu_item::TuItem,
                tu_object::TuObject,
            },
            widgets::{
                horbu_scrolled::HorbuScrolled,
                hortu_scrolled::HortuScrolled,
                item_actionbox::ItemActionsBox,
                tu_overview_item::imp::ViewGroup,
                utils::TuItemBuildExt,
            },
        },
        utils::spawn_g_timeout,
    };
    // Object holding the state
    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/other.ui")]
    #[properties(wrapper_type = super::OtherPage)]
    pub struct OtherPage {
        #[property(get, set, construct_only)]
        pub item: OnceCell<TuItem>,
        #[template_child]
        pub picbox: TemplateChild<gtk::Box>,
        #[template_child]
        pub inscription: TemplateChild<gtk::Inscription>,
        #[template_child]
        pub inforevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub title: TemplateChild<gtk::Label>,

        #[template_child]
        pub actorhortu: TemplateChild<HortuScrolled>,
        #[template_child]
        pub moviehortu: TemplateChild<HortuScrolled>,
        #[template_child]
        pub serieshortu: TemplateChild<HortuScrolled>,
        #[template_child]
        pub episodehortu: TemplateChild<HortuScrolled>,
        #[template_child]
        pub videohortu: TemplateChild<HortuScrolled>,

        #[template_child]
        pub studioshorbu: TemplateChild<HorbuScrolled>,
        #[template_child]
        pub tagshorbu: TemplateChild<HorbuScrolled>,
        #[template_child]
        pub genreshorbu: TemplateChild<HorbuScrolled>,
        #[template_child]
        pub linkshorbu: TemplateChild<HorbuScrolled>,

        #[template_child]
        pub actionbox: TemplateChild<ItemActionsBox>,
        #[template_child]
        pub play_button: TemplateChild<gtk::Button>,

        #[template_child]
        pub information_box: TemplateChild<gtk::Box>,

        #[template_child]
        pub episode_list: TemplateChild<gtk::ListView>,
        #[template_child]
        pub episode_list_revealer: TemplateChild<gtk::Revealer>,

        pub selection: gtk::SingleSelection,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for OtherPage {
        const NAME: &'static str = "OtherPage";
        type Type = super::OtherPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            HortuScrolled::ensure_type();
            HorbuScrolled::ensure_type();
            ItemActionsBox::ensure_type();
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for OtherPage {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            let store = gio::ListStore::new::<TuObject>();

            spawn_g_timeout(glib::clone!(
                #[weak]
                obj,
                async move {
                    obj.setup_pic();
                    obj.get_item().await;
                }
            ));

            self.actionbox.set_id(Some(obj.item().id()));
            self.selection.set_model(Some(&store));
            self.episode_list.set_factory(Some(
                SignalListItemFactory::new().tu_overview_item(ViewGroup::EpisodesView),
            ));
            self.episode_list.set_model(Some(&self.selection));
        }
    }

    impl WidgetImpl for OtherPage {}

    impl WindowImpl for OtherPage {}

    impl ApplicationWindowImpl for OtherPage {}

    impl adw::subclass::navigation_page::NavigationPageImpl for OtherPage {}
}

glib::wrapper! {
    pub struct OtherPage(ObjectSubclass<imp::OtherPage>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget ,adw::NavigationPage,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

#[template_callbacks]
impl OtherPage {
    pub fn new(item: &TuItem) -> Self {
        Object::builder().property("item", item).build()
    }

    pub fn setup_pic(&self) {
        let imp = self.imp();
        let id = self.item().id();
        let pic = PictureLoader::new(&id, "Primary", None);
        pic.set_size_request(218, 328);
        pic.set_halign(gtk::Align::Fill);
        pic.set_valign(gtk::Align::Start);
        imp.picbox.append(&pic);
    }

    pub async fn get_item(&self) {
        let id = self.item().id();

        fraction_reset!(self);
        let item = match fetch_with_cache(
            &format!("list_{id}"),
            CachePolicy::ReadCacheAndRefresh,
            async move { JELLYFIN_CLIENT.get_item_info(&id).await },
        )
        .await
        {
            Ok(item) => item,
            Err(e) => {
                self.toast(e.to_user_facing());
                return;
            }
        };

        self.set_list(item).await;
        fraction!(self);
    }

    pub async fn set_list(&self, item: SimpleListItem) {
        let imp = self.imp();
        if let Some(overview) = item.overview {
            imp.inscription.set_text(Some(&overview));
        }
        imp.title.set_text(&item.name);
        imp.inforevealer.set_reveal_child(true);
        if let Some(links) = item.external_urls {
            self.add_external_link_horbu(&links);
        }
        if let Some(actors) = item.people {
            self.add_actor_item_hortu(&actors);
        }
        if let Some(studios) = item.studios {
            self.add_sgt_item_horbu(&self.imp().studioshorbu, &studios, "Studios");
        }
        if let Some(genres) = item.genres {
            self.add_sgt_item_horbu(&self.imp().genreshorbu, &genres, "Studios");
        }
        if let Some(tags) = item.tags {
            self.add_sgt_item_horbu(&self.imp().tagshorbu, &tags, "Studios");
        }
        if let Some(media_source) = item.media_sources {
            self.add_media_source(media_source, item.date_created);
        }
        if let Some(userdata) = item.user_data {
            if let Some(is_favorite) = userdata.is_favorite {
                imp.actionbox.set_btn_active(is_favorite);
            }
        }

        match self.item().item_type().as_str() {
            "Person" | "Actor" | "Director" | "Writer" | "Producer" => {
                self.hortu_set_actor_list("Movie").await;
                self.hortu_set_actor_list("Series").await;
                self.hortu_set_actor_list("Episode").await;
            }
            "BoxSet" | "Playlist" => {
                self.hortu_set_boxset_list().await;
            }
            "Audio" => {
                self.imp().play_button.set_visible(true);
                self.imp().play_button.connect_clicked(glib::clone!(
                    #[weak(rename_to = obj)]
                    self,
                    move |_| {
                        obj.item().play_single_audio(&obj);
                    }
                ));
            }
            "Season" => {
                self.hortu_set_season_episode_list().await;
            }
            "TvChannel" => {
                self.imp().play_button.set_visible(true);
                self.imp().play_button.connect_clicked(glib::clone!(
                    #[weak(rename_to = obj)]
                    self,
                    move |_| {
                        obj.item().play_tvchannel(&obj);
                    }
                ));
            }
            _ => {}
        }
    }

    pub fn add_media_source(
        &self, media_sources: Vec<MediaSource>, date_created: Option<DateTime<Utc>>,
    ) {
        let mediainfo_box = self.imp().information_box.get();
        for mediasource in media_sources {
            let info = format!(
                "{}\n{} {} {}\n{}",
                mediasource.path.unwrap_or_default(),
                mediasource.container.unwrap_or_default().to_uppercase(),
                bytefmt::format(mediasource.size.unwrap_or_default()),
                dt(date_created),
                mediasource.name
            );
            let label = gtk::Label::builder()
                .label(&info)
                .halign(gtk::Align::Start)
                .margin_start(15)
                .valign(gtk::Align::Start)
                .margin_top(5)
                .ellipsize(gtk::pango::EllipsizeMode::End)
                .build();
            label.add_css_class("caption-heading");
            mediainfo_box.append(&label);
        }
    }

    async fn hortu_set_season_episode_list(&self) {
        let id = self.item().id();
        let Some(series_id) = self.item().series_id() else {
            return;
        };
        let list = match fetch_with_cache(
            &format!("season_{id}"),
            CachePolicy::ReadCacheAndRefresh,
            async move { JELLYFIN_CLIENT.get_episodes_all(&series_id, &id).await },
        )
        .await
        {
            Ok(history) => history,
            Err(e) => {
                self.toast(e.to_user_facing());
                return;
            }
        };

        let store = self
            .imp()
            .selection
            .model()
            .unwrap()
            .downcast::<gio::ListStore>()
            .unwrap();

        for item in list.items {
            let tu_item = TuItem::from_simple(&item, None);
            tu_item.set_is_resume(true);
            let tu_item = TuObject::new(&tu_item);
            store.append(&tu_item);
        }

        self.imp().episode_list_revealer.set_vexpand(true);
        self.imp().episode_list_revealer.set_reveal_child(true);
    }

    pub fn add_external_link_horbu(&self, links: &[Urls]) {
        let imp = self.imp();
        imp.linkshorbu.set_links(links);
    }

    pub fn add_sgt_item_horbu(&self, horbu: &HorbuScrolled, items: &[SGTitem], type_: &str) {
        horbu.set_items(items, type_);
    }

    #[template_callback]
    fn on_listview_item_activated(&self, position: u32, view: &gtk::ListView) {
        let model = view.model().unwrap();
        let tu_obj = model.item(position).and_downcast::<TuObject>().unwrap();
        tu_obj.activate(view);
    }

    pub fn add_actor_item_hortu(&self, items: &[SimpleListItem]) {
        let hortu = self.imp().actorhortu.get();
        hortu.set_items(items);
    }

    async fn hortu_set_boxset_list(&self) {
        let id = self.item().id();
        let results = match fetch_with_cache(
            &format!("boxset_{id}"),
            CachePolicy::ReadCacheAndRefresh,
            async move { JELLYFIN_CLIENT.get_includedby(&id).await },
        )
        .await
        {
            Ok(history) => history,
            Err(e) => {
                self.toast(e.to_user_facing());
                return;
            }
        };

        self.load_list_items(results.items);
    }

    fn load_list_items(&self, items: Vec<SimpleListItem>) {
        let imp = self.imp();
        let mut movies = Vec::new();
        let mut series = Vec::new();
        let mut episodes = Vec::new();
        items
            .into_iter()
            .for_each(|item| match item.item_type.as_str() {
                "Movie" => movies.push(item),
                "Series" => series.push(item),
                "Episode" => episodes.push(item),
                _ => {}
            });

        imp.moviehortu.set_items(&movies);
        imp.serieshortu.set_items(&series);
        imp.episodehortu.set_items(&episodes);
    }

    async fn hortu_set_actor_list(&self, type_: &str) {
        let hortu = match type_ {
            "Movie" => self.imp().moviehortu.get(),
            "Series" => self.imp().serieshortu.get(),
            "Episode" => self.imp().episodehortu.get(),
            "Video" => self.imp().videohortu.get(),
            _ => return,
        };

        let type1_ = type_.to_string();
        let type2_ = type_.to_string();

        hortu.connect_morebutton(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            move |_| {
                let id = obj.item().id();
                let tag = format!("{} of {}", type1_, obj.item().name());
                let page = crate::ui::widgets::single_grid::SingleGrid::new();
                let type_clone1 = type1_.to_owned();
                let type_clone2 = type1_.to_owned();
                let id_clone1 = id.to_owned();
                let id_clone2 = id.to_owned();
                page.connect_sort_changed_tokio(false, move |sort_by, sort_order, filters_list| {
                    let id_clone1 = id_clone1.to_owned();
                    let type_clone1 = type_clone1.to_owned();
                    async move {
                        JELLYFIN_CLIENT
                            .get_person_large_list(
                                &id_clone1,
                                &type_clone1,
                                &sort_by,
                                &sort_order,
                                0,
                                &filters_list,
                            )
                            .await
                    }
                });
                page.connect_end_edge_overshot_tokio(
                    move |sort_by, sort_order, n_items, filters_list| {
                        let id_clone2 = id_clone2.to_owned();
                        let type_clone2 = type_clone2.to_owned();
                        async move {
                            JELLYFIN_CLIENT
                                .get_person_large_list(
                                    &id_clone2,
                                    &type_clone2,
                                    &sort_by,
                                    &sort_order,
                                    n_items,
                                    &filters_list,
                                )
                                .await
                        }
                    },
                );
                push_page_with_tag(&obj, page, &tag, &tag);
            }
        ));

        let type_ = type2_.to_owned();

        let id = self.item().id();

        let results = match fetch_with_cache(
            &format!("other_{}_{}", type_, &id),
            CachePolicy::ReadCacheAndRefresh,
            async move { JELLYFIN_CLIENT.get_actor_item_list(&id, &type_).await },
        )
        .await
        {
            Ok(history) => history,
            Err(e) => {
                self.toast(e.to_user_facing());
                List::default()
            }
        };

        hortu.set_items(&results.items);
    }
}
