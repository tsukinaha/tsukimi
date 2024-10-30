use adw::prelude::*;
use anyhow::Result;
use gettextrs::gettext;
use glib::Object;
use gtk::{
    gdk::Rectangle,
    gio,
    gio::MenuModel,
    glib,
    glib::subclass::types::{
        ObjectSubclassExt,
        ObjectSubclassIsExt,
    },
    template_callbacks,
    Builder,
    PopoverMenu,
};
use imp::PosterType;
use tracing::warn;

use super::{
    picture_loader::PictureLoader,
    utils::{
        TU_ITEM_BANNER_SIZE,
        TU_ITEM_POST_SIZE,
        TU_ITEM_SQUARE_SIZE,
        TU_ITEM_VIDEO_SIZE,
    },
};
use crate::{
    client::{
        client::EMBY_CLIENT,
        error::UserFacingError,
    },
    toast,
    ui::provider::{
        tu_item::TuItem,
        IS_ADMIN,
    },
    utils::{
        spawn,
        spawn_tokio,
    },
};

pub const PROGRESSBAR_ANIMATION_DURATION: u32 = 2000;

pub mod imp {
    use std::cell::{
        Cell,
        RefCell,
    };

    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{
        glib,
        prelude::*,
        CompositeTemplate,
        PopoverMenu,
    };

    use crate::ui::{
        provider::tu_item::TuItem,
        widgets::picture_loader::PictureLoader,
    };

    #[derive(Default, Hash, Eq, PartialEq, Clone, Copy, glib::Enum, Debug)]
    #[repr(u32)]
    #[enum_type(name = "PosterType")]
    pub enum PosterType {
        Backdrop,
        Banner,
        #[default]
        Poster,
    }

    // Object holding the state
    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/listitem.ui")]
    #[properties(wrapper_type = super::TuListItem)]
    pub struct TuListItem {
        #[property(get, set = Self::set_item)]
        pub item: RefCell<TuItem>,
        #[property(get, set, builder(PosterType::default()))]
        pub poster_type: Cell<PosterType>,
        pub popover: RefCell<Option<PopoverMenu>>,
        #[template_child]
        pub listlabel: TemplateChild<gtk::Label>,
        #[template_child]
        pub label2: TemplateChild<gtk::Label>,
        #[template_child]
        pub overlay: TemplateChild<gtk::Overlay>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for TuListItem {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "TuListItem";
        type Type = super::TuListItem;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            PictureLoader::ensure_type();
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

    impl TuListItem {
        pub fn set_item(&self, item: TuItem) {
            self.item.replace(item);
            let obj = self.obj();
            obj.set_up();
            obj.gesture();
        }
    }
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
    pub fn new(item: TuItem) -> Self {
        Object::builder().property("item", item).build()
    }

    pub fn default() -> Self {
        Object::new()
    }

    pub fn set_up(&self) {
        let imp = self.imp();
        let item = self.item();
        let item_type = item.item_type();
        match item_type.as_str() {
            "Movie" => {
                let year = if item.production_year() != 0 {
                    item.production_year().to_string()
                } else {
                    String::default()
                };
                imp.listlabel.set_text(&item.name());
                imp.label2.set_text(&year);
                imp.overlay.set_size_request(TU_ITEM_POST_SIZE.0, TU_ITEM_POST_SIZE.1);
                self.set_picture();
                self.set_played();
                if item.is_resume() {
                    self.set_played_percentage(self.get_played_percentage());
                    return;
                }
                self.set_rating();
            }
            "Video" => {
                imp.listlabel.set_text(&item.name());
                imp.label2.set_visible(false);
                imp.overlay.set_size_request(TU_ITEM_VIDEO_SIZE.0, TU_ITEM_VIDEO_SIZE.1);
                self.set_picture();
            }
            "TvChannel" => {
                imp.listlabel.set_text(&format!(
                    "{} - {}",
                    item.name(),
                    item.program_name().unwrap_or_default()
                ));
                imp.overlay.set_size_request(TU_ITEM_VIDEO_SIZE.0, TU_ITEM_VIDEO_SIZE.1);
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
                imp.overlay.set_size_request(TU_ITEM_VIDEO_SIZE.0, TU_ITEM_VIDEO_SIZE.1);
                self.set_apicture();
            }
            "Series" => {
                let year = if item.production_year() != 0 {
                    item.production_year().to_string()
                } else {
                    String::from("")
                };
                imp.listlabel.set_text(&item.name());
                if let Some(status) = item.status() {
                    if status == "Continuing" {
                        imp.label2.set_text(&format!("{} - {}", year, gettext("Present")));
                    } else if status == "Ended" {
                        if let Some(end_date) = item.end_date() {
                            let end_year = end_date.year();
                            if end_year != year.parse::<i32>().unwrap_or_default() {
                                imp.label2.set_text(&format!("{} - {}", year, end_date.year()));
                            } else {
                                imp.label2.set_text(&format!("{}", end_year));
                            }
                        } else {
                            imp.label2.set_text(&format!("{} - Unknown", year));
                        }
                    }
                } else {
                    imp.label2.set_text(&year);
                }
                imp.overlay.set_size_request(TU_ITEM_POST_SIZE.0, TU_ITEM_POST_SIZE.1);
                self.set_picture();
                self.set_played();
                self.set_count();
                self.set_rating();
            }
            "BoxSet" => {
                imp.listlabel.set_text(&item.name());
                imp.label2.set_visible(false);
                imp.overlay.set_size_request(TU_ITEM_POST_SIZE.0, TU_ITEM_POST_SIZE.1);
                self.set_picture();
            }
            "Tag" | "Genre" => {
                imp.overlay.set_size_request(TU_ITEM_SQUARE_SIZE.0, TU_ITEM_SQUARE_SIZE.1);
                imp.listlabel.set_text(&item.name());
                imp.label2.set_visible(false);
                self.set_picture();
            }
            "Episode" => {
                imp.listlabel.set_text(&item.series_name().unwrap_or_default());
                imp.label2.set_text(&format!(
                    "S{}E{}: {}",
                    item.parent_index_number(),
                    item.index_number(),
                    item.name()
                ));
                imp.overlay.set_size_request(TU_ITEM_VIDEO_SIZE.0, TU_ITEM_VIDEO_SIZE.1);
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
                imp.overlay.set_size_request(TU_ITEM_SQUARE_SIZE.0, TU_ITEM_SQUARE_SIZE.1);
                self.set_picture();
            }
            "Actor" | "Person" | "Director" => {
                imp.listlabel.set_text(&item.name());
                imp.label2.set_text(&item.role().unwrap_or("".to_string()));
                imp.overlay.set_size_request(TU_ITEM_POST_SIZE.0, TU_ITEM_POST_SIZE.1);
                self.set_picture();
            }
            "Audio" => {
                imp.listlabel.set_text(&item.name());
                imp.overlay.set_size_request(TU_ITEM_SQUARE_SIZE.0, TU_ITEM_SQUARE_SIZE.1);
                self.set_picture();
            }
            _ => {
                self.set_visible(false);
                warn!("Unknown item type: {}", item_type)
            }
        }
        self.set_tooltip_text(Some(&item.name()));
    }

    fn set_overlay_size(overlay: &gtk::Overlay, width: i32, height: i32) {
        overlay.set_size_request(width, height);
    }

    fn get_image_type_and_tag(&self, item: &TuItem) -> (&str, Option<String>, String) {
        let imp = self.imp();
        if self.poster_type() != PosterType::Poster {
            if let Some(imag_tags) = item.image_tags() {
                match self.poster_type() {
                    PosterType::Banner => {
                        Self::set_overlay_size(
                            &imp.overlay,
                            TU_ITEM_BANNER_SIZE.0,
                            TU_ITEM_BANNER_SIZE.1,
                        );
                        if imag_tags.banner().is_some() {
                            return ("Banner", None, item.id());
                        } else if imag_tags.thumb().is_some() {
                            return ("Thumb", None, item.id());
                        } else if imag_tags.backdrop().is_some() {
                            return ("Backdrop", Some(0.to_string()), item.id());
                        }
                    }
                    PosterType::Backdrop => {
                        Self::set_overlay_size(
                            &imp.overlay,
                            TU_ITEM_VIDEO_SIZE.0,
                            TU_ITEM_VIDEO_SIZE.1,
                        );
                        if imag_tags.backdrop().is_some() {
                            return ("Backdrop", Some(0.to_string()), item.id());
                        } else if imag_tags.thumb().is_some() {
                            return ("Thumb", None, item.id());
                        }
                    }
                    _ => {}
                }
            }
        }
        if item.is_resume() {
            Self::set_overlay_size(&imp.overlay, TU_ITEM_VIDEO_SIZE.0, TU_ITEM_VIDEO_SIZE.1);
            if let Some(parent_thumb_item_id) = item.parent_thumb_item_id() {
                ("Thumb", None, parent_thumb_item_id)
            } else if let Some(parent_backdrop_item_id) = item.parent_backdrop_item_id() {
                ("Backdrop", Some(0.to_string()), parent_backdrop_item_id)
            } else {
                ("Backdrop", Some(0.to_string()), item.id())
            }
        } else if let Some(img_tags) = item.primary_image_item_id() {
            ("Primary", None, img_tags)
        } else {
            ("Primary", None, item.id())
        }
    }

    pub fn set_picture(&self) {
        let imp = self.imp();
        let item = self.item();
        let (image_type, tag, id) = self.get_image_type_and_tag(&item);
        let picture_loader = PictureLoader::new(&id, image_type, tag);
        imp.overlay.set_child(Some(&picture_loader));
    }

    pub fn set_apicture(&self) {
        let imp = self.imp();
        let item = self.item();
        let (image_type, tag, id) = self.get_image_type_and_tag(&item);
        let picture_loader = PictureLoader::new_animated(&id, image_type, tag);
        imp.overlay.set_child(Some(&picture_loader));
    }

    pub fn set_played(&self) {
        let imp = self.imp();
        let item = self.item();
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
        let item = self.item();
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
        let item = self.item();
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
        let item = self.item();
        item.played_percentage()
    }

    pub fn set_played_percentage(&self, percentage: f64) {
        let imp = self.imp();

        let progress = gtk::ProgressBar::builder()
            .fraction(0.)
            .margin_end(3)
            .margin_start(3)
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
        let builder = Builder::from_resource("/moe/tsuna/tsukimi/ui/pop-menu.ui");
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
                imp.obj().insert_action_group("item", imp.obj().set_action().as_ref());
                if let Some(popover) = imp.popover.borrow().as_ref() {
                    popover.set_pointing_to(Some(&Rectangle::new(x as i32, y as i32, 0, 0)));
                    popover.popup();
                };
            }
        ));
        self.add_controller(gesture);
    }

    pub fn set_action(&self) -> Option<gio::SimpleActionGroup> {
        let item_type = self.item().item_type();
        match item_type.as_str() {
            "Movie" | "Series" | "Episode" => self.set_item_action(true, true, true),
            "MusicAlbum" | "BoxSet" | "Tag" | "Genre" | "Views" | "Actor" | "Person"
            | "TvChannel" => self.set_item_action(false, true, true),
            "CollectionFolder" | "UserView" | "Audio" => self.set_item_action(false, false, false),
            _ => None,
        }
    }

    pub fn set_item_action(
        &self, is_playable: bool, is_editable: bool, is_favouritable: bool,
    ) -> Option<gio::SimpleActionGroup> {
        let action_group = gio::SimpleActionGroup::new();

        if is_editable {
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
                                let dialog =
                                    crate::ui::widgets::image_dialog::ImagesDialog::new(&id);
                                crate::insert_editm_dialog!(obj, dialog);
                            }
                        ))
                    }
                ))
                .build()]);
        }

        if IS_ADMIN.load(std::sync::atomic::Ordering::Relaxed) {
            action_group.add_action_entries([gio::ActionEntry::builder("scan")
                .activate(glib::clone!(
                    #[weak(rename_to = obj)]
                    self,
                    move |_, _, _| {
                        spawn(glib::clone!(
                            #[weak]
                            obj,
                            async move {
                                let id = obj.item().id();
                                match spawn_tokio(async move { EMBY_CLIENT.scan(&id).await }).await
                                {
                                    Ok(_) => {
                                        toast!(obj, gettext("Scanning..."));
                                    }
                                    Err(e) => {
                                        toast!(obj, e.to_user_facing());
                                    }
                                }
                            }
                        ))
                    }
                ))
                .build()]);

            if is_editable && !self.item().is_resume() {
                action_group.add_action_entries([gio::ActionEntry::builder("identify")
                    .activate(glib::clone!(
                        #[weak(rename_to = obj)]
                        self,
                        move |_, _, _| {
                            spawn(glib::clone!(
                                #[weak]
                                obj,
                                async move {
                                    let id = obj.item().id();
                                    let type_ = obj.item().item_type();
                                    let dialog =
                                        crate::ui::widgets::identify_dialog::IdentifyDialog::new(
                                            &id, &type_,
                                        );
                                    crate::insert_editm_dialog!(obj, dialog);
                                }
                            ))
                        }
                    ))
                    .build()]);

                action_group.add_action_entries([gio::ActionEntry::builder("refresh")
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
                                        crate::ui::widgets::refresh_dialog::RefreshDialog::new(&id);
                                    crate::insert_editm_dialog!(obj, dialog);
                                }
                            ))
                        }
                    ))
                    .build()]);
            }
        }

        if is_favouritable {
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

        if self.item().is_resume() {
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

    async fn perform_action_inner(id: &str, action: &Action) -> Result<()> {
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
                            let store =
                                selection.model().unwrap().downcast::<gio::ListStore>().unwrap();
                            store.remove(selection.selected());
                        } else if let Some(grid_view) = parent.downcast_ref::<gtk::GridView>() {
                            let selection = grid_view
                                .model()
                                .unwrap()
                                .downcast::<gtk::SingleSelection>()
                                .unwrap();
                            let store =
                                selection.model().unwrap().downcast::<gio::ListStore>().unwrap();
                            store.remove(selection.selected());
                        }
                    }
                ));
            }
        }
        self.gesture();
    }

    pub async fn process_item(
        &self, action: fn(&String) -> Result<(), Box<dyn std::error::Error>>,
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
                toast!(obj, gettext("Success"));
            }
        ));
    }
}
