use crate::client::{network::*, structs::*};
use crate::fraction;
use crate::utils::{get_data_with_cache, spawn, tu_list_view_connect_activate};
use glib::Object;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

use super::tu_list_item::tu_list_item_register;
use crate::ui::widgets::fix::ScrolledWindowFixExt;
mod imp {
    use glib::subclass::InitializingObject;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate};

    use crate::utils::spawn;

    // Object holding the state
    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/moe/tsukimi/history.ui")]
    pub struct HistoryPage {
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
            spawn(glib::clone!(@weak obj =>async move {
                obj.set_lists().await;
            }));
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

    pub async fn set_lists(&self) {
        self.sets("Movie").await;
        self.sets("Series").await;
        self.sets("Episode").await;
        self.sets("People").await;
        self.sets("MusicAlbum").await;
        fraction!(self);
    }

    pub async fn sets(&self, types: &str) {
        let imp = self.imp();
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
            let latest: std::cell::Ref<SimpleListItem> = entry.borrow();
            if list_item.child().is_none() {
                tu_list_item_register(&latest, list_item, "latest")
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
                imp.moviescrolled.fix();
            }
            "Series" => {
                list = imp.serieslist.get();
                selection = &imp.seriesselection;
                revealer = imp.seriesrevealer.get();
                imp.seriesscrolled.fix();
            }
            "Episode" => {
                list = imp.episodelist.get();
                selection = &imp.episodeselection;
                revealer = imp.episoderevealer.get();
                imp.episodescrolled.fix();
            }
            "People" => {
                list = imp.peoplelist.get();
                selection = &imp.peopleselection;
                revealer = imp.peoplerevealer.get();
                imp.peoplescrolled.fix();
            }
            "MusicAlbum" => {
                list = imp.albumlist.get();
                selection = &imp.albumselection;
                revealer = imp.albumrevealer.get();
                imp.albumscrolled.fix();
            }
            _ => {
                list = imp.episodelist.get();
                selection = &imp.episodeselection;
                revealer = imp.episoderevealer.get();
                imp.episodescrolled.fix();
            }
        }
        list.set_factory(Some(&factory));
        selection.set_model(Some(&store));
        selection.set_autoselect(false);
        list.set_model(Some(selection));
        let media_type = types.to_string();

        let items = get_data_with_cache("0".to_string(), &media_type.to_string(), async move {
            like_item(&media_type.to_string()).await
        })
        .await
        .unwrap_or_else(|_| Vec::new());

        spawn(async move {
            let items_len = items.len();
            for item in items {
                let object = glib::BoxedAnyObject::new(item);
                store.append(&object);
            }
            if items_len != 0 {
                revealer.set_reveal_child(true);
            }
        });
        list.connect_activate(glib::clone!(@weak self as obj =>move |listview, position| {
            let model = listview.model().unwrap();
            let item = model
                .item(position)
                .and_downcast::<glib::BoxedAnyObject>()
                .unwrap();
            let recommend: std::cell::Ref<SimpleListItem> = item.borrow();
            let window = obj.root().and_downcast::<super::window::Window>().unwrap();
            tu_list_view_connect_activate(window, &recommend, None);
        }));
    }
}
