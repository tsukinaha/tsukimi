use gtk::glib;
use gtk::glib::prelude::*;
use gtk::glib::subclass::prelude::*;
use std::cell::RefCell;

pub mod imp {
    use gtk::glib::Properties;

    use super::*;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::TuItem)]
    pub struct TuItem {
        #[property(get, set)]
        id: RefCell<String>,
        #[property(get, set)]
        name: RefCell<String>,
        #[property(get, set)]
        index_number: RefCell<u32>,
        #[property(get, set)]
        parent_index_number: RefCell<u32>,
        #[property(get, set)]
        series_name: RefCell<String>,
        #[property(get, set)]
        played_percentage: RefCell<f64>,
        #[property(get, set)]
        played: RefCell<bool>,
        #[property(get, set)]
        unplayed_item_count: RefCell<u32>,
        #[property(get, set)]
        is_favorite: RefCell<bool>,
        #[property(get, set)]
        item_type: RefCell<String>,
        #[property(get, set)]
        production_year: RefCell<u32>,
    }

    #[glib::derived_properties]
    impl ObjectImpl for TuItem {}

    #[glib::object_subclass]
    impl ObjectSubclass for TuItem {
        const NAME: &'static str = "TuItem";
        type Type = super::TuItem;
    }

    impl TuItem {
        fn set_id(&self, id: &str) {
            self.id.replace(id.to_string());
        }

        fn set_played_percentage(&self, played_percentage: f64) {
            self.played_percentage.replace(played_percentage);
        }

        fn set_played(&self, played: bool) {
            self.played.replace(played);
        }

        fn set_unplayed_item_count(&self, unplayed_item_count: u32) {
            self.unplayed_item_count.replace(unplayed_item_count);
        }

        fn set_is_favorite(&self, is_favorite: bool) {
            self.is_favorite.replace(is_favorite);
        }

        fn set_name(&self, name: &str) {
            self.name.replace(name.to_string());
        }

        fn set_index_number(&self, index_number: u32) {
            self.index_number.replace(index_number);
        }

        fn set_parent_index_number(&self, parent_index_number: u32) {
            self.parent_index_number.replace(parent_index_number);
        }

        fn set_series_name(&self, series_name: &str) {
            self.series_name.replace(series_name.to_string());
        }

        fn set_item_type(&self, item_type: &str) {
            self.item_type.replace(item_type.to_string());
        }

        fn set_production_year(&self, production_year: u32) {
            self.production_year.replace(production_year);
        }

        fn episode_inside_item(
            &self,
            id: &str,
            played_percentage: f64,
            played: bool,
            name: &str,
            index_number: u32,
        ) {
            self.set_id(id);
            self.set_played_percentage(played_percentage);
            self.set_played(played);
            self.set_name(name);
            self.set_index_number(index_number);
        }

        fn list_post(
            &self,
            id: &str,
            name: &str,
            played: bool,
            unplayed_item_count: u32,
            is_favorite: bool,
            production_year: u32,
        ) {
            self.set_id(id);
            self.set_name(name);
            self.set_played(played);
            self.set_unplayed_item_count(unplayed_item_count);
            self.set_is_favorite(is_favorite);
            self.set_production_year(production_year);
        }
    }
}

glib::wrapper! {
    pub struct TuItem(ObjectSubclass<imp::TuItem>);
}
