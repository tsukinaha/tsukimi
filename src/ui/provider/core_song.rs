use gtk::glib;
use gtk::glib::prelude::*;
use gtk::glib::subclass::prelude::*;
use std::cell::RefCell;

use crate::ui::widgets::song_widget::State;

pub mod imp {
    use std::cell::Cell;

    use gst::glib::Value;
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
            println!("Setting state to {:?}", state);
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
        glib::object::Object::builder().property("id", id).property("state", State::UNPLAYED).build()
    }
}

impl Default for CoreSong {
    fn default() -> Self {
        Self::new("")
    }
}
