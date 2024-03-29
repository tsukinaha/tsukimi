use adw::subclass::prelude::*;
use glib::Object;
use gtk::prelude::*;
use gtk::{gio, glib};

use crate::ui::network::SeriesInfo;

mod imp {
    use crate::ui::network;
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
        #[property(get, set, construct_only)]
        pub inid: OnceCell<String>,
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
        #[template_child]
        pub overviewrevealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub itemoverview: TemplateChild<gtk::Inscription>,
        #[template_child]
        pub selecteditemoverview: TemplateChild<gtk::Inscription>,
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
            klass.install_action("item.first", None, move |window, _action, _parameter| {
                window.itemfirst();
            });
            klass.install_action("item.previous", None, move |window, _action, _parameter| {
                window.itemprevious();
            });
            klass.install_action("item.next", None, move |window, _action, _parameter| {
                window.itemnext();
            });
            klass.install_action("item.last", None, move |window, _action, _parameter| {
                window.itemlast();
            });
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
            let idc = id.clone();
            let inid = obj.inid();
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
            let itemlist = self.itemlist.get();
            glib::spawn_future_local(glib::clone!(@weak obj =>async move {
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
                    if info.ParentIndexNumber == 1 {
                        let object = glib::BoxedAnyObject::new(info.clone());
                        store.append(&object);
                    }
                    if inid != idc {
                        if info.Id == inid {
                            let seriesinfo = network::SeriesInfo {
                                Id: inid.clone(),
                                Name: info.Name.clone(),
                                IndexNumber: info.IndexNumber,
                                ParentIndexNumber: info.ParentIndexNumber,
                                UserData: info.UserData.clone(),
                                Overview: info.Overview.clone(),
                            };
                            obj.selectepisode(seriesinfo.clone());
                        }
                    }
                }
                seasonlist.set_selected(position);
                if idc == inid {
                    itemlist.first_child().unwrap().activate();
                }
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
                    itemlist.first_child().unwrap().activate();
                });
                itemrevealer.set_reveal_child(true);
            }));
            obj.setoverview();
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
                let overlay = gtk::Overlay::new();
                let img = crate::ui::image::setimage(seriesinfo.Id.clone(), mutex.clone());
                img.set_size_request(250, 141);
                overlay.set_child(Some(&img));
                let progressbar = gtk::ProgressBar::new();
                progressbar.set_valign(gtk::Align::End);
                if let Some(userdata) = &seriesinfo.UserData {
                    if let Some(percentage) = userdata.PlayedPercentage {
                        progressbar.set_fraction(percentage / 100.0);
                    }
                    if userdata.Played {
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
            self.itemlist.connect_activate(glib::clone!(@weak obj =>move |listview, position| {
                let model = listview.model().unwrap();
                let item = model
                    .item(position)
                    .and_downcast::<glib::BoxedAnyObject>()
                    .unwrap();
                let seriesinfo: Ref<network::SeriesInfo> = item.borrow();
                obj.selectepisode(seriesinfo.clone());
            }));
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
    pub fn new(id: String, inid: String) -> Self {
        Object::builder()
            .property("id", id)
            .property("inid", inid)
            .build()
    }

    pub fn logoset(&self, osd: gtk::Box) {
        let id = self.id();
        let mutex = std::sync::Arc::new(tokio::sync::Mutex::new(()));
        let logo = crate::ui::image::setlogoimage(id.clone(), mutex.clone());
        osd.append(&logo);
        osd.add_css_class("logo");
    }

    pub fn itemfirst(&self) {
        let imp = self.imp();
        imp.itemlist
            .scroll_to(0, gtk::ListScrollFlags::SELECT, None);
        imp.itemlist.first_child().unwrap().activate();
    }

    pub fn itemprevious(&self) {
        let imp = self.imp();
        let selection = &imp.selection;
        let position = selection.selected();
        if position > 0 {
            imp.itemlist
                .scroll_to(position - 1, gtk::ListScrollFlags::SELECT, None);
            // Todo: activate the previous item
        }
    }

    pub fn itemnext(&self) {
        let imp = self.imp();
        let selection = &imp.selection;
        let position = selection.selected();
        if position < imp.itemlist.model().unwrap().n_items() {
            imp.itemlist
                .scroll_to(position + 1, gtk::ListScrollFlags::SELECT, None);
            // Todo: activate the next item
        }
    }

    pub fn itemlast(&self) {
        let imp = self.imp();
        imp.itemlist.scroll_to(
            imp.itemlist.model().unwrap().n_items() - 1,
            gtk::ListScrollFlags::SELECT,
            None,
        );
        imp.itemlist.last_child().unwrap().activate();
    }

    pub fn selectepisode(&self, seriesinfo: SeriesInfo) {
        let info = seriesinfo.clone();
        let imp = self.imp();
        let osdbox = imp.osdbox.get();
        let dropdownspinner = imp.dropdownspinner.get();
        let id = seriesinfo.Id.clone();
        dropdownspinner.set_visible(true);
        if let Some(widget) = osdbox.last_child() {
            if widget.is::<gtk::Box>() {
                osdbox.remove(&widget);
            }
        }
        let mutex = std::sync::Arc::new(tokio::sync::Mutex::new(()));
        let (sender, receiver) = async_channel::bounded::<crate::ui::network::Media>(1);
        crate::ui::network::runtime().spawn(async move {
            let playback = crate::ui::network::playbackinfo(id).await.expect("msg");
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

        if let Some(overview) = seriesinfo.Overview {
            imp.selecteditemoverview.set_text(Some(&overview));
        }
    }

    pub fn setoverview(&self) {
        let imp = self.imp();
        let id = imp.id.get().unwrap().clone();
        let itemoverview = imp.itemoverview.get();
        let overviewrevealer = imp.overviewrevealer.get();
        let (sender, receiver) = async_channel::bounded::<String>(1);
        crate::ui::network::runtime().spawn(async move {
            let overview = crate::ui::network::get_item_overview(id.to_string()).await.expect("msg");
            sender.send(overview).await.expect("msg");
        });
        glib::spawn_future_local(async move {
            while let Ok(overview) = receiver.recv().await {
                itemoverview.set_text(Some(&overview));
                overviewrevealer.set_reveal_child(true);
            }
        });
    }
}
