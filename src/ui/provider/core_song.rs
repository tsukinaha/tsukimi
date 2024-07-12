use gtk::glib;
use gtk::glib::prelude::*;
use gtk::glib::subclass::prelude::*;
use std::cell::RefCell;

use crate::ui::widgets::song_widget::State;

pub mod imp {
    use std::cell::Cell;

    use gtk::glib::Properties;

    use crate::ui::widgets::song_widget::State;

    use super::*;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::CoreSong)]
    pub struct CoreSong {
        #[property(get, set)]
        pub id: RefCell<String>,
        #[property(get, set = Self::set_state, explicit_notify, builder(State::default()))]
        pub state: Cell<State>,
        #[property(get, set)]
        pub name: RefCell<String>,
        #[property(get, set)]
        pub artist: RefCell<String>,
        #[property(get, set)]
        pub album_id: RefCell<String>,
        #[property(get, set)]
        pub have_single_track_image: RefCell<bool>,
        #[property(get, set)]
        pub duration: RefCell<u64>,
    }

    #[glib::derived_properties]
    impl ObjectImpl for CoreSong {}

    #[glib::object_subclass]
    impl ObjectSubclass for CoreSong {
        const NAME: &'static str = "CoreSong";
        type Type = super::CoreSong;
    }

    impl CoreSong {
        fn set_state(&self, state: State) {
            if self.state.get() == state {
                return;
            }
            self.state.set(state);
            self.obj().notify_state();
        }
    }
}

glib::wrapper! {
    pub struct CoreSong(ObjectSubclass<imp::CoreSong>);
}

impl CoreSong {
    pub fn new(id: &str) -> CoreSong {
        glib::object::Object::builder()
            .property("id", id)
            .property("state", State::Unplayed)
            .build()
    }
}

impl Default for CoreSong {
    fn default() -> Self {
        Self::new("")
    }
}
