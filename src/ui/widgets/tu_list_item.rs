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

use super::tu_item::{
    PROGRESSBAR_ANIMATION_DURATION,
    TuItemBasic,
    TuItemMenuPrelude,
    TuItemOverlay,
    TuItemOverlayPrelude,
};
use crate::{
    ui::{
        GlobalToast,
        provider::tu_item::TuItem,
        widgets::utils::{
            TU_ITEM_BANNER_SIZE,
            TU_ITEM_VIDEO_SIZE,
        },
    },
    utils::spawn,
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
            hover_scale::{
                HoverScale,
                MAX_SCALE,
            },
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
        backdrop_y: f32,
        backdrop_h: f32,
        widget_w: f32,
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
        pub title: TemplateChild<gtk::Label>,
        #[template_child]
        pub overlay: TemplateChild<gtk::Overlay>,
        #[property(get, set = Self::set_progress)]
        pub progress: Cell<f64>,
        #[template_child]
        pub played_mark: TemplateChild<gtk::Button>,
        #[template_child]
        pub folder_mark: TemplateChild<gtk::Button>,
        #[template_child]
        pub direct_play_button: TemplateChild<gtk::Button>,

        #[template_child]
        pub hover_scale: TemplateChild<HoverScale>,

        pub hover_scale_progress: Cell<f32>,
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

            let obj = self.obj().downgrade();
            style_manager.connect_dark_notify(move |sm| {
                if let Some(obj) = obj.upgrade() {
                    obj.imp().is_dark.set(sm.is_dark());
                    obj.queue_draw();
                }
            });

            if let Some(animation) = self.hover_scale.imp().animation.get() {
                let obj = self.obj().downgrade();
                animation.connect_value_notify(move |a| {
                    if let Some(this) = obj.upgrade() {
                        this.imp().hover_scale_progress.set(a.value() as f32);
                    }
                });
            }
        }

        fn dispose(&self) {
            if let Some(popover) = self.popover.borrow().as_ref() {
                popover.unparent();
            };
        }
    }

    impl WidgetImpl for TuListItem {
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            if !self.title.is_visible() {
                self.parent_snapshot(snapshot);
                return;
            }

            let Some((paintable, pic_bounds)) = self.compute_blur_info() else {
                self.parent_snapshot(snapshot);
                return;
            };

            let obj = self.obj();
            let w = pic_bounds.width() as i32; // Use picture width for caching, so that the cache can be reused when the widget is resized within the same picture size
            let h = obj.height();

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

            let cache_ref = self.backdrop_cache.borrow();
            if let Some(cache) = cache_ref.as_ref() {
                let hover_progress = self.hover_scale_progress.get();
                let progress = self.progress.get() as f32;

                let alpha = if self.is_dark.get() { 0.2 } else { 0.4 };

                if hover_progress > 0.0 {
                    let scale = 1.0 + (MAX_SCALE - 1.0) * hover_progress;
                    let wf = w as f32;
                    let hf = h as f32;
                    snapshot.save();
                    snapshot.translate(&graphene::Point::new(wf / 2.0, hf / 2.0));
                    snapshot.scale(scale, scale);
                    snapshot.translate(&graphene::Point::new(-wf / 2.0, -hf / 2.0));
                    snapshot.append_node(&cache.node);
                    Self::draw_progress_fill(snapshot, cache, progress, alpha);
                    snapshot.restore();
                } else {
                    snapshot.append_node(&cache.node);
                    Self::draw_progress_fill(snapshot, cache, progress, alpha);
                }
            }

            self.parent_snapshot(snapshot);
        }
    }

    impl TuListItem {
        fn compute_blur_info(&self) -> Option<(gdk::Paintable, graphene::Rect)> {
            let obj = self.obj();

            let picture_loader = self.overlay.child()?.downcast::<PictureLoader>().ok()?;

            let paintable = picture_loader.imp().picture.paintable()?;

            let pic_bounds = picture_loader.compute_bounds(&*obj)?;

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
                    gsk::ColorStop::new(0.0, gdk::RGBA::new(0.0, 0.0, 0.0, 0.55)),
                    gsk::ColorStop::new(1.0, gdk::RGBA::new(0.0, 0.0, 0.0, 0.75)),
                ]
            } else {
                [
                    gsk::ColorStop::new(0.0, gdk::RGBA::new(1.0, 1.0, 1.0, 0.15)),
                    gsk::ColorStop::new(1.0, gdk::RGBA::new(1.0, 1.0, 1.0, 0.35)),
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
            Some(BackdropNodeCache {
                node,
                backdrop_y,
                backdrop_h,
                widget_w,
                key,
            })
        }

        fn draw_progress_fill(
            snapshot: &gtk::Snapshot, cache: &BackdropNodeCache, progress: f32, alpha: f32,
        ) {
            if progress <= 0.0 {
                return;
            }

            let Ok(base) = gdk::RGBA::parse(SETTINGS.accent_color_code()) else {
                return;
            };

            const CR: f32 = 10.0;
            let fill_w = (cache.widget_w * progress).min(cache.widget_w);
            let fill_rect = graphene::Rect::new(0.0, cache.backdrop_y, fill_w, cache.backdrop_h);
            let fill_clip = gsk::RoundedRect::new(
                graphene::Rect::new(0.0, cache.backdrop_y, cache.widget_w, cache.backdrop_h),
                graphene::Size::new(0.0, 0.0),
                graphene::Size::new(0.0, 0.0),
                graphene::Size::new(CR, CR),
                graphene::Size::new(CR, CR),
            );

            snapshot.push_rounded_clip(&fill_clip);
            snapshot.append_color(
                &gdk::RGBA::new(base.red(), base.green(), base.blue(), alpha),
                &fill_rect,
            );
            snapshot.pop();
        }
    }

    impl BinImpl for TuListItem {}

    impl TuListItem {
        pub fn set_item(&self, item: TuItem) {
            let obj = self.obj();
            self.item.replace(item);

            obj.item_setted();
            obj.gesture();
        }

        pub fn set_progress(&self, progress: f64) {
            self.progress.set(progress);
            self.obj().set_progress_anim(progress);
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

    fn set_progress_anim(&self, percentage: f64) {
        let weak = self.downgrade();
        let start_value = self.imp().progress.get();

        let target = adw::CallbackAnimationTarget::new(move |_| {
            if let Some(this) = weak.upgrade() {
                this.queue_draw();
            }
        });

        let animation = adw::TimedAnimation::builder()
            .duration(PROGRESSBAR_ANIMATION_DURATION)
            .widget(self)
            .target(&target)
            .easing(adw::Easing::EaseOutQuart)
            .value_from(start_value)
            .value_to(percentage)
            .build();

        spawn(glib::clone!(
            #[weak]
            animation,
            async move {
                animation.play();
            }
        ));
    }

    pub fn item_setted(&self) {
        let imp = self.imp();
        let item = self.item();

        if let Some(picture_loader) = item.loaded_picture_loader() {
            imp.overlay.set_child(Some(&picture_loader));
        } else {
            let picture_loader = if item.need_animated_picture() {
                self.set_animated_picture()
            } else {
                self.set_picture()
            };

            item.set_loaded_picture_loader(picture_loader);
        }

        let (w, h) = self.size_hint();

        imp.overlay.set_size_request(w, h);

        if let Some(p) = item.fmt_percentage() {
            self.set_progress(p / 100.);
        } else {
            self.set_progress(0.);
        }

        imp.played_mark.set_visible(item.has_played_mark());

        imp.folder_mark.set_visible(item.has_folder_mark());

        imp.direct_play_button.set_visible(item.has_direct_play_mark());

        self.set_tooltip_text(Some(&item.name()));

        if let Some(title) = item.list_item_title() {
            imp.title.set_text(&title);
            imp.title.set_visible(true);
        } else {
            imp.title.set_visible(false);
        }
    }

    pub fn unbind_item(&self) {
        self.imp().overlay.set_child(None::<&gtk::Widget>);
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

        self.toast(gettext("Waiting for mediasource ..."));
        item.play_video(self).await;
    }
}
