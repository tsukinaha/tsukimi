use glib::Object;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};
mod imp {
    use std::cell::RefCell;

    use glib::subclass::InitializingObject;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use gtk::{gio, glib, CompositeTemplate, Entry, Label, Picture};

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
            let spinner = self.spinner.get();
            let (sender, receiver) = async_channel::bounded::<Vec<crate::ui::network::SearchResult>>(1);
            self.searchentry.connect_activate(glib::clone!(@strong sender,@weak spinner=> move |entry| {
                spinner.set_visible(true);
                let search_content = entry.text().to_string();
                crate::ui::network::runtime().spawn(glib::clone!(@strong sender => async move {
                    let search_results = crate::ui::network::search(search_content).await.unwrap_or_else(|e| {
                        eprintln!("Error: {}", e);
                        Vec::<crate::ui::network::SearchResult>::new()
                    });
                    sender.send(search_results).await.expect("search results not received.");
                }));
            }));
            
            let store = gio::ListStore::new::<glib::BoxedAnyObject>();
            glib::spawn_future_local(glib::clone!(@weak store=> async move {
                while let Ok(search_results) = receiver.recv().await {
                    spinner.set_visible(false);
                    store.remove_all();
                    for result in search_results {
                        if result.Type == "Series" || result.Type == "Movie" {
                            let object = glib::BoxedAnyObject::new(result);
                            store.append(&object);
                        }
                    }
                }
            }));

            self.selection.set_model(Some(&store));
            let factory = gtk::SignalListItemFactory::new();
            factory.connect_bind(|_, item| {
                let listitem = item.downcast_ref::<gtk::ListItem>().unwrap();
                let entry = listitem
                    .item()
                    .and_downcast::<glib::BoxedAnyObject>()
                    .unwrap();
                let result: std::cell::Ref<crate::ui::network::SearchResult> = entry.borrow();
                let vbox = gtk::Box::new(gtk::Orientation::Vertical, 2);
                let overlay = gtk::Overlay::new();
                let imgbox = crate::ui::image::set_image(result.Id.clone());
                imgbox.set_size_request(167, 275);
                overlay.set_child(Some(&imgbox));
                overlay.set_size_request(167, 275);
                vbox.append(&overlay);
                let label = Label::new(Some(&result.Name));
                let markup = format!("{}", result.Name);
                label.set_markup(markup.as_str());
                label.set_wrap(true);
                label.set_size_request(-1, 24);
                label.set_ellipsize(gtk::pango::EllipsizeMode::End);
                let labeltype = Label::new(Some(&result.Type));
                let markup = format!("<span color='lightgray' font='8'>{}</span>", result.Type);
                labeltype.set_markup(markup.as_str());
                labeltype.set_size_request(-1, 24);
                vbox.append(&label);
                vbox.append(&labeltype);
                listitem.set_child(Some(&vbox));
            });
            self.searchgrid.set_factory(Some(&factory));
            self.searchgrid.set_model(Some(&self.selection));
            self.searchgrid.connect_activate(glib::clone!(@weak obj => move |gridview, position| {
                let model = gridview.model().unwrap();
                let item = model.item(position).and_downcast::<glib::BoxedAnyObject>().unwrap();
                let result: std::cell::Ref<crate::ui::network::SearchResult> = item.borrow();
                let item_page;
                if result.Type == "Movie" {
                    item_page = crate::ui::movie_page::movie_page(result);
                } else {
                    item_page = crate::ui::item_page::itempage(result);
                }
                obj.set(item_page);
            }));
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

    fn set(&self, item_page: gtk::Box) {
        let imp = imp::SearchPage::from_obj(self);
        imp.searchscrolled.set_child(Some(&item_page));
    }
}
