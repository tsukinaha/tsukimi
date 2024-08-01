use std::borrow::Borrow;

use crate::client::structs::Back;
use gettextrs::gettext;
use glib::Object;
use gtk::subclass::prelude::*;
use gtk::prelude::*;
use gtk::{gio, glib};

use super::mpvglarea::{MPVGLArea, MPV};
static MIN_MOTION_TIME: i64 = 100000;

mod imp {

    use std::cell::RefCell;

    use glib::subclass::InitializingObject;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate};

    use crate::client::structs::Back;
    use crate::ui::mpv::mpvglarea::MPVGLArea;
    use crate::ui::widgets::video_scale::VideoScale;

    // Object holding the state
    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsukimi/mpvpage.ui")]
    #[properties(wrapper_type = super::MPVPage)]
    pub struct MPVPage {
        #[property(get, set, nullable)]
        pub url: RefCell<Option<String>>,
        #[template_child]
        pub video: TemplateChild<MPVGLArea>,
        #[template_child]
        pub bottom_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub play_pause_image: TemplateChild<gtk::Image>,
        #[template_child]
        pub video_scale: TemplateChild<VideoScale>,
        pub timeout: RefCell<Option<glib::source::SourceId>>,
        pub back: RefCell<Option<Back>>,
        pub x: RefCell<f64>,
        pub y: RefCell<f64>,
        pub last_motion_time: RefCell<i64>,
        pub toolbar_revealed: RefCell<bool>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for MPVPage {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "MPVPage";
        type Type = super::MPVPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            MPVGLArea::ensure_type();
            VideoScale::ensure_type();
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // Trait shared by all GObjects
    #[glib::derived_properties]
    impl ObjectImpl for MPVPage {
        fn constructed(&self) {
            self.parent_constructed();

            self.video_scale.set_player(Some(&self.video.get()));
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for MPVPage {}

    // Trait shared by all windows
    impl WindowImpl for MPVPage {}

    // Trait shared by all application windows
    impl ApplicationWindowImpl for MPVPage {}

    impl adw::subclass::navigation_page::NavigationPageImpl for MPVPage {}
}

glib::wrapper! {
    pub struct MPVPage(ObjectSubclass<imp::MPVPage>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget ,adw::NavigationPage,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

use crate::ui::mpv::mpvglarea::EmbyPlayer;

impl Default for MPVPage {
    fn default() -> Self {
        Self::new()
    }
}

#[gtk::template_callbacks]
impl MPVPage {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn play(&self,
        url: &str,
        suburi: Option<&str>,
        name: Option<&str>,
        line2: Option<&str>,
        back: Option<Back>,
        percentage: f64
    ) {
        self.imp().video.play(url, suburi, name, line2, back, percentage);
    }

    #[template_callback]
    fn on_motion(&self, x: f64, y: f64) {
        let old_x = *self.x();
        let old_y = *self.y();
        
        if old_x == x && old_y == y {
            return;
        }

        let imp = self.imp();

        *imp.x.borrow_mut() = x;
        *imp.y.borrow_mut() = y;

        let now = glib::monotonic_time();

        if now - *self.last_motion_time() < MIN_MOTION_TIME {
            return;
        }
        
        let is_threshold = (old_x - x).abs() > 5.0 || (old_y - y).abs() > 5.0;
        
        
        
        if is_threshold {
            if *imp.toolbar_revealed.borrow() {
                self.reset_fade_timeout();
            } else {
                self.set_reveal_overlay(true);
            } 
        }

        *imp.last_motion_time.borrow_mut() = now;

    }

    #[template_callback]
    fn on_leave(&self) {
        let imp = self.imp();
        *imp.x.borrow_mut() = -1.0;
        *imp.y.borrow_mut() = -1.0;

        if *imp.toolbar_revealed.borrow() && imp.timeout.borrow().is_none() {
            self.reset_fade_timeout();
        }
    }

    #[template_callback]
    fn on_enter(&self) {
        let imp = self.imp();
        
        if *imp.toolbar_revealed.borrow() {
            self.reset_fade_timeout();
        } else {
            self.set_reveal_overlay(true);
        }
    }

    fn reset_fade_timeout(&self) {
        let imp = self.imp();
        if let Some(timeout) = imp.timeout.borrow_mut().take() {
            glib::source::SourceId::remove(timeout);
        }
        let timeout = glib::timeout_add_seconds_local_once(
        3, 
        glib::clone!(
            #[weak(rename_to = obj)]
            self,
            move || {
                obj.fade_overlay_delay_cb();
            }
        ));
        *imp.timeout.borrow_mut() = Some(timeout);
    }

    fn x(&self) -> impl std::ops::Deref<Target=f64>  + '_ {
        self.imp().x.borrow()
    }

    fn y(&self) -> impl std::ops::Deref<Target=f64>  + '_ {
        self.imp().y.borrow()
    }

    fn last_motion_time(&self) -> impl std::ops::Deref<Target=i64>  + '_ {
        self.imp().last_motion_time.borrow()
    }

    fn toolbar_revealed(&self) -> impl std::ops::Deref<Target=bool>  + '_ {
        self.imp().toolbar_revealed.borrow()
    }

    fn fade_overlay_delay_cb(&self) {
        *self.imp().timeout.borrow_mut() = None;

        if *self.toolbar_revealed() && self.can_fade_overlay() {
            self.set_reveal_overlay(false);
        }
    }

    fn can_fade_overlay(&self) -> bool {
        let x = *self.x();
        let y = *self.y();
        if x >= 0.0 && y >= 0.0 {
            let widget = self.pick(x, y, gtk::PickFlags::DEFAULT);
            if let Some(widget) = widget {
                if !widget.is::<MPVGLArea>() {
                    return false;
                }
            }
        }
        true
    }

    fn set_reveal_overlay(&self, reveal: bool) {
        let imp = self.imp();
        *imp.toolbar_revealed.borrow_mut() = reveal;
        if reveal {
            imp.bottom_revealer.set_visible(true);
        }
        imp.bottom_revealer.set_reveal_child(reveal);
    }

    #[template_callback]
    fn on_play_pause_clicked(&self) {
        let play_pause_image = &self.imp().play_pause_image.get();

        let mut mpv = MPV.lock().unwrap();
        let paused = mpv.paused();

        if paused {
            play_pause_image.set_icon_name(Some("media-playback-pause-symbolic"));
            play_pause_image.set_tooltip_text(Some(&gettext("Pause")));
        } else {
            play_pause_image.set_icon_name(Some("media-playback-start-symbolic"));
            play_pause_image.set_tooltip_text(Some(&gettext("Play")));
        }

        mpv.pause(!paused);
    }

    #[template_callback]
    fn on_stop_clicked(&self) {
        MPV.lock().unwrap().quit();

        let root = self.root();
        let window = root.and_downcast_ref::<crate::ui::widgets::window::Window>().unwrap();
        window.imp().stack.set_visible_child_name("main");
    }
}
