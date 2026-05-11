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
use crate::ui::provider::tu_item::TuItem;

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
        #[template_child]
        pub overlay_button_box: TemplateChild<gtk::Box>,

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
        }

        fn dispose(&self) {
            if let Some(popover) = self.popover.borrow().as_ref() {
                popover.unparent();
            };
        }
    }

    impl WidgetImpl for TuListItem {
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            if self.title.is_visible() {
                if let Some((paintable, pic_bounds)) = self.compute_blur_info() {
                    let hover_progress = self
                        .obj()
                        .child()
                        .and_then(|c| c.downcast::<HoverScale>().ok())
                        .map(|hs| hs.animation_progress())
                        .unwrap_or(0.0);

                    if hover_progress > 0.0 {
                        let scale = 1.0 + (MAX_SCALE - 1.0) * hover_progress;
                        let w = self.obj().width() as f32;
                        let h = self.obj().height() as f32;
                        snapshot.save();
                        snapshot.translate(&graphene::Point::new(w / 2.0, h / 2.0));
                        snapshot.scale(scale, scale);
                        snapshot.translate(&graphene::Point::new(-w / 2.0, -h / 2.0));
                        self.draw_blur_backdrop(snapshot, &paintable, &pic_bounds);
                        snapshot.restore();
                    } else {
                        self.draw_blur_backdrop(snapshot, &paintable, &pic_bounds);
                    }
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

        fn draw_blur_backdrop(
            &self, snapshot: &gtk::Snapshot, paintable: &gdk::Paintable,
            pic_bounds: &graphene::Rect,
        ) {
            const CORNER_RADIUS: f32 = 10.0;
            const BLUR_RADIUS: f64 = 20.0;

            let obj = self.obj();
            let widget_w = obj.width() as f32;
            let widget_h = obj.height() as f32;

            if widget_w <= 0.0 || widget_h <= 0.0 {
                return;
            }

            let pic_bottom = pic_bounds.y() + pic_bounds.height();
            let backdrop_y = pic_bottom - CORNER_RADIUS;
            let backdrop_h = widget_h - backdrop_y;

            if backdrop_h <= 0.0 {
                return;
            }

            let backdrop_rect = graphene::Rect::new(0.0, backdrop_y, widget_w, backdrop_h);

            // Sharp top corners -> fills the picture's bottom rounded cutouts.
            let rounded = gsk::RoundedRect::new(
                backdrop_rect,
                graphene::Size::new(0.0, 0.0),
                graphene::Size::new(0.0, 0.0),
                graphene::Size::new(CORNER_RADIUS, CORNER_RADIUS),
                graphene::Size::new(CORNER_RADIUS, CORNER_RADIUS),
            );
            snapshot.push_rounded_clip(&rounded);

            snapshot.push_blur(BLUR_RADIUS);

            let pic_w = pic_bounds.width();
            let pic_h = pic_bounds.height();

            let scale = if pic_w > 0.0 { widget_w / pic_w } else { 1.0 };
            let scaled_h = pic_h * scale;

            let draw_y = widget_h - scaled_h;

            snapshot.save();
            snapshot.translate(&graphene::Point::new(0.0, draw_y));
            snapshot.scale(scale, scale);
            paintable.snapshot(
                snapshot.upcast_ref::<gdk::Snapshot>(),
                pic_w as f64,
                pic_h as f64,
            );
            snapshot.restore();

            snapshot.pop(); // blur

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

            snapshot.append_linear_gradient(
                &backdrop_rect,
                &graphene::Point::new(0.0, backdrop_y),
                &graphene::Point::new(0.0, backdrop_y + backdrop_h),
                &stops,
            );

            snapshot.pop(); // rounded clip
        }
    }

    impl BinImpl for TuListItem {}

    impl TuListItem {
        pub fn set_item(&self, item: TuItem) {
            self.item.replace(item);
            let obj = self.obj();

            while let Some(child) = self.overlay_button_box.first_child() {
                self.overlay_button_box.remove(&child);
            }

            obj.set_up();
            obj.gesture();
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

    fn overlay_button_box(&self) -> gtk::Box {
        self.imp().overlay_button_box.get()
    }
}

impl TuItemMenuPrelude for TuListItem {
    fn popover(&self) -> &RefCell<Option<gtk::PopoverMenu>> {
        &self.imp().popover
    }
}

impl TuItemProgressbarAnimationPrelude for TuListItem {
    fn overlay(&self) -> gtk::Overlay {
        self.imp().overlay.get()
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

    pub fn set_up(&self) {
        let imp = self.imp();
        let item = self.item();

        if item.need_animated_picture() {
            println!("need animated picture");
            self.set_animated_picture();
        } else {
            self.set_picture();
        }

        let (w, h) = item.size_hint();

        imp.overlay.set_size_request(w, h);

        if let Some(p) = item.fmt_percentage() {
            self.set_progress(p);
        }

        if item.has_played_mark() {
            self.set_played();
        }

        if item.has_folder_mark() {
            self.set_folder();
        }

        self.set_tooltip_text(Some(&item.name()));

        let name = item.name();
        if item.need_title() && !name.is_empty() {
            imp.title.set_visible(true);
            imp.title.set_label(&name);
        }
    }
}
