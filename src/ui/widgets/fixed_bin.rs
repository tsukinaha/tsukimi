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

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::FixedBin)]
    pub struct FixedBin {
        #[property(get, set = Self::set_child, explicit_notify, nullable)]
        pub child: RefCell<Option<gtk::Widget>>,
        #[property(get, set = Self::set_fixed_width, minimum = 0, default = 0)]
        pub fixed_width: Cell<i32>,
        #[property(get, set = Self::set_fixed_height, minimum = 0, default = 0)]
        pub fixed_height: Cell<i32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FixedBin {
        const NAME: &'static str = "FixedBin";
        type Type = super::FixedBin;
        type ParentType = gtk::Widget;
    }

    #[glib::derived_properties]
    impl ObjectImpl for FixedBin {
        fn dispose(&self) {
            if let Some(child) = self.child.borrow_mut().take() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for FixedBin {
        fn compute_expand(&self, hexpand: &mut bool, vexpand: &mut bool) {
            *hexpand = false;
            *vexpand = false;
        }

        fn measure(&self, orientation: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            let fixed_size = match orientation {
                gtk::Orientation::Horizontal => self.fixed_width.get(),
                gtk::Orientation::Vertical => self.fixed_height.get(),
                _ => unreachable!(),
            };

            if fixed_size > 0 {
                return (fixed_size, fixed_size, -1, -1);
            }

            self.child
                .borrow()
                .as_ref()
                .map(|child| child.measure(orientation, for_size))
                .unwrap_or((0, 0, -1, -1))
        }

        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            let Some(child) = self.child.borrow().as_ref().cloned() else {
                return;
            };
            let child_width = match self.fixed_width.get() {
                fixed if fixed > 0 => fixed,
                _ => width,
            };
            let child_height = match self.fixed_height.get() {
                fixed if fixed > 0 => fixed,
                _ => height,
            };
            child.allocate(child_width, child_height, baseline, None);
        }
    }

    impl FixedBin {
        fn set_fixed_width(&self, width: i32) {
            if self.fixed_width.replace(width) != width {
                self.obj().queue_resize();
            }
        }

        fn set_fixed_height(&self, height: i32) {
            if self.fixed_height.replace(height) != height {
                self.obj().queue_resize();
            }
        }

        fn set_child(&self, child: Option<&gtk::Widget>) {
            if self.child.borrow().as_ref() == child {
                return;
            }
            if let Some(old_child) = self.child.borrow_mut().take() {
                old_child.unparent();
            }
            if let Some(child) = child {
                child.set_parent(&*self.obj());
                self.child.replace(Some(child.clone()));
            }
            self.obj().notify_child();
        }
    }
}

glib::wrapper! {
    pub struct FixedBin(ObjectSubclass<imp::FixedBin>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl FixedBin {
    pub fn set_fixed_size(&self, width: i32, height: i32) {
        self.set_fixed_width(width.max(0));
        self.set_fixed_height(height.max(0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_size_overrides_child_natural_size_and_extra_allocation() {
        gtk::init().expect("GTK test display must be available");

        let child = gtk::Box::new(gtk::Orientation::Vertical, 0);
        child.set_size_request(400, 500);
        let bin: FixedBin = glib::Object::builder().property("child", &child).build();
        bin.set_fixed_size(80, 90);

        assert_eq!(
            bin.measure(gtk::Orientation::Horizontal, -1),
            (80, 80, -1, -1)
        );
        assert_eq!(
            bin.measure(gtk::Orientation::Vertical, 80),
            (90, 90, -1, -1)
        );
        bin.allocate(200, 220, -1, None);
        assert_eq!((child.width(), child.height()), (80, 90));

        bin.set_fixed_size(80, 0);
        assert_eq!(
            bin.measure(gtk::Orientation::Horizontal, -1),
            (80, 80, -1, -1)
        );
        assert_eq!(
            bin.measure(gtk::Orientation::Vertical, 80),
            (500, 500, -1, -1)
        );
        bin.allocate(200, 220, -1, None);
        assert_eq!((child.width(), child.height()), (80, 220));

        let builder = gtk::Builder::from_string(
            r#"
            <interface>
              <object class="FixedBin" id="bin">
                <property name="fixed-width">70</property>
                <property name="fixed-height">75</property>
                <property name="child">
                  <object class="GtkOverlay"/>
                </property>
              </object>
            </interface>
            "#,
        );
        let xml_bin = builder.object::<FixedBin>("bin").unwrap();
        assert!(
            xml_bin
                .child()
                .is_some_and(|child| child.is::<gtk::Overlay>())
        );
        assert_eq!(
            xml_bin.measure(gtk::Orientation::Horizontal, -1),
            (70, 70, -1, -1)
        );
        assert_eq!(
            xml_bin.measure(gtk::Orientation::Vertical, 70),
            (75, 75, -1, -1)
        );
    }
}
