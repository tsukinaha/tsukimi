use super::{
    episode_switcher::{
        EpisodeButton,
        EpisodeSwitcher,
    },
    fix::ScrolledWindowFixExt,
    hor_controls::HorControlsExt,
    hortu_scrolled::UnifySize,
    item_utils::*,
    song_widget::format_duration,
    utils::{
        GlobalToast,
        run_time_ticks_to_label,
    },
    window::Window,
};
use crate::{
    client::{
        error::UserFacingError,
        jellyfin_client::JELLYFIN_CLIENT,
        structs::*,
    },
    ui::{
        SETTINGS,
        mpv::page::{
            PlaybackDirectMode,
            media_source_stream_url,
        },
        provider::{
            dropdown_factory::{
                DropdownList,
                DropdownListBuilder,
            },
            tu_item::TuItem,
            tu_object::TuObject,
        },
    },
    utils::{
        CacheEvent,
        CachePolicy,
        fetch_with_cache,
        get_image_with_cache,
        spawn,
        spawn_tokio,
    },
};
use adw::{
    prelude::*,
    subclass::prelude::*,
};
use chrono::{
    DateTime,
    Utc,
};
use gettextrs::gettext;
use glib::Object;
use gtk::{
    ListScrollFlags,
    ListView,
    PositionType,
    ScrolledWindow,
    gio,
    glib,
    template_callbacks,
};

pub(crate) mod imp {
    use std::cell::{
        Cell,
        OnceCell,
        RefCell,
    };

    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        glib,
        prelude::*,
    };

    use super::SimpleListItem;
    use crate::{
        ui::{
            provider::{
                dropdown_factory::factory,
                tu_item::TuItem,
                tu_object::TuObject,
            },
            widgets::{
                EpisodeSwitcher,
                fix::ScrolledWindowFixExt,
                hor_controls::HorControlsExt,
                horbu_scrolled::HorbuScrolled,
                hortu_scrolled::HortuScrolled,
                item_actionbox::ItemActionsBox,
                item_carousel::ItemCarousel,
                star_toggle::StarToggle,
                tu_overview_item::imp::ViewGroup,
                utils::TuItemBuildExt,
            },
        },
        utils::spawn_g_timeout,
    };

    // Object holding the state
    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/item.ui")]
    #[properties(wrapper_type = super::ItemPage)]
    pub struct ItemPage {
        #[property(get, set, construct_only)]
        pub item: OnceCell<TuItem>,

        #[template_child]
        pub actorhortu: TemplateChild<HortuScrolled>,
        #[template_child]
        pub recommendhortu: TemplateChild<HortuScrolled>,
        #[template_child]
        pub includehortu: TemplateChild<HortuScrolled>,
        #[template_child]
        pub additionalhortu: TemplateChild<HortuScrolled>,
        #[template_child]
        pub seasonshortu: TemplateChild<HortuScrolled>,

        #[template_child]
        pub studioshorbu: TemplateChild<HorbuScrolled>,
        #[template_child]
        pub tagshorbu: TemplateChild<HorbuScrolled>,
        #[template_child]
        pub genreshorbu: TemplateChild<HorbuScrolled>,
        #[template_child]
        pub linkshorbu: TemplateChild<HorbuScrolled>,

        #[template_child]
        pub itemlist: TemplateChild<gtk::ListView>,
        #[template_child]
        pub logobox: TemplateChild<gtk::Box>,
        #[template_child]
        pub seasonlist: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub season_view_more: TemplateChild<gtk::Button>,

        #[template_child]
        pub mediainfobox: TemplateChild<gtk::Box>,
        #[template_child]
        pub mediainforevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub scrolled: TemplateChild<gtk::ScrolledWindow>,

        #[template_child]
        pub line1: TemplateChild<gtk::Label>,
        #[template_child]
        pub episode_line: TemplateChild<gtk::Label>,
        #[template_child]
        pub line2: TemplateChild<gtk::Label>,
        #[template_child]
        pub crating: TemplateChild<gtk::Label>,
        #[template_child]
        pub orating: TemplateChild<gtk::Label>,
        #[template_child]
        pub star: TemplateChild<gtk::Image>,

        #[template_child]
        pub playbutton: TemplateChild<gtk::Button>,
        #[template_child]
        pub namedropdown: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub subdropdown: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub carousel: TemplateChild<ItemCarousel>,
        #[template_child]
        pub actionbox: TemplateChild<ItemActionsBox>,
        #[template_child]
        pub tagline: TemplateChild<gtk::Label>,
        #[template_child]
        pub toolbar: TemplateChild<gtk::Box>,
        #[template_child]
        pub episode_list_bin: TemplateChild<adw::Bin>,

        #[template_child]
        pub spinner: TemplateChild<adw::Spinner>,

        #[template_child]
        pub buttoncontent: TemplateChild<adw::ButtonContent>,

        #[template_child]
        pub indicator: TemplateChild<adw::CarouselIndicatorDots>,

        pub selection: gtk::SingleSelection,
        pub seasonselection: gtk::SingleSelection,
        pub playbuttonhandlerid: RefCell<Option<glib::SignalHandlerId>>,

        #[property(get, set, construct_only)]
        pub name: RefCell<Option<String>>,
        pub selected: RefCell<Option<String>>,

        pub videoselection: gtk::SingleSelection,
        pub subselection: gtk::SingleSelection,

        #[template_child]
        pub main_carousel: TemplateChild<adw::Carousel>,

        #[template_child]
        pub left_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub right_button: TemplateChild<gtk::Button>,

        #[template_child]
        pub episode_stack: TemplateChild<gtk::Stack>,

        #[template_child]
        pub episode_switcher: TemplateChild<EpisodeSwitcher>,

        pub show_left_animation: OnceCell<adw::TimedAnimation>,
        pub hide_left_animation: OnceCell<adw::TimedAnimation>,
        pub show_right_animation: OnceCell<adw::TimedAnimation>,
        pub hide_right_animation: OnceCell<adw::TimedAnimation>,
        pub is_hovering: Cell<bool>,

        #[property(get, set, nullable)]
        pub current_item: RefCell<Option<TuItem>>,

        // None if season not changed
        #[property(get, set, nullable)]
        pub current_season: RefCell<Option<String>>,
        #[property(get, set, nullable)]
        pub play_session_id: RefCell<Option<String>>,

        pub season_list_vec: RefCell<Vec<SimpleListItem>>,

        pub episode_list_vec: RefCell<Vec<SimpleListItem>>,

        pub video_version_matcher: RefCell<Option<String>>,
        pub pending_external_sub: RefCell<Option<String>>,
        pub imdb_id: RefCell<Option<String>>,
        pub tv_subtitle_btn: RefCell<Option<gtk::Button>>,
        pub mediainfo_selected: Cell<Option<usize>>,
        pub episode_subzone: Cell<u8>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ItemPage {
        const NAME: &'static str = "ItemPage";
        type Type = super::ItemPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            ItemCarousel::ensure_type();
            StarToggle::ensure_type();
            HortuScrolled::ensure_type();
            HorbuScrolled::ensure_type();
            EpisodeSwitcher::ensure_type();
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ItemPage {
        fn constructed(&self) {
            self.parent_constructed();
            self.scrolled.fix();

            self.indicator
                .set_carousel(Some(&self.carousel.imp().carousel));

            let namedropdown = self.namedropdown.get();
            let subdropdown = self.subdropdown.get();
            namedropdown.set_factory(Some(&factory::<true>()));
            namedropdown.set_list_factory(Some(&factory::<false>()));
            subdropdown.set_factory(Some(&factory::<true>()));
            subdropdown.set_list_factory(Some(&factory::<false>()));

            let store = gtk::gio::ListStore::new::<TuObject>();
            self.selection.set_model(Some(&store));
            self.itemlist.set_model(Some(&self.selection));
            self.itemlist.set_factory(Some(
                gtk::SignalListItemFactory::new().tu_overview_item(ViewGroup::EpisodesView),
            ));
            self.obj().connect_scroll_controls();

            let item = self.obj().item();

            if item.item_type() == "Series"
                || (item.item_type() == "Episode" && item.series_name().is_some())
            {
                self.toolbar.set_visible(true);
                self.episode_list_bin.set_visible(true);
                self.episode_line.set_visible(true);
            }

            let obj = self.obj();
            spawn_g_timeout(glib::clone!(
                #[weak]
                obj,
                async move {
                    obj.setup().await;
                }
            ));
        }
    }

    impl WidgetImpl for ItemPage {}

    impl WindowImpl for ItemPage {}

    impl ApplicationWindowImpl for ItemPage {}

    impl adw::subclass::navigation_page::NavigationPageImpl for ItemPage {}
}

glib::wrapper! {
    pub struct ItemPage(ObjectSubclass<imp::ItemPage>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget ,adw::NavigationPage,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

#[template_callbacks]
impl ItemPage {
    pub fn new(item: &TuItem) -> Self {
        Object::builder().property("item", item).build()
    }

    pub async fn setup(&self) {
        let item = self.item();
        let type_ = item.item_type();
        let imp = self.imp();

        if let Some(series_name) = item.series_name() {
            imp.line1.set_text(&series_name);
        } else {
            imp.line1.set_text(&item.name());
        }

        if type_ == "Series" {
            let series_id = item.id();

            if let Some(item) = self.set_shows_next_up(&series_id).await {
                // ensure current_item available before season episodes load
                self.set_current_item(Some(&item));
                spawn(glib::clone!(
                    #[weak(rename_to = obj)]
                    self,
                    #[strong]
                    item,
                    async move {
                        obj.set_intro::<false>(&item).await;
                    }
                ));
            } else {
                let imp = self.imp();
                imp.episode_line.set_text(&gettext("No episode selected"));
                imp.buttoncontent.set_label(&gettext("Select an episode"));
            }

            self.imp().actionbox.set_id(Some(series_id.to_owned()));
            self.setup_item(&series_id).await;
            self.setup_seasons(&series_id).await;
        } else if type_ == "Episode" && item.series_name().is_some() {
            let series_id = item.series_id().unwrap_or(item.id());
            self.set_current_item(Some(&item));
            spawn(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                #[weak]
                item,
                async move {
                    obj.set_intro::<false>(&item).await;
                }
            ));

            self.imp().actionbox.set_id(Some(series_id.to_owned()));
            self.setup_item(&series_id).await;
            self.setup_seasons(&series_id).await;
        } else {
            let id = item.id();

            spawn(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                async move {
                    obj.set_intro::<true>(&item).await;
                }
            ));

            self.imp().actionbox.set_id(Some(id.to_owned()));
            self.setup_item(&id).await;
        }
    }

    pub async fn update_intro(&self, current_item: TuItem) {
        let item = self.item();

        let id = current_item.id();
        let current_item =
            match spawn_tokio(async move { JELLYFIN_CLIENT.get_item_info(&id).await }).await {
                Ok(item) => TuItem::from_simple(item),
                Err(e) => {
                    self.toast(e.to_user_facing());
                    current_item
                }
            };

        if item.item_type() == "Series" || item.item_type() == "Episode" {
            self.set_intro::<false>(&current_item).await;
            self.on_season_selected(None, self.imp().seasonlist.get())
                .await;
        }

        if item.item_type() == "Video" || item.item_type() == "Movie" {
            self.set_intro::<true>(&current_item).await;
        }
    }

    async fn setup_item(&self, id: &str) {
        let id = id.to_string();
        let id_clone = id.to_owned();

        spawn(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                obj.set_logo(&id_clone).await;
            }
        ));

        self.setup_background(&id).await;
        self.set_overview(&id).await;
        self.set_lists(&id).await;
    }

    async fn set_intro<const IS_VIDEO: bool>(&self, intro: &TuItem) {
        let intro_id = intro.id();
        let play_button = self.imp().playbutton.get();
        let spinner = self.imp().spinner.get();

        self.set_now_item::<IS_VIDEO>(intro);

        play_button.set_sensitive(false);
        spinner.set_visible(true);

        let intro_id_clone = intro_id.to_owned();
        let playback = match spawn_tokio(async move {
            JELLYFIN_CLIENT
                .get_playbackinfo(
                    &intro_id_clone,
                    None,
                    None,
                    false,
                    PlaybackDirectMode::direct(),
                )
                .await
        })
        .await
        {
            Ok(playback) => playback,
            Err(e) => {
                self.toast(e.to_user_facing());
                return;
            }
        };

        self.set_current_item(Some(intro));
        self.set_dropdown(&playback).await;
        self.set_play_session_id(playback.play_session_id.to_owned());

        play_button.set_sensitive(true);
        spinner.set_visible(false);

        self.createmediabox(playback.media_sources, None).await;
    }

    #[template_callback]
    async fn on_season_selected(&self, _param: Option<glib::ParamSpec>, dropdown: gtk::DropDown) {
        let item = self.item();
        let item_type = item.item_type();
        if item_type != "Series" && item_type != "Episode" {
            return;
        }

        let imp = self.imp();
        imp.episode_stack.set_visible_child_name("loading");

        let series_id = item.series_id().unwrap_or(item.id());
        let position = dropdown.selected();

        let current_item = self.current_item();
        let current_season_id = current_item.as_ref().and_then(|item| item.season_id());

        let items = match (position, current_season_id) {
            (0, None) => vec![],
            (0, Some(season_id)) => {
                self.set_current_season(Some(season_id.to_owned()));
                match spawn_tokio(async move {
                    JELLYFIN_CLIENT
                        .get_episodes_all(&series_id, &season_id)
                        .await
                })
                .await
                {
                    Ok(res) => res.items,
                    Err(e) => {
                        self.toast(e.to_user_facing());
                        return;
                    }
                }
            }
            _ => {
                let season_id = {
                    let season_list = imp.season_list_vec.borrow();
                    let Some(season) = season_list.get(position.saturating_sub(1) as usize) else {
                        return;
                    };
                    self.set_current_season(Some(season.id.to_owned()));
                    season.id.to_owned()
                };
                match spawn_tokio(async move {
                    JELLYFIN_CLIENT
                        .get_episodes_all(&series_id, &season_id)
                        .await
                })
                .await
                {
                    Ok(res) => res.items,
                    Err(e) => {
                        self.toast(e.to_user_facing());
                        return;
                    }
                }
            }
        };

        let start_idx = if let Some(current_item) = current_item
            && self.current_season() == current_item.season_id()
        {
            Self::search_episode_index(&items, &current_item)
                .map(|_| {
                    current_item.index_number().saturating_sub(1) as usize
                        / EpisodeSwitcher::EPISODES_PER_GROUP
                        * EpisodeSwitcher::EPISODES_PER_GROUP
                })
                .unwrap_or_default()
        } else {
            0
        };

        let max_episode_number = items
            .last()
            .and_then(|item| item.index_number)
            .unwrap_or_default() as usize;

        self.set_episode_list(items, start_idx);
        self.imp().episode_switcher.load_from_range(
            max_episode_number,
            glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move |btn| {
                    spawn(glib::clone!(
                        #[weak]
                        obj,
                        #[weak]
                        btn,
                        async move {
                            obj.on_episode_switcher_clicked(&btn).await;
                        }
                    ))
                }
            ),
        );
    }

    fn set_episode_list(&self, list: Vec<SimpleListItem>, start_index: usize) {
        let imp = self.imp();
        imp.episode_list_vec.replace(list);
        self.set_episode_list_range(start_index);
    }

    fn set_episode_list_range(&self, start_index: usize) {
        let imp = self.imp();
        let store_model = imp.selection.model();
        let Some(store) = store_model.and_downcast_ref::<gio::ListStore>() else {
            return;
        };
        let list = imp.episode_list_vec.borrow();
        if list.is_empty() {
            imp.episode_stack.set_visible_child_name("fallback");
            return;
        }
        let (start_episode, end_episode) = (
            start_index as u32 + 1,
            start_index as u32 + EpisodeSwitcher::EPISODES_PER_GROUP as u32,
        );
        let (left, right) = (
            list.partition_point(|item| {
                item.index_number
                    .expect("index_number should be present in SimpleListItem")
                    < start_episode
            }),
            list.partition_point(|item| {
                item.index_number
                    .expect("index_number should be present in SimpleListItem")
                    <= end_episode
            }),
        );
        let slice = &list[left..right];
        let scroll_to = match self.current_item() {
            None => None,
            Some(item) => {
                let (season_id, index_number) = (item.season_id(), item.index_number());
                if self.current_season() != season_id
                    || index_number < start_episode
                    || index_number > end_episode
                {
                    None
                } else {
                    Self::search_episode_index(slice, &item)
                }
            }
        }
        .or_else(|| {
            // If the current item is not in this range, reset the list to its first item.
            (!self.is_at_lower() || imp.selection.selected() != 0).then_some(0)
        });

        let items = slice
            .iter()
            .map(|item| TuObject::from_simple(item.to_owned()))
            .collect::<Vec<_>>();
        store.splice(0, store.n_items(), &items);
        imp.episode_stack.set_visible_child_name("view");

        if let Some(scroll_index) = scroll_to {
            let itemlist = imp.itemlist.get();
            // Wait one frame so GtkListView can allocate rows before scrolling
            itemlist.add_tick_callback(move |itemlist, _| {
                let itemlist = itemlist.clone();
                glib::idle_add_local_once(move || {
                    itemlist.scroll_to(scroll_index as u32, ListScrollFlags::all(), None);
                });
                glib::ControlFlow::Break
            });
        }
    }

    fn search_episode_index(list: &[SimpleListItem], current_item: &TuItem) -> Option<usize> {
        let index_number = current_item.index_number();
        list.binary_search_by_key(&index_number, |item| {
            item.index_number
                .expect("index_number should be present in SimpleListItem")
        })
        .ok()
    }

    async fn on_episode_switcher_clicked(&self, btn: &EpisodeButton) {
        let start_index = btn.start_index();
        self.set_episode_list_range(start_index as usize);
    }

    async fn set_shows_next_up(&self, id: &str) -> Option<TuItem> {
        let id = id.to_string();
        let next_up =
            match spawn_tokio(async move { JELLYFIN_CLIENT.get_shows_next_up(&id).await }).await {
                Ok(next_up) => next_up,
                Err(e) => {
                    self.toast(e.to_user_facing());
                    return None;
                }
            };

        let next_up_item = next_up.items.into_iter().next()?;

        let tu_item = TuItem::from_simple(next_up_item);

        self.set_now_item::<false>(&tu_item);

        Some(tu_item)
    }

    fn set_now_item<const IS_VIDEO: bool>(&self, item: &TuItem) {
        let imp = self.imp();

        if IS_VIDEO {
            imp.episode_line.set_text(&item.name());
        } else {
            imp.episode_line.set_text(&format!(
                "S{}E{}: {}",
                item.parent_index_number(),
                item.index_number(),
                item.name()
            ));
        }

        let sec = item.playback_position_ticks() / 10000000;
        if sec > 10 {
            imp.buttoncontent.set_label(&format!(
                "{} {}",
                gettext("Resume"),
                format_duration(sec as i64)
            ));
        } else {
            imp.buttoncontent.set_label(&gettext("Play"));
        }
    }

    pub async fn set_dropdown(&self, playbackinfo: &Media) {
        let imp = self.imp();
        let namedropdown = imp.namedropdown.get();
        let subdropdown = imp.subdropdown.get();

        let matcher = imp.video_version_matcher.borrow().to_owned();

        let vstore = gtk::gio::ListStore::new::<glib::BoxedAnyObject>();
        imp.videoselection.set_model(Some(&vstore));

        let sstore = gtk::gio::ListStore::new::<glib::BoxedAnyObject>();
        imp.subselection.set_model(Some(&sstore));

        namedropdown.set_model(Some(&imp.videoselection));
        subdropdown.set_model(Some(&imp.subselection));

        let media_sources = playbackinfo.media_sources.to_owned();

        let mut v_dl: Vec<String> = Vec::new();

        namedropdown.connect_selected_item_notify(glib::clone!(
            #[weak]
            imp,
            move |dropdown| {
                let Some(entry) = dropdown
                    .selected_item()
                    .and_downcast::<glib::BoxedAnyObject>()
                else {
                    return;
                };

                let dl: std::cell::Ref<DropdownList> = entry.borrow();
                let selected = &dl.id;

                let mut objects = Vec::new();
                let mut subtitle_choice = None;
                for media in &media_sources {
                    if selected.as_deref().is_some_and(|s| s == media.id) {
                        let mut lang_list = Vec::new();
                        for stream in &media.media_streams {
                            if stream.stream_type == "Subtitle" {
                                let Ok(dl) = DropdownListBuilder::default()
                                    .line1(stream.display_title.to_owned())
                                    .line2(stream.title.to_owned())
                                    .sub_lang(stream.language.to_owned())
                                    .index(Some(stream.index))
                                    .url(stream.delivery_url.to_owned())
                                    .is_external(Some(stream.is_external))
                                    .build()
                                else {
                                    continue;
                                };

                                lang_list
                                    .push((stream.index, dl.line1.to_owned().unwrap_or_default()));
                                objects.push(glib::BoxedAnyObject::new(dl));
                            }
                        }

                        subtitle_choice = make_subtitle_version_choice(lang_list);
                        break;
                    }
                }
                sstore.splice(0, sstore.n_items(), &objects);
                if let Some(u) = subtitle_choice {
                    subdropdown.set_selected(u.1 as u32);
                }

                imp.video_version_matcher.replace(dl.line1.to_owned());
            }
        ));

        let mut objects = Vec::new();
        for media in &playbackinfo.media_sources {
            let line2 = media
                .bit_rate
                .map(|bit_rate| format!("{:.2} Kbps", bit_rate as f64 / 1_000.0))
                .unwrap_or_default();
            let play_url = media_source_stream_url(media).await;
            let Ok(dl) = DropdownListBuilder::default()
                .line1(Some(media.name.to_owned()))
                .line2(Some(line2))
                .url(play_url)
                .id(Some(media.id.to_owned()))
                .build()
            else {
                continue;
            };

            v_dl.push(dl.line1.to_owned().unwrap_or_default());
            objects.push(glib::BoxedAnyObject::new(dl));
        }

        vstore.extend_from_slice(&objects);

        if let Some(matcher) = matcher {
            if let Some(p) = make_video_version_choice_from_matcher(v_dl, &matcher) {
                namedropdown.set_selected(p as u32);
            }
        } else if let Some(p) = make_video_version_choice_from_filter(v_dl) {
            namedropdown.set_selected(p as u32);
        }
    }

    pub async fn setup_background(&self, id: &str) {
        let imp = self.imp();

        let backdrop = imp.carousel.imp().backdrop.get();
        let path = get_image_with_cache(id.to_string(), "Backdrop".to_string(), Some(0))
            .await
            .unwrap_or_default();
        let file = gtk::gio::File::for_path(&path);
        backdrop.set_file(Some(&file));
        self.imp()
            .carousel
            .imp()
            .backrevealer
            .set_reveal_child(true);
        spawn(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                let Some(window) = obj.root().and_downcast::<super::window::Window>() else {
                    return;
                };
                window.set_rootpic(file);
            }
        ));
    }

    pub async fn add_backdrops(&self, image_tags: Vec<String>, id: &str) {
        let imp = self.imp();
        let tags = image_tags.len();
        let carousel = imp.carousel.imp().carousel.get();
        for tag_num in 1..tags {
            let path =
                get_image_with_cache(id.to_string(), "Backdrop".to_string(), Some(tag_num as u8))
                    .await
                    .unwrap_or_default();
            let file = gtk::gio::File::for_path(&path);
            let picture = gtk::Picture::builder()
                .halign(gtk::Align::Fill)
                .valign(gtk::Align::Fill)
                .content_fit(gtk::ContentFit::Cover)
                .file(&file)
                .build();
            carousel.append(&picture);
        }
    }

    pub async fn setup_seasons(&self, id: &str) {
        let imp = self.imp();
        let id = id.to_string();

        let Some(season_list_store) = imp.seasonlist.model().and_downcast::<gtk::StringList>()
        else {
            return;
        };

        let mut events = fetch_with_cache(
            &format!("season_{}", id),
            CachePolicy::ReadCacheAndRefresh,
            async move { JELLYFIN_CLIENT.get_season_list(&id).await },
        )
        .await;

        while let Some(event) = events.recv().await {
            match event {
                CacheEvent::Data { data, .. } => {
                    let season_list = data.items;
                    let names = season_list
                        .iter()
                        .map(|season| season.name.as_str())
                        .collect::<Vec<_>>();
                    season_list_store.splice(
                        1,
                        season_list_store.n_items().saturating_sub(1),
                        &names,
                    );
                    imp.seasonshortu.set_items(season_list.to_owned());
                    imp.season_list_vec.replace(season_list);
                    self.on_season_selected(None, imp.seasonlist.get()).await;
                }
                CacheEvent::Error(e) => {
                    self.toast(e.to_user_facing());
                    return;
                }
            }
        }
    }

    #[template_callback]
    async fn on_item_activated(&self, position: u32, view: &ListView) {
        let Some(model) = view.model() else {
            return;
        };
        let Some(item) = model.item(position).and_downcast::<TuObject>() else {
            return;
        };
        self.set_intro::<false>(&item.item()).await;
    }

    pub async fn set_logo(&self, id: &str) {
        let logo = super::logo::set_logo(id.to_string(), "Logo", None).await;
        self.imp().logobox.append(&logo);
    }

    pub async fn set_overview(&self, id: &str) {
        let id = id.to_string();

        let mut events = fetch_with_cache(
            &format!("item_{}", id),
            CachePolicy::ReadCacheAndRefresh,
            async move { JELLYFIN_CLIENT.get_item_info(&id).await },
        )
        .await;

        while let Some(event) = events.recv().await {
            match event {
                CacheEvent::Data { data: item, .. } => spawn(glib::clone!(
                    #[weak(rename_to = obj)]
                    self,
                    async move {
                        {
                            let mut str = String::new();
                            if let Some(communityrating) = item.community_rating {
                                let formatted_rating = format!("{communityrating:.1}");
                                let crating = obj.imp().crating.get();
                                crating.set_text(&formatted_rating);
                                crating.set_visible(true);
                                obj.imp().star.get().set_visible(true);
                            }
                            if let Some(rating) = item.official_rating {
                                let orating = obj.imp().orating.get();
                                orating.set_text(&rating);
                                orating.set_visible(true);
                            }
                            if let Some(year) = item.production_year {
                                str.push_str(&year.to_string());
                                str.push_str("  ");
                            }
                            if let Some(runtime) = item.run_time_ticks {
                                let time_string = run_time_ticks_to_label(runtime);
                                str.push_str(&time_string);
                                str.push_str("  ");
                            }
                            if let Some(genres) = &item.genres {
                                for genre in genres {
                                    str.push_str(&genre.name);
                                    str.push(',');
                                }
                                str.pop();
                            }
                            obj.imp().line2.get().set_text(&str);

                            if let Some(taglines) = item.taglines
                                && let Some(tagline) = taglines.first()
                            {
                                obj.imp().tagline.set_text(tagline);
                                obj.imp().tagline.set_visible(true);
                            }
                        }
                        if let Some(links) = item.external_urls {
                            obj.set_flowlinks(links);
                        }
                        if let Some(actor) = item.people {
                            obj.setactorscrolled(actor).await;
                        }
                        if let Some(studios) = item.studios {
                            obj.set_flowbuttons(studios, "Studios");
                        }
                        if let Some(tags) = item.tags {
                            obj.set_flowbuttons(tags, "Tags");
                        }
                        if let Some(genres) = item.genres {
                            obj.set_flowbuttons(genres, "Genres");
                        }
                        if let Some(provider_ids) = item.provider_ids {
                            obj.imp().imdb_id.replace(provider_ids.imdb);
                        }
                        if let Some(image_tags) = item.backdrop_image_tags {
                            obj.add_backdrops(image_tags, &item.id).await;
                        }
                        if let Some(part_count) = item.part_count
                            && part_count > 1
                        {
                            obj.sets("Additional Parts", &item.id).await;
                        }
                        if let Some(ref user_data) = item.user_data {
                            let imp = obj.imp();
                            if let Some(is_favourite) = user_data.is_favorite {
                                imp.actionbox.set_btn_active(is_favourite);
                            }
                            imp.actionbox.set_played(user_data.played);
                            imp.actionbox.bind_edit();
                        }
                    }
                )),
                CacheEvent::Error(e) => {
                    self.toast(e.to_user_facing());
                    return;
                }
            }
        }
    }

    pub async fn createmediabox(
        &self, media_sources: Vec<MediaSource>, date_created: Option<DateTime<Utc>>,
    ) {
        let imp = self.imp();
        let mediainfobox = imp.mediainfobox.get();
        let mediainforevealer = imp.mediainforevealer.get();

        while let Some(child) = mediainfobox.last_child() {
            mediainfobox.remove(&child)
        }

        for mediasource in media_sources {
            let singlebox = gtk::Box::new(gtk::Orientation::Vertical, 5);
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
            singlebox.append(&label);

            let mediascrolled = gtk::ScrolledWindow::builder()
                .hscrollbar_policy(gtk::PolicyType::Automatic)
                .vscrollbar_policy(gtk::PolicyType::Never)
                .margin_start(15)
                .margin_end(15)
                .overlay_scrolling(true)
                .build();

            let mediascrolled = mediascrolled.fix();

            let mediabox = gtk::Box::builder()
                .orientation(gtk::Orientation::Horizontal)
                .halign(gtk::Align::Start)
                .spacing(5)
                .build();
            for mediapart in mediasource.media_streams {
                if mediapart.stream_type == "Attachment" {
                    continue;
                }
                let mediapartbox = gtk::Box::builder()
                    .orientation(gtk::Orientation::Vertical)
                    .spacing(0)
                    .width_request(300)
                    .build();
                let icon = gtk::Image::builder().margin_end(5).build();
                if mediapart.stream_type == "Video" {
                    icon.set_icon_name(Some("video-x-generic-symbolic"))
                } else if mediapart.stream_type == "Audio" {
                    icon.set_icon_name(Some("audio-x-generic-symbolic"))
                } else if mediapart.stream_type == "Subtitle" {
                    icon.set_icon_name(Some("media-view-subtitles-symbolic"))
                } else {
                    icon.set_icon_name(Some("text-x-generic-symbolic"))
                }
                let typebox = gtk::Box::builder()
                    .orientation(gtk::Orientation::Horizontal)
                    .spacing(5)
                    .build();
                typebox.append(&icon);
                let label = gtk::Label::builder()
                    .label(gettext(mediapart.stream_type))
                    .attributes(
                        &gtk::pango::AttrList::from_string("0 4294967295 weight bold")
                            .expect("Failed to create attribute list"),
                    )
                    .build();
                typebox.append(&label);
                let mut str: String = Default::default();
                if let Some(codec) = mediapart.codec {
                    str.push_str(format!("{}: {}", gettext("Codec"), codec).as_str());
                }
                if let Some(language) = mediapart.display_language {
                    str.push_str(format!("\n{}: {}", gettext("Language"), language).as_str());
                }
                if let Some(title) = mediapart.title {
                    str.push_str(format!("\n{}: {}", gettext("Title"), title).as_str());
                }
                if let Some(bitrate) = mediapart.bit_rate {
                    str.push_str(
                        format!("\n{}: {}it/s", gettext("Bitrate"), bytefmt::format(bitrate))
                            .as_str(),
                    );
                }
                if let Some(bitdepth) = mediapart.bit_depth {
                    str.push_str(format!("\n{}: {} bit", gettext("BitDepth"), bitdepth).as_str());
                }
                if let Some(samplerate) = mediapart.sample_rate {
                    str.push_str(
                        format!("\n{}: {} Hz", gettext("SampleRate"), samplerate).as_str(),
                    );
                }
                if let Some(height) = mediapart.height {
                    str.push_str(format!("\n{}: {}", gettext("Height"), height).as_str());
                }
                if let Some(width) = mediapart.width {
                    str.push_str(format!("\n{}: {}", gettext("Width"), width).as_str());
                }
                if let Some(colorspace) = mediapart.color_space {
                    str.push_str(format!("\n{}: {}", gettext("ColorSpace"), colorspace).as_str());
                }
                if let Some(displaytitle) = mediapart.display_title {
                    str.push_str(
                        format!("\n{}: {}", gettext("DisplayTitle"), displaytitle).as_str(),
                    );
                }
                if let Some(channel) = mediapart.channels {
                    str.push_str(format!("\n{}: {}", gettext("Channel"), channel).as_str());
                }
                if let Some(channellayout) = mediapart.channel_layout {
                    str.push_str(
                        format!("\n{}: {}", gettext("ChannelLayout"), channellayout).as_str(),
                    );
                }
                if let Some(averageframerate) = mediapart.average_frame_rate {
                    str.push_str(
                        format!("\n{}: {}", gettext("AverageFrameRate"), averageframerate).as_str(),
                    );
                }
                if let Some(pixelformat) = mediapart.pixel_format {
                    str.push_str(format!("\n{}: {}", gettext("PixelFormat"), pixelformat).as_str());
                }
                let inscription = gtk::Inscription::builder()
                    .text(&str)
                    .min_lines(14)
                    .hexpand(true)
                    .margin_start(15)
                    .margin_end(15)
                    .yalign(0.0)
                    .build();
                mediapartbox.append(&typebox);
                mediapartbox.append(&inscription);
                mediapartbox.add_css_class("card");
                mediapartbox.add_css_class("sbackground");
                mediabox.append(&mediapartbox);
            }

            mediascrolled.set_child(Some(&mediabox));
            singlebox.append(mediascrolled);
            mediainfobox.append(&singlebox);
        }
        mediainforevealer.set_reveal_child(true);
    }

    pub async fn setactorscrolled(&self, actors: Vec<SimpleListItem>) {
        let hortu = self.imp().actorhortu.get();
        hortu.set_items(actors);
    }

    pub async fn set_lists(&self, id: &str) {
        self.sets("Recommend", id).await;
        self.sets("Included In", id).await;
    }

    pub async fn sets(&self, types: &str, id: &str) {
        let hortu = match types {
            "Recommend" => self.imp().recommendhortu.get(),
            "Included In" => self.imp().includehortu.get(),
            "Additional Parts" => self.imp().additionalhortu.get(),
            _ => return,
        };

        let id = id.to_string();
        let types = types.to_string();

        let mut events = fetch_with_cache(
            &format!("item_{types}_{id}"),
            CachePolicy::ReadCacheAndRefresh,
            async move {
                match types.as_str() {
                    "Recommend" => JELLYFIN_CLIENT.get_similar(&id).await,
                    "Included In" => JELLYFIN_CLIENT.get_included(&id).await,
                    "Additional Parts" => JELLYFIN_CLIENT.get_additional(&id).await,
                    _ => Ok(List::default()),
                }
            },
        )
        .await;

        hortu.set_unify_size(UnifySize::Majority);

        while let Some(event) = events.recv().await {
            match event {
                CacheEvent::Data { data, .. } => {
                    hortu.set_items(data.items);
                }
                CacheEvent::Error(e) => {
                    self.toast(e.to_user_facing());
                }
            }
        }
    }

    pub fn set_flowbuttons(&self, infos: Vec<SGTitem>, type_: &str) {
        let imp = self.imp();
        let horbu = match type_ {
            "Genres" => imp.genreshorbu.get(),
            "Studios" => imp.studioshorbu.get(),
            "Tags" => imp.tagshorbu.get(),
            _ => return,
        };

        horbu.set_items(&infos, type_);
    }

    pub fn set_flowlinks(&self, links: Vec<Urls>) {
        self.imp().linkshorbu.set_links(&links);
    }

    pub fn window(&self) -> Window {
        self.root().unwrap().downcast::<Window>().unwrap()
    }

    #[template_callback]
    fn edge_overshot_cb(&self, pos: PositionType, _window: &ScrolledWindow) {
        if pos != gtk::PositionType::Top {
            return;
        }

        let carousel = self.imp().main_carousel.get();
        carousel.scroll_to(&carousel.nth_page(0), true);
    }

    #[template_callback]
    async fn play_cb(&self) {
        let video_dropdown = self.imp().namedropdown.get();
        let sub_dropdown = self.imp().subdropdown.get();

        let Some(video_object) = video_dropdown
            .selected_item()
            .and_downcast::<glib::BoxedAnyObject>()
        else {
            self.toast(gettext("No video source found"));
            return;
        };

        let sub_dl = sub_dropdown
            .selected_item()
            .and_downcast::<glib::BoxedAnyObject>()
            .map(|obj| obj.borrow::<DropdownList>().to_owned());

        let video_dl: std::cell::Ref<DropdownList> = video_object.borrow();
        let (sub_index, sub_lang) = sub_dl
            .map(|sub_dl| {
                (
                    sub_dl.index.unwrap_or_default(),
                    sub_dl.sub_lang.to_owned().unwrap_or_default(),
                )
            })
            .unwrap_or_default();

        let info = SelectedVideoSubInfo {
            sub_index,
            video_index: video_dl.index.unwrap_or_default(),
            sub_lang,
            media_source_id: video_dl.id.to_owned().unwrap_or_default(),
        };

        let item = self.current_item().unwrap_or(self.item());
        let start_seconds = item.playback_position_ticks() as f64 / 10_000_000.0;

        let episode_list = self.imp().episode_list_vec.borrow();
        let episode_list: Vec<TuItem> = episode_list
            .iter()
            .map(|item| TuItem::from_simple(item.to_owned()))
            .collect();

        let matcher = self.imp().video_version_matcher.borrow().to_owned();
        let external_sub = self.imp().pending_external_sub.borrow_mut().take();

        self.window().play_media(
            Some(info),
            item,
            episode_list,
            matcher,
            start_seconds,
            external_sub,
        );
    }

    #[template_callback]
    fn on_rightbutton_clicked(&self) {
        self.scroll_controls_anime::<true>();
    }

    #[template_callback]
    fn on_enter_focus(&self) {
        self.on_enter_scroll_controls();
    }

    #[template_callback]
    fn on_leave_focus(&self) {
        self.on_leave_scroll_controls();
    }

    #[template_callback]
    fn on_leftbutton_clicked(&self) {
        self.scroll_controls_anime::<false>();
    }

    #[template_callback]
    async fn on_season_view_more_clicked(&self) {
        let object = self.imp().seasonlist.selected_item();
        let Some(season_name) = object.and_downcast_ref::<gtk::StringObject>() else {
            return;
        };

        let season_name = season_name.string().to_string();

        let season_list = self.imp().season_list_vec.borrow();
        let Some(season) = season_list.iter().find(|s| s.name == season_name) else {
            self.toast(gettext(
                "Season not found. Is this a continue watching list?",
            ));
            return;
        };

        let item = TuItem::from_simple(season.to_owned());
        item.activate(self);
    }

    pub fn focus_hortu_rows(&self) -> Vec<super::hortu_scrolled::HortuScrolled> {
        let imp = self.imp();
        vec![
            imp.includehortu.get(),
            imp.additionalhortu.get(),
            imp.seasonshortu.get(),
            imp.actorhortu.get(),
            imp.recommendhortu.get(),
        ]
    }

    pub fn focus_horbu_rows(&self) -> Vec<super::horbu_scrolled::HorbuScrolled> {
        let imp = self.imp();
        vec![
            imp.linkshorbu.get(),
            imp.studioshorbu.get(),
            imp.genreshorbu.get(),
            imp.tagshorbu.get(),
        ]
    }

    pub fn scroll_to_hero_page(&self) {
        let carousel = self.imp().main_carousel.get();
        if carousel.n_pages() > 0 {
            carousel.scroll_to(&carousel.nth_page(0), true);
        }
    }

    pub fn scroll_to_details_page(&self) {
        let carousel = self.imp().main_carousel.get();
        if carousel.n_pages() > 1 {
            carousel.scroll_to(&carousel.nth_page(1), true);
        }
    }

    pub fn focus_details_row(&self, focus: impl FnOnce() + 'static) {
        self.scroll_to_details_page();
        glib::idle_add_local_once(focus);
    }

    pub fn has_dropdowns(&self) -> bool {
        let imp = self.imp();
        imp.namedropdown.get().is_visible() || imp.subdropdown.get().is_visible()
    }

    pub fn has_episode_toolbar(&self) -> bool {
        self.imp().toolbar.get().is_visible()
    }

    pub fn episode_toolbar_widgets(&self) -> Vec<gtk::Widget> {
        let imp = self.imp();
        if !imp.toolbar.get().is_visible() {
            return Vec::new();
        }
        vec![
            imp.seasonlist.get().upcast(),
            imp.season_view_more.get().upcast(),
        ]
    }

    pub fn clear_episode_toolbar_focus(&self) {
        for widget in self.episode_toolbar_widgets() {
            crate::tv::set_tv_focused(&widget, false);
        }
    }

    pub fn focus_episode_toolbar(&self, index: usize) {
        self.clear_episode_toolbar_focus();
        self.clear_episode_focus();
        if let Some(widget) = self.episode_toolbar_widgets().get(index) {
            crate::tv::set_tv_focused(widget, true);
        }
        self.imp().episode_subzone.set(0);
    }

    pub fn focus_episode_list(&self) {
        self.clear_episode_toolbar_focus();
        self.focus_default_episode();
        self.imp().episode_subzone.set(1);
    }

    pub fn is_episode_toolbar_focused(&self) -> bool {
        self.imp().episode_subzone.get() == 0
    }

    pub fn navigate_episode_toolbar(&self, delta: i32) {
        let widgets = self.episode_toolbar_widgets();
        if widgets.is_empty() {
            return;
        }
        let current = widgets
            .iter()
            .position(|widget| widget.has_css_class("tv-focused"))
            .unwrap_or(0) as i32;
        let next = (current + delta).clamp(0, widgets.len() as i32 - 1) as usize;
        self.focus_episode_toolbar(next);
    }

    pub fn activate_episode_toolbar(&self) {
        let widgets = self.episode_toolbar_widgets();
        let index = widgets
            .iter()
            .position(|widget| widget.has_css_class("tv-focused"))
            .unwrap_or(0);
        let imp = self.imp();
        if index == 0 {
            let dropdown = imp.seasonlist.get();
            dropdown.grab_focus();
            gtk::prelude::WidgetExt::activate(&dropdown);
        } else if index == 1 {
            imp.season_view_more.get().emit_clicked();
        }
    }

    pub fn has_episode_list(&self) -> bool {
        let imp = self.imp();
        imp.episode_list_bin.get().is_visible()
            && imp.episode_stack.visible_child_name().as_deref() == Some("view")
            && imp.selection.n_items() > 0
    }

    pub fn focus_default_action(&self) {
        self.set_action_focus(0);
    }

    pub fn clear_action_focus(&self) {
        for widget in self.action_focus_widgets() {
            crate::tv::set_tv_focused(&widget, false);
        }
    }

    pub fn action_focus_widgets(&self) -> Vec<gtk::Widget> {
        let imp = self.imp();
        vec![
            imp.playbutton.get().upcast(),
            imp.actionbox.favourite_button().upcast(),
            imp.actionbox.menu_button().upcast(),
        ]
    }

    pub fn top_bar_action_widgets(&self) -> Vec<gtk::Widget> {
        let mut widgets = self.action_focus_widgets();
        if let Some(btn) = self.ensure_tv_subtitle_button() {
            widgets.push(btn.upcast());
        }
        widgets
    }

    pub fn media_focus_widgets(&self) -> Vec<gtk::Widget> {
        let imp = self.imp();
        let mut widgets = Vec::new();
        if imp.namedropdown.get().is_visible() {
            widgets.push(imp.namedropdown.get().upcast());
        }
        if imp.subdropdown.get().is_visible() {
            widgets.push(imp.subdropdown.get().upcast());
        }
        widgets
    }

    pub fn clear_media_focus(&self) {
        for widget in self.media_focus_widgets() {
            crate::tv::set_tv_focused(&widget, false);
        }
    }

    pub fn set_media_focus(&self, index: usize) {
        self.clear_media_focus();
        if let Some(widget) = self.media_focus_widgets().get(index) {
            crate::tv::set_tv_focused(widget, true);
        }
    }

    pub fn top_bar_spatial_widgets(&self) -> Vec<gtk::Widget> {
        self.top_bar_focus_widgets()
    }

    pub fn focused_top_bar_widget(&self) -> Option<gtk::Widget> {
        self.top_bar_spatial_widgets()
            .into_iter()
            .find(|widget| widget.has_css_class("tv-focused"))
    }

    pub fn set_top_bar_widget_focus(&self, widget: &gtk::Widget) {
        self.clear_top_bar_focus();
        crate::tv::set_tv_focused(widget, true);
    }

    pub fn navigate_top_bar_spatial(&self, dx: i32, dy: i32) -> bool {
        let actions = self.top_bar_action_widgets();
        let media = self.media_focus_widgets();
        if actions.is_empty() && media.is_empty() {
            return false;
        }

        let current = self.focused_top_bar_widget().unwrap_or_else(|| {
            actions
                .first()
                .or_else(|| media.first())
                .cloned()
                .expect("top bar widgets exist")
        });

        let in_actions = actions.iter().any(|widget| widget == &current);
        let in_media = media.iter().any(|widget| widget == &current);

        if dy != 0 {
            if in_media {
                let imp = self.imp();
                let video = imp.namedropdown.get().upcast::<gtk::Widget>();
                let sub = imp.subdropdown.get().upcast::<gtk::Widget>();
                if current == video && dy > 0 && sub.is_visible() {
                    self.set_top_bar_widget_focus(&sub);
                    return true;
                }
                if current == sub && dy < 0 && video.is_visible() {
                    self.set_top_bar_widget_focus(&video);
                    return true;
                }
            }
            return false;
        }

        if dx == 0 {
            return false;
        }

        if in_media {
            let pos = media.iter().position(|widget| widget == &current);
            if let Some(index) = pos {
                let next = (index as i32 + dx).clamp(0, media.len() as i32 - 1) as usize;
                if next != index {
                    self.set_top_bar_widget_focus(&media[next]);
                    return true;
                }
            }
            if dx < 0
                && let Some(widget) = actions.last()
            {
                self.set_top_bar_widget_focus(widget);
                return true;
            }
            return false;
        }

        if in_actions {
            let pos = actions.iter().position(|widget| widget == &current);
            if let Some(index) = pos {
                let next = (index as i32 + dx).clamp(0, actions.len() as i32 - 1) as usize;
                if next != index {
                    self.set_top_bar_widget_focus(&actions[next]);
                    return true;
                }
            }
            if dx > 0
                && let Some(widget) = media.first()
            {
                self.set_top_bar_widget_focus(widget);
                return true;
            }
        }

        false
    }

    pub fn activate_focused_top_bar(&self) {
        let Some(widget) = self.focused_top_bar_widget() else {
            return;
        };
        let imp = self.imp();
        let play = imp.playbutton.get().upcast::<gtk::Widget>();
        let fav = imp.actionbox.favourite_button().upcast::<gtk::Widget>();
        let menu = imp.actionbox.menu_button().upcast::<gtk::Widget>();
        let video = imp.namedropdown.get().upcast::<gtk::Widget>();
        let sub = imp.subdropdown.get().upcast::<gtk::Widget>();
        if widget == play {
            imp.playbutton.emit_clicked();
        } else if widget == fav {
            imp.actionbox.favourite_button().emit_clicked();
        } else if widget == menu {
            let btn = imp.actionbox.menu_button();
            btn.grab_focus();
            btn.popup();
        } else if imp
            .tv_subtitle_btn
            .borrow()
            .as_ref()
            .is_some_and(|btn| widget == btn.clone().upcast::<gtk::Widget>())
        {
            self.open_subtitle_search();
        } else if widget == video || widget == sub {
            widget.grab_focus();
            gtk::prelude::WidgetExt::activate(&widget);
        }
    }

    pub fn focus_default_top_bar_spatial(&self) {
        if let Some(widget) = self.top_bar_spatial_widgets().first() {
            self.set_top_bar_widget_focus(widget);
        }
    }

    pub fn top_bar_focus_widgets(&self) -> Vec<gtk::Widget> {
        let mut widgets = self.top_bar_action_widgets();
        widgets.extend(self.media_focus_widgets());
        widgets
    }

    pub fn subtitle_search_available(&self) -> bool {
        if !crate::tv::is_tv_mode_active() {
            return false;
        }
        if !(SETTINGS.subtitle_provider_enabled("opensubtitles")
            || SETTINGS.subtitle_provider_enabled("subdl"))
        {
            return false;
        }
        let item = self.item();
        matches!(
            item.item_type().as_str(),
            "Movie" | "Episode" | "Video" | "Series"
        )
    }

    pub fn ensure_tv_subtitle_button(&self) -> Option<gtk::Button> {
        if !self.subtitle_search_available() {
            return None;
        }
        let imp = self.imp();
        if let Some(btn) = imp.tv_subtitle_btn.borrow().clone() {
            btn.set_visible(true);
            return Some(btn);
        }
        let btn = gtk::Button::with_label(&gettext("Find Subtitles"));
        btn.add_css_class("pill");
        btn.set_valign(gtk::Align::Center);
        let obj = self.clone();
        btn.connect_clicked(move |_| obj.open_subtitle_search());
        if let Some(parent) = imp
            .playbutton
            .get()
            .parent()
            .and_then(|widget| widget.downcast::<gtk::Box>().ok())
        {
            parent.insert_child_after(&btn, Some(&imp.actionbox.get()));
        }
        *imp.tv_subtitle_btn.borrow_mut() = Some(btn.clone());
        Some(btn)
    }

    pub fn clear_top_bar_focus(&self) {
        for widget in self.top_bar_focus_widgets() {
            crate::tv::set_tv_focused(&widget, false);
        }
    }

    pub fn set_top_bar_focus(&self, index: usize) {
        self.clear_top_bar_focus();
        if let Some(widget) = self.top_bar_focus_widgets().get(index) {
            crate::tv::set_tv_focused(widget, true);
        }
    }

    pub fn focus_default_top_bar(&self) {
        self.set_top_bar_focus(0);
    }

    pub fn navigate_top_bar(&self, delta: i32) {
        let count = self.top_bar_focus_widgets().len() as i32;
        if count == 0 {
            return;
        }
        let current = self
            .top_bar_focus_widgets()
            .iter()
            .position(|widget| widget.has_css_class("tv-focused"))
            .unwrap_or(0) as i32;
        let next = (current + delta).clamp(0, count - 1) as usize;
        self.set_top_bar_focus(next);
    }

    pub fn activate_top_bar_action_at(&self, index: usize) {
        let imp = self.imp();
        let widgets = self.top_bar_action_widgets();
        let Some(widget) = widgets.get(index) else {
            return;
        };
        if widget == &imp.playbutton.get().upcast::<gtk::Widget>() {
            imp.playbutton.emit_clicked();
        } else if widget == &imp.actionbox.favourite_button().upcast::<gtk::Widget>() {
            imp.actionbox.favourite_button().emit_clicked();
        } else if widget == &imp.actionbox.menu_button().upcast::<gtk::Widget>() {
            imp.actionbox.menu_button().popup();
        } else if imp
            .tv_subtitle_btn
            .borrow()
            .as_ref()
            .is_some_and(|btn| widget == &btn.clone().upcast::<gtk::Widget>())
        {
            self.open_subtitle_search();
        }
    }

    pub fn activate_media_at(&self, index: usize) {
        let widgets = self.media_focus_widgets();
        let Some(widget) = widgets.get(index) else {
            return;
        };
        widget.grab_focus();
        gtk::prelude::WidgetExt::activate(widget);
    }

    pub fn activate_top_bar_at(&self, index: usize) {
        let imp = self.imp();
        let widgets = self.top_bar_focus_widgets();
        let Some(widget) = widgets.get(index) else {
            return;
        };
        if widget == &imp.playbutton.get().upcast::<gtk::Widget>() {
            imp.playbutton.emit_clicked();
        } else if widget == &imp.actionbox.favourite_button().upcast::<gtk::Widget>() {
            imp.actionbox.favourite_button().emit_clicked();
        } else if widget == &imp.actionbox.menu_button().upcast::<gtk::Widget>() {
            imp.actionbox.menu_button().popup();
        } else if imp
            .tv_subtitle_btn
            .borrow()
            .as_ref()
            .is_some_and(|btn| widget == &btn.clone().upcast::<gtk::Widget>())
        {
            self.open_subtitle_search();
        } else if widget == &imp.namedropdown.get().upcast::<gtk::Widget>()
            || widget == &imp.subdropdown.get().upcast::<gtk::Widget>()
        {
            widget.grab_focus();
            gtk::prelude::WidgetExt::activate(widget);
        }
    }

    pub fn set_action_focus(&self, index: usize) {
        self.clear_action_focus();
        if let Some(widget) = self.action_focus_widgets().get(index) {
            crate::tv::set_tv_focused(widget, true);
        }
    }

    pub fn navigate_actions(&self, delta: i32) {
        let count = self.action_focus_widgets().len() as i32;
        if count == 0 {
            return;
        }
        let current = self
            .action_focus_widgets()
            .iter()
            .position(|widget| widget.has_css_class("tv-focused"))
            .unwrap_or(0) as i32;
        let next = (current + delta).clamp(0, count - 1) as usize;
        self.set_action_focus(next);
    }

    pub fn activate_action_at(&self, index: usize) {
        let imp = self.imp();
        match index {
            0 => imp.playbutton.emit_clicked(),
            1 => imp.actionbox.favourite_button().emit_clicked(),
            2 => imp.actionbox.menu_button().popup(),
            _ => {}
        }
    }

    pub fn activate_focused_action(&self) {
        let index = self
            .action_focus_widgets()
            .iter()
            .position(|widget| widget.has_css_class("tv-focused"))
            .unwrap_or(0);
        self.activate_action_at(index);
    }

    pub fn open_subtitle_search(&self) {
        use crate::subtitles::{
            SubtitleSearchDialog,
            preferred_subtitle_language_code,
        };

        let title = self
            .current_item()
            .map(|item| item.name())
            .or_else(|| self.name())
            .unwrap_or_default();
        let imdb_id = self.imp().imdb_id.borrow().clone();
        let dialog = SubtitleSearchDialog::new(
            &title,
            &preferred_subtitle_language_code(),
            imdb_id.as_deref(),
        );
        dialog.connect_subtitle_downloaded(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            move |path| {
                obj.imp()
                    .pending_external_sub
                    .replace(Some(path.display().to_string()));
                obj.toast(gettext("Subtitle will load on play"));
            }
        ));
        dialog.present(self.root().as_ref());
    }

    pub fn focus_default_dropdown(&self) {
        let imp = self.imp();
        if imp.namedropdown.get().is_visible() {
            crate::tv::set_tv_focused(&imp.namedropdown.get(), true);
        } else if imp.subdropdown.get().is_visible() {
            crate::tv::set_tv_focused(&imp.subdropdown.get(), true);
        }
    }

    pub fn clear_dropdown_focus(&self) {
        let imp = self.imp();
        crate::tv::set_tv_focused(&imp.namedropdown.get(), false);
        crate::tv::set_tv_focused(&imp.subdropdown.get(), false);
    }

    pub fn navigate_dropdowns(&self, delta: i32) {
        let imp = self.imp();
        if delta > 0 && imp.namedropdown.get().is_visible() && imp.subdropdown.get().is_visible() {
            self.clear_dropdown_focus();
            crate::tv::set_tv_focused(&imp.subdropdown.get(), true);
        } else if delta < 0 && imp.subdropdown.get().is_visible() {
            self.clear_dropdown_focus();
            crate::tv::set_tv_focused(&imp.namedropdown.get(), true);
        } else {
            self.focus_default_dropdown();
        }
    }

    pub fn activate_focused_dropdown(&self) {
        let imp = self.imp();
        let dropdown = if imp.subdropdown.get().has_css_class("tv-focused") {
            imp.subdropdown.get()
        } else {
            imp.namedropdown.get()
        };
        dropdown.grab_focus();
        gtk::prelude::WidgetExt::activate(&dropdown);
    }

    pub fn focus_default_episode(&self) {
        let imp = self.imp();
        if imp.selection.n_items() > 0 {
            imp.selection.set_selected(0);
            imp.itemlist.get().scroll_to(0, ListScrollFlags::NONE, None);
        }
        self.scroll_episode_list_into_view();
    }

    pub fn scroll_episode_list_into_view(&self) {
        let imp = self.imp();
        if !imp.episode_list_bin.get().is_visible() {
            return;
        }
        self.scroll_to_hero_page();
        super::fix::scroll_widget_to_row_center(&imp.episode_list_bin.get());
    }

    pub fn mediainfo_card_widgets(&self) -> Vec<gtk::Widget> {
        let imp = self.imp();
        if !imp.mediainforevealer.reveals_child() {
            return Vec::new();
        }
        let mut cards = Vec::new();
        let mut source = imp.mediainfobox.first_child();
        while let Some(singlebox) = source {
            let mut child = singlebox.first_child();
            while let Some(widget) = child {
                if widget.is::<gtk::ScrolledWindow>()
                    && let Some(row) = widget.first_child()
                {
                    let mut card = row.first_child();
                    while let Some(card_widget) = card {
                        if card_widget.has_css_class("card") {
                            cards.push(card_widget.clone());
                        }
                        card = card_widget.next_sibling();
                    }
                }
                child = widget.next_sibling();
            }
            source = singlebox.next_sibling();
        }
        cards
    }

    pub fn mediainfo_card_count(&self) -> usize {
        self.mediainfo_card_widgets().len()
    }

    pub fn clear_mediainfo_focus(&self) {
        for widget in self.mediainfo_card_widgets() {
            crate::tv::set_tv_focused(&widget, false);
        }
        self.imp().mediainfo_selected.set(None);
    }

    pub fn ensure_mediainfo_selection(&self) {
        if self.mediainfo_card_count() == 0 {
            return;
        }
        if self.imp().mediainfo_selected.get().is_none() {
            self.set_mediainfo_selection(0);
        }
    }

    fn set_mediainfo_selection(&self, index: usize) {
        let cards = self.mediainfo_card_widgets();
        if cards.is_empty() {
            return;
        }
        let index = index.min(cards.len() - 1);
        let prev = self.imp().mediainfo_selected.get();
        if prev == Some(index) {
            return;
        }
        if let Some(prev_index) = prev
            && let Some(widget) = cards.get(prev_index)
        {
            crate::tv::set_tv_focused(widget, false);
        }
        if let Some(widget) = cards.get(index) {
            crate::tv::set_tv_focused(widget, true);
            super::fix::scroll_widget_to_column_center(widget);
        }
        self.imp().mediainfo_selected.set(Some(index));
    }

    pub fn move_mediainfo_selection(&self, delta: i32) {
        let count = self.mediainfo_card_count();
        if count == 0 {
            return;
        }
        let current = self.imp().mediainfo_selected.get().unwrap_or(0);
        let next = (current as i32 + delta).clamp(0, count as i32 - 1) as usize;
        self.set_mediainfo_selection(next);
    }

    pub fn scroll_mediainfo_into_view(&self) {
        let obj = self.clone();
        self.focus_details_row(move || {
            super::fix::scroll_widget_to_row_center(&obj.imp().mediainforevealer.get());
            obj.ensure_mediainfo_selection();
        });
    }

    pub fn clear_episode_focus(&self) {
        self.imp()
            .selection
            .set_selected(gtk::INVALID_LIST_POSITION);
    }

    pub fn navigate_episodes(&self, delta: i32) {
        let imp = self.imp();
        let count = imp.selection.n_items() as i32;
        if count == 0 {
            return;
        }
        let current = if imp.selection.selected() == gtk::INVALID_LIST_POSITION {
            0
        } else {
            imp.selection.selected() as i32
        };
        let next = (current + delta).clamp(0, count - 1) as u32;
        imp.selection.set_selected(next);
        imp.itemlist
            .get()
            .scroll_to(next, ListScrollFlags::NONE, None);
    }

    pub fn activate_focused_episode(&self) {
        let imp = self.imp();
        let index = imp.selection.selected();
        if index == gtk::INVALID_LIST_POSITION {
            return;
        }
        if let Some(item) = imp.selection.item(index).and_downcast::<TuObject>() {
            item.item().activate(self);
        }
    }
}

impl HorControlsExt for ItemPage {
    fn scroll_widget(&self) -> gtk::ScrolledWindow {
        self.imp().scrolled.get()
    }

    fn left_button(&self) -> gtk::Button {
        self.imp().left_button.get()
    }

    fn right_button(&self) -> gtk::Button {
        self.imp().right_button.get()
    }

    fn show_left_animation_cell(&self) -> &std::cell::OnceCell<adw::TimedAnimation> {
        &self.imp().show_left_animation
    }

    fn hide_left_animation_cell(&self) -> &std::cell::OnceCell<adw::TimedAnimation> {
        &self.imp().hide_left_animation
    }

    fn show_right_animation_cell(&self) -> &std::cell::OnceCell<adw::TimedAnimation> {
        &self.imp().show_right_animation
    }

    fn hide_right_animation_cell(&self) -> &std::cell::OnceCell<adw::TimedAnimation> {
        &self.imp().hide_right_animation
    }

    fn is_hovering(&self) -> &std::cell::Cell<bool> {
        &self.imp().is_hovering
    }
}

pub fn dt(date: Option<chrono::DateTime<Utc>>) -> String {
    let Some(date) = date else {
        return "".to_string();
    };
    date.naive_local().format("%Y-%m-%d %H:%M:%S").to_string()
}

#[derive(Debug, Clone)]
pub struct SelectedVideoSubInfo {
    pub sub_lang: String,
    pub sub_index: i64,
    pub video_index: i64,
    pub media_source_id: String,
}
