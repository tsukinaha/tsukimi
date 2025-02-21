use gtk::{
    ScrolledWindow,
    prelude::*,
};

pub trait ScrolledWindowFixExt {
    fn fix(&self) -> &Self;
}

/// fix scrolledwindow fucking up the vscroll event
impl ScrolledWindowFixExt for ScrolledWindow {
    fn fix(&self) -> &Self {
        for object in self.observe_controllers().into_iter() {
            if let Some(controller) = object.ok().and_downcast_ref::<gtk::EventControllerScroll>() {
                controller.set_flags(
                    gtk::EventControllerScrollFlags::HORIZONTAL
                        | gtk::EventControllerScrollFlags::KINETIC,
                );
            }
        }
        self
    }
}
