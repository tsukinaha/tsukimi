use gtk::glib;
use gtk::glib::prelude::*;
use gtk::glib::subclass::prelude::*;
use gtk::prelude::ListModelExt;
use std::cell::RefCell;

use crate::client::structs::SimpleListItem;

use super::tu_item::TuItem;

pub mod imp {
    use gtk::glib::Properties;

    use crate::ui::provider::tu_item::TuItem;

    use super::*;

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

impl TuObject {
    pub fn new(item: &TuItem) -> Self {
        glib::Object::builder().property("item", item).build()
    }

    pub fn from_simple(latest: &SimpleListItem, poster: Option<&str>) -> Self {
        let tu_item = TuItem::from_simple(latest, poster);
        TuObject::new(&tu_item)
    }

    pub fn activate<T>(&self, listview: &T)
    where
        T: glib::clone::Downgrade + gtk::prelude::IsA<gtk::Widget>,
    {
        let item = self.item();
        let poster = self.poster();
        item.activate(listview, poster);
    }
}
