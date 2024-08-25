use gtk::glib;
use gtk::{prelude::*, SignalListItemFactory};

use crate::client::structs::SimpleListItem;
use crate::ui::provider::tu_item::TuItem;
use crate::ui::provider::tu_object::TuObject;

use super::tu_list_item::TuListItem;
pub trait TuItemBuildExt {
    fn tu_item(&self) -> &Self;
}

impl TuItemBuildExt for SignalListItemFactory {
    fn tu_item(&self) -> &Self {
        self.connect_setup(move |_, item| {
            let tu_item = TuListItem::default();
            let list_item = item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem");
            list_item.set_child(Some(&tu_item));
            list_item
                .property_expression("item")
                .chain_property::<TuObject>("item")
                .bind(&tu_item, "item", gtk::Widget::NONE);
        });
        self
    }
}
