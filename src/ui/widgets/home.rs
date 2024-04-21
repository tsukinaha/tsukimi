use adw::prelude::NavigationPageExt;
use glib::Object;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};
use std::env;

use crate::config::get_cache_dir;
use crate::ui::network::Latest;

use super::{fix::fix, item::ItemPage, list::ListPage, movie::MoviePage, window::Window};

mod imp {

    use glib::subclass::InitializingObject;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate};

    use crate::utils::spawn_g_timeout;
    // Object holding the state
    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/moe/tsukimi/home.ui")]
    pub struct HomePage {
        #[template_child]
        pub root: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub libscrolled: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub librevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub liblist: TemplateChild<gtk::ListView>,
        #[template_child]
        pub libsbox: TemplateChild<gtk::Box>,
        #[template_child]
        pub toast: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub libsrevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub spinner: TemplateChild<gtk::Spinner>,
        pub selection: gtk::SingleSelection,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for HomePage {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "HomePage";
        type Type = super::HomePage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // Trait shared by all GObjects
    impl ObjectImpl for HomePage {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            spawn_g_timeout(glib::clone!(@weak obj => async move {
                obj.set_library();
            }));
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for HomePage {}

    // Trait shared by all windows
    impl WindowImpl for HomePage {}

    // Trait shared by all application windows
    impl ApplicationWindowImpl for HomePage {}

    impl adw::subclass::navigation_page::NavigationPageImpl for HomePage {}
}

glib::wrapper! {
    pub struct HomePage(ObjectSubclass<imp::HomePage>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget ,adw::NavigationPage,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl Default for HomePage {
    fn default() -> Self {
        Self::new()
    }
}

impl HomePage {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn set_library(&self) {
        self.set_libraryscorll();
    }

    pub fn set_libraryscorll(&self) {
        let imp = self.imp();
        let libscrolled = fix(imp.libscrolled.get());
        imp.librevealer.set_reveal_child(true);
        let store = gtk::gio::ListStore::new::<glib::BoxedAnyObject>();
        imp.selection.set_autoselect(false);
        imp.selection.set_model(Some(&store));
        let selection = &imp.selection;
        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_, item| {
            let list_item = item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem");
            let listbox = gtk::Box::new(gtk::Orientation::Vertical, 5);
            let picture = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .height_request(150)
                .width_request(300)
                .build();
            let label = gtk::Label::builder()
                .halign(gtk::Align::Center)
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
            let view: std::cell::Ref<crate::ui::network::View> = entry.borrow();
            if picture.is::<gtk::Box>() {
                if let Some(_revealer) = picture
                    .downcast_ref::<gtk::Box>()
                    .expect("Needs to be Box")
                    .first_child()
                {
                } else {
                    let img = crate::ui::image::setimage(view.id.clone());
                    picture
                        .downcast_ref::<gtk::Box>()
                        .expect("Needs to be Box")
                        .append(&img);
                }
            }
            if label.is::<gtk::Label>() {
                let str = view.name.to_string();
                label
                    .downcast_ref::<gtk::Label>()
                    .expect("Needs to be Label")
                    .set_text(&str);
            }
        });
        imp.liblist.set_factory(Some(&factory));
        imp.liblist.set_model(Some(selection));
        let liblist = imp.liblist.get();
        liblist.connect_activate(
            glib::clone!(@weak self as obj => move |listview, position| {
                let model = listview.model().unwrap();
                let item = model.item(position).and_downcast::<glib::BoxedAnyObject>().unwrap();
                let view: std::cell::Ref<crate::ui::network::View> = item.borrow();
                let item_page = ListPage::new(view.id.clone());
                item_page.set_tag(Some(&view.name));
                let window = obj.root().and_downcast::<Window>().unwrap();
                window.imp().homeview.push(&item_page);
                window.set_title(&view.name);
                window.change_pop_visibility();
                env::set_var("HOME_TITLE", &view.name)
            }),
        );
        libscrolled.set_child(Some(&liblist));
        let pathbuf = get_cache_dir(env::var("EMBY_NAME").unwrap()).join("views.json");
        if pathbuf.exists() {
            let data = std::fs::read_to_string(&pathbuf).expect("Unable to read file");
            let views: Vec<crate::ui::network::View> =
                serde_json::from_str(&data).expect("JSON was not well-formatted");
            for view in &views {
                let object = glib::BoxedAnyObject::new(view.clone());
                store.append(&object);
            }
            self.get_librarysscroll(&views);
        } else {
            let (sender, receiver) = async_channel::bounded::<Vec<crate::ui::network::View>>(3);
            crate::ui::network::RUNTIME.spawn(async move {
                let views = crate::ui::network::get_library().await.expect("msg");
                sender.send(views).await.expect("msg");
            });
            glib::spawn_future_local(glib::clone!(@weak self as obj =>async move {
                while let Ok(views) = receiver.recv().await {
                    for view in &views {
                        let object = glib::BoxedAnyObject::new(view.clone());
                        store.append(&object);
                    }
                    obj.get_librarysscroll(&views);
                }
            }));
        }
    }

    pub fn get_librarysscroll(&self, views: &[crate::ui::network::View]) {
        let libsrevealer = self.imp().libsrevealer.get();
        libsrevealer.set_reveal_child(true);
        let libsbox = self.imp().libsbox.get();
        for _ in 0..libsbox.observe_children().n_items() {
            libsbox.remove(&libsbox.last_child().unwrap());
        }
        for view in views.iter().cloned() {
            let libsbox = self.imp().libsbox.get();
            let scrolledwindow = gtk::ScrolledWindow::builder()
                .hscrollbar_policy(gtk::PolicyType::Automatic)
                .vscrollbar_policy(gtk::PolicyType::Never)
                .overlay_scrolling(true)
                .build();
            let scrolledwindow = fix(scrolledwindow);
            let scrollbox = gtk::Box::new(gtk::Orientation::Vertical, 15);
            let revealer = gtk::Revealer::builder()
                .reveal_child(false)
                .child(&scrollbox)
                .build();
            libsbox.append(&revealer);
            let view_name = view.name.replace('&', "&amp;");
            let label = gtk::Label::builder()
                .label(format!("<b>Latest {}</b>", view_name))
                .halign(gtk::Align::Start)
                .use_markup(true)
                .margin_top(15)
                .margin_start(10)
                .build();
            scrollbox.append(&label);
            scrollbox.append(&scrolledwindow);

            let pathbuf = get_cache_dir(env::var("EMBY_NAME").unwrap())
                .join(format!("latest_{}.json", &view.id));
            if pathbuf.exists() {
                let data = std::fs::read_to_string(&pathbuf).expect("Unable to read file");
                let latest: Vec<crate::ui::network::Latest> =
                    serde_json::from_str(&data).expect("JSON was not well-formatted");
                self.set_librarysscroll(latest.clone());
                let listview = self.set_librarysscroll(latest);
                scrolledwindow.set_child(Some(&listview));
                revealer.set_reveal_child(true);
            }

            let (sender, receiver) = async_channel::bounded::<Vec<crate::ui::network::Latest>>(3);
            crate::ui::network::RUNTIME.spawn(async move {
                let latest = crate::ui::network::get_latest(view.id.clone())
                    .await
                    .expect("msg");
                sender.send(latest).await.expect("msg");
            });
            glib::spawn_future_local(glib::clone!(@weak self as obj =>async move {
                while let Ok(latest) = receiver.recv().await {
                    obj.set_librarysscroll(latest.clone());
                    let listview = obj.set_librarysscroll(latest);
                    scrolledwindow.set_child(Some(&listview));
                    if !revealer.reveals_child() {
                        revealer.set_reveal_child(true);
                    }
                }
            }));
        }
        self.imp().spinner.set_visible(false);
    }

    pub fn set_librarysscroll(&self, latests: Vec<crate::ui::network::Latest>) -> gtk::ListView {
        let store = gtk::gio::ListStore::new::<glib::BoxedAnyObject>();
        for latest in latests {
            let object = glib::BoxedAnyObject::new(latest.clone());
            store.append(&object);
        }
        let selection = gtk::SingleSelection::builder()
            .model(&store)
            .autoselect(false)
            .build();
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
                .valign(gtk::Align::Start)
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
            if latest.latest_type == "MusicAlbum" {
                picture.set_size_request(210, 210);
                picture.set_valign(gtk::Align::Center);
            }
            if picture.is::<gtk::Box>() {
                if let Some(_revealer) = picture
                    .downcast_ref::<gtk::Box>()
                    .expect("Needs to be Box")
                    .first_child()
                {
                } else {
                    let img = crate::ui::image::setimage(latest.id.clone());
                    let overlay = gtk::Overlay::builder().child(&img).build();
                    if let Some(userdata) = &latest.user_data {
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
                let mut str = latest.name.to_string();
                if let Some(productionyear) = latest.production_year {
                    str.push_str(&format!("\n{}", productionyear));
                }
                label
                    .downcast_ref::<gtk::Label>()
                    .expect("Needs to be Label")
                    .set_text(&str);
            }
        });
        let listview = gtk::ListView::new(Some(selection), Some(factory));
        listview.set_orientation(gtk::Orientation::Horizontal);
        listview.connect_activate(
            glib::clone!(@weak self as obj => move |listview, position| {
                    let window = obj.root().and_downcast::<Window>().unwrap();
                    let model = listview.model().unwrap();
                    let item = model.item(position).and_downcast::<glib::BoxedAnyObject>().unwrap();
                    let result: std::cell::Ref<Latest> = item.borrow();
                    if result.latest_type == "Movie" {
                        let item_page = MoviePage::new(result.id.clone(),result.name.clone());
                        item_page.set_tag(Some(&result.name));
                        window.imp().homeview.push(&item_page);
                        window.set_title(&result.name);
                        window.change_pop_visibility();
                        env::set_var("HOME_TITLE", &result.name)
                    } else if result.latest_type == "Series" {
                        let item_page = ItemPage::new(result.id.clone(),result.id.clone());
                        item_page.set_tag(Some(&result.name));
                        window.imp().homeview.push(&item_page);
                        window.set_title(&result.name);
                        window.change_pop_visibility();
                        env::set_var("HOME_TITLE", &result.name)
                    } else {
                        let toast = adw::Toast::builder()
                            .title(format!("{} is not supported",result.latest_type))
                            .timeout(3)
                            .build();
                        obj.imp().toast.add_toast(toast);
                    }
            }),
        );
        listview
    }
}
