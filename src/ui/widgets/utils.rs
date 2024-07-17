use gtk::glib;
use gtk::{prelude::*, SignalListItemFactory};

use crate::client::structs::SimpleListItem;
use crate::ui::provider::tu_item::TuItem;

use super::tu_list_item::TuListItem;
pub trait TuItemBuildExt {
    fn tu_item(&self, is_resume: bool) -> &Self;
}

impl TuItemBuildExt for SignalListItemFactory {
    fn tu_item(&self, is_resume: bool) -> &Self {
        self.connect_bind(move |_, item| {
            let list_item = item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem");
            let entry = item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem")
                .item()
                .and_downcast::<glib::BoxedAnyObject>()
                .expect("Needs to be BoxedAnyObject");
            let item: std::cell::Ref<SimpleListItem> = entry.borrow();
            if list_item.child().is_none() {
                let tu_item = TuItem::from_simple(&item, None);
                let list_child = TuListItem::new(tu_item, &item.latest_type, is_resume);
                list_item.set_child(Some(&list_child));
            }
        });
        self
    }
}
