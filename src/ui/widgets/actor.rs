use crate::ui::image::setimage;
use adw::prelude::NavigationPageExt;
use glib::Object;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

use super::fix::fix;
use super::item::ItemPage;
use super::movie::MoviePage;

mod imp {
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::prelude::*;
    use gtk::{glib, CompositeTemplate};
    use std::cell::OnceCell;
    // Object holding the state
    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsukimi/actor.ui")]
    #[properties(wrapper_type = super::ActorPage)]
    pub struct ActorPage {
        #[property(get, set, construct_only)]
        pub id: OnceCell<String>,
        #[template_child]
        pub actorpicbox: TemplateChild<gtk::Box>,
        #[template_child]
        pub inscription: TemplateChild<gtk::Inscription>,
        #[template_child]
        pub inforevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub spinner: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub title: TemplateChild<gtk::Label>,
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
        pub linksrevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub linksscrolled: TemplateChild<gtk::ScrolledWindow>,
        pub movieselection: gtk::SingleSelection,
        pub seriesselection: gtk::SingleSelection,
        pub episodeselection: gtk::SingleSelection,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for ActorPage {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "ActorPage";
        type Type = super::ActorPage;
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
    impl ObjectImpl for ActorPage {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_pic();
            obj.get_item();
            obj.set_lists();
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
    pub fn new(id: &str) -> Self {
        Object::builder().property("id", id).build()
    }

    pub fn setup_pic(&self) {
        let imp = self.imp();
        let id = self.id();
        let pic = setimage(id);
        pic.set_size_request(218, 328);
        pic.set_halign(gtk::Align::Start);
        pic.set_valign(gtk::Align::Start);
        imp.actorpicbox.append(&pic);
    }

    pub fn get_item(&self) {
        let imp = self.imp();
        let id = self.id();
        let inscription = imp.inscription.get();
        let inforevealer = imp.inforevealer.get();
        let spinner = imp.spinner.get();
        let title = imp.title.get();
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
                    inscription.set_text(Some(&overview));
                }
                if let Some(links) = item.external_urls {
                    obj.setlinksscrolled(links);
                }
                title.set_text(&item.name);
                inforevealer.set_reveal_child(true);
                spinner.set_visible(false);
            }
        }));
    }

    pub fn set_lists(&self) {
        self.sets("Movie");
        self.sets("Series");
        self.sets("Episode");
    }

    pub fn sets(&self, types: &str) {
        let imp = self.imp();
        let id = self.id();
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
        crate::ui::network::runtime().spawn(async move {
            let item = crate::ui::network::person_item(&id, &media_type.to_string())
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
                    _ => {
                    }
                }
            }),
        );
    }

    pub fn setlinksscrolled(&self, links: Vec<crate::ui::network::Urls>) {
        let imp = self.imp();
        let linksscrolled = fix(imp.linksscrolled.get());
        let linksrevealer = imp.linksrevealer.get();
        if !links.is_empty() {
            linksrevealer.set_reveal_child(true);
        }
        let linkbox = gtk::Box::new(gtk::Orientation::Horizontal, 5);
        linkbox.add_css_class("flat");
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
}
