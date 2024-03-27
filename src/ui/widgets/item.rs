use glib::Object;
use gtk::prelude::*;
use gtk::{gio, glib};
mod imp {
    use crate::ui::network::{self, runtime};
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::prelude::*;
    use gtk::{glib, CompositeTemplate};
    use std::cell::{OnceCell, Ref};
    use std::collections::{HashMap, HashSet};
    use std::path::PathBuf;
    // Object holding the state
    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsukimi/item.ui")]
    #[properties(wrapper_type = super::ItemPage)]
    pub struct ItemPage {
        #[property(get, set, construct_only)]
        pub id: OnceCell<String>,
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
        pub selection: gtk::SingleSelection,
        pub seasonselection: gtk::SingleSelection,
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
            let itemrevealer = self.itemrevealer.get();
            let id = obj.id();
            let path = format!(
                "{}/.local/share/tsukimi/b{}.png",
                dirs::home_dir().expect("msg").display(),
                id
            );
            let pathbuf = PathBuf::from(&path);
            let backdrop = self.backdrop.get();
            let (sender, receiver) = async_channel::bounded::<String>(1);
            let idclone = id.clone();
            if pathbuf.exists() {
                backdrop.set_file(Some(&gtk::gio::File::for_path(&path)));
            } else {
                crate::ui::network::runtime().spawn(async move {
                    let id = crate::ui::network::get_backdropimage(idclone)
                        .await
                        .expect("msg");
                    sender
                        .send(id.clone())
                        .await
                        .expect("The channel needs to be open.");
                });
            }

            let idclone = id.clone();

            glib::spawn_future_local(async move {
                while let Ok(_) = receiver.recv().await {
                    let path = format!(
                        "{}/.local/share/tsukimi/b{}.png",
                        dirs::home_dir().expect("msg").display(),
                        idclone
                    );
                    let file = gtk::gio::File::for_path(&path);
                    backdrop.set_file(Some(&file));
                }
            });

            let store = gtk::gio::ListStore::new::<glib::BoxedAnyObject>();
            
            self.selection.set_model(Some(&store));

            let (sender, receiver) = async_channel::bounded::<Vec<network::SeriesInfo>>(1);
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
            self.seasonselection.set_model(Some(&seasonstore));
            let seasonlist = self.seasonlist.get();
            seasonlist.set_model(Some(&self.seasonselection));
            glib::spawn_future_local(async move {
                let series_info = receiver.recv().await.expect("series_info not received.");
                let mut season_set: HashSet<u32> = HashSet::new();
                let mut season_map: HashMap<String, u32> = HashMap::new();
                let mut position = 0;
                let mut _infor = 0;
                for info in &series_info {
                    if !season_set.contains(&info.ParentIndexNumber) {
                        let seasonstring = format!("Season {}", info.ParentIndexNumber);
                        seasonstore.append(&seasonstring);
                        season_set.insert(info.ParentIndexNumber);
                        season_map.insert(seasonstring.clone(), info.ParentIndexNumber);
                        if _infor <= 1 {
                            if info.ParentIndexNumber < 1 {
                                position += 1;
                            }
                        }
                        _infor += 1;
                    }
                }
                for info in &series_info {
                    if info.ParentIndexNumber == 1 {
                        let object = glib::BoxedAnyObject::new(info.clone());
                        store.append(&object);
                    }
                }
                seasonlist.set_selected(position);
                seasonlist.connect_selected_item_notify(move |dropdown| {
                    let selected = dropdown.selected_item();
                    let selected = selected.and_downcast_ref::<gtk::StringObject>().unwrap();
                    let selected = selected.string().to_string();
                    store.remove_all();
                    let season_number = season_map[&selected];
                    for info in &series_info {
                        if info.ParentIndexNumber == season_number {
                            let object = glib::BoxedAnyObject::new(info.clone());
                            store.append(&object);
                        }
                    }
                });
                itemrevealer.set_reveal_child(true);
            });

            let factory = gtk::SignalListItemFactory::new();
            factory.connect_bind(|_, item| {
                let listitem = item.downcast_ref::<gtk::ListItem>().unwrap();
                let entry = listitem
                    .item()
                    .and_downcast::<glib::BoxedAnyObject>()
                    .unwrap();
                let seriesinfo: Ref<network::SeriesInfo> = entry.borrow();
                let vbox = gtk::Box::new(gtk::Orientation::Vertical, 5);
                let label = gtk::Label::new(Some(&seriesinfo.Name));
                label.set_halign(gtk::Align::Start);
                let markup = format!("{}. {}", seriesinfo.IndexNumber, seriesinfo.Name);
                label.set_markup(markup.as_str());
                label.set_ellipsize(gtk::pango::EllipsizeMode::End);
                label.set_size_request(-1, 20);
                label.set_valign(gtk::Align::Start);
                let mutex = std::sync::Arc::new(tokio::sync::Mutex::new(()));
                let img = crate::ui::image::setimage(seriesinfo.Id.clone(), mutex.clone());
                img.set_size_request(250, 141);
                vbox.append(&img);
                vbox.append(&label);
                vbox.set_valign(gtk::Align::Start);
                vbox.set_size_request(250, 150);
                listitem.set_child(Some(&vbox));
            });
            factory.connect_unbind(|_, item| {
                let listitem = item.downcast_ref::<gtk::ListItem>().unwrap();
                listitem.set_child(None::<&gtk::Widget>);
            });
            self.itemlist.set_factory(Some(&factory));
            self.itemlist.set_model(Some(&self.selection));
            let logobox = self.logobox.get();
            obj.logoset(logobox);
            let osdbox = self.osdbox.get();
            let dropdownspinner = self.dropdownspinner.get();
            self.itemlist.connect_activate(move |listview, position| {
                let model = listview.model().unwrap();
                let item = model
                    .item(position)
                    .and_downcast::<glib::BoxedAnyObject>()
                    .unwrap();
                let seriesinfo: Ref<network::SeriesInfo> = item.borrow();
                let info = seriesinfo.clone();
                let id = seriesinfo.Id.clone();
                dropdownspinner.set_visible(true);
                if let Some(widget) = osdbox.last_child() {
                    if widget.is::<gtk::Box>() {
                        osdbox.remove(&widget);
                    }
                }
                let mutex = std::sync::Arc::new(tokio::sync::Mutex::new(()));
                let (sender, receiver) = async_channel::bounded::<crate::ui::network::Media>(1);
                runtime().spawn(async move {
                    let playback = network::playbackinfo(id).await.expect("msg");
                    sender.send(playback).await.expect("msg");
                });
                glib::spawn_future_local(
                    glib::clone!(@weak dropdownspinner,@weak osdbox=>async move {
                        while let Ok(playback) = receiver.recv().await {
                            let _ = mutex.lock().await;
                            let info = info.clone();
                            let dropdown = crate::ui::new_dropsel::newmediadropsel(playback, info);
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
            });
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
    pub fn new(id: String) -> Self {
        Object::builder().property("id", id).build()
    }

    pub fn logoset(&self, osd: gtk::Box) {
        let id = self.id();
        let mutex = std::sync::Arc::new(tokio::sync::Mutex::new(()));
        let logo = crate::ui::image::setlogoimage(id.clone(), mutex.clone());
        osd.append(&logo);
        osd.add_css_class("logo");
    }
}
