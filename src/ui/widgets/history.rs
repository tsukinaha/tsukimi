use adw::prelude::NavigationPageExt;
use glib::Object;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

use super::actor::ActorPage;
use super::fix::fix;
use super::item::ItemPage;
use super::movie::MoviePage;
mod imp {
    use glib::subclass::InitializingObject;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate};

    // Object holding the state
    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/moe/tsukimi/history.ui")]
    pub struct HistoryPage {
        #[template_child]
        pub historylist: TemplateChild<gtk::ListView>,
        #[template_child]
        pub spinner: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub hisscrolled: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub historyrevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub movierevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub moviescrolled: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub movielist: TemplateChild<gtk::ListView>,
        #[template_child]
        pub seriesrevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub seriesscrolled: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub serieslist: TemplateChild<gtk::ListView>,
        #[template_child]
        pub episoderevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub episodescrolled: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub episodelist: TemplateChild<gtk::ListView>,
        #[template_child]
        pub peoplerevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub peoplescrolled: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub peoplelist: TemplateChild<gtk::ListView>,
        #[template_child]
        pub albumrevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub albumscrolled: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub albumlist: TemplateChild<gtk::ListView>,
        pub movieselection: gtk::SingleSelection,
        pub seriesselection: gtk::SingleSelection,
        pub episodeselection: gtk::SingleSelection,
        pub peopleselection: gtk::SingleSelection,
        pub albumselection: gtk::SingleSelection,
        pub selection: gtk::SingleSelection,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for HistoryPage {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "HistoryPage";
        type Type = super::HistoryPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // Trait shared by all GObjects
    impl ObjectImpl for HistoryPage {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_history();
            obj.set_lists();
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for HistoryPage {}

    // Trait shared by all windows
    impl WindowImpl for HistoryPage {}

    // Trait shared by all application windows
    impl ApplicationWindowImpl for HistoryPage {}

    impl adw::subclass::navigation_page::NavigationPageImpl for HistoryPage {}
}

glib::wrapper! {
    pub struct HistoryPage(ObjectSubclass<imp::HistoryPage>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget ,adw::NavigationPage,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl Default for HistoryPage {
    fn default() -> Self {
        Self::new()
    }
}

impl HistoryPage {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn setup_history(&self) {
        let imp = self.imp();
        let spinner = imp.spinner.get();
        let historyrevealer = imp.historyrevealer.get();
        spinner.set_visible(true);
        fix(imp.hisscrolled.get());
        let (sender, receiver) = async_channel::bounded::<Vec<crate::ui::network::Resume>>(1);
        crate::ui::network::RUNTIME.spawn(glib::clone!(@strong sender => async move {
            let history_results = crate::ui::network::resume().await.unwrap_or_else(|e| {
                eprintln!("Error: {}", e);
                Vec::<crate::ui::network::Resume>::new()
            });
            sender.send(history_results).await.expect("history results not received.");
        }));
        let store = gio::ListStore::new::<glib::BoxedAnyObject>();
        glib::spawn_future_local(glib::clone!(@weak store=> async move {
            while let Ok(history_results) = receiver.recv().await {
                for result in history_results {
                    let object = glib::BoxedAnyObject::new(result);
                    store.append(&object);
                }
                spinner.set_visible(false);
                historyrevealer.set_reveal_child(true);
            }
        }));
        imp.selection.set_autoselect(false);
        imp.selection.set_model(Some(&store));
        let factory = gtk::SignalListItemFactory::new();
        factory.connect_bind(move |_factory, item| {
            let listitem = item.downcast_ref::<gtk::ListItem>().unwrap();
            let entry = listitem
                .item()
                .and_downcast::<glib::BoxedAnyObject>()
                .unwrap();
            let result: std::cell::Ref<crate::ui::network::Resume> = entry.borrow();
            let vbox = gtk::Box::new(gtk::Orientation::Vertical, 5);
            let overlay = gtk::Overlay::new();
            let imgbox;
            if result.parent_thumb_item_id.is_some() && result.resume_type == "Episode" {
                imgbox = crate::ui::image::setthumbimage(
                    result.parent_thumb_item_id.as_ref().expect("").clone(),
                );
            } else if result.resume_type == "Movie" {
                imgbox = crate::ui::image::setbackdropimage(result.id.clone(), 0);
            } else if result.parent_thumb_item_id.is_some() {
                imgbox = crate::ui::image::setthumbimage(
                    result.series_id.as_ref().expect("").to_string(),
                );
            } else {
                imgbox = crate::ui::image::setimage(result.id.clone());
            }
            imgbox.set_size_request(250, 141);
            overlay.set_child(Some(&imgbox));
            let progressbar = gtk::ProgressBar::new();
            progressbar.set_valign(gtk::Align::End);
            if let Some(userdata) = &result.user_data {
                if let Some(percentage) = userdata.played_percentage {
                    progressbar.set_fraction(percentage / 100.0);
                }
                if userdata.played {
                    let mark = gtk::Image::from_icon_name("object-select-symbolic");
                    mark.set_halign(gtk::Align::End);
                    mark.set_valign(gtk::Align::Start);
                    mark.set_height_request(25);
                    mark.set_width_request(25);
                    overlay.add_overlay(&mark);
                }
            }
            overlay.add_overlay(&progressbar);
            vbox.append(&overlay);
            let label = gtk::Label::builder().label(&result.name).build();
            let labeltype = gtk::Label::new(Some(&result.resume_type));
            if result.resume_type == "Episode" {
                let markup = result.series_name.as_ref().expect("").clone().to_string();
                label.set_markup(markup.as_str());
                let markup = format!(
                    "<span color='lightgray' font='small'>S{}E{}: {}</span>",
                    result.parent_index_number.as_ref().expect("").clone(),
                    result.index_number.as_ref().expect("").clone(),
                    result.name
                );
                labeltype.set_markup(markup.as_str());
            } else {
                let markup = result.name.to_string();
                label.set_markup(markup.as_str());
                let markup = format!(
                    "<span color='lightgray' font='small'>{}</span>",
                    result.resume_type
                );
                labeltype.set_markup(markup.as_str());
            }
            label.set_wrap(true);
            label.set_size_request(-1, 5);
            label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            labeltype.set_ellipsize(gtk::pango::EllipsizeMode::End);
            label.set_size_request(-1, 5);
            vbox.append(&label);
            vbox.append(&labeltype);
            listitem.set_child(Some(&vbox));
        });
        factory.connect_unbind(|_, item| {
            let listitem = item.downcast_ref::<gtk::ListItem>().unwrap();
            listitem.set_child(None::<&gtk::Widget>);
        });
        imp.historylist.set_factory(Some(&factory));
        imp.historylist.set_model(Some(&imp.selection));
        imp.historylist.connect_activate(glib::clone!(@weak self as obj => move |gridview, position| {
                let model = gridview.model().unwrap();
                let item = model.item(position).and_downcast::<glib::BoxedAnyObject>().unwrap();
                let result: std::cell::Ref<crate::ui::network::Resume> = item.borrow();
                let window = obj.root().and_downcast::<super::window::Window>().unwrap();
                if result.resume_type == "Movie" {
                    let item_page = MoviePage::new(result.id.clone(),result.name.clone());
                    item_page.set_tag(Some(&result.name));
                    window.imp().historyview.push(&item_page);
                    window.change_pop_visibility();
                } else if result.parent_thumb_item_id.is_none() {
                    let item_page = ItemPage::new(result.series_id.as_ref().expect("msg").clone(),result.id.clone());
                    item_page.set_tag(Some(&result.name));
                    window.imp().historyview.push(&item_page);
                    window.change_pop_visibility();
                } else {
                    let item_page = ItemPage::new(result.parent_thumb_item_id.as_ref().expect("msg").clone(),result.id.clone());
                    item_page.set_tag(Some(&result.name));
                    window.imp().historyview.push(&item_page);
                    window.change_pop_visibility();
                }
                if let Some(seriesname) = &result.series_name {
                    window.set_title(seriesname);
                    std::env::set_var("HISTORY_TITLE", seriesname)
                } else {
                    window.set_title(&result.name);
                    std::env::set_var("HISTORY_TITLE", &result.name)
                }
            }));
    }

    pub fn set_lists(&self) {
        self.sets("Movie");
        self.sets("Series");
        self.sets("Episode");
        self.sets("People");
        self.sets("MusicAlbum");
    }

    pub fn sets(&self, types: &str) {
        let imp = self.imp();
        let store = gtk::gio::ListStore::new::<glib::BoxedAnyObject>();
        let factory = gtk::SignalListItemFactory::new();
        let media_type = types.to_string();
        factory.connect_setup(move |_, item| {
            let list_item = item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem");
            let listbox = gtk::Box::new(gtk::Orientation::Vertical, 5);
            let picture = if media_type == "Episode" {
                gtk::Box::builder()
                    .orientation(gtk::Orientation::Vertical)
                    .height_request(141)
                    .width_request(250)
                    .build()
            } else if media_type == "MusicAlbum" {
                gtk::Box::builder()
                    .orientation(gtk::Orientation::Vertical)
                    .height_request(220)
                    .width_request(220)
                    .build()
            } else {
                gtk::Box::builder()
                    .orientation(gtk::Orientation::Vertical)
                    .height_request(273)
                    .width_request(182)
                    .build()
            };
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
        let listtype = types.to_string();
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
            let item: std::cell::Ref<crate::ui::network::Item> = entry.borrow();
            if picture.is::<gtk::Box>() {
                if let Some(_revealer) = picture
                    .downcast_ref::<gtk::Box>()
                    .expect("Needs to be Box")
                    .first_child()
                {
                } else {
                    let img = crate::ui::image::setimage(item.id.clone());
                    let overlay = gtk::Overlay::builder().child(&img).build();
                    if let Some(userdata) = &item.user_data {
                        if let Some(percentage) = userdata.played_percentage {
                            let progressbar = gtk::ProgressBar::new();
                            progressbar.set_fraction(percentage / 100.0);
                            progressbar.set_valign(gtk::Align::End);
                            overlay.add_overlay(&progressbar);
                        }
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
                let mut str: String;
                if listtype == "Episode" {
                    str = item.series_name.as_ref().unwrap().to_string();
                    if let Some(season) = item.parent_index_number {
                        str.push_str(&format!("\nS{}", season));
                    }
                    if let Some(episode) = item.index_number {
                        str.push_str(&format!(":E{} - {}", episode, item.name));
                    }
                } else if listtype == "MusicAlbum" {
                    str = item.name.to_string();
                    if let Some(artist) = &item.album_artist {
                        str.push_str(&format!("\n{}", artist));
                    }
                } else {
                    str = item.name.to_string();
                    if let Some(productionyear) = item.production_year {
                        str.push_str(&format!("\n{}", productionyear));
                    }
                }
                label
                    .downcast_ref::<gtk::Label>()
                    .expect("Needs to be Label")
                    .set_text(&str);
            }
        });
        let list;
        let selection;
        let revealer;
        match types {
            "Movie" => {
                list = imp.movielist.get();
                selection = &imp.movieselection;
                revealer = imp.movierevealer.get();
                fix(imp.moviescrolled.get());
            }
            "Series" => {
                list = imp.serieslist.get();
                selection = &imp.seriesselection;
                revealer = imp.seriesrevealer.get();
                fix(imp.seriesscrolled.get());
            }
            "Episode" => {
                list = imp.episodelist.get();
                selection = &imp.episodeselection;
                revealer = imp.episoderevealer.get();
                fix(imp.episodescrolled.get());
            }
            "People" => {
                list = imp.peoplelist.get();
                selection = &imp.peopleselection;
                revealer = imp.peoplerevealer.get();
                fix(imp.peoplescrolled.get());
            }
            "MusicAlbum" => {
                list = imp.albumlist.get();
                selection = &imp.albumselection;
                revealer = imp.albumrevealer.get();
                fix(imp.albumscrolled.get());
            }
            _ => {
                list = imp.episodelist.get();
                selection = &imp.episodeselection;
                revealer = imp.episoderevealer.get();
                fix(imp.episodescrolled.get());
            }
        }
        list.set_factory(Some(&factory));
        selection.set_model(Some(&store));
        selection.set_autoselect(false);
        list.set_model(Some(selection));
        let media_type = types.to_string();
        let (sender, receiver) = async_channel::bounded::<Vec<crate::ui::network::Item>>(1);
        crate::ui::network::RUNTIME.spawn(async move {
            let item = crate::ui::network::like_item(&media_type.to_string())
                .await
                .expect("msg");
            sender.send(item).await.expect("msg");
        });
        glib::spawn_future_local(async move {
            while let Ok(items) = receiver.recv().await {
                let items_len = items.len();
                for item in items {
                    let object = glib::BoxedAnyObject::new(item);
                    store.append(&object);
                }
                if items_len != 0 {
                    revealer.set_reveal_child(true);
                }
            }
        });
        let types = types.to_string();
        list.connect_activate(
            glib::clone!(@weak self as obj =>move |listview, position| {
                let model = listview.model().unwrap();
                let item = model
                    .item(position)
                    .and_downcast::<glib::BoxedAnyObject>()
                    .unwrap();
                let recommend: std::cell::Ref<crate::ui::network::Item> = item.borrow();
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
                match types.as_str() {
                    "Movie" => {
                        let item_page = MoviePage::new(recommend.id.clone(),recommend.name.clone());
                        if view.find_page(recommend.name.as_str()).is_some() {
                            view.pop_to_tag(recommend.name.as_str());
                        } else {
                            item_page.set_tag(Some(recommend.name.as_str()));
                            view.push(&item_page);
                        }
                    }
                    "Series" => {
                        let item_page = ItemPage::new(recommend.id.clone(),recommend.id.clone());
                        if view.find_page(recommend.name.as_str()).is_some() {
                            view.pop_to_tag(recommend.name.as_str());
                        } else {
                            item_page.set_tag(Some(recommend.name.as_str()));
                            view.push(&item_page);
                        }
                    }
                    "Episode" => {
                        let item_page = ItemPage::new(recommend.series_id.clone().unwrap(),recommend.id.clone());
                        if view.find_page(recommend.name.as_str()).is_some() {
                            view.pop_to_tag(recommend.name.as_str());
                        } else {
                            item_page.set_tag(Some(recommend.name.as_str()));
                            view.push(&item_page);
                        }
                    }
                    "People" => {
                        let item_page = ActorPage::new(&recommend.id);
                        if view.find_page(recommend.name.as_str()).is_some() {
                            view.pop_to_tag(recommend.name.as_str());
                        } else {
                            item_page.set_tag(Some(recommend.name.as_str()));
                            view.push(&item_page);
                        }
                    }
                    _ => {
                    }
                }
            }),
        );
    }
}
