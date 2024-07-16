use adw::prelude::*;
use gettextrs::gettext;
use glib::Object;
use gtk::gdk::Rectangle;
use gtk::gio::MenuModel;
use gtk::glib::subclass::types::ObjectSubclassExt;
use gtk::glib::subclass::types::ObjectSubclassIsExt;
use gtk::template_callbacks;
use gtk::Builder;
use gtk::PopoverMenu;
use gtk::{gio, glib};
use tracing::debug;
use tracing::warn;

use crate::client::client::EMBY_CLIENT;
use crate::client::error::UserFacingError;
use crate::client::structs::SimpleListItem;
use crate::toast;
use crate::ui::models::emby_cache_path;
use crate::ui::provider::tu_item::TuItem;
use crate::utils::spawn;
use crate::utils::spawn_tokio;

use super::window::Window;

pub const PROGRESSBAR_ANIMATION_DURATION: u32 = 2000;

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
        pub label2: TemplateChild<gtk::Label>,
        #[template_child]
        pub overlay: TemplateChild<gtk::Overlay>,
        #[template_child]
        pub revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub spinner: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub broken: TemplateChild<gtk::Box>,
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
            klass.bind_template_instance_callbacks();
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
    Remove,
}

#[template_callbacks]
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
                imp.listlabel.set_text(&item.name());
                imp.label2.set_text(&year);
                imp.overlay.set_size_request(167, 260);
                self.set_picture();
                self.set_played();
                if let Some(true) = imp.isresume.get() {
                    self.set_played_percentage(self.get_played_percentage());
                    return;
                }
                self.set_rating();
            }
            "Video" => {
                imp.listlabel.set_text(&item.name());
                imp.label2.set_visible(false);
                imp.overlay.set_size_request(250, 141);
                self.set_picture();
            }
            "TvChannel" => {
                imp.listlabel.set_text(&format!(
                    "{} - {}",
                    item.name(),
                    item.program_name().unwrap_or_default()
                ));
                imp.overlay.set_size_request(250, 141);
                self.set_picture();

                let Some(program_start_time) = item.program_start_time() else {
                    return;
                };

                let program_start_time = program_start_time.to_local().unwrap();

                let Some(program_end_time) = item.program_end_time() else {
                    return;
                };

                let program_end_time = program_end_time.to_local().unwrap();

                let now = glib::DateTime::now_local().unwrap();

                let progress = (now.to_unix() - program_start_time.to_unix()) as f64
                    / (program_end_time.to_unix() - program_start_time.to_unix()) as f64;

                self.set_played_percentage(progress * 100.0);
                imp.label2.set_text(&format!(
                    "{} - {}",
                    program_start_time.format("%H:%M").unwrap(),
                    program_end_time.format("%H:%M").unwrap()
                ));
            }
            "CollectionFolder" | "UserView" => {
                imp.listlabel.set_text(&item.name());
                imp.label2.set_visible(false);
                imp.overlay.set_size_request(250, 141);
                self.set_picture();
            }
            "Series" => {
                let year = if item.production_year() != 0 {
                    item.production_year().to_string()
                } else {
                    String::from("")
                };
                imp.listlabel.set_text(&item.name());
                imp.label2.set_text(&year);
                imp.overlay.set_size_request(167, 260);
                self.set_picture();
                self.set_played();
                self.set_count();
                self.set_rating();
            }
            "BoxSet" => {
                imp.listlabel.set_text(&item.name());
                imp.label2.set_visible(false);
                imp.overlay.set_size_request(167, 260);
                self.set_picture();
            }
            "Tag" | "Genre" => {
                imp.overlay.set_size_request(190, 190);
                imp.listlabel.set_text(&item.name());
                imp.label2.set_visible(false);
                self.set_picture();
            }
            "Episode" => {
                imp.listlabel.set_text(&item.series_name());
                imp.label2.set_text(&format!(
                    "S{}E{}: {}",
                    item.parent_index_number(),
                    item.index_number(),
                    item.name()
                ));
                imp.overlay.set_size_request(250, 141);
                self.set_picture();
                self.set_played();
                self.set_played_percentage(self.get_played_percentage());
            }
            "Views" => {
                imp.listlabel.set_text(&item.name());
                imp.label2.set_visible(false);
                self.set_picture();
            }
            "MusicAlbum" => {
                imp.listlabel.set_text(&item.name());
                imp.label2.set_text(&item.albumartist_name());
                imp.overlay.set_size_request(190, 190);
                self.set_picture();
            }
            "Actor" | "Person" | "Director" => {
                imp.listlabel.set_text(&item.name());
                imp.label2.set_text(&item.role().unwrap_or("".to_string()));
                imp.overlay.set_size_request(167, 260);
                self.set_picture();
            }
            "Audio" => {
                imp.listlabel.set_text(&item.name());
                imp.overlay.set_size_request(190, 190);
                self.set_picture();
            }
            _ => {
                self.set_visible(false);
                warn!("Unknown item type: {}", item_type)
            }
        }
        self.set_tooltip_text(Some(&item.name()));
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
                            self.set_image(id, "Banner", None)
                        } else if imag_tags.thumb().is_some() {
                            self.set_image(id, "Thumb", None)
                        } else if imag_tags.backdrop().is_some() {
                            self.set_image(id, "Backdrop", Some(0))
                        } else {
                            self.set_image(id, "Primary", None)
                        }
                    } else {
                        self.set_image(id, "Primary", None)
                    }
                }
                "backdrop" => {
                    imp.overlay.set_size_request(250, 141);
                    if let Some(imag_tags) = item.image_tags() {
                        if imag_tags.backdrop().is_some() {
                            self.set_image(id, "Backdrop", Some(0))
                        } else if imag_tags.thumb().is_some() {
                            self.set_image(id, "Thumb", None)
                        } else {
                            self.set_image(id, "Primary", None)
                        }
                    } else {
                        self.set_image(id, "Primary", None)
                    }
                }
                _ => self.set_image(id, "Primary", None),
            };
            imp.overlay.set_child(Some(&image));
        } else {
            let image = if let Some(true) = imp.isresume.get() {
                if let Some(parent_thumb_item_id) = item.parent_thumb_item_id() {
                    imp.overlay.set_size_request(250, 141);
                    self.set_image(parent_thumb_item_id, "Thumb", None)
                } else if let Some(parent_backdrop_item_id) = item.parent_backdrop_item_id() {
                    imp.overlay.set_size_request(250, 141);
                    self.set_image(parent_backdrop_item_id, "Backdrop", Some(0))
                } else {
                    imp.overlay.set_size_request(250, 141);
                    self.set_image(id, "Backdrop", Some(0))
                }
            } else if let Some(img_tags) = item.primary_image_item_id() {
                self.set_image(img_tags, "Primary", None)
            } else {
                self.set_image(id, "Primary", None)
            };
            imp.overlay.set_child(Some(&image));
        }
    }

    pub fn set_image(&self, id: String, image_type: &str, tag: Option<u8>) -> gtk::Revealer {
        let image = gtk::Picture::new();
        image.set_halign(gtk::Align::Fill);
        image.set_content_fit(gtk::ContentFit::Cover);
        let revealer = gtk::Revealer::builder()
            .transition_type(gtk::RevealerTransitionType::Crossfade)
            .child(&image)
            .reveal_child(false)
            .vexpand(true)
            .transition_duration(400)
            .build();

        let cache_path = emby_cache_path();
        let path = format!("{}-{}-{}", id, image_type, tag.unwrap_or(0));

        let spinner = self.imp().spinner.get();
        let broken = self.imp().broken.get();

        let id = id.to_string();

        let pathbuf = cache_path.join(path);

        if pathbuf.exists() {
            if image.file().is_none() {
                image.set_file(Some(&gtk::gio::File::for_path(pathbuf)));
                revealer.set_reveal_child(true);
                spinner.stop();
            }
            return revealer;
        }

        let image_type = image_type.to_string();

        spawn(glib::clone!(
            #[weak]
            image,
            #[weak]
            revealer,
            #[weak]
            spinner,
            async move {
                spawn_tokio(async move {
                    let mut retries = 0;
                    while retries < 3 {
                        match EMBY_CLIENT.get_image(&id, &image_type, tag).await {
                            Ok(_) => {
                                break;
                            }
                            Err(e) => {
                                warn!("Failed to get image: {}, retrying...", e);
                                retries += 1;
                            }
                        }
                    }
                })
                .await;

                debug!("Setting image: {}", &pathbuf.display());

                spinner.stop();

                if !pathbuf.exists() {
                    broken.set_visible(true);
                }

                let file = gtk::gio::File::for_path(pathbuf);

                image.set_file(Some(&file));
                revealer.set_reveal_child(true);
            }
        ));

        revealer
    }

    pub fn set_played(&self) {
        let imp = self.imp();
        let item = imp.item.get().unwrap();
        if item.played() {
            let mark = gtk::Image::from_icon_name("object-select-symbolic");
            mark.set_halign(gtk::Align::End);
            mark.set_valign(gtk::Align::Start);
            mark.set_height_request(30);
            mark.set_width_request(30);
            imp.overlay.add_overlay(&mark);
        }
    }

    pub fn set_rating(&self) {
        let imp = self.imp();
        let item = imp.item.get().unwrap();
        if let Some(rating) = item.rating() {
            let rating = gtk::Label::new(Some(&rating));
            rating.set_halign(gtk::Align::Start);
            rating.set_valign(gtk::Align::End);
            rating.set_height_request(40);
            rating.set_width_request(60);
            imp.overlay.add_overlay(&rating);
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

    pub fn get_played_percentage(&self) -> f64 {
        let imp = self.imp();
        let item = imp.item.get().unwrap();
        item.played_percentage()
    }

    pub fn set_played_percentage(&self, percentage: f64) {
        let imp = self.imp();

        let progress = gtk::ProgressBar::builder()
            .show_text(true)
            .fraction(0.)
            .valign(gtk::Align::End)
            .build();

        imp.overlay.add_overlay(&progress);

        spawn(glib::clone!(
            #[weak]
            progress,
            async move {
                let target = adw::CallbackAnimationTarget::new(glib::clone!(
                    #[weak]
                    progress,
                    move |process| progress.set_fraction(process)
                ));

                let animation = adw::TimedAnimation::builder()
                    .duration(PROGRESSBAR_ANIMATION_DURATION)
                    .widget(&progress)
                    .target(&target)
                    .easing(adw::Easing::EaseOutQuart)
                    .value_from(0.)
                    .value_to(percentage / 100.0)
                    .build();

                glib::timeout_future_seconds(1).await;
                animation.play();
            }
        ));
    }

    pub fn gesture(&self) {
        let imp = self.imp();
        let builder = Builder::from_resource("/moe/tsukimi/pop-menu.ui");
        let menu = builder.object::<MenuModel>("rightmenu");
        match menu {
            Some(popover) => {
                let popover = PopoverMenu::builder()
                    .menu_model(&popover)
                    .halign(gtk::Align::Start)
                    .has_arrow(false)
                    .build();
                popover.set_parent(self);
                let _ = imp.popover.replace(Some(popover));
            }
            None => eprintln!("Failed to load popover"),
        }
        let gesture = gtk::GestureClick::new();
        gesture.set_button(3);
        gesture.connect_released(glib::clone!(
            #[weak]
            imp,
            move |gesture, _n, x, y| {
                gesture.set_state(gtk::EventSequenceState::Claimed);
                if let Some(popover) = imp.popover.borrow().as_ref() {
                    popover.set_pointing_to(Some(&Rectangle::new(x as i32, y as i32, 0, 0)));
                    popover.popup();
                };
            }
        ));
        self.add_controller(gesture);
    }

    pub fn set_action(&self) -> Option<gio::SimpleActionGroup> {
        let item_type = self.imp().itemtype.get().unwrap();
        match item_type.as_str() {
            "Movie" | "Series" | "Episode" => self.set_item_action(true),
            "MusicAlbum" | "BoxSet" | "Tag" | "Genre" | "Views" | "Actor" | "Person"
            | "TvChannel" => self.set_item_action(false),
            _ => None,
        }
    }

    pub fn set_item_action(&self, is_playable: bool) -> Option<gio::SimpleActionGroup> {
        let action_group = gio::SimpleActionGroup::new();

        action_group.add_action_entries([gio::ActionEntry::builder("editm")
            .activate(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move |_, _, _| {
                    spawn(glib::clone!(
                        #[weak]
                        obj,
                        async move {
                            let id = obj.item().id();
                            let dialog =
                                crate::ui::widgets::metadata_dialog::MetadataDialog::new(&id);
                            crate::insert_editm_dialog!(obj, dialog);
                        }
                    ))
                }
            ))
            .build()]);

        action_group.add_action_entries([gio::ActionEntry::builder("editi")
            .activate(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move |_, _, _| {
                    spawn(glib::clone!(
                        #[weak]
                        obj,
                        async move {
                            let id = obj.item().id();
                            let dialog = crate::ui::widgets::image_dialog::ImagesDialog::new(&id);
                            crate::insert_editm_dialog!(obj, dialog);
                        }
                    ))
                }
            ))
            .build()]);

        match self.item().is_favorite() {
            true => action_group.add_action_entries([gio::ActionEntry::builder("unlike")
                .activate(glib::clone!(
                    #[weak(rename_to = obj)]
                    self,
                    move |_, _, _| {
                        spawn(glib::clone!(
                            #[weak]
                            obj,
                            async move {
                                obj.perform_action(Action::Unlike).await;
                            }
                        ))
                    }
                ))
                .build()]),
            false => action_group.add_action_entries([gio::ActionEntry::builder("like")
                .activate(glib::clone!(
                    #[weak(rename_to = obj)]
                    self,
                    move |_, _, _| {
                        spawn(glib::clone!(
                            #[weak]
                            obj,
                            async move {
                                obj.perform_action(Action::Like).await;
                            }
                        ))
                    }
                ))
                .build()]),
        }

        if is_playable {
            match self.item().played() {
                true => action_group.add_action_entries([gio::ActionEntry::builder("unplayed")
                    .activate(glib::clone!(
                        #[weak(rename_to = obj)]
                        self,
                        move |_, _, _| {
                            spawn(glib::clone!(
                                #[weak]
                                obj,
                                async move {
                                    obj.perform_action(Action::Unplayed).await;
                                }
                            ))
                        }
                    ))
                    .build()]),
                false => action_group.add_action_entries([gio::ActionEntry::builder("played")
                    .activate(glib::clone!(
                        #[weak(rename_to = obj)]
                        self,
                        move |_, _, _| {
                            spawn(glib::clone!(
                                #[weak]
                                obj,
                                async move {
                                    obj.perform_action(Action::Played).await;
                                }
                            ))
                        }
                    ))
                    .build()]),
            }
        }

        if let Some(true) = self.imp().isresume.get() {
            action_group.add_action_entries([gio::ActionEntry::builder("remove")
                .activate(glib::clone!(
                    #[weak(rename_to = obj)]
                    self,
                    move |_, _, _| {
                        spawn(glib::clone!(
                            #[weak]
                            obj,
                            async move {
                                obj.perform_action(Action::Remove).await;
                            }
                        ))
                    }
                ))
                .build()]);
        }
        Some(action_group)
    }

    async fn perform_action_inner(id: &str, action: &Action) -> Result<(), reqwest::Error> {
        match action {
            Action::Like => EMBY_CLIENT.like(id).await,
            Action::Unlike => EMBY_CLIENT.unlike(id).await,
            Action::Played => EMBY_CLIENT.set_as_played(id).await,
            Action::Unplayed => EMBY_CLIENT.set_as_unplayed(id).await,
            Action::Remove => EMBY_CLIENT.hide_from_resume(id).await,
        }
    }

    pub async fn perform_action(&self, action: Action) {
        let id = self.item().id().clone();
        self.update_state(&action);
        let result = spawn_tokio(async move { Self::perform_action_inner(&id, &action).await });

        match result.await {
            Ok(_) => {
                toast!(self, gettext("Success"))
            }
            Err(e) => {
                toast!(self, e.to_user_facing());
            }
        }

        let obj = self.imp().obj();
        obj.insert_action_group("item", obj.set_action().as_ref());
    }

    pub fn update_state(&self, action: &Action) {
        match action {
            Action::Like => self.item().set_is_favorite(true),
            Action::Unlike => self.item().set_is_favorite(false),
            Action::Played => self.item().set_played(true),
            Action::Unplayed => self.item().set_played(false),
            Action::Remove => {
                self.imp().revealer.set_reveal_child(false);
                spawn(glib::clone!(
                    #[weak(rename_to = obj)]
                    self,
                    async move {
                        let parent = obj.parent().unwrap().parent().unwrap();
                        if let Some(list_view) = parent.downcast_ref::<gtk::ListView>() {
                            let selection = list_view
                                .model()
                                .unwrap()
                                .downcast::<gtk::SingleSelection>()
                                .unwrap();
                            let store = selection
                                .model()
                                .unwrap()
                                .downcast::<gio::ListStore>()
                                .unwrap();
                            store.remove(selection.selected());
                        } else if let Some(grid_view) = parent.downcast_ref::<gtk::GridView>() {
                            let selection = grid_view
                                .model()
                                .unwrap()
                                .downcast::<gtk::SingleSelection>()
                                .unwrap();
                            let store = selection
                                .model()
                                .unwrap()
                                .downcast::<gio::ListStore>()
                                .unwrap();
                            store.remove(selection.selected());
                        }
                    }
                ));
            }
        }
        self.gesture();
    }

    pub fn reveals(&self) {
        let imp = self.imp();
        let revealer = imp.revealer.get();
        spawn(async move {
            revealer.set_reveal_child(true);
        });
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
        spawn(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                let window = obj.root().and_downcast::<super::window::Window>().unwrap();
                window.toast("Success");
            }
        ));
    }

    #[template_callback]
    fn on_view_pic_clicked(&self) {
        match self
            .imp()
            .overlay
            .child()
            .unwrap()
            .downcast::<gtk::Revealer>()
            .unwrap()
            .child()
            .unwrap()
            .downcast::<gtk::Picture>()
        {
            Ok(picture) => {
                let window = self
                    .ancestor(Window::static_type())
                    .and_downcast::<Window>()
                    .unwrap();
                window.reveal_image(&picture);
                window.media_viewer_show_paintable(picture.paintable());
            }
            Err(_) => {
                toast!(self, gettext("Error loading image"));
            }
        }
    }
}

pub fn tu_list_poster(
    latest: &SimpleListItem,
    list_item: &gtk::ListItem,
    is_resume: bool,
    poster: &str,
) {
    let tu_item = TuItem::from_simple(latest, Some(poster));
    match latest.latest_type.as_str() {
        "Movie" | "Series" => {
            set_list_child(tu_item, list_item, &latest.latest_type, is_resume);
        }
        _ => {}
    }
}

fn set_list_child(tu_item: TuItem, list_item: &gtk::ListItem, latest_type: &str, is_resume: bool) {
    let list_child = TuListItem::new(tu_item, latest_type, is_resume);
    list_item.set_child(Some(&list_child));
}
