use glib::Object;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

use self::imp::Page;

mod imp {

    use glib::subclass::InitializingObject;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use gtk::{gio, glib, CompositeTemplate};

    use crate::ui::widgets::item::ItemPage;
    use crate::ui::widgets::movie::MoviePage;

    pub enum Page {
        Movie(Box<gtk::Widget>),
        Item(Box<gtk::Widget>),
    }

    // Object holding the state
    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/moe/tsukimi/list.ui")]
    pub struct ListPage {
        #[template_child]
        pub listgrid: TemplateChild<gtk::GridView>,
        #[template_child]
        pub spinner: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub listscrolled: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub listrevealer: TemplateChild<gtk::Revealer>,
        pub selection: gtk::SingleSelection,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for ListPage {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "ListPage";
        type Type = super::ListPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // Trait shared by all GObjects
    impl ObjectImpl for ListPage {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            let spinner = self.spinner.get();
            let listrevealer = self.listrevealer.get();
            spinner.set_visible(true);
            let (sender, receiver) = async_channel::bounded::<Vec<crate::ui::network::Resume>>(1);
            crate::ui::network::runtime().spawn(glib::clone!(@strong sender => async move {
                let list_results = crate::ui::network::resume().await.unwrap_or_else(|e| {
                    eprintln!("Error: {}", e);
                    Vec::<crate::ui::network::Resume>::new()
                });
                sender.send(list_results).await.expect("list results not received.");
            }));
            let store = gio::ListStore::new::<glib::BoxedAnyObject>();
            glib::spawn_future_local(glib::clone!(@weak store=> async move {
                while let Ok(list_results) = receiver.recv().await {
                    for result in list_results {
                        let object = glib::BoxedAnyObject::new(result);
                        store.append(&object);
                    }
                    spinner.set_visible(false);
                    listrevealer.set_reveal_child(true);
                }
            }));

            self.selection.set_model(Some(&store));
            let factory = gtk::SignalListItemFactory::new();
            factory.connect_setup(move |_, item| {
                let list_item = item
                    .downcast_ref::<gtk::ListItem>()
                    .expect("Needs to be ListItem");
                let listbox = gtk::Box::new(gtk::Orientation::Vertical, 5);
                let picture = gtk::Box::builder()
                    .orientation(gtk::Orientation::Vertical)
                    .height_request(240)
                    .width_request(167)
                    .build();
                let label = gtk::Label::builder()
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
                let latest: std::cell::Ref<crate::ui::network::Latest> = entry.borrow();
                if latest.Type == "MusicAlbum" {
                    picture.set_size_request(167, 167);
                }
                if picture.is::<gtk::Box>() {
                    if let Some(_revealer) = picture
                        .downcast_ref::<gtk::Box>()
                        .expect("Needs to be Box")
                        .first_child()
                    {
                    } else {
                        let mutex = std::sync::Arc::new(tokio::sync::Mutex::new(()));
                        let img = crate::ui::image::setimage(latest.Id.clone(), mutex.clone());
                        let overlay = gtk::Overlay::builder()
                            .child(&img)
                            .build();
                        if let Some(userdata) = &latest.UserData {
                            if let Some(unplayeditemcount) = userdata.UnplayedItemCount {
                                if unplayeditemcount > 0 {
                                    let mark = gtk::Label::new(Some(&userdata.UnplayedItemCount.expect("no unplayeditemcount").to_string()));
                                    mark.set_valign(gtk::Align::Start);
                                    mark.set_halign(gtk::Align::End);
                                    mark.set_height_request(40);
                                    mark.set_width_request(40);
                                    overlay.add_overlay(&mark);
                                }
                            }
                        }
                        picture
                            .downcast_ref::<gtk::Box>()
                            .expect("Needs to be Box")
                            .append(&overlay);
                    }
                }
                if label.is::<gtk::Label>() {
                    let mut str = format!("{}", latest.Name);
                    if let Some(productionyear) = latest.ProductionYear {
                        str.push_str(&format!("\n{}", productionyear));
                    }
                    label
                        .downcast_ref::<gtk::Label>()
                        .expect("Needs to be Label")
                        .set_text(&str);
                }
            });
            self.listgrid.set_factory(Some(&factory));
            self.listgrid.set_model(Some(&self.selection));
            self.listgrid.connect_activate(glib::clone!(@weak obj => move |gridview, position| {
                let model = gridview.model().unwrap();
                let item = model.item(position).and_downcast::<glib::BoxedAnyObject>().unwrap();
                let result: std::cell::Ref<crate::ui::network::Resume> = item.borrow();
                let item_page;
                if result.Type == "Movie" {
                    item_page = Page::Movie(Box::new(MoviePage::new(result.Id.clone(),result.Name.clone()).into()));
                } else {
                    if result.ParentThumbItemId == None {
                        item_page = Page::Item(Box::new(ItemPage::new(result.SeriesId.as_ref().expect("msg").clone(),result.Id.clone()).into()));
                    } else {
                        item_page = Page::Item(Box::new(ItemPage::new(result.ParentThumbItemId.as_ref().expect("msg").clone(),result.Id.clone()).into()));
                    }
                }
                obj.set(item_page);
            }));
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for ListPage {}

    // Trait shared by all windows
    impl WindowImpl for ListPage {}

    // Trait shared by all application windows
    impl ApplicationWindowImpl for ListPage {}

    impl adw::subclass::navigation_page::NavigationPageImpl for ListPage {}
}

glib::wrapper! {
    pub struct ListPage(ObjectSubclass<imp::ListPage>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget ,adw::NavigationPage,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl Default for ListPage {
    fn default() -> Self {
        Self::new()
    }
}

impl ListPage {
    pub fn new() -> Self {
        Object::builder().build()
    }

    fn set(&self, page: Page) {
        let imp = imp::ListPage::from_obj(self);
        let widget = match page {
            Page::Movie(widget) => widget,
            Page::Item(widget) => widget,
        };
        imp.listscrolled.set_child(Some(&*widget));
    }
}
