use gettextrs::gettext;
use glib::Object;
use gtk::{
    gio,
    glib,
    prelude::*,
    subclass::prelude::*,
};

use super::utils::GlobalToast;
use crate::{
    client::{
        error::UserFacingError,
        jellyfin_client::JELLYFIN_CLIENT,
        structs::*,
    },
    fraction,
    fraction_reset,
    utils::{
        spawn,
        spawn_tokio,
    },
};
mod imp {
    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        glib,
        subclass::prelude::*,
    };

    use crate::ui::widgets::hortu_scrolled::HortuScrolled;

    // Object holding the state
    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/liked.ui")]
    pub struct LikedPage {
        #[template_child]
        pub moviehortu: TemplateChild<HortuScrolled>,
        #[template_child]
        pub serieshortu: TemplateChild<HortuScrolled>,
        #[template_child]
        pub episodehortu: TemplateChild<HortuScrolled>,
        #[template_child]
        pub peoplehortu: TemplateChild<HortuScrolled>,
        #[template_child]
        pub albumhortu: TemplateChild<HortuScrolled>,
        #[template_child]
        pub boxsethortu: TemplateChild<HortuScrolled>,
        #[template_child]
        pub tvhortu: TemplateChild<HortuScrolled>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LikedPage {
        const NAME: &'static str = "LikedPage";
        type Type = super::LikedPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for LikedPage {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.update();
        }
    }

    impl WidgetImpl for LikedPage {}

    impl WindowImpl for LikedPage {}

    impl ApplicationWindowImpl for LikedPage {}

    impl adw::subclass::navigation_page::NavigationPageImpl for LikedPage {}
}

glib::wrapper! {
    pub struct LikedPage(ObjectSubclass<imp::LikedPage>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget ,adw::NavigationPage,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl Default for LikedPage {
    fn default() -> Self {
        Self::new()
    }
}

impl LikedPage {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn update(&self) {
        spawn(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                obj.set_lists().await;
            }
        ));
    }

    pub async fn set_lists(&self) {
        fraction_reset!(self);
        self.sets("Movie").await;
        self.sets("Series").await;
        self.sets("Episode").await;
        self.sets("People").await;
        self.sets("MusicAlbum").await;
        self.sets("BoxSet").await;
        self.sets("TvChannel").await;
        self.ensure_items();
        fraction!(self);
    }

    fn ensure_items(&self) {
        let imp = self.imp();
        if !imp.moviehortu.is_visible()
            && !imp.serieshortu.is_visible()
            && !imp.episodehortu.is_visible()
            && !imp.peoplehortu.is_visible()
            && !imp.albumhortu.is_visible()
            && !imp.boxsethortu.is_visible()
            && !imp.tvhortu.is_visible()
        {
            imp.stack.set_visible_child_name("fallback");
        }
    }

    pub async fn sets(&self, types: &str) {
        let hortu = match types {
            "Movie" => self.imp().moviehortu.get(),
            "Series" => self.imp().serieshortu.get(),
            "Episode" => self.imp().episodehortu.get(),
            "People" => self.imp().peoplehortu.get(),
            "MusicAlbum" => self.imp().albumhortu.get(),
            "BoxSet" => self.imp().boxsethortu.get(),
            "TvChannel" => self.imp().tvhortu.get(),
            _ => return,
        };

        hortu.set_title(format!("{} {}", gettext("Favourite"), gettext(types)));

        let types = types.to_string();

        let type_ = types.to_owned();

        let results = match spawn_tokio(async move {
            JELLYFIN_CLIENT
                .get_favourite(&types, 0, 12, "SortName", "Ascending", &Default::default())
                .await
        })
        .await
        {
            Ok(history) => history,
            Err(e) => {
                self.toast(e.to_user_facing());
                List::default()
            }
        };

        if results.items.is_empty() {
            hortu.set_visible(false);
            return;
        }

        hortu.set_items(&results.items);

        hortu.connect_morebutton(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            move |_| {
                let tag = format!("{} {}", "Favourite", type_);
                let page = crate::ui::widgets::single_grid::SingleGrid::new();
                let type_clone1 = type_.to_owned();
                let type_clone2 = type_.to_owned();
                page.connect_sort_changed_tokio(false, move |sort_by, sort_order, filters_list| {
                    let type_clone1 = type_clone1.to_owned();
                    async move {
                        JELLYFIN_CLIENT
                            .get_favourite(
                                &type_clone1,
                                0,
                                50,
                                &sort_by,
                                &sort_order,
                                &filters_list,
                            )
                            .await
                    }
                });
                page.connect_end_edge_overshot_tokio(
                    move |sort_by, sort_order, n_items, filters_list| {
                        let type_clone2 = type_clone2.to_owned();
                        async move {
                            JELLYFIN_CLIENT
                                .get_favourite(
                                    &type_clone2,
                                    n_items,
                                    50,
                                    &sort_by,
                                    &sort_order,
                                    &filters_list,
                                )
                                .await
                        }
                    },
                );
                push_page_with_tag(&obj, page, &tag, &tag);
            }
        ));
    }
}
