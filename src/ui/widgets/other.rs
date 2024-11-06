use glib::Object;
use gtk::{
    gio,
    glib,
    prelude::*,
    subclass::prelude::*,
};

use super::{horbu_scrolled::HorbuScrolled, picture_loader::PictureLoader};
use crate::{
    client::{
        client::EMBY_CLIENT,
        error::UserFacingError,
        structs::*,
    },
    fraction,
    fraction_reset,
    toast,
    ui::provider::tu_item::TuItem,
    utils::{
        fetch_with_cache,
        CachePolicy,
    },
};

pub(crate) mod imp {
    use std::cell::OnceCell;

    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{
        glib,
        prelude::*,
        CompositeTemplate,
    };

    use crate::{
        ui::{
            provider::tu_item::TuItem,
            widgets::{
                horbu_scrolled::HorbuScrolled,
                hortu_scrolled::HortuScrolled,
                item_actionbox::ItemActionsBox,
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
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for OtherPage {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "OtherPage";
        type Type = super::OtherPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            HortuScrolled::ensure_type();
            HorbuScrolled::ensure_type();
            ItemActionsBox::ensure_type();
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // Trait shared by all GObjects
    #[glib::derived_properties]
    impl ObjectImpl for OtherPage {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            spawn_g_timeout(glib::clone!(
                #[weak]
                obj,
                async move {
                    obj.setup_pic();
                    obj.get_item().await;
                }
            ));

            self.actionbox.set_id(Some(obj.item().id()));
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for OtherPage {}

    // Trait shared by all windows
    impl WindowImpl for OtherPage {}

    // Trait shared by all application windows
    impl ApplicationWindowImpl for OtherPage {}

    impl adw::subclass::navigation_page::NavigationPageImpl for OtherPage {}
}

glib::wrapper! {
    pub struct OtherPage(ObjectSubclass<imp::OtherPage>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget ,adw::NavigationPage,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

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
            &format!("list_{}", id),
            CachePolicy::ReadCacheAndRefresh,
            async move { EMBY_CLIENT.get_item_info(&id).await },
        )
        .await
        {
            Ok(item) => item,
            Err(e) => {
                toast!(self, e.to_user_facing());
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
            },
            "BoxSet" => {
                self.hortu_set_boxset_list().await;
            }
            _ => {

            }
        }
        

    }

    pub fn add_external_link_horbu(&self, links: &[Urls]) {
        let imp = self.imp();
        imp.linkshorbu.set_links(&links);
    }

    pub fn add_sgt_item_horbu(&self, horbu: &HorbuScrolled, items: &[SGTitem], type_: &str) {
        horbu.set_items(&items, type_);
    }

    pub fn add_actor_item_hortu(&self, items: &[SimpleListItem]) {
        let hortu = self.imp().actorhortu.get();
        hortu.set_items(&items);
    }

    async fn hortu_set_boxset_list(&self) {
        let id = self.item().id();
        let imp = self.imp();
        let results = match fetch_with_cache(
            &format!("boxset_{}", id),
            CachePolicy::ReadCacheAndRefresh,
            async move { EMBY_CLIENT.get_includedby(&id).await },
        ).await {
            Ok(history) => history,
            Err(e) => {
                toast!(self, e.to_user_facing());
                return;
            }
        };

        let mut movies = Vec::new();
        let mut series = Vec::new();
        let mut episodes = Vec::new();
        results.items.into_iter().for_each(|item| {
            match item.item_type.as_str() {
                "Movie" => movies.push(item),
                "Series" => series.push(item),
                "Episode" => episodes.push(item),
                _ => {},
            }
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
                let type_clone1 = type1_.clone();
                let type_clone2 = type1_.clone();
                let id_clone1 = id.clone();
                let id_clone2 = id.clone();
                page.connect_sort_changed_tokio(false, move |sort_by, sort_order| {
                    let id_clone1 = id_clone1.clone();
                    let type_clone1 = type_clone1.clone();
                    async move {
                        EMBY_CLIENT
                            .get_person_large_list(
                                &id_clone1,
                                &type_clone1,
                                &sort_by,
                                &sort_order,
                                0,
                            )
                            .await
                    }
                });
                page.connect_end_edge_overshot_tokio(false, move |sort_by, sort_order, n_items| {
                    let id_clone2 = id_clone2.clone();
                    let type_clone2 = type_clone2.clone();
                    async move {
                        EMBY_CLIENT
                            .get_person_large_list(
                                &id_clone2,
                                &type_clone2,
                                &sort_by,
                                &sort_order,
                                n_items,
                            )
                            .await
                    }
                });
                page.emit_by_name::<()>("sort-changed", &[]);
                push_page_with_tag(&obj, page, tag);
            }
        ));

        let type_ = type2_.clone();

        let id = self.item().id();

        let results = match fetch_with_cache(
            &format!("other_{}_{}", type_, &id),
            CachePolicy::ReadCacheAndRefresh,
            async move { EMBY_CLIENT.get_actor_item_list(&id, &type_).await },
        )
        .await
        {
            Ok(history) => history,
            Err(e) => {
                toast!(self, e.to_user_facing());
                List::default()
            }
        };

        hortu.set_items(&results.items);
    }
}
