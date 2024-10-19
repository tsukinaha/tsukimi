use crate::client::client::EMBY_CLIENT;
use crate::client::error::UserFacingError;
use crate::client::structs::*;
use crate::ui::provider::tu_item::TuItem;
use crate::utils::{fetch_with_cache, spawn, CachePolicy};
use crate::{fraction, fraction_reset, toast};
use gettextrs::gettext;
use glib::Object;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

use super::picture_loader::PictureLoader;

pub(crate) mod imp {
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::prelude::*;
    use gtk::{glib, CompositeTemplate};
    use std::cell::OnceCell;

    use crate::ui::provider::tu_item::TuItem;
    use crate::ui::widgets::horbu_scrolled::HorbuScrolled;
    use crate::ui::widgets::hortu_scrolled::HortuScrolled;
    use crate::ui::widgets::item_actionbox::ItemActionsBox;
    use crate::utils::spawn_g_timeout;
    // Object holding the state
    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/com/github/inaha/tsukimi/ui/actor.ui")]
    #[properties(wrapper_type = super::ActorPage)]
    pub struct ActorPage {
        #[property(get, set, construct_only)]
        pub item: OnceCell<TuItem>,
        #[template_child]
        pub actorpicbox: TemplateChild<gtk::Box>,
        #[template_child]
        pub inscription: TemplateChild<gtk::Inscription>,
        #[template_child]
        pub inforevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub title: TemplateChild<gtk::Label>,
        #[template_child]
        pub moviehortu: TemplateChild<HortuScrolled>,
        #[template_child]
        pub serieshortu: TemplateChild<HortuScrolled>,
        #[template_child]
        pub episodehortu: TemplateChild<HortuScrolled>,
        #[template_child]
        pub linkshorbu: TemplateChild<HorbuScrolled>,

        #[template_child]
        pub actionbox: TemplateChild<ItemActionsBox>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for ActorPage {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "ActorPage";
        type Type = super::ActorPage;
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
    impl ObjectImpl for ActorPage {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            spawn_g_timeout(glib::clone!(
                #[weak]
                obj,
                async move {
                    obj.setup_pic();
                    obj.get_item().await;
                    obj.set_lists().await;
                }
            ));

            self.actionbox.set_id(Some(obj.item().id()));
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for ActorPage {}

    // Trait shared by all windows
    impl WindowImpl for ActorPage {}

    // Trait shared by all application windows
    impl ApplicationWindowImpl for ActorPage {}

    impl adw::subclass::navigation_page::NavigationPageImpl for ActorPage {}
}

glib::wrapper! {
    pub struct ActorPage(ObjectSubclass<imp::ActorPage>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget ,adw::NavigationPage,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl ActorPage {
    pub fn new(item: &TuItem) -> Self {
        Object::builder().property("item", item).build()
    }

    pub fn setup_pic(&self) {
        let imp = self.imp();
        let id = self.item().id();
        let pic = PictureLoader::new(&id, "Primary", None);
        pic.set_size_request(218, 328);
        pic.set_halign(gtk::Align::Start);
        pic.set_valign(gtk::Align::Start);
        imp.actorpicbox.append(&pic);
    }

    pub async fn get_item(&self) {
        let imp = self.imp();
        let id = self.item().id();
        let inscription = imp.inscription.get();
        let inforevealer = imp.inforevealer.get();
        let title = imp.title.get();

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

        spawn(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                if let Some(overview) = item.overview {
                    inscription.set_text(Some(&overview));
                }
                if let Some(links) = item.external_urls {
                    obj.setlinksscrolled(links);
                }
                if let Some(userdata) = item.user_data {
                    if let Some(is_favorite) = userdata.is_favorite {
                        if is_favorite {
                            obj.imp().actionbox.set_btn_active(true);
                        } else {
                            obj.imp().actionbox.set_btn_active(false);
                        }
                    }
                }
                title.set_text(&item.name);
                inforevealer.set_reveal_child(true);
            }
        ));
    }

    pub async fn set_lists(&self) {
        fraction_reset!(self);
        self.sets("Movie").await;
        self.sets("Series").await;
        self.sets("Episode").await;
        fraction!(self);
    }

    pub async fn sets(&self, types: &str) {
        let hortu = match types {
            "Movie" => self.imp().moviehortu.get(),
            "Series" => self.imp().serieshortu.get(),
            "Episode" => self.imp().episodehortu.get(),
            _ => return,
        };

        let types_clone = types.to_string();
        hortu.set_title(&gettext(types));
        hortu.connect_morebutton(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            move |_| {
                let id = obj.item().id();
                let tag = format!("{} of {}", types_clone, obj.item().name());
                let page = crate::ui::widgets::single_grid::SingleGrid::new();
                let type_clone1 = types_clone.clone();
                let type_clone2 = types_clone.clone();
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

        let types = types.to_string();

        let id = self.item().id();

        let results = match fetch_with_cache(
            &format!("actor_{}_{}", types, &id),
            CachePolicy::ReadCacheAndRefresh,
            async move { EMBY_CLIENT.get_person(&id, &types).await },
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

    pub fn setlinksscrolled(&self, links: Vec<Urls>) {
        let imp = self.imp();

        let linkshorbu = imp.linkshorbu.get();

        linkshorbu.set_title(&gettext("Links"));

        linkshorbu.set_links(&links);
    }
}
