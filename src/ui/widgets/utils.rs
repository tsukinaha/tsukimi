use gtk::{
    prelude::*,
    SignalListItemFactory,
};
use once_cell::sync::Lazy;

use super::{
    tu_list_item::{
        imp::PosterType,
        TuListItem,
    },
    tu_overview_item::{
        imp::ViewGroup,
        TuOverviewItem,
    },
};
use crate::ui::{
    models::SETTINGS,
    provider::tu_object::TuObject,
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

            let list_item = item.downcast_ref::<gtk::ListItem>().expect("Needs to be ListItem");
            list_item.set_child(Some(&tu_item));
            list_item.property_expression("item").chain_property::<TuObject>("item").bind(
                &tu_item,
                "item",
                gtk::Widget::NONE,
            );
        });
        self
    }

    fn tu_overview_item(&self, view_group: ViewGroup) -> &Self {
        self.connect_setup(move |_, item| {
            let tu_item = TuOverviewItem::default();
            tu_item.set_view_group(view_group);
            let list_item = item.downcast_ref::<gtk::ListItem>().expect("Needs to be ListItem");
            list_item.set_child(Some(&tu_item));
            list_item.property_expression("item").chain_property::<TuObject>("item").bind(
                &tu_item,
                "item",
                gtk::Widget::NONE,
            );
        });
        self
    }
}

pub static TU_ITEM_POST_SIZE: Lazy<(i32, i32)> = Lazy::new(|| init_size(167, 260));
pub static TU_ITEM_VIDEO_SIZE: Lazy<(i32, i32)> = Lazy::new(|| init_size(250, 141));
pub static TU_ITEM_SQUARE_SIZE: Lazy<(i32, i32)> = Lazy::new(|| init_size(190, 190));
pub static TU_ITEM_BANNER_SIZE: Lazy<(i32, i32)> = Lazy::new(|| init_size(375, 70));

fn init_size(width: i32, height: i32) -> (i32, i32) {
    let scale = SETTINGS.post_scale();
    ((width as f64 * scale) as i32, (height as f64 * scale) as i32)
}
