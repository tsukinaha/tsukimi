use glib::Object;
use gtk::gdk::Rectangle;
use gtk::gio::MenuModel;
use gtk::glib::subclass::types::ObjectSubclassIsExt;
use gtk::prelude::*;
use gtk::Builder;
use gtk::PopoverMenu;
use gtk::{gio, glib};

use crate::client::structs::Latest;
use crate::ui::image::setbackdropimage;
use crate::ui::image::setbannerimage;
use crate::ui::image::setimage;
use crate::ui::image::setthumbimage;
use crate::ui::provider::tu_item::TuItem;
use crate::utils::spawn;

mod imp {
    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{glib, CompositeTemplate};
    use gtk::{prelude::*, PopoverMenu};
    use std::cell::{OnceCell, RefCell};

    use crate::ui::provider::tu_item::TuItem;

    // Object holding the state
    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsukimi/listitem.ui")]
    #[properties(wrapper_type = super::TuListItem)]
    pub struct TuListItem {
        #[property(get, set, construct_only)]
        pub item: OnceCell<TuItem>,
        #[property(get, set, construct_only)]
        pub itemtype: OnceCell<String>,
        #[property(get, set, construct_only)]
        pub isresume: OnceCell<bool>,
        pub popover: RefCell<Option<PopoverMenu>>,
        #[template_child]
        pub listlabel: TemplateChild<gtk::Label>,
        #[template_child]
        pub overlay: TemplateChild<gtk::Overlay>,
        #[template_child]
        pub revealer: TemplateChild<gtk::Revealer>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for TuListItem {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "TuListItem";
        type Type = super::TuListItem;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // Trait shared by all GObjects
    #[glib::derived_properties]
    impl ObjectImpl for TuListItem {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.set_up();
            obj.gesture();
            obj.reveals();
        }

        fn dispose(&self) {
            if let Some(popover) = self.popover.borrow().as_ref() {
                popover.unparent();
            };
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for TuListItem {}

    // Trait shared by all windows
    impl WindowImpl for TuListItem {}

    // Trait shared by all application windows
    impl ApplicationWindowImpl for TuListItem {}

    impl BinImpl for TuListItem {}
}

glib::wrapper! {
    pub struct TuListItem(ObjectSubclass<imp::TuListItem>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget ,adw::NavigationPage,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl TuListItem {
    pub fn new(item: TuItem, item_type: &str, isresume: bool) -> Self {
        Object::builder()
            .property("itemtype", item_type)
            .property("item", item)
            .property("isresume", isresume)
            .build()
    }

    pub fn set_up(&self) {
        let imp = self.imp();
        let item = imp.item.get().unwrap();
        let item_type = imp.itemtype.get().unwrap();
        match item_type.as_str() {
            "Movie" => {
                let year = if item.production_year() != 0 {
                    item.production_year().to_string()
                } else {
                    String::from("")
                };
                imp.listlabel
                    .set_text(format!("{}\n{}", item.name(), year).as_str());
                self.set_picture();
                self.set_played();
                if let Some(true) = imp.isresume.get() {
                    self.set_played_percentage();
                }
            }
            "Series" => {
                imp.listlabel
                    .set_text(format!("{}\n{}", item.name(), item.production_year()).as_str());
                self.set_picture();
                self.set_played();
                self.set_count();
            }
            "BoxSet" => {
                imp.listlabel.set_text(item.name().to_string().as_str());
                self.set_picture();
            }
            "Tag" | "Genre" => {
                imp.overlay.set_size_request(190, 190);
                imp.listlabel.set_text(item.name().to_string().as_str());
                self.set_picture();
            }
            "Episode" => {
                imp.listlabel.set_text(&format!(
                    "{}\nS{}E{}: {}",
                    item.series_name(),
                    item.parent_index_number(),
                    item.index_number(),
                    item.name()
                ));
                self.set_picture();
                self.set_played();
                self.set_played_percentage();
            }
            "Views" => {
                imp.listlabel.set_text(item.name().to_string().as_str());
                self.set_picture();
            }
            _ => {}
        }
    }

    pub fn set_picture(&self) {
        let imp = self.imp();
        let item = imp.item.get().unwrap();
        let id = item.id();
        if let Some(poster) = item.poster() {
            let image = match poster.as_str() {
                "banner" => {
                    imp.overlay.set_size_request(375, 70);
                    if let Some(imag_tags) = item.image_tags() {
                        if imag_tags.banner().is_some() {
                            setbannerimage(id)
                        } else if imag_tags.thumb().is_some() {
                            setthumbimage(id)
                        } else if imag_tags.backdrop().is_some() {
                            setbackdropimage(id, 0)
                        } else {
                            setimage(id)
                        }
                    } else {
                        setimage(id)
                    }
                }
                "backdrop" => {
                    imp.overlay.set_size_request(250, 141);
                    if let Some(imag_tags) = item.image_tags() {
                        if imag_tags.backdrop().is_some() {
                            setbackdropimage(id, 0)
                        } else if imag_tags.thumb().is_some() {
                            setthumbimage(id)
                        } else {
                            setimage(id)
                        }
                    } else {
                        setimage(id)
                    }
                }
                _ => setimage(id),
            };
            imp.overlay.set_child(Some(&image));
        } else {
            let image = if let Some(true) = imp.isresume.get() {
                if let Some(parent_thumb_item_id) = item.parent_thumb_item_id() {
                    let parent_thumb_item_id = parent_thumb_item_id;
                    imp.overlay.set_size_request(250, 141);
                    setbackdropimage(parent_thumb_item_id, 0)
                } else if let Some(parent_backdrop_item_id) = item.parent_backdrop_item_id() {
                    let parent_backdrop_item_id = parent_backdrop_item_id;
                    imp.overlay.set_size_request(250, 141);
                    setbackdropimage(parent_backdrop_item_id, 0)
                } else {
                    imp.overlay.set_size_request(250, 141);
                    setbackdropimage(id, 0)
                }
            } else {
                if self.itemtype() == "Episode" || self.itemtype() == "Views" {
                    imp.overlay.set_size_request(250, 141);
                }
                setimage(id)
            };
            imp.overlay.set_child(Some(&image));
        }
    }

    pub fn set_played(&self) {
        let imp = self.imp();
        let item = imp.item.get().unwrap();
        if item.played() {
            let mark = gtk::Image::from_icon_name("object-select-symbolic");
            mark.set_halign(gtk::Align::End);
            mark.set_valign(gtk::Align::Start);
            mark.set_height_request(40);
            mark.set_width_request(40);
            imp.overlay.add_overlay(&mark);
        }
    }

    pub fn set_count(&self) {
        let imp = self.imp();
        let item = imp.item.get().unwrap();
        let count = item.unplayed_item_count();
        if count > 0 {
            let mark = gtk::Label::new(Some(&count.to_string()));
            mark.set_halign(gtk::Align::End);
            mark.set_valign(gtk::Align::Start);
            mark.set_height_request(40);
            mark.set_width_request(40);
            imp.overlay.add_overlay(&mark);
        }
    }

    pub fn set_played_percentage(&self) {
        let imp = self.imp();
        let item = imp.item.get().unwrap();
        let percentage = item.played_percentage();
        let progress = gtk::ProgressBar::builder()
            .show_text(true)
            .fraction(percentage / 100.0)
            .valign(gtk::Align::End)
            .build();
        imp.overlay.add_overlay(&progress);
    }

    pub fn gesture(&self) {
        let imp = self.imp();
        let builder = Builder::from_resource("/moe/tsukimi/pop-menu.ui");
        let menu = builder.object::<MenuModel>("rightmenu");
        match menu {
            Some(popover) => {
                let popover = PopoverMenu::builder()
                    .menu_model(&popover)
                    .has_arrow(false)
                    .build();
                popover.set_parent(self);
                let _ = imp.popover.replace(Some(popover));
            }
            None => eprintln!("Failed to load popover"),
        }
        let gesture = gtk::GestureClick::new();
        gesture.set_button(3);
        gesture.connect_released(glib::clone!(@weak imp => move | gesture, _n, x, y| {
            gesture.set_state(gtk::EventSequenceState::Claimed);
            if let Some(popover) = imp.popover.borrow().as_ref() {
                popover.set_pointing_to(Some(&Rectangle::new(x as i32, y as i32, 1, 1)));
                popover.popup();
            };
        }));
        self.add_controller(gesture);
    }

    pub fn reveals(&self) {
        let imp = self.imp();
        let revealer = imp.revealer.get();
        spawn(glib::clone!(@weak imp => async move {
            revealer.set_reveal_child(true);
        }));
    }
}

pub fn tu_list_item_register(latest: &Latest, list_item: &gtk::ListItem, listtype: &str) {
    match latest.latest_type.as_str() {
        "Movie" => {
            let tu_item: TuItem = glib::object::Object::new();
            tu_item.set_id(latest.id.clone());
            tu_item.set_name(latest.name.clone());
            tu_item.set_production_year(latest.production_year.unwrap_or(0));
            if let Some(userdata) = &latest.user_data {
                tu_item.set_played(userdata.played);
            }
            let list_child = TuListItem::new(tu_item, "Movie", listtype == "resume");
            list_item.set_child(Some(&list_child));
        }
        "Series" => {
            let tu_item: TuItem = glib::object::Object::new();
            tu_item.set_id(latest.id.clone());
            tu_item.set_name(latest.name.clone());
            tu_item.set_production_year(latest.production_year.unwrap_or(0));
            if let Some(userdata) = &latest.user_data {
                tu_item.set_played(userdata.played);
                tu_item.set_unplayed_item_count(userdata.unplayed_item_count.unwrap());
            }
            let list_child = TuListItem::new(tu_item, "Series", listtype == "resume");
            list_item.set_child(Some(&list_child));
        }
        "BoxSet" | "Tag" | "Genre" => {
            let tu_item: TuItem = glib::object::Object::new();
            tu_item.set_id(latest.id.clone());
            tu_item.set_name(latest.name.clone());
            let list_child = TuListItem::new(tu_item, latest.latest_type.as_str(), false);
            list_item.set_child(Some(&list_child));
        }
        "Episode" => {
            let tu_item: TuItem = glib::object::Object::new();
            tu_item.set_id(latest.id.clone());
            tu_item.set_name(latest.name.clone());
            tu_item.set_index_number(latest.index_number.unwrap());
            tu_item.set_parent_index_number(latest.parent_index_number.unwrap());
            tu_item.set_series_name(latest.series_name.as_ref().unwrap().clone());
            tu_item.set_parent_backdrop_item_id(latest.parent_backdrop_item_id.clone());
            tu_item.set_parent_thumb_item_id(latest.parent_thumb_item_id.clone());
            tu_item.set_played_percentage(
                latest
                    .user_data
                    .as_ref()
                    .unwrap()
                    .played_percentage
                    .unwrap_or(0.0),
            );
            if let Some(userdata) = &latest.user_data {
                tu_item.set_played(userdata.played);
            }
            let list_child = TuListItem::new(tu_item, "Episode", listtype == "resume");
            list_item.set_child(Some(&list_child));
        }
        _ => {}
    }
}

pub fn tu_list_poster(latest: &Latest, list_item: &gtk::ListItem, listtype: &str, poster: &str) {
    match latest.latest_type.as_str() {
        "Movie" => {
            let tu_item: TuItem = glib::object::Object::new();
            tu_item.set_id(latest.id.clone());
            tu_item.set_name(latest.name.clone());
            tu_item.set_production_year(latest.production_year.unwrap_or(0));
            if let Some(userdata) = &latest.user_data {
                tu_item.set_played(userdata.played);
            }
            tu_item.set_poster(poster);
            tu_item.imp().set_image_tags(latest.image_tags.clone());
            let list_child = TuListItem::new(tu_item, "Movie", listtype == "resume");
            list_item.set_child(Some(&list_child));
        }
        "Series" => {
            let tu_item: TuItem = glib::object::Object::new();
            tu_item.set_id(latest.id.clone());
            tu_item.set_name(latest.name.clone());
            tu_item.set_production_year(latest.production_year.unwrap());
            if let Some(userdata) = &latest.user_data {
                tu_item.set_played(userdata.played);
                tu_item.set_unplayed_item_count(userdata.unplayed_item_count.unwrap());
            }
            tu_item.set_poster(poster);
            tu_item.imp().set_image_tags(latest.image_tags.clone());
            let list_child = TuListItem::new(tu_item, "Series", listtype == "resume");
            list_item.set_child(Some(&list_child));
        }
        _ => {}
    }
}
