use std::path::PathBuf;

use adw::prelude::NavigationPageExt;
use adw::subclass::prelude::*;
use glib::Object;
use gtk::prelude::*;
use gtk::{gio, glib};

use crate::ui::network::{runtime, similar};

use super::actor::ActorPage;
use super::fix::fix;
use super::item::ItemPage;
mod imp {
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::prelude::*;
    use gtk::{glib, CompositeTemplate};
    use std::cell::OnceCell;
    // Object holding the state
    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsukimi/movie.ui")]
    #[properties(wrapper_type = super::MoviePage)]
    pub struct MoviePage {
        #[property(get, set, construct_only)]
        pub id: OnceCell<String>,
        #[property(get, set, construct_only)]
        pub moviename: OnceCell<String>,
        #[template_child]
        pub backdrop: TemplateChild<gtk::Picture>,
        #[template_child]
        pub osdbox: TemplateChild<gtk::Box>,
        #[template_child]
        pub logobox: TemplateChild<gtk::Box>,
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
        pub actorlist: TemplateChild<gtk::ListView>,
        #[template_child]
        pub overviewrevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub itemoverview: TemplateChild<gtk::Inscription>,
        #[template_child]
        pub recommendlist: TemplateChild<gtk::ListView>,
        #[template_child]
        pub recommendrevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub recommendscrolled: TemplateChild<gtk::ScrolledWindow>,
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
        pub selection: gtk::SingleSelection,
        pub actorselection: gtk::SingleSelection,
        pub recommendselection: gtk::SingleSelection,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for MoviePage {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "MoviePage";
        type Type = super::MoviePage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // Trait shared by all GObjects
    #[glib::derived_properties]
    impl ObjectImpl for MoviePage {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_background();
            obj.logoset();
            obj.setoverview();
            obj.createmediabox();
            obj.get_similar();
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for MoviePage {}

    // Trait shared by all windows
    impl WindowImpl for MoviePage {}

    // Trait shared by all application windows
    impl ApplicationWindowImpl for MoviePage {}

    impl adw::subclass::navigation_page::NavigationPageImpl for MoviePage {}
}

glib::wrapper! {
    pub struct MoviePage(ObjectSubclass<imp::MoviePage>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget ,adw::NavigationPage,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl MoviePage {
    pub fn new(id: String, name: String) -> Self {
        Object::builder()
            .property("id", id)
            .property("moviename", name)
            .build()
    }

    fn setup_background(&self) {
        let imp = self.imp();
        let id = self.id();
        let path = format!(
            "{}/.local/share/tsukimi/b{}.png",
            dirs::home_dir().expect("msg").display(),
            id
        );
        let pathbuf = PathBuf::from(&path);
        let backdrop = imp.backdrop.get();
        let settings = gtk::gio::Settings::new(crate::APP_ID);
        backdrop.set_height_request(settings.int("background-height"));
        let (sender, receiver) = async_channel::bounded::<String>(1);
        let id2 = id.clone();
        if pathbuf.exists() {
            backdrop.set_file(Some(&gtk::gio::File::for_path(&path)));
            glib::spawn_future_local(glib::clone!(@weak self as obj =>async move {
                let window = obj.root().and_downcast::<super::window::Window>().unwrap();
                window.set_rootpic(gtk::gio::File::for_path(&path));
            }));
        } else {
            crate::ui::network::runtime().spawn(async move {
                let id = crate::ui::network::get_backdropimage(id)
                    .await
                    .expect("msg");
                sender
                    .send(id)
                    .await
                    .expect("The channel needs to be open.");
            });
        }
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
    pub fn logoset(&self) {
        let osd = &self.imp().logobox;
        let id = self.id();
        let logo = crate::ui::image::setlogoimage(id);
        osd.append(&logo);
        osd.add_css_class("logo");
    }

    pub fn setoverview(&self) {
        let imp = self.imp();
        let id = imp.id.get().unwrap().clone();
        let idclone = id.clone();
        let itemoverview = imp.itemoverview.get();
        let overviewrevealer = imp.overviewrevealer.get();
        let (sender, receiver) = async_channel::bounded::<crate::ui::network::Item>(1);
        crate::ui::network::runtime().spawn(async move {
            let item = crate::ui::network::get_item_overview(id)
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
                if let Some(userdata) = item.user_data {
                    obj.dropdown(idclone.clone(), item.name.clone(), Some(userdata));
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

    pub fn dropdown(
        &self,
        id: String,
        name: String,
        userdata: Option<crate::ui::network::UserData>,
    ) {
        let imp = self.imp();
        let osdbox = imp.osdbox.get();
        self.imp().playbutton.set_sensitive(false);
        self.imp().line1spinner.set_visible(true);
        let idclone = id.clone();
        let (sender, receiver) = async_channel::bounded::<crate::ui::network::Media>(1);
        crate::ui::network::runtime().spawn(async move {
            let playback = crate::ui::network::get_playbackinfo(id).await.expect("msg");
            sender.send(playback).await.expect("msg");
        });
        glib::spawn_future_local(
            glib::clone!(@weak osdbox,@weak self as obj =>async move {
                while let Ok(playback) = receiver.recv().await {
                    let info:crate::ui::network::SearchResult = crate::ui::network::SearchResult {
                        id: idclone.clone(),
                        name: name.clone(),
                        result_type: String::from("Movie"),
                        user_data: userdata.clone(),
                        production_year: None
                    };
                    obj.imp().line1.set_text(&format!("{}", info.name));
                    obj.imp().line1spinner.set_visible(false);
                    let info = info.clone();
                    crate::ui::moviedrop::newmediadropsel(playback.clone(), info, obj.imp().namedropdown.get(), obj.imp().subdropdown.get(), obj.imp().playbutton.get());
                    obj.imp().playbutton.set_sensitive(true);
                }
            }),
        );
    }

    pub fn createmediabox(&self) {
        let id = self.id();
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
        fix(imp.actorscrolled.get());
        let actorrevealer = imp.actorrevealer.get();
        if !actors.is_empty() {
            actorrevealer.set_reveal_child(true);
        }
        let store = gtk::gio::ListStore::new::<glib::BoxedAnyObject>();
        for people in actors {
            let object = glib::BoxedAnyObject::new(people);
            store.append(&object);
        }
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
        actorrevealer.set_reveal_child(true);
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
                    picture
                        .downcast_ref::<gtk::Box>()
                        .expect("Needs to be Box")
                        .append(&img);
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
