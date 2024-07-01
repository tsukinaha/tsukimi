use gtk::glib;
use gtk::prelude::*;

use crate::ui::widgets::item::DropdownList;

pub fn factory(upbind: bool) -> gtk::SignalListItemFactory {
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_bind(move |_, item| {
        let list_item = item
            .downcast_ref::<gtk::ListItem>()
            .expect("Needs to be ListItem");

        if list_item.child().is_some() && !upbind {
            return;
        }

        if let Some(entry) = item
            .downcast_ref::<gtk::ListItem>()
            .expect("Needs to be ListItem")
            .item()
            .and_downcast::<glib::BoxedAnyObject>()
        {
            let dl: std::cell::Ref<DropdownList> = entry.borrow();

            let list_dropdown = crate::ui::widgets::list_dropdown::ListDropdown::new();

            list_dropdown.set_label1(&dl.line1);

            if !upbind {
                list_dropdown.set_label2(&dl.line2);
            }

            list_item.set_child(Some(&list_dropdown));
        }
    });
    factory
}
