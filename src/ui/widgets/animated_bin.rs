use std::cell::{
    Cell,
    RefCell,
};

use adw::{
    CallbackAnimationTarget,
    Easing,
    TimedAnimation,
    prelude::AnimationExt,
};
use gtk::{
    glib,
    prelude::*,
    subclass::prelude::*,
};

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct AnimatedBin {
        pub child: RefCell<Option<gtk::Widget>>,
        pub offset_x: Cell<f64>,
        pub offset_y: Cell<f64>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AnimatedBin {
        const NAME: &'static str = "TsukimiAnimatedBin";
        type Type = super::AnimatedBin;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for AnimatedBin {
        fn dispose(&self) {
            if let Some(child) = self.child.borrow_mut().take() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for AnimatedBin {
        fn measure(&self, orientation: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            self.child
                .borrow()
                .as_ref()
                .map(|child| child.measure(orientation, for_size))
                .unwrap_or((0, 0, -1, -1))
        }

        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            if let Some(child) = self.child.borrow().as_ref() {
                child.allocate(width, height, baseline, None);
            }
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let Some(child) = self.child.borrow().as_ref().cloned() else {
                return;
            };

            snapshot.save();
            snapshot.translate(&gtk::graphene::Point::new(
                self.offset_x.get() as f32,
                self.offset_y.get() as f32,
            ));
            self.obj().snapshot_child(&child, snapshot);
            snapshot.restore();
        }
    }
}

glib::wrapper! {
    pub struct AnimatedBin(ObjectSubclass<imp::AnimatedBin>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl AnimatedBin {
    pub fn new(child: &impl IsA<gtk::Widget>) -> Self {
        let obj: Self = glib::Object::new();
        obj.set_child(Some(child));
        obj
    }

    pub fn set_child(&self, child: Option<&impl IsA<gtk::Widget>>) {
        let imp = self.imp();
        if let Some(old_child) = imp.child.borrow_mut().take() {
            old_child.unparent();
        }

        if let Some(child) = child {
            let child = child.as_ref();
            child.set_parent(self);
            *imp.child.borrow_mut() = Some(child.clone());
        }
    }

    pub fn set_offset(&self, offset_x: f64, offset_y: f64) {
        let imp = self.imp();
        imp.offset_x.set(offset_x);
        imp.offset_y.set(offset_y);
        self.queue_draw();
    }
}

/// Sets a translation offset on `container` along its layout axis.
///
/// Horizontal orientation → offset_x; Vertical orientation → offset_y.
pub fn set_axis_offset(container: &AnimatedBin, orientation: gtk::Orientation, offset: f64) {
    match orientation {
        gtk::Orientation::Horizontal => container.set_offset(offset, 0.0),
        gtk::Orientation::Vertical => container.set_offset(0.0, offset),
        _ => container.set_offset(0.0, 0.0),
    }
}

/// Runs an Adwaita timed animation, returning the animation handle.
///
/// The returned `TimedAnimation` is kept alive until its `done` signal fires.
pub fn animate(
    widget: &impl IsA<gtk::Widget>, from: f64, to: f64, duration: u32, easing: Easing,
    callback: impl Fn(f64) + 'static,
) -> TimedAnimation {
    let target = CallbackAnimationTarget::new(callback);
    let animation = TimedAnimation::new(widget, from, to, duration, target);
    animation.set_easing(easing);

    eprintln!(
        "[AnimatedBin] animate: from={:.1}, to={:.1}, duration={}ms",
        from, to, duration
    );

    let keep_alive = std::rc::Rc::new(RefCell::new(Some(animation.clone())));
    let keep_alive_on_done = keep_alive.clone();
    animation.connect_done(move |_| {
        keep_alive_on_done.borrow_mut().take();
    });

    animation.play();
    animation
}

/// Linear interpolation between two `f64` values.
pub fn lerp_f64(from: f64, to: f64, progress: f64) -> f64 {
    from + (to - from) * progress
}
