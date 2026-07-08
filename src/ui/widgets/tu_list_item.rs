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
        tu_item_banner_size,
        tu_item_video_size,
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
        gdk,
        glib,
        graphene,
        gsk,
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

    pub struct BackdropNodeCache {
        node: gsk::RenderNode,
        /// Validity key: (width_px, height_px, is_dark, paintable_ptr)
        key: (i32, i32, bool, usize),
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
        pub content_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub scaled_title_slot: TemplateChild<gtk::Box>,
        #[template_child]
        pub plain_title_slot: TemplateChild<gtk::Box>,
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
        #[template_child]
        pub focus_title_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub focus_title: TemplateChild<gtk::Label>,

        pub backdrop_cache: RefCell<Option<BackdropNodeCache>>,
        pub is_dark: Cell<bool>,
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

            let style_manager = adw::StyleManager::default();
            self.is_dark.set(style_manager.is_dark());
            let obj = self.obj();

            self.update_item_card_style();

            style_manager.connect_dark_notify(glib::clone!(
                #[weak]
                obj,
                move |sm| {
                    obj.imp().is_dark.set(sm.is_dark());
                    obj.queue_draw();
                }
            ));

            self.hover_scale.set_underlay(glib::clone!(
                #[weak]
                obj,
                move |snapshot| {
                    obj.imp().draw_backdrop(snapshot);
                }
            ));

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
                Some("item-text-display"),
                glib::clone!(
                    #[weak]
                    obj,
                    move |_, _| obj.update_title()
                ),
            );

            SETTINGS.connect_changed(
                Some("item-card-style"),
                glib::clone!(
                    #[weak]
                    obj,
                    move |_, _| {
                        obj.imp().update_item_card_style();
                        obj.queue_draw();
                    }
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

    impl TuListItem {
        fn update_item_card_style(&self) {
            let integrated = SETTINGS.item_card_style_is_integrated();
            let title_box = self.title_box.get();
            let target = if integrated {
                self.scaled_title_slot.get()
            } else {
                self.plain_title_slot.get()
            };

            if title_box.parent().as_ref() != Some(target.upcast_ref()) {
                if let Some(parent) = title_box.parent()
                    && let Ok(parent) = parent.downcast::<gtk::Box>()
                {
                    parent.remove(&title_box);
                }
                target.append(&title_box);
            }

            if integrated {
                self.content_box.add_css_class("tulistitem");
            } else {
                self.content_box.remove_css_class("tulistitem");
            }
            self.update_title_slot_visibility(integrated, self.title.get_visible());
        }

        pub(super) fn update_title_slot_visibility(&self, integrated: bool, has_title: bool) {
            self.scaled_title_slot.set_visible(integrated && has_title);
            self.plain_title_slot.set_visible(!integrated && has_title);
        }

        fn draw_backdrop(&self, snapshot: &gtk::Snapshot) {
            if !SETTINGS.item_card_style_is_integrated() || !self.title_box.is_visible() {
                return;
            }

            let Some((paintable, pic_bounds)) = self.compute_blur_info() else {
                return;
            };

            let hover_scale = self.hover_scale.get();
            let w = pic_bounds.width() as i32;
            let h = hover_scale.height();
            let key = (w, h, self.is_dark.get(), paintable.as_ptr() as usize);

            let stale = self
                .backdrop_cache
                .borrow()
                .as_ref()
                .is_none_or(|c| c.key != key);
            if stale {
                *self.backdrop_cache.borrow_mut() =
                    self.build_backdrop_node(&paintable, &pic_bounds, w as f32, h as f32);
            }

            if let Some(cache) = self.backdrop_cache.borrow().as_ref() {
                snapshot.append_node(&cache.node);
            }
        }

        fn compute_blur_info(&self) -> Option<(gdk::Paintable, graphene::Rect)> {
            let hover_scale = self.hover_scale.get();
            let picture_loader = self.overlay.child()?.downcast::<PictureLoader>().ok()?;
            let paintable = picture_loader.imp().picture.paintable()?;
            let pic_bounds = picture_loader.compute_bounds(&hover_scale)?;

            Some((paintable, pic_bounds))
        }

        fn build_backdrop_node(
            &self, paintable: &gdk::Paintable, pic_bounds: &graphene::Rect, widget_w: f32,
            widget_h: f32,
        ) -> Option<BackdropNodeCache> {
            const CORNER_RADIUS: f32 = 10.0;
            const BLUR_RADIUS: f64 = 20.0;

            let pic_bottom = pic_bounds.y() + pic_bounds.height();
            let backdrop_y = pic_bottom - CORNER_RADIUS;
            let backdrop_h = widget_h - backdrop_y;

            if backdrop_h <= 0.0 {
                return None;
            }

            let backdrop_rect = graphene::Rect::new(0.0, backdrop_y, widget_w, backdrop_h);
            let rounded = gsk::RoundedRect::new(
                backdrop_rect,
                graphene::Size::new(0.0, 0.0),
                graphene::Size::new(0.0, 0.0),
                graphene::Size::new(CORNER_RADIUS, CORNER_RADIUS),
                graphene::Size::new(CORNER_RADIUS, CORNER_RADIUS),
            );

            let sub = gtk::Snapshot::new();
            sub.push_rounded_clip(&rounded);
            sub.push_blur(BLUR_RADIUS);

            let pic_w = pic_bounds.width();
            let pic_h = pic_bounds.height();
            let scale = if pic_w > 0.0 { widget_w / pic_w } else { 1.0 };
            let draw_y = widget_h - pic_h * scale;

            sub.save();
            sub.translate(&graphene::Point::new(0.0, draw_y));
            sub.scale(scale, scale);
            paintable.snapshot(
                sub.upcast_ref::<gdk::Snapshot>(),
                pic_w as f64,
                pic_h as f64,
            );
            sub.restore();
            sub.pop(); // blur

            let stops = if self.is_dark.get() {
                [
                    gsk::ColorStop::new(0.0, gdk::RGBA::new(0.0, 0.0, 0.0, 0.35)),
                    gsk::ColorStop::new(1.0, gdk::RGBA::new(0.0, 0.0, 0.0, 0.45)),
                ]
            } else {
                [
                    gsk::ColorStop::new(0.0, gdk::RGBA::new(1.0, 1.0, 1.0, 0.15)),
                    gsk::ColorStop::new(1.0, gdk::RGBA::new(1.0, 1.0, 1.0, 0.25)),
                ]
            };
            sub.append_linear_gradient(
                &backdrop_rect,
                &graphene::Point::new(0.0, backdrop_y),
                &graphene::Point::new(0.0, backdrop_y + backdrop_h),
                &stops,
            );
            sub.pop(); // rounded clip

            let node = sub.to_node()?;
            let key = (
                widget_w as i32,
                widget_h as i32,
                self.is_dark.get(),
                paintable.as_ptr() as usize,
            );
            Some(BackdropNodeCache { node, key })
        }
    }

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
        let item = self.item();
        let has_title = if let Some(title) = item.list_item_title() {
            imp.title.set_text(&title);
            imp.title.set_visible(true);
            if let Some(subtitle) = item.list_item_subtitle() {
                imp.subtitle.set_text(&subtitle);
                imp.subtitle.set_visible(true);
            } else {
                imp.subtitle.set_visible(false);
            }
            true
        } else {
            imp.title.set_visible(false);
            imp.subtitle.set_visible(false);
            false
        };

        imp.update_title_slot_visibility(SETTINGS.item_card_style_is_integrated(), has_title);
    }

    pub fn refresh_item(&self) {
        let imp = self.imp();
        let item = self.item();

        self.set_picture();

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

    pub fn set_poster_focused(&self, focused: bool) {
        let imp = self.imp();
        let focused_now = imp.content_box.has_css_class("poster-focused");
        if focused == focused_now {
            return;
        }
        if focused {
            imp.content_box.add_css_class("poster-focused");
            imp.hover_scale.set_highlighted(true);
            if crate::tv::is_tv_mode_active() {
                let title = self
                    .item()
                    .list_item_title()
                    .unwrap_or_else(|| self.item().name());
                imp.focus_title.set_text(&title);
                imp.focus_title_revealer.set_reveal_child(true);
            }
        } else {
            imp.content_box.remove_css_class("poster-focused");
            imp.hover_scale.set_highlighted(false);
            imp.focus_title_revealer.set_reveal_child(false);
        }
    }

    fn size_hint(&self) -> (i32, i32) {
        match self.poster_type() {
            PosterType::Banner => tu_item_banner_size(),
            PosterType::Backdrop => tu_item_video_size(),
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
