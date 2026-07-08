use gtk::{
    glib,
    prelude::*,
    subclass::prelude::*,
};

use super::tsukimi_mpv::ChapterList;

mod imp {
    use std::cell::{
        Cell,
        RefCell,
    };

    use gtk::{
        glib,
        prelude::*,
        subclass::prelude::*,
    };

    use crate::ui::mpv::mpvglarea::MPVGLArea;

    type ScrubCallback = Box<dyn Fn(f64)>;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::VideoScale)]
    pub struct VideoScale {
        #[property(get, set = Self::set_player, explicit_notify, nullable)]
        pub player: glib::WeakRef<MPVGLArea>,

        pub is_dragging: Cell<bool>,
        pub scrub_callback: RefCell<Option<ScrubCallback>>,
        pub scrub_finished_callback: RefCell<Option<Box<dyn Fn()>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VideoScale {
        const NAME: &'static str = "VideoScale";
        type Type = super::VideoScale;
        type ParentType = gtk::Scale;
    }

    #[glib::derived_properties]
    impl ObjectImpl for VideoScale {
        fn constructed(&self) {
            self.parent_constructed();

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
                    imp.on_click_pressed();
                }
            ));

            gesture.connect_released(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |_, _, _, _| {
                    imp.on_click_released();
                }
            ));

            self.obj().connect_value_changed(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |scale| {
                    if imp.is_dragging.get()
                        && let Some(callback) = imp.scrub_callback.borrow().as_ref()
                    {
                        callback(scale.value());
                    }
                }
            ));
        }
    }
    impl WidgetImpl for VideoScale {}
    impl RangeImpl for VideoScale {}
    impl ScaleImpl for VideoScale {}

    impl VideoScale {
        fn set_player(&self, player: Option<MPVGLArea>) {
            if self.player.upgrade() == player {
                return;
            }
            self.player.set(player.as_ref());
        }

        fn on_click_pressed(&self) {
            self.is_dragging.set(true);
            let value = self.obj().value();
            if let Some(callback) = self.scrub_callback.borrow().as_ref() {
                callback(value);
            }
        }

        fn on_click_released(&self) {
            let obj = self.obj();
            self.on_seek_finished(obj.value());
            self.is_dragging.set(false);
            if let Some(callback) = self.scrub_finished_callback.borrow().as_ref() {
                callback();
            }
        }

        fn on_seek_finished(&self, value: f64) {
            self.player.upgrade().unwrap().set_position(value);
        }
    }
}

glib::wrapper! {
    pub struct VideoScale(ObjectSubclass<imp::VideoScale>)
        @extends gtk::Widget, gtk::Scale, gtk::Range, @implements gtk::Accessible, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl Default for VideoScale {
    fn default() -> Self {
        Self::new()
    }
}

impl VideoScale {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn connect_scrub_position_changed<F>(&self, callback: F)
    where
        F: Fn(f64) + 'static,
    {
        *self.imp().scrub_callback.borrow_mut() = Some(Box::new(callback));
    }

    pub fn connect_scrub_finished<F>(&self, callback: F)
    where
        F: Fn() + 'static,
    {
        *self.imp().scrub_finished_callback.borrow_mut() = Some(Box::new(callback));
    }

    pub fn update_position_callback(&self) -> glib::ControlFlow {
        let position = &self.player().unwrap().position();
        if *position > 0.0 {
            self.set_value(*position);
        }
        glib::ControlFlow::Continue
    }

    pub fn set_cache_end_time(&self, end_time: i64) {
        self.set_fill_level(end_time as f64);
    }

    pub fn reset_scale(&self) {
        self.set_value(0.0);
        self.set_fill_level(0.0);
    }

    pub fn is_dragging(&self) -> bool {
        self.imp().is_dragging.get()
    }

    pub fn set_chapter_list(&self, chapter_list: ChapterList) {
        self.clear_marks();

        for chapter in chapter_list {
            self.add_mark(chapter.time, gtk::PositionType::Top, None);
        }
    }
}
