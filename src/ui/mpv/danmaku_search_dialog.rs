use adw::{
    prelude::*,
    subclass::prelude::*,
};
use dandanapi_client::{
    AnimeType,
    SearchSearchEpisodesParams,
};
use gettextrs::gettext;
use gtk::{
    glib,
    template_callbacks,
};

use crate::{
    client::structs::SimpleListItem,
    ui::{
        provider::tu_item::{
            DANMAKU_ANIME,
            TuItem,
        },
        widgets::{
            single_grid::imp::ViewType,
            tuview_scrolled::TuViewScrolled,
        },
    },
    utils::{
        spawn,
        spawn_tokio,
    },
};

use super::danmaku_client::DanmakuClient;

mod imp {
    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        glib,
    };

    use super::*;

    #[derive(Default, CompositeTemplate, glib::Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/danmaku_search_dialog.ui")]
    #[properties(wrapper_type = super::DanmakuSearchDialog)]
    pub struct DanmakuSearchDialog {
        #[template_child]
        pub view: TemplateChild<TuViewScrolled>,
        #[template_child]
        pub episode_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub navigation_view: TemplateChild<adw::NavigationView>,
        #[template_child]
        pub episodes_page: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub title_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub anime_type_dropdown: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub search_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub toast: TemplateChild<adw::ToastOverlay>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DanmakuSearchDialog {
        const NAME: &'static str = "DanmakuSearchDialog";
        type Type = super::DanmakuSearchDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
            klass.install_action_async(
                "danmaku-search-dialog.search",
                None,
                |dialog, _, _| async move {
                    dialog.search().await;
                },
            );
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for DanmakuSearchDialog {
        fn constructed(&self) {
            self.parent_constructed();
            self.view.set_view_type(ViewType::GridView);
        }
    }

    impl WidgetImpl for DanmakuSearchDialog {}
    impl AdwDialogImpl for DanmakuSearchDialog {}
}

glib::wrapper! {
    pub struct DanmakuSearchDialog(ObjectSubclass<imp::DanmakuSearchDialog>)
        @extends gtk::Widget, adw::Dialog, adw::PreferencesDialog, @implements gtk::Accessible, gtk::Root, gtk::Buildable, gtk::ConstraintTarget;
}

#[template_callbacks]
impl DanmakuSearchDialog {
    pub fn new() -> Self {
        glib::Object::new()
    }

    #[template_callback]
    fn search_cb(&self) {
        let _ = self.activate_action("danmaku-search-dialog.search", None);
    }

    fn set_searching(&self, searching: bool) {
        self.imp()
            .search_stack
            .set_visible_child_name(if searching { "loading" } else { "search" });
        self.action_set_enabled("danmaku-search-dialog.search", !searching);
    }

    fn show_toast(&self, message: impl Into<String>) {
        self.imp().toast.add_toast(
            adw::Toast::builder()
                .timeout(2)
                .use_markup(false)
                .title(message.into())
                .build(),
        );
    }

    fn selected_anime_type(&self) -> AnimeType {
        match self.imp().anime_type_dropdown.selected() {
            0 => AnimeType::Tvseries,
            1 => AnimeType::Tvspecial,
            2 => AnimeType::Ova,
            3 => AnimeType::Movie,
            4 => AnimeType::Musicvideo,
            5 => AnimeType::Web,
            6 => AnimeType::Other,
            7 => AnimeType::Jpmovie,
            8 => AnimeType::Jpdrama,
            9 => AnimeType::Tmdbtv,
            10 => AnimeType::Tmdbmovie,
            _ => AnimeType::Tvseries,
        }
    }

    async fn search(&self) {
        let title = self.imp().title_entry.text().trim().to_string();
        if title.chars().count() < 2 {
            self.show_toast(gettext("Enter at least two characters"));
            return;
        }

        let anime_type = self.selected_anime_type();
        self.set_searching(true);
        let result = spawn_tokio(async move {
            let client = DanmakuClient::new()?;
            client.search_anime_details(title, anime_type).await
        })
        .await;
        self.set_searching(false);

        match result {
            Ok(animes) => {
                let items = animes
                    .into_iter()
                    .map(SimpleListItem::from)
                    .collect::<Vec<_>>();
                let is_empty = items.is_empty();
                self.imp().view.set_store::<true>(items);
                if is_empty {
                    self.show_toast(gettext("No matching anime found"));
                }
            }
            Err(error) => {
                self.show_toast(format!("{}: {error}", gettext("Search failed")));
            }
        }
    }

    fn set_episode_items(&self, items: Vec<SimpleListItem>) {
        while let Some(child) = self.imp().episode_list.first_child() {
            self.imp().episode_list.remove(&child);
        }

        for simple_item in items {
            let item = TuItem::from_simple(simple_item);
            let row = adw::ActionRow::builder()
                .title(item.name())
                .activatable(true)
                .build();
            row.connect_activated(glib::clone!(
                #[strong]
                item,
                move |row| item.activate(row)
            ));
            self.imp().episode_list.append(&row);
        }
    }

    pub fn open_anime(&self, item: TuItem) {
        if item.item_type() != DANMAKU_ANIME {
            return;
        }

        self.imp().episodes_page.set_title(&item.name());
        self.set_episode_items(Vec::new());
        self.imp()
            .navigation_view
            .push(&self.imp().episodes_page.get());

        let anime_id = item.id();
        let anime_title = item.name();
        spawn(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                let result = spawn_tokio(async move {
                    let client = DanmakuClient::new()?;
                    client
                        .search_animes(SearchSearchEpisodesParams {
                            anime: Some(anime_title),
                            tmdb_id: None,
                            tmdb_id_type: 0,
                            episode: None,
                            v2: true,
                        })
                        .await
                })
                .await;

                match result {
                    Ok(animes) => {
                        let episodes = animes
                            .into_iter()
                            .find(|anime| {
                                anime.anime_id.map(|id| id.to_string()).as_deref()
                                    == Some(anime_id.as_str())
                            })
                            .and_then(|anime| anime.episodes)
                            .unwrap_or_default()
                            .into_iter()
                            .map(SimpleListItem::from)
                            .collect::<Vec<_>>();
                        let is_empty = episodes.is_empty();
                        obj.set_episode_items(episodes);
                        if is_empty {
                            obj.show_toast(gettext("No episodes found"));
                        }
                    }
                    Err(error) => {
                        obj.show_toast(format!("{}: {error}", gettext("Failed to load episodes")));
                    }
                }
            }
        ));
    }
}
