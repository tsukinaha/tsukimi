use std::cell::RefCell;

use adw::prelude::*;
use glib::Object;
use gtk::{
    gio,
    glib,
    glib::subclass::types::ObjectSubclassIsExt,
    template_callbacks,
};
use imp::PosterType;

use super::tu_item::{
    TuItemBasic,
    TuItemMenuPrelude,
    TuItemOverlay,
    TuItemOverlayPrelude,
    TuItemProgressbarAnimation,
    TuItemProgressbarAnimationPrelude,
};
use crate::ui::{
    SETTINGS,
    provider::tu_item::TuItem,
    widgets::utils::{
        TU_ITEM_BANNER_SIZE,
        TU_ITEM_VIDEO_SIZE,
    },
};

pub mod imp {
    use std::cell::{
        Cell,
        RefCell,
    };

    use adw::{
        prelude::*,
        subclass::prelude::*,
    };
    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        PopoverMenu,
        glib,
    };

    use crate::ui::{
        SETTINGS,
        provider::tu_item::TuItem,
        widgets::{
            hover_scale::HoverScale,
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
        NoRequest,
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
        pub title_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub title: TemplateChild<gtk::Label>,
        #[template_child]
        pub subtitle: TemplateChild<gtk::Label>,
        #[template_child]
        pub overlay: TemplateChild<gtk::Overlay>,
        #[template_child]
        pub progress_bar: TemplateChild<gtk::ProgressBar>,
        #[template_child]
        pub played_mark: TemplateChild<gtk::Button>,
        #[template_child]
        pub folder_mark: TemplateChild<gtk::Button>,
        #[template_child]
        pub direct_play_button: TemplateChild<gtk::Button>,

        #[template_child]
        pub hover_scale: TemplateChild<HoverScale>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TuListItem {
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

    #[glib::derived_properties]
    impl ObjectImpl for TuListItem {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            obj.add_controller(obj.gesture_click());
            obj.set_has_tooltip(true);
            obj.connect_query_tooltip(|obj, _, _, _, tooltip| {
                let name = obj.item().name();
                if name.is_empty() {
                    return false;
                }
                tooltip.set_text(Some(&name));
                true
            });

            SETTINGS.connect_changed(
                Some("full-item-display-mode"),
                glib::clone!(
                    #[weak]
                    obj,
                    move |_, _| obj.update_title()
                ),
            );
        }

        fn dispose(&self) {
            if let Some(popover) = self.popover.borrow().as_ref() {
                popover.unparent();
            };
        }
    }

    impl WidgetImpl for TuListItem {}

    impl BinImpl for TuListItem {}

    impl TuListItem {
        pub fn set_item(&self, item: TuItem) {
            let obj = self.obj();
            self.item.replace(item);
            obj.refresh_item();
        }
    }
}

glib::wrapper! {
    pub struct TuListItem(ObjectSubclass<imp::TuListItem>)
        @extends adw::Bin, gtk::Widget,
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

impl TuItemProgressbarAnimationPrelude for TuListItem {
    fn progress_bar(&self) -> gtk::ProgressBar {
        self.imp().progress_bar.get()
    }
}

impl Default for TuListItem {
    fn default() -> Self {
        Self::new(TuItem::default())
    }
}

#[template_callbacks]
impl TuListItem {
    pub fn new(item: TuItem) -> Self {
        Object::builder().property("item", item).build()
    }

    fn update_title(&self) {
        let imp = self.imp();
        let full_display_mode = SETTINGS.full_item_display_mode();
        let lines = if full_display_mode { 1 } else { -1 };
        let ellipsize = if full_display_mode {
            gtk::pango::EllipsizeMode::End
        } else {
            gtk::pango::EllipsizeMode::None
        };
        imp.title.set_lines(lines);
        imp.title.set_ellipsize(ellipsize);
        imp.subtitle.set_lines(lines);
        imp.subtitle.set_ellipsize(ellipsize);

        if let Some((title, subtitle)) = self.item().list_item_text() {
            imp.title.set_text(&title);
            if let Some(subtitle) = subtitle {
                imp.subtitle.set_text(&subtitle);
                imp.subtitle.set_visible(true);
            } else {
                imp.subtitle.set_visible(false);
            }
            imp.title_box.set_visible(true);
        } else {
            imp.title_box.set_visible(false);
            imp.subtitle.set_visible(false);
        }
    }

    pub fn refresh_item(&self) {
        let imp = self.imp();
        let item = self.item();

        if item.need_animated_picture() {
            self.set_animated_picture()
        } else {
            self.set_picture()
        };

        let (w, h) = self.size_hint();

        imp.overlay.set_size_request(w, h);

        if let Some(p) = item.fmt_percentage() {
            self.set_progress(p);
        } else {
            self.clear_progress();
        }

        imp.played_mark.set_visible(item.has_played_mark());

        imp.folder_mark.set_visible(item.has_folder_mark());

        imp.direct_play_button
            .set_visible(item.has_direct_play_mark());

        self.update_title();
    }

    pub fn unbind_item(&self) {
        let imp = self.imp();

        if let Some(child) = imp.overlay.child() {
            super::picture_loader::PictureLoader::reset_in(&child);
        }
    }

    fn size_hint(&self) -> (i32, i32) {
        match self.poster_type() {
            PosterType::Banner => TU_ITEM_BANNER_SIZE,
            PosterType::Backdrop => TU_ITEM_VIDEO_SIZE,
            _ => self.item().size_hint(),
        }
    }

    #[template_callback]
    async fn on_play_clicked(&self) {
        let item = self.item();
        if !item.can_direct_play() {
            return;
        }
        item.play_video(self).await;
    }
}
