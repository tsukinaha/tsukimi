use adw::{
    prelude::*,
    subclass::prelude::*,
};
use gtk::{
    glib,
    graphene,
};

pub const MAX_SCALE: f32 = 1.08;
const ANIMATION_DURATION: u32 = 250;

mod imp {
    use std::cell::OnceCell;

    use glib::clone;

    use super::*;

    #[derive(Debug, Default)]
    pub struct HoverScale {
        pub animation: OnceCell<adw::TimedAnimation>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for HoverScale {
        const NAME: &'static str = "HoverScale";
        type Type = super::HoverScale;
        type ParentType = adw::Bin;
    }

    impl ObjectImpl for HoverScale {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

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
                move |_, _, _| {
                    let animation = obj.imp().animation.get().unwrap();
                    animation.set_value_from(animation.value());
                    animation.set_value_to(1.0);
                    animation.play();
                }
            ));

            controller.connect_leave(clone!(
                #[weak]
                obj,
                move |_| {
                    let animation = obj.imp().animation.get().unwrap();
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

            let progress = self.animation.get().unwrap().value() as f32;

            if progress == 0.0 {
                obj.snapshot_child(&child, snapshot);
                return;
            }

            let w = obj.width() as f32;
            let h = obj.height() as f32;

            if w == 0.0 || h == 0.0 {
                obj.snapshot_child(&child, snapshot);
                return;
            }

            let scale = 1.0 + (super::MAX_SCALE - 1.0) * progress;

            snapshot.translate(&graphene::Point::new(w / 2.0, h / 2.0));
            snapshot.scale(scale, scale);
            snapshot.translate(&graphene::Point::new(-w / 2.0, -h / 2.0));

            obj.snapshot_child(&child, snapshot);
        }
    }

    impl BinImpl for HoverScale {}
}

glib::wrapper! {
    pub struct HoverScale(ObjectSubclass<imp::HoverScale>)
        @extends gtk::Widget, adw::Bin;
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
}

impl Default for HoverScale {
    fn default() -> Self {
        Self::new()
    }
}
