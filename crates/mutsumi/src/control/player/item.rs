use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

mod imp {
    use std::cell::{Cell, RefCell};

    use super::*;

    #[derive(Debug, Default, glib::Properties)]
    #[properties(wrapper_type = super::PlaylistItem)]
    pub struct PlaylistItem {
        #[property(get, set)]
        pub filename: RefCell<String>,
        #[property(get, set)]
        pub title: RefCell<String>,
        #[property(get, set)]
        pub current: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PlaylistItem {
        const NAME: &'static str = "MutsumiPlaylistItem";
        type Type = super::PlaylistItem;
    }

    #[glib::derived_properties]
    impl ObjectImpl for PlaylistItem {}
}

glib::wrapper! {
    /// A single entry of the player's playlist store.
    pub struct PlaylistItem(ObjectSubclass<imp::PlaylistItem>);
}

impl Default for PlaylistItem {
    fn default() -> Self {
        Self::new()
    }
}

impl PlaylistItem {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn with_values(filename: &str, title: &str, current: bool) -> Self {
        glib::Object::builder()
            .property("filename", filename)
            .property("title", title)
            .property("current", current)
            .build()
    }

    pub fn with_full_uri(uri: &str) -> Self {
        let title = title_from_uri(uri);
        Self::with_values(uri, &title, false)
    }
}

pub fn title_from_uri(uri: &str) -> String {
    if let Ok(parsed) = glib::Uri::parse(uri, glib::UriFlags::NONE) {
        let path = parsed.path();
        if let Some(name) = basename(&path) {
            return name.to_owned();
        }

        if !path.is_empty() {
            return path.to_string();
        }
        if let Some(host) = parsed.host()
            && !host.is_empty()
        {
            return host.to_string();
        }
        return uri.to_owned();
    }

    basename(uri).unwrap_or(uri).to_owned()
}

fn basename(path: &str) -> Option<&str> {
    path.rsplit('/').find(|segment| !segment.is_empty())
}
