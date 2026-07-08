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
use imp::ViewGroup;

use super::{
    tu_item::{
        TuItemBasic,
        TuItemMenuPrelude,
        TuItemOverlay,
        TuItemOverlayPrelude,
        TuItemProgressbarAnimation,
        TuItemProgressbarAnimationPrelude,
    },
    tu_list_item::imp::PosterType,
    utils::{
        run_time_ticks_to_label,
        tu_item_post_size,
        tu_item_video_size,
    },
};
use crate::ui::provider::tu_item::TuItem;

pub mod imp {
    use std::cell::{
        Cell,
        RefCell,
    };

    use adw::subclass::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        PopoverMenu,
        glib,
        prelude::*,
    };

    #[derive(Default, Hash, Eq, PartialEq, Clone, Copy, glib::Enum, Debug)]
    #[repr(u32)]
    #[enum_type(name = "ViewGroup")]
    pub enum ViewGroup {
        ListView,
        #[default]
        EpisodesView,
    }

    use crate::ui::{
        provider::tu_item::TuItem,
        widgets::{
            hover_scale::HoverScale,
            picture_loader::PictureLoader,
            tu_item::TuItemAction,
        },
    };

    // Object holding the state
    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/tu_overview_item.ui")]
    #[properties(wrapper_type = super::TuOverviewItem)]
    pub struct TuOverviewItem {
        #[property(get, set = Self::set_item)]
        pub item: RefCell<TuItem>,
        #[template_child]
        pub overview: TemplateChild<gtk::Inscription>,
        #[template_child]
        pub inline_overview: TemplateChild<gtk::Label>,
        #[property(get, set = Self::set_view_group, builder(ViewGroup::default()))]
        pub view_group: Cell<ViewGroup>,
        pub popover: RefCell<Option<PopoverMenu>>,
        #[template_child]
        pub listlabel: TemplateChild<gtk::Label>,
        #[template_child]
        pub label2: TemplateChild<gtk::Label>,
        #[template_child]
        pub progress_bar: TemplateChild<gtk::ProgressBar>,
        #[template_child]
        pub overlay: TemplateChild<gtk::Overlay>,
        #[template_child]
        pub time_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub detail_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub overlay_button_box: TemplateChild<gtk::Box>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TuOverviewItem {
        const NAME: &'static str = "TuOverviewItem";
        type Type = super::TuOverviewItem;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            HoverScale::ensure_type();
            PictureLoader::ensure_type();
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for TuOverviewItem {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.add_controller(obj.gesture_click());
        }

        fn dispose(&self) {
            self.dispose_template();
            if let Some(popover) = self.popover.borrow().as_ref() {
                popover.unparent();
            };
        }
    }

    impl WidgetImpl for TuOverviewItem {}

    impl BinImpl for TuOverviewItem {}

    impl TuOverviewItem {
        pub fn set_item(&self, item: TuItem) {
            self.item.replace(item);
            self.obj().set_up();
        }

        fn set_view_group(&self, view_group: ViewGroup) {
            self.view_group.set(view_group);
        }
    }
}

glib::wrapper! {
    pub struct TuOverviewItem(ObjectSubclass<imp::TuOverviewItem>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget, adw::Bin ,adw::NavigationPage,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl TuItemBasic for TuOverviewItem {
    fn item(&self) -> TuItem {
        self.item()
    }
}

impl TuItemOverlayPrelude for TuOverviewItem {
    fn overlay(&self) -> gtk::Overlay {
        self.imp().overlay.get()
    }

    fn poster_type_ext(&self) -> PosterType {
        match self.view_group() {
            ViewGroup::EpisodesView => PosterType::NoRequest,
            ViewGroup::ListView => PosterType::Poster,
        }
    }
}

impl TuItemMenuPrelude for TuOverviewItem {
    fn popover(&self) -> &RefCell<Option<gtk::PopoverMenu>> {
        &self.imp().popover
    }
}

impl TuItemProgressbarAnimationPrelude for TuOverviewItem {
    fn progress_bar(&self) -> gtk::ProgressBar {
        self.imp().progress_bar.get()
    }
}

#[template_callbacks]
impl TuOverviewItem {
    pub fn new(item: TuItem, is_resume: bool) -> Self {
        Object::builder()
            .property("item", item)
            .property("is_resume", is_resume)
            .build()
    }

    pub fn default() -> Self {
        Object::new()
    }

    pub fn set_up(&self) {
        let imp = self.imp();
        let item = self.item();
        match self.view_group() {
            ViewGroup::EpisodesView => {
                imp.listlabel.set_text(&format!(
                    "S{}E{}: {}",
                    item.parent_index_number(),
                    item.index_number(),
                    item.name()
                ));
                imp.overlay
                    .set_size_request(tu_item_video_size().0, tu_item_video_size().1);
                if let Some(premiere_date) = item.premiere_date() {
                    imp.time_label.set_visible(true);
                    imp.time_label
                        .set_text(&premiere_date.format("%Y-%m-%d").unwrap_or_default());
                }
                imp.label2
                    .set_text(&run_time_ticks_to_label(item.run_time_ticks()));
                imp.overview.set_text(Some(
                    &item
                        .overview()
                        .unwrap_or("No Inscription".to_string())
                        .replace(['\n', '\r'], " "),
                ));
                self.set_progress(self.item().played_percentage());
            }
            ViewGroup::ListView => {
                imp.overview.set_visible(false);
                imp.inline_overview.set_visible(true);
                imp.inline_overview.set_text(
                    &item
                        .overview()
                        .unwrap_or_default()
                        .replace(['\n', '\r'], " "),
                );
                let item_type = item.item_type();
                if item_type == "Episode" {
                    imp.listlabel.set_text(&format!(
                        "S{}E{}: {}",
                        item.parent_index_number(),
                        item.index_number(),
                        item.name()
                    ));
                    imp.overlay
                        .set_size_request(tu_item_video_size().0, tu_item_video_size().1);
                } else {
                    imp.listlabel.set_text(&item.name());
                    imp.overlay
                        .set_size_request(tu_item_post_size().0, tu_item_post_size().1);
                }
                let year = if item.production_year() != 0 {
                    item.production_year().to_string()
                } else {
                    String::default()
                };
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
                                imp.label2.set_text(&format!("{end_year}"));
                            }
                        } else {
                            imp.label2.set_text(&format!("{year} - Unknown"));
                        }
                    }
                } else {
                    imp.label2.set_text(&year);
                }
                if let Some(tagline) = item.tagline() {
                    imp.time_label.set_text(&tagline);
                }
            }
        }
        self.set_picture();
        self.set_tooltip_text(Some(&item.name()));
    }
}
