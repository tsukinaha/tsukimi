use glib::Object;
use gtk::gdk::Rectangle;
use gtk::gio::MenuModel;
use gtk::glib::subclass::types::ObjectSubclassExt;
use gtk::glib::subclass::types::ObjectSubclassIsExt;
use gtk::prelude::*;
use gtk::Builder;
use gtk::PopoverMenu;
use gtk::{gio, glib};

use crate::client::network::like;
use crate::client::network::played;
use crate::client::network::unlike;
use crate::client::network::unplayed;
use crate::client::structs::SimpleListItem;
use crate::ui::image::set_image;
use crate::ui::provider::tu_item::TuItem;
use crate::utils::spawn;
use crate::utils::spawn_tokio;

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
            obj.insert_action_group("item", obj.set_action().as_ref());
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

    impl BinImpl for TuListItem {}
}

glib::wrapper! {
    pub struct TuListItem(ObjectSubclass<imp::TuListItem>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget ,adw::NavigationPage,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

pub enum Action {
    Like,
    Unlike,
    Played,
    Unplayed,
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
            "MusicAlbum" => {
                imp.listlabel.set_text(&format!(
                    "{}\n{}",
                    item.name(),
                    item.album_artist().unwrap_or("".to_string())
                ));
                imp.overlay.set_size_request(190, 190);
                self.set_picture();
            }
            "Actor" => {
                imp.listlabel.set_text(&format!(
                    "{}\n{}",
                    item.name(),
                    item.role().unwrap_or("".to_string())
                ));
                self.set_picture();
            }
            _ => {
                println!("Unknown item type: {}", item_type)
            }
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
                            set_image(id, "Banner", None)
                        } else if imag_tags.thumb().is_some() {
                            set_image(id, "Thumb", None)
                        } else if imag_tags.backdrop().is_some() {
                            set_image(id, "Backdrop", Some(0))
                        } else {
                            set_image(id, "Primary", None)
                        }
                    } else {
                        set_image(id, "Primary", None)
                    }
                }
                "backdrop" => {
                    imp.overlay.set_size_request(250, 141);
                    if let Some(imag_tags) = item.image_tags() {
                        if imag_tags.backdrop().is_some() {
                            set_image(id, "Backdrop", Some(0))
                        } else if imag_tags.thumb().is_some() {
                            set_image(id, "Thumb", None)
                        } else {
                            set_image(id, "Primary", None)
                        }
                    } else {
                        set_image(id, "Primary", None)
                    }
                }
                _ => set_image(id, "Primary", None),
            };
            imp.overlay.set_child(Some(&image));
        } else {
            let image = if let Some(true) = imp.isresume.get() {
                if let Some(parent_thumb_item_id) = item.parent_thumb_item_id() {
                    let parent_thumb_item_id = parent_thumb_item_id;
                    imp.overlay.set_size_request(250, 141);
                    set_image(parent_thumb_item_id, "Thumb", None)
                } else if let Some(parent_backdrop_item_id) = item.parent_backdrop_item_id() {
                    let parent_backdrop_item_id = parent_backdrop_item_id;
                    imp.overlay.set_size_request(250, 141);
                    set_image(parent_backdrop_item_id, "Backdrop", Some(0))
                } else {
                    imp.overlay.set_size_request(250, 141);
                    set_image(id, "Backdrop", Some(0))
                }
            } else {
                if self.itemtype() == "Episode" || self.itemtype() == "Views" {
                    imp.overlay.set_size_request(250, 141);
                }
                set_image(id, "Primary", None)
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

    pub fn set_action(&self) -> Option<gio::SimpleActionGroup> {
        let item_type = self.imp().itemtype.get().unwrap();
        match item_type.as_str() {
            "Movie" | "Series" | "Episode" | "MusicAlbum" | "BoxSet" => self.set_item_action(),
            _ => None,
        }
    }

    pub fn set_item_action(&self) -> Option<gio::SimpleActionGroup> {
        let action_group = gio::SimpleActionGroup::new();

        match self.item().is_favorite() {
            true => action_group.add_action_entries([gio::ActionEntry::builder("unlike")
                .activate(glib::clone!(@weak self as obj => move |_, _, _| {
                    spawn(glib::clone!(@weak obj => async move {
                        obj.perform_action(Action::Unlike).await;
                    }))
                }))
                .build()]),
            false => action_group.add_action_entries([gio::ActionEntry::builder("like")
                .activate(glib::clone!(@weak self as obj => move |_, _, _| {
                    spawn(glib::clone!(@weak obj => async move {
                        obj.perform_action(Action::Like).await;
                    }))
                }))
                .build()]),
        }

        match self.item().played() {
            true => action_group.add_action_entries([gio::ActionEntry::builder("unplayed")
                .activate(glib::clone!(@weak self as obj => move |_, _, _| {
                    spawn(glib::clone!(@weak obj => async move {
                        obj.perform_action(Action::Unplayed).await;
                    }))
                }))
                .build()]),
            false => action_group.add_action_entries([gio::ActionEntry::builder("played")
                .activate(glib::clone!(@weak self as obj => move |_, _, _| {
                    spawn(glib::clone!(@weak obj => async move {
                        obj.perform_action(Action::Played).await;
                    }))
                }))
                .build()]),
        }

        Some(action_group)
    }

    async fn perform_action_inner(id: &str, action: &Action) -> Result<(), reqwest::Error> {
        match action {
            Action::Like => like(id).await,
            Action::Unlike => unlike(id).await,
            Action::Played => played(id).await,
            Action::Unplayed => unplayed(id).await,
        }
    }

    pub async fn perform_action(&self, action: Action) {
        let id = self.item().id().clone();
        self.update_state(&action);
        let result = spawn_tokio(async move { Self::perform_action_inner(&id, &action).await });

        result.await.unwrap();

        spawn(glib::clone!(@weak self as obj => async move {
            let window = obj.root().and_downcast::<super::window::Window>().unwrap();
            window.toast("Success");
        }));

        let obj = self.imp().obj();
        obj.insert_action_group("item", obj.set_action().as_ref());
    }

    pub fn update_state(&self, action: &Action) {
        match action {
            Action::Like => self.item().set_is_favorite(true),
            Action::Unlike => self.item().set_is_favorite(false),
            Action::Played => self.item().set_played(true),
            Action::Unplayed => self.item().set_played(false),
        }
        self.gesture();
    }

    pub fn reveals(&self) {
        let imp = self.imp();
        let revealer = imp.revealer.get();
        spawn(glib::clone!(@weak imp => async move {
            revealer.set_reveal_child(true);
        }));
    }

    pub async fn process_item(
        &self,
        action: fn(&String) -> Result<(), Box<dyn std::error::Error>>,
    ) {
        let id = self.item().id();
        spawn_tokio(async move {
            action(&id).unwrap();
        })
        .await;
        spawn(glib::clone!(@weak self as obj=>async move {
            let window = obj.root().and_downcast::<super::window::Window>().unwrap();
            window.toast("Success");
        }));
    }
}

pub fn tu_list_item_register(latest: &SimpleListItem, list_item: &gtk::ListItem, listtype: &str) {
    let tu_item = create_tu_item(latest, None);
    match latest.latest_type.as_str() {
        "Movie" | "Series" | "Episode" | "MusicAlbum" | "BoxSet" | "Tag" | "Genre" | "Views"
        | "Actor" => {
            set_list_child(
                tu_item,
                list_item,
                &latest.latest_type,
                listtype == "resume",
            );
        }
        _ => {}
    }
}

pub fn tu_list_poster(
    latest: &SimpleListItem,
    list_item: &gtk::ListItem,
    listtype: &str,
    poster: &str,
) {
    let tu_item = create_tu_item(latest, Some(poster));
    match latest.latest_type.as_str() {
        "Movie" | "Series" => {
            set_list_child(
                tu_item,
                list_item,
                &latest.latest_type,
                listtype == "resume",
            );
        }
        _ => {}
    }
}

fn create_tu_item(latest: &SimpleListItem, poster: Option<&str>) -> TuItem {
    let tu_item: TuItem = glib::object::Object::new();
    tu_item.set_id(latest.id.clone());
    tu_item.set_name(latest.name.clone());
    if let Some(production_year) = latest.production_year {
        tu_item.set_production_year(production_year);
    }
    if let Some(index_number) = latest.index_number {
        tu_item.set_index_number(index_number);
    }
    if let Some(parent_index_number) = latest.parent_index_number {
        tu_item.set_parent_index_number(parent_index_number);
    }
    if let Some(userdata) = &latest.user_data {
        tu_item.set_played(userdata.played);
        if let Some(played_percentage) = userdata.played_percentage {
            tu_item.set_played_percentage(played_percentage);
        }
        if let Some(unplayed_item_count) = userdata.unplayed_item_count {
            tu_item.set_unplayed_item_count(unplayed_item_count);
        }
        tu_item.set_is_favorite(userdata.is_favorite.unwrap_or(false));
    }
    if let Some(poster) = poster {
        tu_item.set_poster(poster);
        tu_item.imp().set_image_tags(latest.image_tags.clone());
    }
    if let Some(parent_thumb_item_id) = &latest.parent_thumb_item_id {
        tu_item.set_parent_thumb_item_id(Some(parent_thumb_item_id.clone()));
    }
    if let Some(parent_backdrop_item_id) = &latest.parent_backdrop_item_id {
        tu_item.set_parent_backdrop_item_id(Some(parent_backdrop_item_id.clone()));
    }
    if let Some(series_name) = &latest.series_name {
        tu_item.set_series_name(series_name.clone());
    }
    if let Some(album_artist) = &latest.album_artist {
        tu_item.set_album_artist(Some(album_artist.clone()));
    }
    if let Some(role) = &latest.role {
        tu_item.set_role(Some(role.clone()));
    }
    tu_item
}

fn set_list_child(tu_item: TuItem, list_item: &gtk::ListItem, latest_type: &str, is_resume: bool) {
    let list_child = TuListItem::new(tu_item, latest_type, is_resume);
    list_item.set_child(Some(&list_child));
}
