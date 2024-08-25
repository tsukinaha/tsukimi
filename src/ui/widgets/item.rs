use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::Object;
use gtk::pango::AttrList;
use gtk::template_callbacks;
use gtk::{gio, glib};
use std::cell::Ref;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::client::client::EMBY_CLIENT;
use crate::client::error::UserFacingError;
use crate::client::structs::*;
use crate::toast;
use crate::ui::models::SETTINGS;

use crate::ui::provider::dropdown_factory::factory;
use crate::utils::{get_image_with_cache, req_cache, spawn, spawn_tokio};
use chrono::{DateTime, Utc};

use super::fix::ScrolledWindowFixExt;
use super::picture_loader::PictureLoader;
use super::song_widget::format_duration;
use super::window::Window;

pub(crate) mod imp {
    use crate::ui::widgets::fix::ScrolledWindowFixExt;
    use crate::ui::widgets::horbu_scrolled::HorbuScrolled;
    use crate::ui::widgets::hortu_scrolled::HortuScrolled;
    use crate::ui::widgets::item_actionbox::ItemActionsBox;
    use crate::ui::widgets::star_toggle::StarToggle;
    use crate::utils::spawn_g_timeout;
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::prelude::*;
    use gtk::{glib, CompositeTemplate};
    use std::cell::{OnceCell, RefCell};

    // Object holding the state
    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsukimi/item.ui")]
    #[properties(wrapper_type = super::ItemPage)]
    pub struct ItemPage {
        #[property(get, set, construct_only)]
        pub id: OnceCell<String>,
        #[property(get, set, construct_only)]
        pub inid: RefCell<String>,

        #[template_child]
        pub actorhortu: TemplateChild<HortuScrolled>,
        #[template_child]
        pub recommendhortu: TemplateChild<HortuScrolled>,
        #[template_child]
        pub includehortu: TemplateChild<HortuScrolled>,
        #[template_child]
        pub additionalhortu: TemplateChild<HortuScrolled>,

        #[template_child]
        pub studioshorbu: TemplateChild<HorbuScrolled>,
        #[template_child]
        pub tagshorbu: TemplateChild<HorbuScrolled>,
        #[template_child]
        pub genreshorbu: TemplateChild<HorbuScrolled>,
        #[template_child]
        pub linkshorbu: TemplateChild<HorbuScrolled>,

        #[template_child]
        pub backdrop: TemplateChild<gtk::Picture>,
        #[template_child]
        pub itemlist: TemplateChild<gtk::ListView>,
        #[template_child]
        pub osdbox: TemplateChild<gtk::Box>,
        #[template_child]
        pub itemrevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub logobox: TemplateChild<gtk::Box>,
        #[template_child]
        pub seasonlist: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub overviewrevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub itemoverview: TemplateChild<gtk::TextView>,
        #[template_child]
        pub selecteditemoverview: TemplateChild<gtk::TextView>,
        #[template_child]
        pub mediainfobox: TemplateChild<gtk::Box>,
        #[template_child]
        pub mediainforevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub episodescrolled: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub episodesearchentry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub line1: TemplateChild<gtk::Label>,
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
        pub line1spinner: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub namedropdown: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub subdropdown: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub backrevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub carousel: TemplateChild<adw::Carousel>,
        #[template_child]
        pub indicator: TemplateChild<adw::CarouselIndicatorLines>,
        #[template_child]
        pub actionbox: TemplateChild<ItemActionsBox>,
        #[template_child]
        pub tagline: TemplateChild<gtk::Label>,
        #[template_child]
        pub toolbar: TemplateChild<gtk::Box>,

        #[template_child]
        pub buttoncontent: TemplateChild<adw::ButtonContent>,

        pub selection: gtk::SingleSelection,
        pub seasonselection: gtk::SingleSelection,
        pub playbuttonhandlerid: RefCell<Option<glib::SignalHandlerId>>,

        #[property(get, set, construct_only)]
        pub name: RefCell<Option<String>>,
        pub selected: RefCell<Option<String>>,

        pub videoselection: gtk::SingleSelection,
        pub subselection: gtk::SingleSelection,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for ItemPage {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "ItemPage";
        type Type = super::ItemPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            StarToggle::ensure_type();
            HortuScrolled::ensure_type();
            HorbuScrolled::ensure_type();
            klass.bind_template();
            klass.bind_template_instance_callbacks();
            klass.install_action("item.first", None, move |window, _action, _parameter| {
                window.itemfirst();
            });
            klass.install_action("item.previous", None, move |window, _action, _parameter| {
                window.itemprevious();
            });
            klass.install_action("item.next", None, move |window, _action, _parameter| {
                window.itemnext();
            });
            klass.install_action("item.last", None, move |window, _action, _parameter| {
                window.itemlast();
            });
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // Trait shared by all GObjects
    #[glib::derived_properties]
    impl ObjectImpl for ItemPage {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            let backdrop = self.backdrop.get();
            backdrop.set_height_request(crate::ui::models::SETTINGS.background_height());
            self.actionbox.set_id(Some(obj.id()));
            spawn_g_timeout(glib::clone!(
                #[weak]
                obj,
                async move {
                    obj.imp().episodescrolled.fix();
                    obj.setup_background().await;
                    obj.logoset();
                    obj.setoverview().await;
                    obj.set_lists().await;
                }
            ));
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for ItemPage {}

    // Trait shared by all windows
    impl WindowImpl for ItemPage {}

    // Trait shared by all application windows
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
    pub fn new(id: String, inid: String, name: String) -> Self {
        Object::builder()
            .property("id", id)
            .property("inid", inid)
            .property("name", name)
            .build()
    }

    pub async fn setup_background(&self) {
        let id = self.id();
        let imp = self.imp();

        let backdrop = imp.backdrop.get();
        let path = get_image_with_cache(&id, "Backdrop", Some(0))
            .await
            .unwrap();
        let file = gtk::gio::File::for_path(&path);
        let pathbuf = PathBuf::from(&path);
        if pathbuf.exists() {
            backdrop.set_file(Some(&file));
            self.imp().backrevealer.set_reveal_child(true);
            spawn(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                async move {
                    let window = obj.root().and_downcast::<super::window::Window>().unwrap();
                    window.set_rootpic(file);
                }
            ));
        }
    }

    pub async fn add_backdrops(&self, image_tags: Vec<String>) {
        let imp = self.imp();
        let id = self.id();
        let tags = image_tags.len();
        let carousel = imp.carousel.get();
        let indicator = imp.indicator.get();
        indicator.set_carousel(Some(&carousel));
        for tag_num in 1..tags {
            let path = get_image_with_cache(&id, "Backdrop", Some(tag_num as u8))
                .await
                .unwrap();
            let file = gtk::gio::File::for_path(&path);
            let picture = gtk::Picture::builder()
                .halign(gtk::Align::Fill)
                .valign(gtk::Align::Fill)
                .content_fit(gtk::ContentFit::Cover)
                .height_request(SETTINGS.background_height())
                .file(&file)
                .build();
            carousel.append(&picture);
            carousel.set_allow_scroll_wheel(true);
        }

        if carousel.n_pages() == 1 {
            return;
        }

        glib::timeout_add_seconds_local(10, move || {
            let current_page = carousel.position();
            let n_pages = carousel.n_pages();
            let new_page_position = (current_page + 1. + n_pages as f64) % n_pages as f64;
            carousel.scroll_to(&carousel.nth_page(new_page_position as u32), true);

            glib::ControlFlow::Continue
        });
    }

    pub async fn setup_seasons(&self) {
        let imp = self.imp();
        let itemrevealer = imp.itemrevealer.get();
        let id = self.id();
        let idc = id.clone();
        let inid = self.inid();

        let store = gtk::gio::ListStore::new::<glib::BoxedAnyObject>();
        imp.selection.set_autoselect(false);
        imp.selection.set_model(Some(&store));

        let seasonstore = gtk::StringList::new(&[]);
        imp.seasonselection.set_model(Some(&seasonstore));
        let seasonlist = imp.seasonlist.get();
        seasonlist.set_model(Some(&imp.seasonselection));

        let series_info =
            match spawn_tokio(async move { EMBY_CLIENT.get_series_info(&id).await }).await {
                Ok(item) => item.items,
                Err(e) => {
                    toast!(self, e.to_user_facing());
                    Vec::new()
                }
            };

        spawn(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                let mut season_set: HashSet<u32> = HashSet::new();
                let mut season_map: HashMap<String, u32> = HashMap::new();
                let min_season = series_info
                    .iter()
                    .map(|info| {
                        if info.parent_index_number.unwrap_or(0) == 0 {
                            100
                        } else {
                            info.parent_index_number.unwrap_or(0)
                        }
                    })
                    .min()
                    .unwrap_or(1);
                let mut pos = 0;
                let mut set = true;
                for info in &series_info {
                    if !season_set.contains(&info.parent_index_number.unwrap_or(0)) {
                        let seasonstring =
                            format!("Season {}", info.parent_index_number.unwrap_or(0));
                        seasonstore.append(&seasonstring);
                        season_set.insert(info.parent_index_number.unwrap_or(0));
                        season_map
                            .insert(seasonstring.clone(), info.parent_index_number.unwrap_or(0));
                        if set {
                            if info.parent_index_number.unwrap_or(0) == min_season {
                                set = false;
                            } else {
                                pos += 1;
                            }
                        }
                    }
                    if info.parent_index_number.unwrap_or(0) == min_season {
                        let object = glib::BoxedAnyObject::new(info.clone());
                        store.append(&object);
                    }
                    if inid != idc && info.id == inid {
                        let seriesinfo = SeriesInfo {
                            id: inid.clone(),
                            name: info.name.clone(),
                            index_number: info.index_number,
                            parent_index_number: info.parent_index_number,
                            user_data: info.user_data.clone(),
                            overview: info.overview.clone(),
                        };
                        spawn(glib::clone!(
                            #[weak]
                            obj,
                            async move {
                                obj.selectepisode(seriesinfo.clone()).await;
                            }
                        ));
                    }
                }
                obj.imp().seasonlist.set_selected(pos);
                let seasonlist = obj.imp().seasonlist.get();
                let itemlist = obj.imp().itemlist.get();
                if idc == inid {
                    itemlist.first_child().unwrap().activate();
                }
                let seriesinfo_seasonlist = series_info.clone();
                let seriesinfo_seasonmap = season_map.clone();
                seasonlist.connect_selected_item_notify(glib::clone!(
                    #[weak]
                    store,
                    move |dropdown| {
                        let selected = dropdown.selected_item();
                        let selected = selected.and_downcast_ref::<gtk::StringObject>().unwrap();
                        let selected = selected.string().to_string();
                        store.remove_all();
                        let season_number = seriesinfo_seasonmap[&selected];
                        for info in &seriesinfo_seasonlist {
                            if info.parent_index_number.unwrap_or(0) == season_number {
                                let object = glib::BoxedAnyObject::new(info.clone());
                                store.append(&object);
                            }
                        }
                        itemlist.first_child().unwrap().activate();
                    }
                ));
                let episodesearchentry = obj.imp().episodesearchentry.get();
                episodesearchentry.connect_search_changed(glib::clone!(
                    #[weak]
                    store,
                    move |entry| {
                        let text = entry.text();
                        store.remove_all();
                        for info in &series_info {
                            if (info.name.to_lowercase().contains(&text.to_lowercase())
                                || info
                                    .index_number
                                    .unwrap_or(0)
                                    .to_string()
                                    .contains(&text.to_lowercase()))
                                && info.parent_index_number.unwrap_or(0)
                                    == season_map[&seasonlist
                                        .selected_item()
                                        .and_downcast_ref::<gtk::StringObject>()
                                        .unwrap()
                                        .string()
                                        .to_string()]
                            {
                                let object = glib::BoxedAnyObject::new(info.clone());
                                store.append(&object);
                            }
                        }
                    }
                ));
                itemrevealer.set_reveal_child(true);
            }
        ));

        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_, item| {
            let list_item = item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem");
            let listbox = gtk::Box::new(gtk::Orientation::Vertical, 5);
            let picture = gtk::Overlay::builder()
                .height_request(141)
                .width_request(240)
                .build();
            let label = gtk::Label::builder()
                .halign(gtk::Align::Start)
                .wrap_mode(gtk::pango::WrapMode::WordChar)
                .ellipsize(gtk::pango::EllipsizeMode::End)
                .attributes(&AttrList::from_string("0 -1 scale 0.9\n0 -1 weight bold").unwrap())
                .build();
            listbox.append(&picture);
            listbox.append(&label);
            list_item.set_child(Some(&listbox));
        });
        factory.connect_bind(|_, item| {
            let picture = item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<gtk::Box>()
                .expect("Needs to be Box")
                .first_child()
                .expect("Needs to be Picture");
            let picture = picture
                .downcast_ref::<gtk::Overlay>()
                .expect("Needs to be Box");
            let label = item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<gtk::Box>()
                .expect("Needs to be Box")
                .last_child()
                .expect("Needs to be Picture");
            let label = label
                .downcast_ref::<gtk::Label>()
                .expect("Needs to be Label");
            let entry = item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem")
                .item()
                .and_downcast::<glib::BoxedAnyObject>()
                .expect("Needs to be BoxedAnyObject");
            let seriesinfo: Ref<SeriesInfo> = entry.borrow();
            if picture.first_child().is_none() {
                let img = PictureLoader::new(&seriesinfo.id, "Primary", None);
                picture.set_child(Some(&img));
                let progressbar = gtk::ProgressBar::new();
                progressbar.set_valign(gtk::Align::End);
                if let Some(userdata) = &seriesinfo.user_data {
                    if let Some(percentage) = userdata.played_percentage {
                        progressbar.set_fraction(percentage / 100.0);
                    }
                    if userdata.played {
                        let mark = gtk::Image::from_icon_name("object-select-symbolic");
                        mark.set_halign(gtk::Align::End);
                        mark.set_valign(gtk::Align::Start);
                        mark.set_height_request(25);
                        mark.set_width_request(25);
                        picture.add_overlay(&mark);
                        label.add_css_class("dim-label");
                    }
                }
                picture.add_overlay(&progressbar);
                let markup = format!(
                    "{}. {}",
                    seriesinfo.index_number.unwrap_or(0),
                    seriesinfo.name
                );
                label.set_label(&markup);
            }
        });
        imp.itemlist.set_factory(Some(&factory));
        imp.itemlist.set_model(Some(&imp.selection));

        imp.itemlist.connect_activate(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            move |listview, position| {
                let model = listview.model().unwrap();
                let item = model
                    .item(position)
                    .and_downcast::<glib::BoxedAnyObject>()
                    .unwrap();
                spawn(glib::clone!(
                    #[weak]
                    obj,
                    async move {
                        let series_info = item.borrow::<SeriesInfo>().clone();
                        obj.selectepisode(series_info).await;
                    }
                ));
            }
        ));
    }

    pub fn logoset(&self) {
        let logobox = self.imp().logobox.get();
        let id = self.id();
        let logo = PictureLoader::new(&id, "Logo", None);
        logobox.append(&logo);
        logobox.add_css_class("logo");
    }

    pub fn itemfirst(&self) {
        let imp = self.imp();
        imp.itemlist
            .scroll_to(0, gtk::ListScrollFlags::SELECT, None);
        imp.itemlist.first_child().unwrap().activate();
    }

    pub fn itemprevious(&self) {
        let imp = self.imp();
        let selection = &imp.selection;
        let position = selection.selected();
        if position > 0 {
            imp.itemlist
                .scroll_to(position - 1, gtk::ListScrollFlags::SELECT, None);
            // Todo: activate the previous item
        }
    }

    pub fn itemnext(&self) {
        let imp = self.imp();
        let selection = &imp.selection;
        let position = selection.selected();
        if position < imp.itemlist.model().unwrap().n_items() {
            imp.itemlist
                .scroll_to(position + 1, gtk::ListScrollFlags::SELECT, None);
            // Todo: activate the next item
        }
    }

    pub fn itemlast(&self) {
        let imp = self.imp();
        imp.itemlist.scroll_to(
            imp.itemlist.model().unwrap().n_items() - 1,
            gtk::ListScrollFlags::SELECT,
            None,
        );
        imp.itemlist.last_child().unwrap().activate();
    }

    pub async fn selectmovie(&self, id: String, name: String, userdata: Option<UserData>) {
        let imp = self.imp();
        imp.playbutton.set_sensitive(false);
        imp.line1spinner.set_visible(true);
        let idclone = id.clone();
        let playback =
            match spawn_tokio(async move { EMBY_CLIENT.get_playbackinfo(&id).await }).await {
                Ok(playback) => playback,
                Err(e) => {
                    toast!(self, e.to_user_facing());
                    return;
                }
            };
        let id = idclone.clone();
        let info = SeriesInfo {
            id: id.clone(),
            name: name.clone(),
            user_data: userdata.clone(),
            overview: None,
            index_number: None,
            parent_index_number: None,
        };
        imp.line1.set_text(&info.name.to_string());
        imp.line1spinner.set_visible(false);
        self.set_dropdown(&playback, &info);
        let handlerid = self.bind_button(&playback, &info);
        imp.playbuttonhandlerid.replace(Some(handlerid));
        imp.playbutton.set_sensitive(true);
    }

    pub async fn selectepisode(&self, seriesinfo: SeriesInfo) {
        let info = seriesinfo.clone();
        let imp = self.imp();
        let id = seriesinfo.id.clone();
        imp.inid.replace(id.clone());
        imp.actionbox.set_episode_id(Some(id.clone()));
        imp.actionbox.bind_edit();
        imp.playbutton.set_sensitive(false);
        imp.line1spinner.set_visible(true);
        let playback =
            match spawn_tokio(async move { EMBY_CLIENT.get_playbackinfo(&id).await }).await {
                Ok(playback) => playback,
                Err(e) => {
                    toast!(self, e.to_user_facing());
                    return;
                }
            };

        let media_playback = playback.clone();

        let selected_name = format!(
            "S{}:E{} - {}",
            info.parent_index_number.unwrap_or(0),
            info.index_number.unwrap_or(0),
            info.name
        );
        imp.line1.set_text(&selected_name);
        imp.selected.replace(Some(selected_name));
        imp.line1spinner.set_visible(false);
        let info = info.clone();
        if let Some(handlerid) = imp.playbuttonhandlerid.borrow_mut().take() {
            imp.playbutton.disconnect(handlerid);
        }
        self.set_dropdown(&playback, &info);
        let handlerid = self.bind_button(&playback, &info);
        imp.playbuttonhandlerid.replace(Some(handlerid));
        imp.playbutton.set_sensitive(true);

        if let Some(overview) = seriesinfo.overview {
            imp.selecteditemoverview.buffer().set_text(&overview);
        }
        if let Some(user_data) = seriesinfo.user_data {
            if let Some(is_favourite) = user_data.is_favorite {
                imp.actionbox.set_episode_liked(is_favourite);
            }
            imp.actionbox.set_episode_played(user_data.played);
            imp.actionbox.bind_edit();
        }
        self.createmediabox(media_playback.media_sources, None)
            .await;
    }

    pub async fn setoverview(&self) {
        let imp = self.imp();
        let id = imp.id.get().unwrap().clone();
        let itemoverview = imp.itemoverview.get();
        let overviewrevealer = imp.overviewrevealer.get();

        let item = match req_cache(&format!("item_{}", &id), async move {
            EMBY_CLIENT.get_item_info(&id).await
        })
        .await
        {
            Ok(item) => item,
            Err(e) => {
                toast!(self, e.to_user_facing());
                Item::default()
            }
        };

        spawn(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                {
                    let mut str = String::new();
                    if let Some(communityrating) = item.community_rating {
                        let formatted_rating = format!("{:.1}", communityrating);
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
                        let duration = chrono::Duration::seconds((runtime / 10000000) as i64);
                        let hours = duration.num_hours();
                        let minutes = duration.num_minutes() % 60;
                        let seconds = duration.num_seconds() % 60;

                        let time_string = if hours > 0 {
                            format!("{}:{:02}", hours, minutes)
                        } else {
                            format!("{}:{:02}", minutes, seconds)
                        };
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

                    if let Some(taglines) = item.taglines {
                        if let Some(tagline) = taglines.first() {
                            obj.imp().tagline.set_text(tagline);
                            obj.imp().tagline.set_visible(true);
                        }
                    }
                }
                if let Some(overview) = item.overview {
                    itemoverview.buffer().set_text(&overview);
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
                overviewrevealer.set_reveal_child(true);
                if let Some(image_tags) = item.backdrop_image_tags {
                    obj.add_backdrops(image_tags).await;
                }
                if let Some(ref user_data) = item.user_data {
                    let imp = obj.imp();
                    if let Some(is_favourite) = user_data.is_favorite {
                        imp.actionbox.set_btn_active(is_favourite);
                    }
                    imp.actionbox.set_played(user_data.played);
                    imp.actionbox.bind_edit();
                }

                if let Some(media_sources) = item.media_sources {
                    obj.createmediabox(media_sources, item.date_created).await;
                }

                if item.item_type == "Series" {
                    obj.imp().toolbar.set_visible(true);
                    obj.setup_seasons().await;
                } else {
                    obj.selectmovie(item.id, item.name, item.user_data).await;
                }
            }
        ));
    }

    pub async fn createmediabox(
        &self,
        media_sources: Vec<MediaSource>,
        date_created: Option<DateTime<Utc>>,
    ) {
        let imp = self.imp();
        let mediainfobox = imp.mediainfobox.get();
        let mediainforevealer = imp.mediainforevealer.get();

        while mediainfobox.last_child().is_some() {
            if let Some(child) = mediainfobox.last_child() {
                mediainfobox.remove(&child)
            }
        }
        for mediasource in media_sources {
            let singlebox = gtk::Box::new(gtk::Orientation::Vertical, 5);
            let info = format!(
                "{}\n{} {} {}\n{}",
                mediasource.path.unwrap_or_default(),
                mediasource.container.to_uppercase(),
                bytefmt::format(mediasource.size),
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

            let mediabox = gtk::Box::new(gtk::Orientation::Horizontal, 5);
            for mediapart in mediasource.media_streams {
                if mediapart.stream_type == "Attachment" {
                    continue;
                }
                let mediapartbox = gtk::Box::builder()
                    .orientation(gtk::Orientation::Vertical)
                    .spacing(0)
                    .width_request(300)
                    .build();
                let mut str: String = Default::default();
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
                typebox.append(&gtk::Label::new(Some(&mediapart.stream_type)));
                if let Some(codec) = mediapart.codec {
                    str.push_str(format!("Codec: {}", codec).as_str());
                }
                if let Some(language) = mediapart.display_language {
                    str.push_str(format!("\nLanguage: {}", language).as_str());
                }
                if let Some(title) = mediapart.title {
                    str.push_str(format!("\nTitle: {}", title).as_str());
                }
                if let Some(bitrate) = mediapart.bit_rate {
                    str.push_str(format!("\nBitrate: {}it/s", bytefmt::format(bitrate)).as_str());
                }
                if let Some(bitdepth) = mediapart.bit_depth {
                    str.push_str(format!("\nBitDepth: {} bit", bitdepth).as_str());
                }
                if let Some(samplerate) = mediapart.sample_rate {
                    str.push_str(format!("\nSampleRate: {} Hz", samplerate).as_str());
                }
                if let Some(height) = mediapart.height {
                    str.push_str(format!("\nHeight: {}", height).as_str());
                }
                if let Some(width) = mediapart.width {
                    str.push_str(format!("\nWidth: {}", width).as_str());
                }
                if let Some(colorspace) = mediapart.color_space {
                    str.push_str(format!("\nColorSpace: {}", colorspace).as_str());
                }
                if let Some(displaytitle) = mediapart.display_title {
                    str.push_str(format!("\nDisplayTitle: {}", displaytitle).as_str());
                }
                if let Some(channel) = mediapart.channels {
                    str.push_str(format!("\nChannel: {}", channel).as_str());
                }
                if let Some(channellayout) = mediapart.channel_layout {
                    str.push_str(format!("\nChannelLayout: {}", channellayout).as_str());
                }
                if let Some(averageframerate) = mediapart.average_frame_rate {
                    str.push_str(format!("\nAverageFrameRate: {}", averageframerate).as_str());
                }
                if let Some(pixelformat) = mediapart.pixel_format {
                    str.push_str(format!("\nPixelFormat: {}", pixelformat).as_str());
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

        hortu.set_title("Actors");

        hortu.set_items(&actors);
    }

    pub async fn set_lists(&self) {
        self.sets("Recommend").await;
        self.sets("Included In").await;
        self.sets("Additional Parts").await;
    }

    pub async fn sets(&self, types: &str) {
        let hortu = match types {
            "Recommend" => self.imp().recommendhortu.get(),
            "Included In" => self.imp().includehortu.get(),
            "Additional Parts" => self.imp().additionalhortu.get(),
            _ => return,
        };

        hortu.set_title(types);

        let id = self.id();
        let types = types.to_string();

        let results = match req_cache(&format!("item_{types}_{id}"), async move {
            match types.as_str() {
                "Recommend" => EMBY_CLIENT.get_similar(&id).await,
                "Included In" => EMBY_CLIENT.get_included(&id).await,
                "Additional Parts" => EMBY_CLIENT.get_additional(&id).await,
                _ => Ok(List::default()),
            }
        })
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

    pub fn set_flowbuttons(&self, infos: Vec<SGTitem>, type_: &str) {
        let imp = self.imp();
        let horbu = match type_ {
            "Genres" => imp.genreshorbu.get(),
            "Studios" => imp.studioshorbu.get(),
            "Tags" => imp.tagshorbu.get(),
            _ => return,
        };

        horbu.set_title(type_);

        horbu.set_list_type(Some(type_.to_string()));

        horbu.set_items(&infos);
    }

    pub fn set_flowlinks(&self, links: Vec<Urls>) {
        let imp = self.imp();

        let horbu = imp.linkshorbu.get();

        horbu.set_title("Links");

        horbu.set_links(&links);
    }

    pub fn set_dropdown(&self, playbackinfo: &Media, info: &SeriesInfo) {
        let playbackinfo = playbackinfo.clone();
        let info = info.clone();
        let imp = self.imp();
        let namedropdown = imp.namedropdown.get();
        let subdropdown = imp.subdropdown.get();
        namedropdown.set_factory(Some(&factory(true)));
        namedropdown.set_list_factory(Some(&factory(false)));
        subdropdown.set_factory(Some(&factory(true)));
        subdropdown.set_list_factory(Some(&factory(false)));

        let vstore = gtk::gio::ListStore::new::<glib::BoxedAnyObject>();
        imp.videoselection.set_model(Some(&vstore));

        let sstore = gtk::gio::ListStore::new::<glib::BoxedAnyObject>();
        imp.subselection.set_model(Some(&sstore));

        if let Some(media) = playbackinfo.media_sources.first() {
            for stream in &media.media_streams {
                if stream.stream_type == "Subtitle" {
                    let dl = DropdownList {
                        line1: stream.display_title.clone(),
                        line2: stream.title.clone(),
                    };
                    let object = glib::BoxedAnyObject::new(dl);
                    sstore.append(&object);
                }
            }
        }

        for media in &playbackinfo.media_sources {
            let dl = DropdownList {
                line1: Some(media.name.clone()),
                line2: None,
            };

            let object = glib::BoxedAnyObject::new(dl);
            vstore.append(&object);
        }
        namedropdown.set_model(Some(&imp.videoselection));
        subdropdown.set_model(Some(&imp.subselection));

        namedropdown.connect_selected_item_notify(move |dropdown| {
            if let Some(entry) = dropdown
                .selected_item()
                .and_downcast::<glib::BoxedAnyObject>()
            {
                let dl: std::cell::Ref<DropdownList> = entry.borrow();
                let selected = dl.line1.clone().unwrap();
                for _i in 0..sstore.n_items() {
                    sstore.remove(0);
                }
                for media in playbackinfo.media_sources.clone() {
                    if media.name == selected {
                        for stream in media.media_streams {
                            if stream.stream_type == "Subtitle" {
                                let dl = DropdownList {
                                    line1: stream.display_title,
                                    line2: stream.title,
                                };
                                let object = glib::BoxedAnyObject::new(dl);
                                sstore.append(&object);
                            }
                        }
                        subdropdown.set_selected(0);
                        break;
                    }
                }
            }
        });

        let buttoncontent = &imp.buttoncontent;

        if SETTINGS.resume() {
            if let Some(userdata) = &info.user_data {
                if let Some(ticks) = userdata.playback_position_ticks {
                    if ticks > 0 {
                        let sec = ticks / 10000000;
                        buttoncontent.set_label(&format!(
                            "{} {}",
                            gettext("Resume"),
                            format_duration(sec as i64)
                        ));
                    } else {
                        buttoncontent.set_label(&gettext("Play"));
                    }
                }
            }
        }
    }

    pub fn bind_button(&self, playbackinfo: &Media, info: &SeriesInfo) -> glib::SignalHandlerId {
        let imp = self.imp();
        let playbackinfo = playbackinfo.clone();
        let playbutton = imp.playbutton.get();
        let namedropdown = imp.namedropdown.get();
        let subdropdown = imp.subdropdown.get();
        let info = info.clone();

        playbutton.connect_clicked(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            move |_| {
                let nameselected = if let Some(entry) = namedropdown
                    .selected_item()
                    .and_downcast::<glib::BoxedAnyObject>()
                {
                    let dl: std::cell::Ref<DropdownList> = entry.borrow();
                    dl.line1.clone().unwrap_or_default()
                } else {
                    return;
                };

                let subselected = if let Some(entry) = subdropdown
                    .selected_item()
                    .and_downcast::<glib::BoxedAnyObject>()
                {
                    let dl: std::cell::Ref<DropdownList> = entry.borrow();
                    &dl.line1.clone()
                } else {
                    &None
                };

                for media in playbackinfo.media_sources.clone() {
                    if media.name == nameselected {
                        let medianameselected = nameselected;
                        let url = media.direct_stream_url.clone();
                        let back = Back {
                            id: info.id.clone(),
                            mediasourceid: media.id.clone(),
                            playsessionid: playbackinfo.play_session_id.clone(),
                            tick: info
                                .user_data
                                .as_ref()
                                .map_or(0, |data| data.playback_position_ticks.unwrap_or(0)),
                        };
                        let percentage = info
                            .user_data
                            .as_ref()
                            .map_or(0., |data| data.played_percentage.unwrap_or(0.));
                        let id = info.id.clone();
                        let subselected = subselected.clone();
                        if let Some(url) = url {
                            let name = obj.imp().name.borrow().clone();
                            let selected = obj.imp().selected.borrow().clone();
                            spawn(async move {
                                let suburl = match media.media_streams.iter().find(|&mediastream| {
                                    mediastream.stream_type == "Subtitle"
                                        && Some(
                                            mediastream
                                                .display_title
                                                .as_ref()
                                                .unwrap_or(&"".to_string()),
                                        ) == subselected.as_ref()
                                        && mediastream.is_external
                                }) {
                                    Some(mediastream) => match mediastream.delivery_url.clone() {
                                        Some(url) => Some(url),
                                        None => {
                                            let playbackinfo = match spawn_tokio(async move {
                                                EMBY_CLIENT.get_sub(&id, &media.id).await
                                            })
                                            .await
                                            {
                                                Ok(playbackinfo) => playbackinfo,
                                                Err(e) => {
                                                    toast!(obj, e.to_user_facing());
                                                    return;
                                                }
                                            };
                                            let mediasource = playbackinfo
                                                .media_sources
                                                .iter()
                                                .find(|&media| media.name == medianameselected)
                                                .unwrap();
                                            let suburl = mediasource
                                                .media_streams
                                                .iter()
                                                .find(|&mediastream| {
                                                    mediastream.stream_type == "Subtitle"
                                                        && Some(
                                                            mediastream
                                                                .display_title
                                                                .as_ref()
                                                                .unwrap_or(&"".to_string()),
                                                        ) == subselected.as_ref()
                                                        && mediastream.is_external
                                                })
                                                .unwrap()
                                                .delivery_url
                                                .clone();
                                            suburl
                                        }
                                    },
                                    None => None,
                                };
                                obj.get_window().play_media(
                                    url,
                                    suburl,
                                    name,
                                    Some(back),
                                    selected,
                                    percentage,
                                );
                            });
                        } else {
                            toast!(obj, "No Stream URL found");
                            return;
                        }
                        return;
                    }
                }
            }
        ))
    }

    pub fn get_window(&self) -> Window {
        self.root().unwrap().downcast::<Window>().unwrap()
    }
}

pub struct DropdownList {
    pub line1: Option<String>,
    pub line2: Option<String>,
}

pub fn dt(date: Option<chrono::DateTime<Utc>>) -> String {
    let Some(date) = date else {
        return "".to_string();
    };
    date.format("%Y-%m-%d %H:%M:%S").to_string()
}
