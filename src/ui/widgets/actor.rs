use crate::client::{network::*, structs::*};
use crate::ui::image::setimage;
use crate::utils::{get_data_with_cache, spawn};
use adw::prelude::NavigationPageExt;
use glib::Object;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

use super::fix::fix;
use super::item::ItemPage;
use super::movie::MoviePage;
use super::tu_list_item::tu_list_item_register;

mod imp {
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::prelude::*;
    use gtk::{glib, CompositeTemplate};
    use std::cell::OnceCell;

    use crate::utils::spawn_g_timeout;
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
            spawn_g_timeout(glib::clone!(@weak obj => async move {
                obj.setup_pic();
                obj.get_item().await;
                obj.set_lists().await;
            }));
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

    pub async fn get_item(&self) {
        let imp = self.imp();
        let id = self.id();
        let inscription = imp.inscription.get();
        let inforevealer = imp.inforevealer.get();
        let spinner = imp.spinner.get();
        let title = imp.title.get();
        let item = get_data_with_cache(id.to_string(), "item", async {
            get_item_overview(id).await
        })
        .await
        .unwrap();
        spawn(glib::clone!(@weak self as obj=>async move {
                if let Some(overview) = item.overview {
                    inscription.set_text(Some(&overview));
                }
                if let Some(links) = item.external_urls {
                    obj.setlinksscrolled(links);
                }
                title.set_text(&item.name);
                inforevealer.set_reveal_child(true);
                spinner.set_visible(false);
        }));
    }

    pub async fn set_lists(&self) {
        self.sets("Movie").await;
        self.sets("Series").await;
        self.sets("Episode").await;
    }

    pub async fn sets(&self, types: &str) {
        let imp = self.imp();
        let id = self.id();
        let store = gtk::gio::ListStore::new::<glib::BoxedAnyObject>();
        let factory = gtk::SignalListItemFactory::new();
        factory.connect_bind(move |_, item| {
            let list_item = item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem");
            let entry = item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem")
                .item()
                .and_downcast::<glib::BoxedAnyObject>()
                .expect("Needs to be BoxedAnyObject");
            let latest: std::cell::Ref<Latest> = entry.borrow();
            if list_item.child().is_none() {
                tu_list_item_register(&latest, list_item, &latest.latest_type)
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
        let items = get_data_with_cache(id.to_string(), &media_type.to_string(), async move {
            person_item(&id, &media_type).await
        })
        .await
        .unwrap();
        spawn(async move {
            if !items.is_empty() {
                revealer.set_reveal_child(true);
            }
            for item in items {
                let object = glib::BoxedAnyObject::new(item);
                store.append(&object);
                gtk::glib::timeout_future(std::time::Duration::from_millis(30)).await;
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
                let recommend: std::cell::Ref<Item> = item.borrow();
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

    pub fn setlinksscrolled(&self, links: Vec<Urls>) {
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
