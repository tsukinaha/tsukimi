use ::gtk::prelude::*;

pub fn fix(scrolledwindow: gtk::ScrolledWindow) -> gtk::ScrolledWindow {
    let controller = scrolledwindow.observe_controllers();
    let count = controller.n_items();
    for i in 0..count {
        let item = controller.item(i).unwrap();
        if item.is::<gtk::EventControllerScroll>() {
            let controller = item.downcast::<gtk::EventControllerScroll>().unwrap();
            controller.set_flags(
                gtk::EventControllerScrollFlags::HORIZONTAL
                    | gtk::EventControllerScrollFlags::KINETIC,
            );
        }
    }
    scrolledwindow
}
