use adw::{
    prelude::*,
    subclass::prelude::*,
};
use gtk::{
    gdk,
    glib,
    graphene,
    gsk,
};

pub const MAX_SCALE: f32 = 1.10;
const ANIMATION_DURATION: u32 = 250;
const CORNER_RADIUS: f32 = 10.0;
/// Maximum tilt angle (degrees) for the parallax effect.
const MAX_TILT_ANGLE: f32 = 15.0;
/// Perspective depth (pixels) for the 3D projection.
const PERSPECTIVE_DEPTH: f32 = 800.0;

mod imp {
    use std::cell::{
        Cell,
        OnceCell,
        RefCell,
    };

    use adw::TimedAnimation;
    use glib::clone;

    use super::*;

    type UnderlaySnapshotCallback = Box<dyn Fn(&gtk::Snapshot) + 'static>;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::HoverScale)]
    pub struct HoverScale {
        #[property(get, set, construct, default = MAX_SCALE)]
        pub max_scale: Cell<f32>,
        pub animation: OnceCell<adw::TimedAnimation>,
        /// Optional closure rendered inside the scale transform, before the child.
        pub underlay: RefCell<Option<UnderlaySnapshotCallback>>,

        pub cursor_nx: Cell<f32>,
        pub cursor_ny: Cell<f32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for HoverScale {
        const NAME: &'static str = "HoverScale";
        type Type = super::HoverScale;
        type ParentType = adw::Bin;
    }

    #[glib::derived_properties]
    impl ObjectImpl for HoverScale {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            obj.set_halign(gtk::Align::Center);
            obj.set_overflow(gtk::Overflow::Visible);

            let target = adw::CallbackAnimationTarget::new(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.queue_draw();
                }
            ));

            let animation = adw::TimedAnimation::new(&*obj, 0.0, 1.0, ANIMATION_DURATION, target);
            animation.set_easing(adw::Easing::EaseOutCubic);
            _ = self.animation.set(animation);

            let controller = gtk::EventControllerMotion::new();

            controller.connect_enter(clone!(
                #[weak]
                obj,
                move |_, x, y| {
                    let imp = obj.imp();
                    let w = obj.width() as f64;
                    let h = obj.height() as f64;
                    if w > 0.0 && h > 0.0 {
                        imp.cursor_nx.set((x / w - 0.5) as f32);
                        imp.cursor_ny.set((y / h - 0.5) as f32);
                    }
                    let Some(animation) = imp.animation() else {
                        return;
                    };
                    animation.set_value_from(animation.value());
                    animation.set_value_to(1.0);
                    animation.play();
                }
            ));

            // Update tilt on every pointer move and force a redraw.
            controller.connect_motion(clone!(
                #[weak]
                obj,
                move |_, x, y| {
                    let imp = obj.imp();
                    let w = obj.width() as f64;
                    let h = obj.height() as f64;
                    if w > 0.0 && h > 0.0 {
                        imp.cursor_nx.set((x / w - 0.5) as f32);
                        imp.cursor_ny.set((y / h - 0.5) as f32);
                        obj.queue_draw();
                    }
                }
            ));

            controller.connect_leave(clone!(
                #[weak]
                obj,
                move |_| {
                    let Some(animation) = obj.imp().animation() else {
                        return;
                    };
                    animation.set_value_from(animation.value());
                    animation.set_value_to(0.0);
                    animation.play();
                }
            ));

            obj.add_controller(controller);
        }
    }

    impl WidgetImpl for HoverScale {
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let obj = self.obj();
            let Some(child) = obj.child() else {
                return;
            };

            let Some(animation) = self.animation() else {
                return;
            };

            let progress = animation.value() as f32;

            if progress == 0.0 {
                self.call_underlay(snapshot);
                obj.snapshot_child(&child, snapshot);
                return;
            }

            let w = obj.width() as f32;
            let h = obj.height() as f32;

            if w == 0.0 || h == 0.0 {
                self.call_underlay(snapshot);
                obj.snapshot_child(&child, snapshot);
                return;
            }

            let scale = 1.0 + (self.max_scale.get() - 1.0) * progress;
            let nx = self.cursor_nx.get();
            let ny = self.cursor_ny.get();

            // Tilt angles in degrees, modulated by progress so they ease in/out
            // with the hover animation rather than jumping when the cursor enters.
            let tilt_y_deg = nx * super::MAX_TILT_ANGLE * progress; // yaw  (cursor left/right)
            let tilt_x_deg = -ny * super::MAX_TILT_ANGLE * progress; // pitch (cursor up/down)

            let child_snapshot = gtk::Snapshot::new();
            // push_opacity with a value strictly below 1.0 forces the GPU renderer
            // to composite the entire subtree into a single offscreen FBO before
            // the 3-D transform is applied.  This prevents inter-node seam
            // artifacts (black horizontal line, flickering) that appear when
            // individual render nodes are each transformed independently in 3-D.
            // 0.9999 is visually indistinguishable from 1.0.
            child_snapshot.push_opacity(0.9999);

            self.call_underlay(&child_snapshot);
            obj.snapshot_child(&child, &child_snapshot);

            // Steam-card lighting: cursor above centre → bright overlay that
            // fades downward; cursor below centre → dark overlay that fades upward.
            // ny is in [-0.5, 0.5]: negative = cursor near top.
            {
                let bounds = graphene::Rect::new(0.0, 0.0, w, h);
                let corner = graphene::Size::new(CORNER_RADIUS, CORNER_RADIUS);
                let clip_rect = gsk::RoundedRect::new(bounds, corner, corner, corner, corner);
                let top_pt = graphene::Point::new(w / 2.0, 0.0);
                let bot_pt = graphene::Point::new(w / 2.0, h);

                // Bright layer: active when cursor is above centre.
                let top_alpha = ((-ny * 2.0).max(0.0) * progress * 0.35).min(0.35);
                child_snapshot.push_rounded_clip(&clip_rect);
                child_snapshot.append_linear_gradient(
                    &bounds,
                    &top_pt,
                    &bot_pt,
                    &[
                        gsk::ColorStop::new(0.0, gdk::RGBA::new(1.0, 1.0, 1.0, top_alpha)),
                        gsk::ColorStop::new(1.0, gdk::RGBA::new(1.0, 1.0, 1.0, 0.0)),
                    ],
                );
                child_snapshot.pop();

                // Dark layer: active when cursor is below centre.
                let bot_alpha = ((ny * 2.0).max(0.0) * progress * 0.35).min(0.35);
                child_snapshot.push_rounded_clip(&clip_rect);
                child_snapshot.append_linear_gradient(
                    &bounds,
                    &top_pt,
                    &bot_pt,
                    &[
                        gsk::ColorStop::new(0.0, gdk::RGBA::new(0.0, 0.0, 0.0, 0.0)),
                        gsk::ColorStop::new(1.0, gdk::RGBA::new(0.0, 0.0, 0.0, bot_alpha)),
                    ],
                );
                child_snapshot.pop();
            }

            child_snapshot.pop(); // closes push_opacity

            let Some(node) = child_snapshot.to_node() else {
                return;
            };

            let transform = gsk::Transform::new()
                .translate_3d(&graphene::Point3D::new(w / 2.0, h / 2.0, 0.0))
                .perspective(super::PERSPECTIVE_DEPTH)
                .rotate_3d(tilt_y_deg, &graphene::Vec3::y_axis())
                .rotate_3d(tilt_x_deg, &graphene::Vec3::x_axis())
                .scale(scale, scale)
                .translate_3d(&graphene::Point3D::new(-w / 2.0, -h / 2.0, 0.0));

            snapshot.append_node(gsk::TransformNode::new(&node, Some(&transform)));
        }
    }

    impl HoverScale {
        fn call_underlay(&self, snapshot: &gtk::Snapshot) {
            if let Some(f) = self.underlay.borrow().as_ref() {
                f(snapshot);
            }
        }
    }

    impl BinImpl for HoverScale {}

    impl HoverScale {
        pub fn animation(&self) -> Option<&TimedAnimation> {
            self.animation.get()
        }
    }
}

glib::wrapper! {
    pub struct HoverScale(ObjectSubclass<imp::HoverScale>)
        @extends gtk::Widget, adw::Bin, @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl HoverScale {
    pub fn new() -> Self {
        glib::Object::new()
    }

    /// Returns the current animation progress (0.0 = idle, 1.0 = fully hovered).
    /// Used by parent widgets that need to replicate the same scale transform.
    pub fn animation_progress(&self) -> f32 {
        self.imp()
            .animation
            .get()
            .map(|a| a.value() as f32)
            .unwrap_or(0.0)
    }

    pub fn set_underlay(&self, f: impl Fn(&gtk::Snapshot) + 'static) {
        self.imp().underlay.replace(Some(Box::new(f)));
    }
}

impl Default for HoverScale {
    fn default() -> Self {
        Self::new()
    }
}
