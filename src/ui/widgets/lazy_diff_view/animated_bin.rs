use std::cell::{
    Cell,
    RefCell,
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
        const NAME: &'static str = "MutsumiLazyAnimatedBin";
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
        fn compute_expand(&self, hexpand: &mut bool, vexpand: &mut bool) {
            if let Some(child) = self.child.borrow().as_ref() {
                *hexpand = child.compute_expand(gtk::Orientation::Horizontal);
                *vexpand = child.compute_expand(gtk::Orientation::Vertical);
            }
        }

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

    pub fn child(&self) -> Option<gtk::Widget> {
        self.imp().child.borrow().clone()
    }

    pub fn set_offset(&self, offset_x: f64, offset_y: f64) {
        let imp = self.imp();
        imp.offset_x.set(offset_x);
        imp.offset_y.set(offset_y);
        self.queue_draw();
    }
}
