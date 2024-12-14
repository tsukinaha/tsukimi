use std::cell::RefCell;

use adw::prelude::*;
use gettextrs::gettext;
use glib::Object;
use gtk::{
    gio,
    glib,
    glib::subclass::types::ObjectSubclassIsExt,
    template_callbacks,
};
use imp::PosterType;
use tracing::warn;

use super::{
    tu_item::{
        TuItemBasic,
        TuItemMenuPrelude,
        TuItemOverlay,
        TuItemOverlayPrelude,
    },
    utils::{
        TU_ITEM_POST_SIZE,
        TU_ITEM_SQUARE_SIZE,
        TU_ITEM_VIDEO_SIZE,
    },
};
use crate::{
    toast,
    ui::provider::tu_item::TuItem,
    utils::spawn,
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
        widgets::{
            picture_loader::PictureLoader,
            tu_item::TuItemAction,
        },
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
        #[property(get, set, default = false)]
        pub can_direct_play: Cell<bool>,
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

impl TuItemBasic for TuListItem {
    fn item(&self) -> TuItem {
        self.item()
    }
}

impl TuItemOverlayPrelude for TuListItem {
    fn overlay(&self) -> gtk::Overlay {
        self.imp().overlay.get()
    }

    fn poster_type_ext(&self) -> PosterType {
        self.poster_type()
    }
}

impl TuItemMenuPrelude for TuListItem {
    fn popover(&self) -> &RefCell<Option<gtk::PopoverMenu>> {
        &self.imp().popover
    }
}

#[template_callbacks]
impl TuListItem {
    pub fn new(item: TuItem) -> Self {
        Object::builder().property("item", item).build()
    }

    pub fn default() -> Self {
        Object::new()
    }

    #[template_callback]
    async fn on_play_clicked(&self) {
        let item = self.item();
        let item_type = item.item_type();
        match item_type.as_str() {
            "TvChannel" => {
                item.play_tvchannel(self);
            }
            "Audio" => {
                item.play_single_audio(self);
            }
            "Video" | "MusicVideo" | "AdultVideo" | "Movie" | "Episode" => {
                toast!(self, gettext("Waiting for mediasource ..."));
                item.play_video(self).await;
            }
            "Series" => {
                toast!(self, gettext("Waiting for mediasource ..."));
                item.play_series(self).await;
            }
            "MusicAlbum" => {
                toast!(self, gettext("Waiting for mediasource ..."));
                item.play_album(self).await;
            }
            _ => {
                toast!(self, "Not implemented");
            }
        }
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
                imp.overlay
                    .set_size_request(TU_ITEM_POST_SIZE.0, TU_ITEM_POST_SIZE.1);
                self.set_can_direct_play(true);
                self.set_picture();
                self.set_played();
                if item.is_resume() {
                    self.set_played_percentage(self.get_played_percentage());
                    return;
                }
                self.set_rating();
            }
            "Video" | "MusicVideo" | "AdultVideo" => {
                imp.listlabel.set_text(&item.name());
                imp.label2.set_visible(false);
                imp.overlay
                    .set_size_request(TU_ITEM_VIDEO_SIZE.0, TU_ITEM_VIDEO_SIZE.1);
                self.set_can_direct_play(true);
                self.set_picture();
            }
            "TvChannel" => {
                imp.listlabel.set_text(&format!(
                    "{} - {}",
                    item.name(),
                    item.program_name().unwrap_or_default()
                ));
                imp.overlay
                    .set_size_request(TU_ITEM_VIDEO_SIZE.0, TU_ITEM_VIDEO_SIZE.1);
                self.set_can_direct_play(true);
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
                imp.overlay
                    .set_size_request(TU_ITEM_VIDEO_SIZE.0, TU_ITEM_VIDEO_SIZE.1);
                self.set_animated_picture();
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
                        imp.label2
                            .set_text(&format!("{} - {}", year, gettext("Present")));
                    } else if status == "Ended" {
                        if let Some(end_date) = item.end_date() {
                            let end_year = end_date.year();
                            if end_year != year.parse::<i32>().unwrap_or_default() {
                                imp.label2
                                    .set_text(&format!("{} - {}", year, end_date.year()));
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
                imp.overlay
                    .set_size_request(TU_ITEM_POST_SIZE.0, TU_ITEM_POST_SIZE.1);
                self.set_can_direct_play(true);
                self.set_picture();
                self.set_played();
                self.set_count();
                self.set_rating();
            }
            "BoxSet" => {
                imp.listlabel.set_text(&item.name());
                imp.label2.set_visible(false);
                imp.overlay
                    .set_size_request(TU_ITEM_POST_SIZE.0, TU_ITEM_POST_SIZE.1);
                self.set_picture();
            }
            "Tag" | "Genre" => {
                imp.overlay
                    .set_size_request(TU_ITEM_SQUARE_SIZE.0, TU_ITEM_SQUARE_SIZE.1);
                imp.listlabel.set_text(&item.name());
                imp.label2.set_visible(false);
                self.set_picture();
            }
            "Episode" => {
                if let Some(series_name) = item.series_name() {
                    imp.listlabel.set_text(&series_name);
                    imp.label2.set_text(&format!(
                        "S{}E{}: {}",
                        item.parent_index_number(),
                        item.index_number(),
                        item.name()
                    ));
                } else {
                    imp.listlabel.set_text(&item.name());
                    imp.label2.set_visible(false);
                }
                imp.overlay
                    .set_size_request(TU_ITEM_VIDEO_SIZE.0, TU_ITEM_VIDEO_SIZE.1);
                self.set_can_direct_play(true);
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
                imp.overlay
                    .set_size_request(TU_ITEM_SQUARE_SIZE.0, TU_ITEM_SQUARE_SIZE.1);
                self.set_can_direct_play(true);
                self.set_picture();
            }
            "Actor" | "Person" | "Director" | "Writer" | "Producer" => {
                imp.listlabel.set_text(&item.name());
                imp.label2.set_text(&item.role().unwrap_or("".to_string()));
                imp.overlay
                    .set_size_request(TU_ITEM_POST_SIZE.0, TU_ITEM_POST_SIZE.1);
                self.set_picture();
            }
            "Audio" => {
                imp.listlabel.set_text(&item.name());
                imp.overlay
                    .set_size_request(TU_ITEM_SQUARE_SIZE.0, TU_ITEM_SQUARE_SIZE.1);
                self.set_can_direct_play(true);
                self.set_picture();
            }
            "Folder" => {
                imp.overlay
                    .set_size_request(TU_ITEM_SQUARE_SIZE.0, TU_ITEM_SQUARE_SIZE.1);
                imp.listlabel.set_text(&item.name());
                imp.label2.set_visible(false);
                self.set_picture();
                self.set_folder();
            }
            "Season" => {
                imp.listlabel.set_text(&item.name());
                if let Some(premiere_date) = item.premiere_date() {
                    imp.label2
                        .set_text(&premiere_date.format("%Y-%m-%d").unwrap());
                }
                imp.overlay
                    .set_size_request(TU_ITEM_POST_SIZE.0, TU_ITEM_POST_SIZE.1);
                self.set_picture();
                self.set_played();
                self.set_count();
                self.set_rating();
            }
            _ => {
                imp.overlay
                    .set_size_request(TU_ITEM_SQUARE_SIZE.0, TU_ITEM_SQUARE_SIZE.1);
                imp.listlabel.set_text(&item.name());
                imp.label2.set_visible(false);
                self.set_picture();
                warn!("Unknown item type: {}", item_type)
            }
        }
        self.set_tooltip_text(Some(&item.name()));
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
}
