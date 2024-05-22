use gtk::{glib, prelude::*, subclass::prelude::*};

use crate::gstl::MUSIC_PLAYER;

mod imp {

    use std::cell::RefCell;

    use crate::gstl::list::Player;

    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::SmoothScale)]
    pub struct SmoothScale {
        pub timeout: RefCell<Option<glib::source::SourceId>>,
        #[property(get, set = Self::set_player, explicit_notify, nullable)]
        pub player: glib::WeakRef<Player>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SmoothScale {
        const NAME: &'static str = "SmoothScale";
        type Type = super::SmoothScale;
        type ParentType = gtk::Scale;
    }

    #[glib::derived_properties]
    impl ObjectImpl for SmoothScale {
        fn constructed(&self) {
            self.parent_constructed();

            // new GestureClick with add_controller is doesn't work for connect_released
            //
            // so we need to iterate through the controllers to get the GestureClick
            // and then connect the signals
            let mut gesture = gtk::GestureClick::new();
            self.obj()
                .observe_controllers()
                .into_iter()
                .for_each(|collection| {
                    if let Ok(event) = collection {
                        if event.type_() == gtk::GestureClick::static_type() {
                            gesture = event.downcast::<gtk::GestureClick>().unwrap();
                        }
                    }
                });

            gesture.connect_pressed(glib::clone!(@weak self as imp => move |_, _, _, _|{
                imp.on_click_pressed();
            }));
            gesture.connect_released(glib::clone!(@weak self as imp => move |_, _, _, _|{
                imp.on_click_released();
            }));

            self.obj().duration_changed();
            self.obj().update_timeout();
        }
    }
    impl WidgetImpl for SmoothScale {}
    impl RangeImpl for SmoothScale {}
    impl ScaleImpl for SmoothScale {}

    impl SmoothScale {
        fn set_player(&self, player: Option<Player>) {
            if self.player.upgrade() == player {
                return;
            }
            self.player.set(player.as_ref());
        }

        fn on_click_pressed(&self) {
            let obj = self.obj();
            obj.remove_timeout();
        }

        fn on_click_released(&self) {
            let obj = self.obj();
            self.on_seek_finished(self.obj().value());
            obj.update_timeout();
        }

        fn on_seek_finished(&self, value: f64) {
            MUSIC_PLAYER.set_position(value);
        }
    }
}

glib::wrapper! {
    pub struct SmoothScale(ObjectSubclass<imp::SmoothScale>)
        @extends gtk::Widget, gtk::Scale, gtk::Range;
}

impl Default for SmoothScale {
    fn default() -> Self {
        Self::new()
    }
}

impl SmoothScale {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn update_position_callback(&self) -> glib::ControlFlow {
        let position = &MUSIC_PLAYER.position();
        if *position > 0.0 {
            self.set_value(*position);
        }
        glib::ControlFlow::Continue
    }

    pub fn update_timeout(&self) {
        let width = std::cmp::max(self.width(), 1);
        let timeout_period = std::cmp::min(1000 * 200 / width, 200);
        if let Some(timeout) = self.imp().timeout.borrow_mut().take() {
            glib::source::SourceId::remove(timeout);
        }
        self.imp().timeout.replace(Some(glib::timeout_add_local(
            std::time::Duration::from_millis(timeout_period as u64),
            glib::clone!(@strong self as obj => move || obj.update_position_callback()),
        )));
    }

    pub fn remove_timeout(&self) {
        if let Some(timeout) = self.imp().timeout.borrow_mut().take() {
            glib::source::SourceId::remove(timeout);
        }
    }

    pub fn on_smooth_scale_value_changed(&self) {
        let value = self.value();
        let position = value / 60.0;
        if let Some(player) = self.imp().player.upgrade() {
            player.set_position(position);
        }
    }

    fn duration_changed(&self) {
        self.set_value(0.0);
        self.set_range(0.0, 200.0);
        self.set_increments(300.0, 600.0);
    }
}
