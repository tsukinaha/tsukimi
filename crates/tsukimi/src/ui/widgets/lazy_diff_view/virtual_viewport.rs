use std::cell::{
    Cell,
    RefCell,
};

use gtk::{
    Adjustment,
    ScrollablePolicy,
    glib::{
        self,
        signal::SignalHandlerId,
    },
    prelude::*,
    subclass::prelude::*,
};

#[derive(Clone)]
struct ChildLayout {
    widget: gtk::Widget,
    x: Cell<f64>,
    y: Cell<f64>,
    width: Cell<i32>,
    height: Cell<i32>,
}

mod imp {
    use super::*;

    pub struct VirtualViewport {
        pub(super) children: RefCell<Vec<ChildLayout>>,
        pub(super) content_width: Cell<i32>,
        pub(super) content_height: Cell<i32>,
        pub(super) allocated_width: Cell<i32>,
        pub(super) allocated_height: Cell<i32>,
        pub(super) hadjustment: RefCell<Adjustment>,
        pub(super) vadjustment: RefCell<Adjustment>,
        pub(super) hadjustment_handler: RefCell<Option<SignalHandlerId>>,
        pub(super) vadjustment_handler: RefCell<Option<SignalHandlerId>>,
        pub(super) hscroll_policy: Cell<ScrollablePolicy>,
        pub(super) vscroll_policy: Cell<ScrollablePolicy>,
    }

    impl Default for VirtualViewport {
        fn default() -> Self {
            Self {
                children: RefCell::new(Vec::new()),
                content_width: Cell::new(0),
                content_height: Cell::new(0),
                allocated_width: Cell::new(0),
                allocated_height: Cell::new(0),
                hadjustment: RefCell::new(Adjustment::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0)),
                vadjustment: RefCell::new(Adjustment::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0)),
                hadjustment_handler: RefCell::new(None),
                vadjustment_handler: RefCell::new(None),
                hscroll_policy: Cell::new(ScrollablePolicy::Minimum),
                vscroll_policy: Cell::new(ScrollablePolicy::Minimum),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VirtualViewport {
        const NAME: &'static str = "MutsumiLazyVirtualViewport";
        type Type = super::VirtualViewport;
        type ParentType = gtk::Widget;
        type Interfaces = (gtk::Scrollable,);
    }

    impl ObjectImpl for VirtualViewport {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().connect_adjustment_handlers();
        }

        fn dispose(&self) {
            if let Some(handler) = self.hadjustment_handler.borrow_mut().take() {
                self.hadjustment.borrow().disconnect(handler);
            }
            if let Some(handler) = self.vadjustment_handler.borrow_mut().take() {
                self.vadjustment.borrow().disconnect(handler);
            }

            for child in self.children.borrow_mut().drain(..) {
                child.widget.unparent();
            }
        }

        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: std::sync::OnceLock<Vec<glib::ParamSpec>> =
                std::sync::OnceLock::new();
            PROPERTIES.get_or_init(|| {
                vec![
                    glib::ParamSpecOverride::for_interface::<gtk::Scrollable>("hadjustment"),
                    glib::ParamSpecOverride::for_interface::<gtk::Scrollable>("vadjustment"),
                    glib::ParamSpecOverride::for_interface::<gtk::Scrollable>("hscroll-policy"),
                    glib::ParamSpecOverride::for_interface::<gtk::Scrollable>("vscroll-policy"),
                ]
            })
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "hadjustment" => {
                    let adjustment = value
                        .get::<Option<Adjustment>>()
                        .ok()
                        .flatten()
                        .unwrap_or_else(|| Adjustment::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0));
                    self.obj().set_hadjustment_impl(adjustment);
                }
                "vadjustment" => {
                    let adjustment = value
                        .get::<Option<Adjustment>>()
                        .ok()
                        .flatten()
                        .unwrap_or_else(|| Adjustment::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0));
                    self.obj().set_vadjustment_impl(adjustment);
                }
                "hscroll-policy" => self.hscroll_policy.set(value.get().unwrap()),
                "vscroll-policy" => self.vscroll_policy.set(value.get().unwrap()),
                name => unreachable!("unknown property {name}"),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "hadjustment" => self.hadjustment.borrow().to_value(),
                "vadjustment" => self.vadjustment.borrow().to_value(),
                "hscroll-policy" => self.hscroll_policy.get().to_value(),
                "vscroll-policy" => self.vscroll_policy.get().to_value(),
                name => unreachable!("unknown property {name}"),
            }
        }
    }

    impl WidgetImpl for VirtualViewport {
        fn compute_expand(&self, hexpand: &mut bool, vexpand: &mut bool) {
            for child in self.children.borrow().iter() {
                *hexpand |= child.widget.compute_expand(gtk::Orientation::Horizontal);
                *vexpand |= child.widget.compute_expand(gtk::Orientation::Vertical);
            }
        }

        fn measure(&self, orientation: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            let mut minimum = 0;
            let mut natural = 0;
            for child in self.children.borrow().iter() {
                let cross_size = match orientation {
                    gtk::Orientation::Horizontal if child.height.get() >= 0 => child.height.get(),
                    gtk::Orientation::Vertical if child.width.get() >= 0 => child.width.get(),
                    _ => for_size,
                };
                let (child_min, child_nat, _, _) = child.widget.measure(orientation, cross_size);
                minimum = minimum.max(child_min);
                natural = natural.max(child_nat);
            }
            (minimum, natural, -1, -1)
        }

        fn size_allocate(&self, width: i32, height: i32, _baseline: i32) {
            self.allocated_width.set(width);
            self.allocated_height.set(height);
            self.obj().update_adjustments();

            let scroll_x = self.hadjustment.borrow().value();
            let scroll_y = self.vadjustment.borrow().value();

            for child in self.children.borrow().iter() {
                let child_width = if child.width.get() < 0 {
                    width
                } else {
                    child.width.get()
                };
                let child_height = if child.height.get() < 0 {
                    height
                } else {
                    child.height.get()
                };
                child.widget.allocate(
                    child_width,
                    child_height,
                    -1,
                    Some(
                        gtk::gsk::Transform::new().translate(&gtk::graphene::Point::new(
                            (child.x.get() - scroll_x) as f32,
                            (child.y.get() - scroll_y) as f32,
                        )),
                    ),
                );
            }
        }
    }

    impl ScrollableImpl for VirtualViewport {}
}

glib::wrapper! {
    pub struct VirtualViewport(ObjectSubclass<imp::VirtualViewport>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Scrollable;
}

impl VirtualViewport {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn add_child(
        &self, child: &impl IsA<gtk::Widget>, x: f64, y: f64, width: i32, height: i32,
    ) {
        let child = child.as_ref();
        if child.parent().as_ref() == Some(self.upcast_ref()) {
            return;
        }

        child.set_parent(self);
        self.imp().children.borrow_mut().push(ChildLayout {
            widget: child.clone(),
            x: Cell::new(x),
            y: Cell::new(y),
            width: Cell::new(width),
            height: Cell::new(height),
        });
        self.queue_resize();
    }

    pub fn remove_child(&self, child: &impl IsA<gtk::Widget>) {
        let child_widget = child.as_ref();
        let mut children = self.imp().children.borrow_mut();
        if let Some(index) = children
            .iter()
            .position(|layout| layout.widget == *child_widget)
        {
            let layout = children.remove(index);
            layout.widget.unparent();
            drop(children);
            self.queue_resize();
        }
    }

    pub fn move_child(
        &self, child: &impl IsA<gtk::Widget>, x: f64, y: f64, width: i32, height: i32,
    ) {
        let child_widget = child.as_ref();
        for layout in self.imp().children.borrow().iter() {
            if layout.widget == *child_widget {
                let changed = layout.x.replace(x) != x
                    || layout.y.replace(y) != y
                    || layout.width.replace(width) != width
                    || layout.height.replace(height) != height;
                if changed {
                    self.queue_allocate();
                }
                return;
            }
        }
    }

    pub fn set_content_size(&self, width: i32, height: i32) {
        let width = width.max(0);
        let height = height.max(0);
        let width_changed = self.imp().content_width.replace(width) != width;
        let height_changed = self.imp().content_height.replace(height) != height;
        if width_changed || height_changed {
            self.update_adjustments();
            self.queue_resize();
        }
    }

    fn set_hadjustment_impl(&self, adjustment: Adjustment) {
        let imp = self.imp();
        if *imp.hadjustment.borrow() == adjustment {
            return;
        }

        if let Some(handler) = imp.hadjustment_handler.borrow_mut().take() {
            imp.hadjustment.borrow().disconnect(handler);
        }
        *imp.hadjustment.borrow_mut() = adjustment;
        self.connect_hadjustment_handler();
        self.update_adjustments();
        self.notify("hadjustment");
    }

    fn set_vadjustment_impl(&self, adjustment: Adjustment) {
        let imp = self.imp();
        if *imp.vadjustment.borrow() == adjustment {
            return;
        }

        if let Some(handler) = imp.vadjustment_handler.borrow_mut().take() {
            imp.vadjustment.borrow().disconnect(handler);
        }
        *imp.vadjustment.borrow_mut() = adjustment;
        self.connect_vadjustment_handler();
        self.update_adjustments();
        self.notify("vadjustment");
    }

    fn connect_adjustment_handlers(&self) {
        self.connect_hadjustment_handler();
        self.connect_vadjustment_handler();
    }

    fn connect_hadjustment_handler(&self) {
        let imp = self.imp();
        let weak_self = self.downgrade();
        let handler = imp.hadjustment.borrow().connect_value_changed(move |_| {
            if let Some(viewport) = weak_self.upgrade() {
                viewport.queue_allocate();
            }
        });
        *imp.hadjustment_handler.borrow_mut() = Some(handler);
    }

    fn connect_vadjustment_handler(&self) {
        let imp = self.imp();
        let weak_self = self.downgrade();
        let handler = imp.vadjustment.borrow().connect_value_changed(move |_| {
            if let Some(viewport) = weak_self.upgrade() {
                viewport.queue_allocate();
            }
        });
        *imp.vadjustment_handler.borrow_mut() = Some(handler);
    }

    fn update_adjustments(&self) {
        let imp = self.imp();
        configure_adjustment(
            &imp.hadjustment.borrow(),
            imp.content_width.get(),
            imp.allocated_width.get(),
        );
        configure_adjustment(
            &imp.vadjustment.borrow(),
            imp.content_height.get(),
            imp.allocated_height.get(),
        );
    }
}

fn configure_adjustment(adjustment: &Adjustment, content_size: i32, page_size: i32) {
    let page_size = page_size.max(0) as f64;
    let upper = (content_size.max(0) as f64).max(page_size);
    let max_value = (upper - page_size).max(0.0);
    let value = adjustment.value().clamp(0.0, max_value);
    let step_increment = (page_size * 0.1).max(1.0);
    let page_increment = (page_size * 0.9).max(1.0);

    adjustment.configure(value, 0.0, upper, step_increment, page_increment, page_size);
}
