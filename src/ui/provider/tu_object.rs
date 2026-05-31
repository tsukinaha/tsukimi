use std::cell::RefCell;

use gtk::{
    glib::{
        self,
        prelude::*,
        subclass::prelude::*,
    },
};

use super::tu_item::TuItem;
use crate::{
    client::structs::SimpleListItem,
    ui::widgets::{
        lazy_diff_view::OnSameKey,
        tu_list_item::TuListItem,
    },
};

pub mod imp {
    use gtk::glib::Properties;

    use super::*;
    use crate::ui::provider::tu_item::TuItem;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::TuObject)]
    pub struct TuObject {
        #[property(get, set)]
        item: RefCell<TuItem>,
        #[property(get, set)]
        poster: RefCell<Option<String>>,
    }

    #[glib::derived_properties]
    impl ObjectImpl for TuObject {}

    #[glib::object_subclass]
    impl ObjectSubclass for TuObject {
        const NAME: &'static str = "TuObject";
        type Type = super::TuObject;
    }
}

glib::wrapper! {
    pub struct TuObject(ObjectSubclass<imp::TuObject>);
}

impl OnSameKey for TuObject {
    fn on_same_key(&self, widget: &gtk::Widget) {
        let Some(list_item) = widget.downcast_ref::<TuListItem>() else {
            tracing::error!("Failed to downcast to TuListItem");
            return;
        };

        let Some(percentage) = self.item().fmt_percentage() else {
            return;
        };

        list_item.set_progress(percentage / 100.);
    }
}

impl TuObject {
    pub fn new(item: TuItem) -> Self {
        glib::Object::builder().property("item", item).build()
    }

    pub fn from_simple(item: SimpleListItem) -> Self {
        TuObject::new(TuItem::from_simple(item))
    }

    pub fn activate<T>(&self, listview: &T)
    where
        T: glib::clone::Downgrade + gtk::prelude::IsA<gtk::Widget>,
    {
        let item = self.item();
        item.activate(listview);
    }
}
