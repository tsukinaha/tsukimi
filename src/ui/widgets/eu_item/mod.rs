mod eu_list_item;
mod eu_object;
mod eu_property;

pub use eu_list_item::EuListItem;
pub use eu_object::EuObject;
pub use eu_property::EuItem;
use gtk::prelude::*;

pub trait EuListItemExt {
    fn eu_item(&self) -> &Self;
}

impl EuListItemExt for gtk::SignalListItemFactory {
    fn eu_item(&self) -> &Self {
        self.connect_setup(move |_, list_item| {
            let eu_item = EuListItem::default();

            let list_item = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem");
            list_item.set_child(Some(&eu_item));
            list_item
                .property_expression("item")
                .chain_property::<EuObject>("item")
                .bind(&eu_item, "item", gtk::Widget::NONE);
        });
        self
    }
}
