use gtk::{prelude::*, SignalListItemFactory};
use crate::ui::provider::tu_object::TuObject;

use super::{
    tu_list_item::{imp::PosterType, TuListItem},
    tu_overview_item::{imp::ViewGroup, TuOverviewItem},
};
pub trait TuItemBuildExt {
    fn tu_item(&self, poster: PosterType) -> &Self;
    fn tu_overview_item(&self, view_group: ViewGroup) -> &Self;
}

impl TuItemBuildExt for SignalListItemFactory {
    fn tu_item(&self, poster: PosterType) -> &Self {
        self.connect_setup(move |_, item| {
            let tu_item = TuListItem::default();
            tu_item.set_poster_type(poster);

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

    fn tu_overview_item(&self, view_group: ViewGroup) -> &Self {
        self.connect_setup(move |_, item| {
            let tu_item = TuOverviewItem::default();
            tu_item.set_view_group(view_group);
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
