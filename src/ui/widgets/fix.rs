use gtk::{
    prelude::*,
    ScrolledWindow,
};

pub trait ScrolledWindowFixExt {
    fn fix(&self) -> &Self;
}

/// fix scrolledwindow fucking up the vscroll event
impl ScrolledWindowFixExt for ScrolledWindow {
    fn fix(&self) -> &Self {
        let controller_model = self.observe_controllers();
        for i in 0..controller_model.n_items() {
            if let Some(controller) = controller_model
                .item(i)
                .and_downcast_ref::<gtk::EventControllerScroll>()
            {
                controller.set_flags(
                    gtk::EventControllerScrollFlags::HORIZONTAL
                        | gtk::EventControllerScrollFlags::KINETIC,
                );
            }
        }
        self
    }
}
