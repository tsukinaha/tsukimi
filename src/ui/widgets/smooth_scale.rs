use gtk::{glib, prelude::*, subclass::prelude::*};

use crate::gstl::MUSIC_PLAYER;

mod imp {

    use std::cell::RefCell;

    use super::*;

    #[derive(Default)]
    pub struct SmoothScale {
        pub timeout: RefCell<Option<glib::source::SourceId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SmoothScale {
        const NAME: &'static str = "SmoothScale";
        type Type = super::SmoothScale;
        type ParentType = gtk::Scale;
    }

    impl ObjectImpl for SmoothScale {
        fn constructed(&self) {
            self.parent_constructed();

            self.obj().duration_changed();
        }
    }
    impl WidgetImpl for SmoothScale {}
    impl RangeImpl for SmoothScale {}
    impl ScaleImpl for SmoothScale {}
}

glib::wrapper! {
    pub struct SmoothScale(ObjectSubclass<imp::SmoothScale>)
        @extends gtk::Widget, gtk::Scale, gtk::Range;
}

impl Default for SmoothScale {
    fn default() -> Self {
        Self::new()
    }
}

impl SmoothScale {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn update_position_callback(&self) -> glib::ControlFlow {
        let position = &MUSIC_PLAYER.position();
        if *position > 0.0 {
            self.set_value(*position);
        }
        self.update_timeout();
        glib::ControlFlow::Continue
    }

    pub fn update_timeout(&self) {
        let width = std::cmp::max(self.width(), 1);
        let timeout_period = std::cmp::min(1000 * 200 / width, 200);
        if let Some(timeout) = self.imp().timeout.borrow_mut().take() {
            glib::source::SourceId::remove(timeout);
        }
        self.imp().timeout.replace(Some(glib::timeout_add_local(
            std::time::Duration::from_millis(timeout_period as u64),
            glib::clone!(@strong self as obj => move || obj.update_position_callback()),
        )));
    }

    pub fn remove_timeout(&self) {
        if let Some(timeout) = self.imp().timeout.borrow_mut().take() {
            glib::source::SourceId::remove(timeout);
        }
    }

    fn duration_changed(&self) {
        self.set_value(0.0);
        self.set_range(0.0, 200.0);
        self.set_increments(300.0, 600.0);
    }
}
