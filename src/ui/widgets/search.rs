use std::env;

use crate::client::{network::*, structs::*};
use crate::ui::widgets::item::ItemPage;
use crate::ui::widgets::movie::MoviePage;
use crate::ui::widgets::window::Window;
use crate::utils::{spawn_tokio, tu_list_item_factory, tu_list_view_connect_activate};
use adw::prelude::NavigationPageExt;
use glib::Object;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

mod imp {

    use glib::subclass::InitializingObject;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate};

    // Object holding the state
    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/moe/tsukimi/search.ui")]
    pub struct SearchPage {
        #[template_child]
        pub searchentry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub searchgrid: TemplateChild<gtk::GridView>,
        #[template_child]
        pub spinner: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub searchscrolled: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub searchrevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub recommendbox: TemplateChild<gtk::Box>,
        #[template_child]
        pub movie: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub series: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub boxset: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub person: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub music: TemplateChild<gtk::ToggleButton>,
        pub selection: gtk::SingleSelection,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for SearchPage {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "SearchPage";
        type Type = super::SearchPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // Trait shared by all GObjects
    impl ObjectImpl for SearchPage {
        fn constructed(&self) {
            let obj = self.obj();
            self.parent_constructed();
            obj.setup_recommend();
            obj.setup_search();
            obj.search();
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for SearchPage {}

    // Trait shared by all windows
    impl WindowImpl for SearchPage {}

    // Trait shared by all application windows
    impl ApplicationWindowImpl for SearchPage {}

    impl adw::subclass::navigation_page::NavigationPageImpl for SearchPage {}
}

glib::wrapper! {
    pub struct SearchPage(ObjectSubclass<imp::SearchPage>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget ,adw::NavigationPage,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl Default for SearchPage {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchPage {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn setup_recommend(&self) {
        let (sender, receiver) = async_channel::bounded::<List>(1);
        RUNTIME.spawn(async move {
            let list = get_search_recommend().await.expect("msg");
            sender
                .send(list)
                .await
                .expect("The channel needs to be open.");
        });
        glib::spawn_future_local(glib::clone!(@weak self as obj =>async move {
            while let Ok(list) = receiver.recv().await {
                let imp = obj.imp();
                let recommendbox = imp.recommendbox.get();
                for item in list.items {
                    let button = gtk::Button::new();
                    let buttoncontent = adw::ButtonContent::builder()
                            .label(&item.name)
                            .icon_name(if item.latest_type == "Movie" {
                                "video-display-symbolic"
                            } else {
                                "video-x-generic"
                            })
                            .build();
                    button.set_halign(gtk::Align::Center);
                    button.set_child(Some(&buttoncontent));
                    button.connect_clicked(glib::clone!(@weak obj => move |_| {
                        let window = obj.root().and_downcast::<Window>().unwrap();
                        if item.latest_type == "Movie" {
                            let item_page = MoviePage::new(item.id.clone(),item.name.clone());
                            item_page.set_tag(Some(&item.name));
                            window.imp().searchview.push(&item_page);
                            window.set_title(&item.name);
                            window.change_pop_visibility();
                            env::set_var("HOME_TITLE", &item.name)
                        } else if item.latest_type == "Series" {
                            let item_page = ItemPage::new(item.id.clone(),item.id.clone());
                            item_page.set_tag(Some(&item.name));
                            window.imp().searchview.push(&item_page);
                            window.set_title(&item.name);
                            window.change_pop_visibility();
                            env::set_var("HOME_TITLE", &item.name)
                        }
                    }));
                    recommendbox.append(&button);
                }
            }
        }));
    }

    pub fn setup_search(&self) {
        let imp = self.imp();

        let store = gio::ListStore::new::<glib::BoxedAnyObject>();
        imp.selection.set_model(Some(&store));
        let factory = tu_list_item_factory("".to_string());
        imp.searchgrid.set_factory(Some(&factory));
        imp.searchgrid.set_model(Some(&imp.selection));
        imp.searchgrid.set_min_columns(1);
        imp.searchgrid.set_max_columns(15);
        imp.searchgrid.connect_activate(
            glib::clone!(@weak self as obj => move |listview, position| {
                    let window = obj.root().and_downcast::<Window>().unwrap();
                    let model = listview.model().unwrap();
                    let item = model.item(position).and_downcast::<glib::BoxedAnyObject>().unwrap();
                    let result: std::cell::Ref<SimpleListItem> = item.borrow();
                    tu_list_view_connect_activate(window, &result, None);
            }),
        );
    }

    pub fn search(&self) {
        let imp = self.imp();
        let spinner = imp.spinner.get();
        let searchrevealer = imp.searchrevealer.get();
        let recommendbox = imp.recommendbox.get();
        let store = imp
            .selection
            .model()
            .unwrap()
            .downcast::<gio::ListStore>()
            .unwrap();
        imp.searchentry.connect_activate(
            glib::clone!(@weak spinner,@weak imp => move |entry| {
                spinner.set_visible(true);
                recommendbox.set_visible(false);
                let search_content = entry.text().to_string();
                let search_filter = {
                    let mut filter = Vec::new();
                    if imp.movie.is_active() {
                        filter.push("Movie");
                    }
                    if imp.series.is_active() {
                        filter.push("Series");
                    }
                    if imp.boxset.is_active() {
                        filter.push("BoxSet");
                    }
                    if imp.person.is_active() {
                        filter.push("Person");
                    }
                    if imp.music.is_active() {
                        filter.push("MusicAlbum");
                    }
                    filter
                };

                glib::spawn_future_local(glib::clone!(@weak store, @weak searchrevealer=> async move {
                    let search_results = spawn_tokio(async move {
                        search(search_content,&search_filter).await
                    }).await.unwrap();
                    spinner.set_visible(false);
                    store.remove_all();
                    searchrevealer.set_reveal_child(true);
                    for result in search_results {
                        let object = glib::BoxedAnyObject::new(result);
                        store.append(&object);
                    }
                }));
            }),
        );
    }
}
