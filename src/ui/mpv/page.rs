use crate::client::client::EMBY_CLIENT;
use crate::client::structs::Back;
use crate::toast;
use crate::ui::provider::dropdown_factory::DropdownList;
use crate::ui::widgets::song_widget::format_duration;
use adw::prelude::*;
use gettextrs::gettext;
use glib::Object;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

use super::mpvglarea::MPVGLArea;
use super::tsukimi_mpv::{ListenEvent, MpvTrack, ACTIVE, MPV_EVENT_CHANNEL, PAUSED};
use super::video_scale::VideoScale;
static MIN_MOTION_TIME: i64 = 100000;

mod imp {

    use std::cell::RefCell;

    use adw::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate};

    use crate::client::structs::Back;
    use crate::ui::mpv::mpvglarea::MPVGLArea;
    use crate::ui::mpv::video_scale::VideoScale;
    use crate::ui::provider::dropdown_factory::factory;

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
        pub top_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub play_pause_image: TemplateChild<gtk::Image>,
        #[template_child]
        pub video_scale: TemplateChild<VideoScale>,
        #[template_child]
        pub progress_time_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub duration_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub spinner: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub loading_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub network_speed_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub menu_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub menu_popover: TemplateChild<gtk::Popover>,
        #[template_child]
        pub title: TemplateChild<gtk::Label>,
        #[template_child]
        pub audio_tracks_combo_row: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub subtitle_tracks_combo_row: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub speed_spin: TemplateChild<gtk::SpinButton>,
        #[template_child]
        pub volume_spin: TemplateChild<gtk::SpinButton>,
        pub timeout: RefCell<Option<glib::source::SourceId>>,
        pub back: RefCell<Option<Back>>,
        pub x: RefCell<f64>,
        pub y: RefCell<f64>,
        pub last_motion_time: RefCell<i64>,
        pub suburl: RefCell<Option<String>>,
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

            self.menu_popover.set_position(gtk::PositionType::Top);
            self.menu_popover.set_offset(0, -20);

            self.video_scale.set_player(Some(&self.video.get()));

            self.audio_tracks_combo_row
                .set_factory(Some(&factory(true)));
            self.subtitle_tracks_combo_row
                .set_factory(Some(&factory(true)));
            self.audio_tracks_combo_row
                .set_list_factory(Some(&factory(false)));
            self.subtitle_tracks_combo_row
                .set_list_factory(Some(&factory(false)));

            self.obj().listen_events();
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

    pub fn play(
        &self,
        url: &str,
        suburi: Option<&str>,
        name: Option<&str>,
        back: Option<Back>,
        percentage: f64,
    ) {
        let imp = self.imp();

        imp.spinner.start();
        imp.loading_box.set_visible(true);
        imp.network_speed_label.set_text("Initializing...");
        if let Some(name) = name {
            imp.title.set_text(name);
        }
        imp.suburl
            .replace(suburi.map(|suburi| EMBY_CLIENT.get_streaming_url(suburi)));
        imp.video.play(url, name, back, percentage);
    }

    fn set_audio_and_video_tracks_dropdown(&self, count: i64) {
        let imp = self.imp();
        let (audio_tracks, subtitle_tracks) = imp.video.get_audio_and_subtitle_tracks(count);
        let audio_tracks_store = self.vec_to_model(audio_tracks);
        let subtitle_tracks_store = self.vec_to_model(subtitle_tracks);
        imp.audio_tracks_combo_row
            .set_model(Some(&audio_tracks_store));
        imp.subtitle_tracks_combo_row
            .set_model(Some(&subtitle_tracks_store));
    }

    fn vec_to_model(&self, vec: Vec<MpvTrack>) -> gtk::gio::ListStore {
        let store = gtk::gio::ListStore::new::<glib::BoxedAnyObject>();
        for item in vec {
            let dl = DropdownList {
                line1: Some(item.title),
                line2: Some(item.lang),
            };
            let object = glib::BoxedAnyObject::new(dl);
            store.append(&object);
        }
        store
    }

    #[template_callback]
    fn on_progress_value_changed(&self, progress_scale: &VideoScale) {
        let label = &self.imp().progress_time_label.get();
        let position = progress_scale.value();
        label.set_text(&format_duration(position as i64));
    }

    #[template_callback]
    fn on_info_clicked(&self) {
        let mpv = &self.imp().video;
        mpv.display_stats_toggle();
    }

    fn listen_events(&self) {
        glib::spawn_future_local(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                while let Ok(value) = MPV_EVENT_CHANNEL.rx.recv_async().await {
                    match value {
                        ListenEvent::Duration(value) => {
                            obj.update_duration(value);
                        }
                        ListenEvent::Seek => {
                            obj.update_seeking(true);
                        }
                        ListenEvent::Eof(value) => {
                            obj.on_end_file(value);
                        }
                        ListenEvent::Error(value) => {
                            obj.on_error(&value);
                        }
                        ListenEvent::Pause(value) => {
                            obj.on_pause_update(value);
                        }
                        ListenEvent::CacheSpeed(value) => {
                            obj.on_cache_speed_update(value);
                        }
                        ListenEvent::PlaybackRestart => {
                            obj.update_seeking(false);
                        }
                        ListenEvent::StartFile => {
                            obj.on_start_file();
                        }
                        ListenEvent::TrackListCount(value) => {
                            obj.set_audio_and_video_tracks_dropdown(value);
                        }
                        ListenEvent::Volume(value) => {
                            obj.volume_cb(value);
                        }
                        ListenEvent::Speed(value) => {
                            obj.speed_cb(value);
                        }
                    }
                }
            }
        ));
    }

    fn update_duration(&self, value: f64) {
        let imp = self.imp();
        imp.video_scale.set_range(0.0, value as f64);
        imp.duration_label.set_text(&format_duration(value as i64));
        imp.video_scale.update_timeout();
    }

    fn speed_cb(&self, value: f64) {
        let imp = self.imp();
        imp.speed_spin.set_value(value);
    }

    fn volume_cb(&self, value: i64) {
        let imp = self.imp();
        imp.volume_spin.set_value(value as f64);
    }

    #[template_callback]
    fn on_speed_value_changed(&self, btn: &gtk::SpinButton) {
        let imp = self.imp();
        imp.video.set_speed(btn.value());
    }

    #[template_callback]
    fn on_volume_value_changed(&self, btn: &gtk::SpinButton) {
        let imp = self.imp();
        imp.video.set_volume(btn.value() as i64);
    }

    fn on_start_file(&self) {
        let imp = self.imp();
        if let Some(suburl) = imp.suburl.borrow().as_ref() {
            imp.video.add_sub(suburl);
        }
        imp.video_scale.update_timeout();
    }

    fn update_seeking(&self, seeking: bool) {
        let spinner = &self.imp().spinner;
        let loading_box = &self.imp().loading_box;
        if seeking {
            loading_box.set_visible(true);
            spinner.start();
        } else {
            loading_box.set_visible(false);
            spinner.stop();
        }
    }

    fn on_end_file(&self, value: u32) {
        if value == 2 {
            return;
        }
        self.on_stop_clicked();
    }

    fn on_error(&self, value: &str) {
        toast!(self, value);
    }

    fn on_pause_update(&self, value: bool) {
        self.pause_icon_set(value);
    }

    fn on_cache_speed_update(&self, value: i64) {
        let label = &self.imp().network_speed_label;
        label.set_text(&format!("{} KiB/s", value / 1024));
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
            if self.toolbar_revealed() {
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

        if self.toolbar_revealed() && imp.timeout.borrow().is_none() {
            self.reset_fade_timeout();
        }
    }

    #[template_callback]
    fn on_enter(&self) {
        if self.toolbar_revealed() {
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
            ),
        );
        *imp.timeout.borrow_mut() = Some(timeout);
    }

    fn x(&self) -> impl std::ops::Deref<Target = f64> + '_ {
        self.imp().x.borrow()
    }

    fn y(&self) -> impl std::ops::Deref<Target = f64> + '_ {
        self.imp().y.borrow()
    }

    fn last_motion_time(&self) -> impl std::ops::Deref<Target = i64> + '_ {
        self.imp().last_motion_time.borrow()
    }

    fn toolbar_revealed(&self) -> bool {
        self.imp().top_revealer.is_child_revealed()
    }

    fn fade_overlay_delay_cb(&self) {
        *self.imp().timeout.borrow_mut() = None;

        if self.toolbar_revealed() && self.can_fade_overlay() {
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
        if self.imp().menu_button.is_active() {
            return false;
        }
        true
    }

    fn set_reveal_overlay(&self, reveal: bool) {
        let imp = self.imp();
        imp.bottom_revealer.set_reveal_child(reveal);
        imp.top_revealer.set_reveal_child(reveal);
    }

    #[template_callback]
    fn on_play_pause_clicked(&self) {
        let mpv_area = self.imp().video.get();

        let paused = mpv_area.imp().mpv.paused();

        self.pause_icon_set(!paused);

        mpv_area.imp().mpv.pause(!paused);
    }

    fn pause_icon_set(&self, paused: bool) {
        let play_pause_image = &self.imp().play_pause_image.get();
        if paused {
            play_pause_image.set_icon_name(Some("media-playback-start-symbolic"));
            play_pause_image.set_tooltip_text(Some(&gettext("Play")));
        } else {
            play_pause_image.set_icon_name(Some("media-playback-pause-symbolic"));
            play_pause_image.set_tooltip_text(Some(&gettext("Pause")));
        }
    }

    #[template_callback]
    fn on_stop_clicked(&self) {
        self.imp().video_scale.remove_timeout();
        let mpv = &self.imp().video.imp().mpv;
        mpv.pause(true);
        mpv.stop();
        self.imp().loading_box.set_visible(true);
        mpv.event_thread_alive
            .store(PAUSED, std::sync::atomic::Ordering::SeqCst);
        let root = self.root();
        let window = root
            .and_downcast_ref::<crate::ui::widgets::window::Window>()
            .unwrap();
        window.imp().stack.set_visible_child_name("main");
    }
}
