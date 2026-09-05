use crate::{
    ChapterList,
    MutsumiVideoPlayer,
};
use gtk::{
    glib,
    prelude::*,
    subclass::prelude::*,
};

mod imp {
    use std::cell::Cell;

    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::MutsumiVideoScale)]
    pub struct MutsumiVideoScale {
        #[property(get, set = Self::set_player, explicit_notify, nullable)]
        pub player: glib::WeakRef<MutsumiVideoPlayer>,

        pub dragging: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MutsumiVideoScale {
        const NAME: &'static str = "MutsumiVideoScale";
        type Type = super::MutsumiVideoScale;
        type ParentType = gtk::Scale;
    }

    #[glib::derived_properties]
    impl ObjectImpl for MutsumiVideoScale {
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
                    if let Ok(event) = collection
                        && event.type_() == gtk::GestureClick::static_type()
                    {
                        gesture = event.downcast::<gtk::GestureClick>().unwrap();
                    }
                });

            gesture.connect_pressed(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |_, _, _, _| {
                    imp.dragging.set(true);
                }
            ));

            gesture.connect_released(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |_, _, _, _| {
                    imp.dragging.set(false);
                    imp.on_seek_finished(imp.obj().value());
                }
            ));
        }
    }

    impl WidgetImpl for MutsumiVideoScale {}
    impl RangeImpl for MutsumiVideoScale {}
    impl ScaleImpl for MutsumiVideoScale {}

    impl MutsumiVideoScale {
        fn set_player(&self, player: Option<MutsumiVideoPlayer>) {
            if self.player.upgrade() == player {
                return;
            }
            self.player.set(player.as_ref());
        }

        fn on_seek_finished(&self, value: f64) {
            let Some(player) = self.player.upgrade() else {
                return;
            };

            player.set_position(value);
        }
    }
}

glib::wrapper! {
    pub struct MutsumiVideoScale(ObjectSubclass<imp::MutsumiVideoScale>)
    @extends gtk::Widget, gtk::Scale, gtk::Range, @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl Default for MutsumiVideoScale {
    fn default() -> Self {
        Self::new()
    }
}

impl MutsumiVideoScale {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn is_dragging(&self) -> bool {
        self.imp().dragging.get()
    }

    pub fn set_cache_end_time(&self, end_time: i64) {
        self.set_fill_level(end_time as f64);
    }

    pub fn reset_scale(&self) {
        self.set_value(0.0);
        self.set_fill_level(0.0);
        self.clear_marks();
    }

    pub fn set_chapter_list(&self, chapter_list: ChapterList) {
        self.clear_marks();

        for chapter in chapter_list {
            self.add_mark(chapter.time, gtk::PositionType::Top, None);
        }
    }
}
