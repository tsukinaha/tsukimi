use adw::prelude::NavigationPageExt;
use adw::subclass::prelude::*;
use glib::Object;
use gtk::prelude::*;
use gtk::{gio, glib};
use std::cell::Ref;
use std::collections::{HashMap, HashSet};

use crate::ui::network::{self, runtime, similar, SeriesInfo};

use super::actor::ActorPage;
use super::fix::fix;
use super::movie::MoviePage;

mod imp {
    use crate::ui::widgets::fix::fix;
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::prelude::*;
    use gtk::{glib, CompositeTemplate};
    use std::cell::OnceCell;

    // Object holding the state
    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsukimi/item.ui")]
    #[properties(wrapper_type = super::ItemPage)]
    pub struct ItemPage {
        #[property(get, set, construct_only)]
        pub id: OnceCell<String>,
        #[property(get, set, construct_only)]
        pub inid: OnceCell<String>,
        #[template_child]
        pub dropdownspinner: TemplateChild<gtk::Spinner>,
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
        pub itemoverview: TemplateChild<gtk::Inscription>,
        #[template_child]
        pub selecteditemoverview: TemplateChild<gtk::Inscription>,
        #[template_child]
        pub mediainfobox: TemplateChild<gtk::Box>,
        #[template_child]
        pub linksscrolled: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub mediainforevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub linksrevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub actorrevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub actorscrolled: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub recommendrevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub recommendscrolled: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub episodescrolled: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub recommendlist: TemplateChild<gtk::ListView>,
        #[template_child]
        pub actorlist: TemplateChild<gtk::ListView>,
        #[template_child]
        pub studiosscrolled: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub studiosrevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub tagsscrolled: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub tagsrevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub genresscrolled: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub genresrevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub episodesearchentry: TemplateChild<gtk::SearchEntry>,
        pub selection: gtk::SingleSelection,
        pub seasonselection: gtk::SingleSelection,
        pub actorselection: gtk::SingleSelection,
        pub recommendselection: gtk::SingleSelection,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for ItemPage {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "ItemPage";
        type Type = super::ItemPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
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
            fix(self.episodescrolled.get());
            obj.setup_background();
            obj.setup_seasons();
            obj.logoset();
            obj.setoverview();
            obj.get_similar();
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

impl ItemPage {
    pub fn new(id: String, inid: String) -> Self {
        Object::builder()
            .property("id", id)
            .property("inid", inid)
            .build()
    }

    pub fn setup_background(&self) {
        let id = self.id();
        let id1 = self.id();
        let imp = self.imp();
        let path = format!(
            "{}/.local/share/tsukimi/b{}.png",
            dirs::home_dir().expect("msg").display(),
            id1
        );
        let pathbuf = std::path::PathBuf::from(&path);
        let backdrop = imp.backdrop.get();
        let settings = gtk::gio::Settings::new(crate::APP_ID);
        backdrop.set_height_request(settings.int("background-height"));
        let (sender, receiver) = async_channel::bounded::<String>(1);
        if pathbuf.exists() {
            backdrop.set_file(Some(&gtk::gio::File::for_path(&path)));
            glib::spawn_future_local(glib::clone!(@weak self as obj =>async move {
                let window = obj.root().and_downcast::<super::window::Window>().unwrap();
                window.set_rootpic(gtk::gio::File::for_path(&path));
            }));
        } else {
            crate::ui::network::runtime().spawn(async move {
                let id = crate::ui::network::get_backdropimage(id1)
                    .await
                    .expect("msg");
                sender
                    .send(id)
                    .await
                    .expect("The channel needs to be open.");
            });
        }
        let id2 = id.to_string();
        glib::spawn_future_local(glib::clone!(@weak self as obj =>async move {
            while receiver.recv().await.is_ok() {
                let path = format!(
                    "{}/.local/share/tsukimi/b{}.png",
                    dirs::home_dir().expect("msg").display(),
                    id2
                );
                if pathbuf.exists() {
                    let file = gtk::gio::File::for_path(&path);
                    backdrop.set_file(Some(&file));
                    let window = obj.root().and_downcast::<super::window::Window>().unwrap();
                    window.set_rootpic(file);
                }
            }
        }));
    }

    pub fn setup_seasons(&self) {
        let imp = self.imp();
        let itemrevealer = imp.itemrevealer.get();
        let id = self.id();
        let idc = id.clone();
        let inid = self.inid();

        let store = gtk::gio::ListStore::new::<glib::BoxedAnyObject>();
        imp.selection.set_autoselect(false);
        imp.selection.set_model(Some(&store));

        let (sender, receiver) = async_channel::bounded::<Vec<crate::ui::network::SeriesInfo>>(1);
        network::runtime().spawn(async move {
            match network::get_series_info(id).await {
                Ok(series_info) => {
                    sender
                        .send(series_info)
                        .await
                        .expect("series_info not received.");
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        });

        let seasonstore = gtk::StringList::new(&[]);
        imp.seasonselection.set_model(Some(&seasonstore));
        let seasonlist = imp.seasonlist.get();
        seasonlist.set_model(Some(&imp.seasonselection));
        let itemlist = imp.itemlist.get();
        glib::spawn_future_local(glib::clone!(@weak self as obj,@weak store =>async move {
            let series_info = receiver.recv().await.expect("series_info not received.");
            let mut season_set: HashSet<u32> = HashSet::new();
            let mut season_map: HashMap<String,u32> = HashMap::new();
            let min_season = series_info.iter().map(|info| if info.parent_index_number == 0 { 100 } else { info.parent_index_number }).min().unwrap_or(1);
            let mut pos = 0;
            let mut set = true;
            for info in &series_info {
                if !season_set.contains(&info.parent_index_number) {
                    let seasonstring = format!("Season {}", info.parent_index_number);
                    seasonstore.append(&seasonstring);
                    season_set.insert(info.parent_index_number);
                    season_map.insert(seasonstring.clone(), info.parent_index_number);
                    if set {
                        if info.parent_index_number == min_season {
                            set = false;
                        } else {
                            pos += 1;
                        }
                    }
                }
                if info.parent_index_number == min_season {
                    let object = glib::BoxedAnyObject::new(info.clone());
                    store.append(&object);
                }
                if inid != idc && info.id == inid {
                    let seriesinfo = network::SeriesInfo {
                        id: inid.clone(),
                        name: info.name.clone(),
                        index_number: info.index_number,
                        parent_index_number: info.parent_index_number,
                        user_data: info.user_data.clone(),
                        overview: info.overview.clone(),
                    };
                    obj.selectepisode(seriesinfo.clone());
                }
            }
            seasonlist.set_selected(pos);
            if idc == inid {
                itemlist.first_child().unwrap().activate();
            }
            let seriesinfo_seasonlist = series_info.clone();
            let seriesinfo_seasonmap = season_map.clone();
            seasonlist.connect_selected_item_notify(glib::clone!(@weak store => move |dropdown| {
                let selected = dropdown.selected_item();
                let selected = selected.and_downcast_ref::<gtk::StringObject>().unwrap();
                let selected = selected.string().to_string();
                store.remove_all();
                let season_number = seriesinfo_seasonmap[&selected];
                for info in &seriesinfo_seasonlist {
                    if info.parent_index_number == season_number {
                        let object = glib::BoxedAnyObject::new(info.clone());
                        store.append(&object);
                    }
                }
                itemlist.first_child().unwrap().activate();
            }));
            let episodesearchentry = obj.imp().episodesearchentry.get();
            episodesearchentry.connect_search_changed(glib::clone!(@weak store => move |entry| {
                let text = entry.text();
                store.remove_all();
                for info in &series_info {
                    if (info.name.to_lowercase().contains(&text.to_lowercase()) || info.index_number.to_string().contains(&text.to_lowercase())) && info.parent_index_number == season_map[&seasonlist.selected_item().and_downcast_ref::<gtk::StringObject>().unwrap().string().to_string()] {
                        let object = glib::BoxedAnyObject::new(info.clone());
                        store.append(&object);
                    }
                }
            }));
            itemrevealer.set_reveal_child(true);
        }));

        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_, item| {
            let list_item = item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem");
            let listbox = gtk::Box::new(gtk::Orientation::Vertical, 5);
            let picture = gtk::Overlay::builder()
                .height_request(141)
                .width_request(250)
                .build();
            let label = gtk::Label::builder()
                .halign(gtk::Align::Start)
                .wrap_mode(gtk::pango::WrapMode::WordChar)
                .ellipsize(gtk::pango::EllipsizeMode::End)
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
            let seriesinfo: Ref<network::SeriesInfo> = entry.borrow();
            if picture.first_child().is_none() {
                let img = crate::ui::image::setimage(seriesinfo.id.clone());
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
                    }
                }
                picture.add_overlay(&progressbar);
                let markup = format!("{}. {}", seriesinfo.index_number, seriesinfo.name);
                label.set_label(&markup);
            }
        });
        imp.itemlist.set_factory(Some(&factory));
        imp.itemlist.set_model(Some(&imp.selection));

        imp.itemlist
            .connect_activate(glib::clone!(@weak self as obj =>move |listview, position| {
                let model = listview.model().unwrap();
                let item = model
                    .item(position)
                    .and_downcast::<glib::BoxedAnyObject>()
                    .unwrap();
                obj.selectepisode(item.borrow::<network::SeriesInfo>().clone());
            }));
    }

    pub fn logoset(&self) {
        let logobox = self.imp().logobox.get();
        let id = self.id();
        let logo = crate::ui::image::setlogoimage(id.clone());
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

    pub fn selectepisode(&self, seriesinfo: SeriesInfo) {
        let info = seriesinfo.clone();
        let imp = self.imp();
        let osdbox = imp.osdbox.get();
        let dropdownspinner = imp.dropdownspinner.get();
        let id = seriesinfo.id.clone();
        let idc = id.clone();
        dropdownspinner.set_visible(true);
        if let Some(widget) = osdbox.last_child() {
            if widget.is::<gtk::Box>() {
                osdbox.remove(&widget);
            }
        }
        let mutex = std::sync::Arc::new(tokio::sync::Mutex::new(()));
        let (sender, receiver) = async_channel::bounded::<crate::ui::network::Media>(1);
        crate::ui::network::runtime().spawn(async move {
            let playback = crate::ui::network::get_playbackinfo(id).await.expect("msg");
            sender.send(playback).await.expect("msg");
        });
        glib::spawn_future_local(
            glib::clone!(@weak dropdownspinner,@weak osdbox=>async move {
                while let Ok(playback) = receiver.recv().await {
                    let _ = mutex.lock().await;
                    let info = info.clone();
                    let dropdown = crate::ui::new_dropsel::newmediadropsel(playback.clone(), info);
                    dropdownspinner.set_visible(false);
                    if let Some(widget) = osdbox.last_child() {
                        if widget.is::<gtk::Box>() {
                            osdbox.remove(&widget);
                        }
                    }
                    osdbox.append(&dropdown);
                }
            }),
        );

        if let Some(overview) = seriesinfo.overview {
            imp.selecteditemoverview.set_text(Some(&overview));
        }
        self.createmediabox(idc);
    }

    pub fn setoverview(&self) {
        let imp = self.imp();
        let id = imp.id.get().unwrap().clone();
        let itemoverview = imp.itemoverview.get();
        let overviewrevealer = imp.overviewrevealer.get();
        let (sender, receiver) = async_channel::bounded::<crate::ui::network::Item>(1);
        crate::ui::network::runtime().spawn(async move {
            let item = crate::ui::network::get_item_overview(id.to_string())
                .await
                .expect("msg");
            sender.send(item).await.expect("msg");
        });
        glib::spawn_future_local(glib::clone!(@weak self as obj=>async move {
            while let Ok(item) = receiver.recv().await {
                if let Some(overview) = item.overview {
                    itemoverview.set_text(Some(&overview));
                }
                if let Some(links) = item.external_urls {
                    obj.setlinksscrolled(links);
                }
                if let Some(actor) = item.people {
                    obj.setactorscrolled(actor);
                }
                if let Some(studios) = item.studios {
                    obj.set_studio(studios);
                }
                if let Some(tags) = item.tags {
                    obj.set_tags(tags);
                }
                if let Some(genres) = item.genres {
                    obj.set_genres(genres);
                }
                overviewrevealer.set_reveal_child(true);
            }
        }));
    }

    pub fn createmediabox(&self, id: String) {
        let imp = self.imp();
        let mediainfobox = imp.mediainfobox.get();
        let mediainforevealer = imp.mediainforevealer.get();
        let (sender, receiver) = async_channel::bounded::<crate::ui::network::Media>(1);
        crate::ui::network::runtime().spawn(async move {
            let media = crate::ui::network::get_mediainfo(id.to_string())
                .await
                .expect("msg");
            sender.send(media).await.expect("msg");
        });
        glib::spawn_future_local(async move {
            while let Ok(media) = receiver.recv().await {
                while mediainfobox.last_child().is_some() {
                    if let Some(child) = mediainfobox.last_child() {
                        mediainfobox.remove(&child)
                    }
                }
                for mediasource in media.media_sources {
                    let singlebox = gtk::Box::new(gtk::Orientation::Vertical, 5);
                    let info = format!(
                        "{} {}\n{}",
                        mediasource.container.to_uppercase(),
                        bytefmt::format(mediasource.size),
                        mediasource.name
                    );
                    let label = gtk::Label::builder()
                        .label(&info)
                        .halign(gtk::Align::Start)
                        .margin_start(15)
                        .valign(gtk::Align::Start)
                        .margin_top(5)
                        .build();
                    singlebox.append(&label);

                    let mediascrolled = gtk::ScrolledWindow::builder()
                        .hscrollbar_policy(gtk::PolicyType::Automatic)
                        .vscrollbar_policy(gtk::PolicyType::Never)
                        .overlay_scrolling(true)
                        .build();

                    let mediascrolled = fix(mediascrolled);

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
                            icon.set_from_icon_name(Some("video-x-generic-symbolic"))
                        } else if mediapart.stream_type == "Audio" {
                            icon.set_from_icon_name(Some("audio-x-generic-symbolic"))
                        } else if mediapart.stream_type == "Subtitle" {
                            icon.set_from_icon_name(Some("media-view-subtitles-symbolic"))
                        } else {
                            icon.set_from_icon_name(Some("text-x-generic-symbolic"))
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
                            str.push_str(
                                format!("\nBitrate: {}it/s", bytefmt::format(bitrate)).as_str(),
                            );
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
                            str.push_str(
                                format!("\nAverageFrameRate: {}", averageframerate).as_str(),
                            );
                        }
                        if let Some(pixelformat) = mediapart.pixel_format {
                            str.push_str(format!("\nPixelFormat: {}", pixelformat).as_str());
                        }
                        let inscription = gtk::Inscription::builder()
                            .text(&str)
                            .min_lines(14)
                            .hexpand(true)
                            .yalign(0.0)
                            .build();
                        mediapartbox.append(&typebox);
                        mediapartbox.append(&inscription);
                        mediabox.append(&mediapartbox);
                    }

                    mediascrolled.set_child(Some(&mediabox));
                    singlebox.append(&mediascrolled);
                    mediainfobox.append(&singlebox);
                }
                mediainforevealer.set_reveal_child(true);
            }
        });
    }

    pub fn setlinksscrolled(&self, links: Vec<crate::ui::network::Urls>) {
        let imp = self.imp();
        let linksscrolled = fix(imp.linksscrolled.get());
        let linksrevealer = imp.linksrevealer.get();
        if !links.is_empty() {
            linksrevealer.set_reveal_child(true);
        }
        let linkbox = gtk::Box::new(gtk::Orientation::Horizontal, 5);
        while linkbox.last_child().is_some() {
            if let Some(child) = linkbox.last_child() {
                linkbox.remove(&child)
            }
        }
        for url in links {
            let linkbutton = gtk::Button::builder()
                .margin_start(10)
                .margin_top(10)
                .build();
            let buttoncontent = adw::ButtonContent::builder()
                .label(&url.name)
                .icon_name("send-to-symbolic")
                .build();
            linkbutton.set_child(Some(&buttoncontent));
            linkbutton.connect_clicked(move |_| {
                let _ = gio::AppInfo::launch_default_for_uri(
                    &url.url,
                    Option::<&gio::AppLaunchContext>::None,
                );
            });
            linkbox.append(&linkbutton);
        }
        linksscrolled.set_child(Some(&linkbox));
        linksrevealer.set_reveal_child(true);
    }

    pub fn setactorscrolled(&self, actors: Vec<crate::ui::network::People>) {
        let imp = self.imp();
        let actorscrolled = fix(imp.actorscrolled.get());
        let actorrevealer = imp.actorrevealer.get();
        if !actors.is_empty() {
            actorrevealer.set_reveal_child(true);
        }
        let store = gtk::gio::ListStore::new::<glib::BoxedAnyObject>();
        for people in actors {
            let object = glib::BoxedAnyObject::new(people);
            store.append(&object);
        }
        imp.actorselection.set_autoselect(false);
        imp.actorselection.set_model(Some(&store));
        let actorselection = &imp.actorselection;
        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_, item| {
            let list_item = item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem");
            let listbox = gtk::Box::new(gtk::Orientation::Vertical, 5);
            let picture = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .height_request(200)
                .width_request(150)
                .build();
            let label = gtk::Label::builder()
                .halign(gtk::Align::Start)
                .ellipsize(gtk::pango::EllipsizeMode::End)
                .build();
            listbox.append(&picture);
            listbox.append(&label);
            list_item.set_child(Some(&listbox));
        });
        factory.connect_bind(move |_, item| {
            let picture = item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<gtk::Box>()
                .expect("Needs to be Box")
                .first_child()
                .expect("Needs to be Picture");
            let label = item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<gtk::Box>()
                .expect("Needs to be Box")
                .last_child()
                .expect("Needs to be Picture");
            let entry = item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem")
                .item()
                .and_downcast::<glib::BoxedAnyObject>()
                .expect("Needs to be BoxedAnyObject");
            let people: std::cell::Ref<crate::ui::network::People> = entry.borrow();
            if picture.is::<gtk::Box>() {
                if let Some(_revealer) = picture
                    .downcast_ref::<gtk::Box>()
                    .expect("Needs to be Box")
                    .first_child()
                {
                } else {
                    let img = crate::ui::image::setimage(people.id.clone());
                    picture
                        .downcast_ref::<gtk::Box>()
                        .expect("Needs to be Box")
                        .append(&img);
                }
            }
            if label.is::<gtk::Label>() {
                if let Some(role) = &people.role {
                    let str = format!("{}\n{}", people.name, role);
                    label
                        .downcast_ref::<gtk::Label>()
                        .expect("Needs to be Label")
                        .set_text(&str);
                }
            }
        });
        imp.actorlist.set_factory(Some(&factory));
        imp.actorlist.set_model(Some(actorselection));
        let actorlist = imp.actorlist.get();
        actorlist.connect_activate(glib::clone!(@weak self as obj =>move |listview, position| {
            let model = listview.model().unwrap();
            let item = model
                .item(position)
                .and_downcast::<glib::BoxedAnyObject>()
                .unwrap();
            let actor: std::cell::Ref<crate::ui::network::People> = item.borrow();
            let window = obj.root().and_downcast::<super::window::Window>().unwrap();
            let view = match window.current_view_name().as_str() {
                "homepage" => {
                    window.set_title(&actor.name);
                    std::env::set_var("HOME_TITLE", &actor.name);
                    &window.imp().homeview
                }
                "searchpage" => {
                    window.set_title(&actor.name);
                    std::env::set_var("SEARCH_TITLE", &actor.name);
                    &window.imp().searchview
                }
                "historypage" => {
                    window.set_title(&actor.name);
                    std::env::set_var("HISTORY_TITLE", &actor.name);
                    &window.imp().historyview
                }
                _ => {
                    &window.imp().searchview
                }
            };
            let item_page = ActorPage::new(&actor.id);
            if view.find_page(actor.name.as_str()).is_some() {
                view.pop_to_tag(actor.name.as_str());
            } else {
                item_page.set_tag(Some(actor.name.as_str()));
                view.push(&item_page);
            }
        }));
        actorscrolled.set_child(Some(&actorlist));
    }

    pub fn get_similar(&self) {
        let id = self.id();
        let (sender, receiver) = async_channel::bounded::<Vec<crate::ui::network::SearchResult>>(1);
        runtime().spawn(async move {
            let id = similar(&id).await.expect("msg");
            sender
                .send(id)
                .await
                .expect("The channel needs to be open.");
        });

        glib::spawn_future_local(glib::clone!(@weak self as obj =>async move {
            while let Ok(result) = receiver.recv().await {
                obj.setrecommendscrolled(result);
            }
        }));
    }

    pub fn setrecommendscrolled(&self, recommend: Vec<crate::ui::network::SearchResult>) {
        let imp = self.imp();
        let recommendscrolled = fix(imp.recommendscrolled.get());
        let recommendrevealer = imp.recommendrevealer.get();
        if !recommend.is_empty() {
            recommendrevealer.set_reveal_child(true);
        }
        let store = gtk::gio::ListStore::new::<glib::BoxedAnyObject>();
        for recommend in recommend {
            let object = glib::BoxedAnyObject::new(recommend);
            store.append(&object);
        }
        imp.recommendselection.set_autoselect(false);
        imp.recommendselection.set_model(Some(&store));
        let recommendselection = &imp.recommendselection;
        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_, item| {
            let list_item = item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem");
            let listbox = gtk::Box::new(gtk::Orientation::Vertical, 5);
            let picture = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .height_request(273)
                .width_request(182)
                .build();
            let label = gtk::Label::builder()
                .valign(gtk::Align::Start)
                .halign(gtk::Align::Center)
                .justify(gtk::Justification::Center)
                .wrap_mode(gtk::pango::WrapMode::WordChar)
                .ellipsize(gtk::pango::EllipsizeMode::End)
                .build();
            listbox.append(&picture);
            listbox.append(&label);
            list_item.set_child(Some(&listbox));
        });
        factory.connect_bind(move |_, item| {
            let picture = item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<gtk::Box>()
                .expect("Needs to be Box")
                .first_child()
                .expect("Needs to be Picture");
            let label = item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<gtk::Box>()
                .expect("Needs to be Box")
                .last_child()
                .expect("Needs to be Picture");
            let entry = item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem")
                .item()
                .and_downcast::<glib::BoxedAnyObject>()
                .expect("Needs to be BoxedAnyObject");
            let recommend: std::cell::Ref<crate::ui::network::SearchResult> = entry.borrow();
            if picture.is::<gtk::Box>() {
                if let Some(_revealer) = picture
                    .downcast_ref::<gtk::Box>()
                    .expect("Needs to be Box")
                    .first_child()
                {
                } else {
                    let img = crate::ui::image::setimage(recommend.id.clone());
                    let overlay = gtk::Overlay::builder().child(&img).build();
                    if let Some(userdata) = &recommend.user_data {
                        if let Some(unplayeditemcount) = userdata.unplayed_item_count {
                            if unplayeditemcount > 0 {
                                let mark = gtk::Label::new(Some(
                                    &userdata
                                        .unplayed_item_count
                                        .expect("no unplayeditemcount")
                                        .to_string(),
                                ));
                                mark.set_valign(gtk::Align::Start);
                                mark.set_halign(gtk::Align::End);
                                mark.set_height_request(40);
                                mark.set_width_request(40);
                                overlay.add_overlay(&mark);
                            }
                        }
                        if userdata.played {
                            let mark = gtk::Image::from_icon_name("object-select-symbolic");
                            mark.set_halign(gtk::Align::End);
                            mark.set_valign(gtk::Align::Start);
                            mark.set_height_request(40);
                            mark.set_width_request(40);
                            overlay.add_overlay(&mark);
                        }
                    }
                    picture
                        .downcast_ref::<gtk::Box>()
                        .expect("Needs to be Box")
                        .append(&overlay);
                }
            }
            if label.is::<gtk::Label>() {
                if let Some(production_year) = &recommend.production_year {
                    let str = format!("{}\n{}", recommend.name, production_year);
                    label
                        .downcast_ref::<gtk::Label>()
                        .expect("Needs to be Label")
                        .set_text(&str);
                }
            }
        });
        imp.recommendlist.set_factory(Some(&factory));
        imp.recommendlist.set_model(Some(recommendselection));
        let recommendlist = imp.recommendlist.get();
        recommendlist.connect_activate(
            glib::clone!(@weak self as obj =>move |listview, position| {
                let model = listview.model().unwrap();
                let item = model
                    .item(position)
                    .and_downcast::<glib::BoxedAnyObject>()
                    .unwrap();
                let recommend: std::cell::Ref<crate::ui::network::SearchResult> = item.borrow();
                let window = obj.root().and_downcast::<super::window::Window>().unwrap();
                let view = match window.current_view_name().as_str() {
                    "homepage" => {
                        window.set_title(&recommend.name);
                        std::env::set_var("HOME_TITLE", &recommend.name);
                        &window.imp().homeview
                    }
                    "searchpage" => {
                        window.set_title(&recommend.name);
                        std::env::set_var("SEARCH_TITLE", &recommend.name);
                        &window.imp().searchview
                    }
                    "historypage" => {
                        window.set_title(&recommend.name);
                        std::env::set_var("HISTORY_TITLE", &recommend.name);
                        &window.imp().historyview
                    }
                    _ => {
                        &window.imp().searchview
                    }
                };
                if recommend.result_type == "Movie" {
                    let item_page = MoviePage::new(recommend.id.clone(),recommend.name.clone());
                    if view.find_page(recommend.name.as_str()).is_some() {
                        view.pop_to_tag(recommend.name.as_str());
                    } else {
                        item_page.set_tag(Some(recommend.name.as_str()));
                        view.push(&item_page);
                    }
                } else {
                    let item_page = ItemPage::new(recommend.id.clone(),recommend.id.clone());
                    if view.find_page(recommend.name.as_str()).is_some() {
                        view.pop_to_tag(recommend.name.as_str());
                    } else {
                        item_page.set_tag(Some(recommend.name.as_str()));
                        view.push(&item_page);
                    }
                }
            }),
        );
        recommendscrolled.set_child(Some(&recommendlist));
    }

    pub fn set_studio(&self, infos: Vec<crate::ui::network::SGTitem>) {
        let imp = self.imp();
        let scrolled = fix(imp.studiosscrolled.get());
        let revealer = imp.studiosrevealer.get();
        self.setup_sgts(revealer, scrolled, infos);
    }

    pub fn set_tags(&self, infos: Vec<crate::ui::network::SGTitem>) {
        let imp = self.imp();
        let scrolled = fix(imp.tagsscrolled.get());
        let revealer = imp.tagsrevealer.get();
        self.setup_sgts(revealer, scrolled, infos);
    }

    pub fn set_genres(&self, infos: Vec<crate::ui::network::SGTitem>) {
        let imp = self.imp();
        let scrolled = fix(imp.genresscrolled.get());
        let revealer = imp.genresrevealer.get();
        self.setup_sgts(revealer, scrolled, infos);
    }

    pub fn setup_sgts(
        &self,
        linksrevealer: gtk::Revealer,
        linksscrolled: gtk::ScrolledWindow,
        infos: Vec<crate::ui::network::SGTitem>,
    ) {
        if !infos.is_empty() {
            linksrevealer.set_reveal_child(true);
        }
        let linkbox = gtk::Box::new(gtk::Orientation::Horizontal, 5);
        for url in infos {
            let linkbutton = gtk::Button::builder()
                .margin_start(10)
                .margin_top(10)
                .label(&url.name)
                .build();
            linkbutton.add_css_class("raised");
            linkbox.append(&linkbutton);
        }
        linksscrolled.set_child(Some(&linkbox));
        linksrevealer.set_reveal_child(true);
    }
}
