use crate::client::client::{BackType, EMBY_CLIENT};
use crate::client::error::UserFacingError;
use crate::client::structs::Back;
use crate::toast;
use crate::ui::models::SETTINGS;
use crate::ui::provider::tu_item::TuItem;
use crate::ui::widgets::check_row::CheckRow;
use crate::ui::widgets::song_widget::format_duration;
use crate::utils::{spawn, spawn_g_timeout, spawn_tokio};
use adw::prelude::*;
use gettextrs::gettext;
use glib::Object;
use gtk::gdk::Rectangle;
use gtk::subclass::prelude::*;
use gtk::{gio, glib, Builder, PopoverMenu};

use super::mpvglarea::MPVGLArea;
use super::tsukimi_mpv::{
    ListenEvent, MpvTrack, MpvTracks, TrackSelection, MPV_EVENT_CHANNEL, PAUSED,
};
use super::video_scale::VideoScale;

const MIN_MOTION_TIME: i64 = 100000;
const NEXT_CHAPTER_KEYVAL: u32 = 65366;
const PREV_CHAPTER_KEYVAL: u32 = 65365;

mod imp {

    use std::cell::{Cell, RefCell};

    use adw::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate, PopoverMenu, ShortcutsWindow};

    use crate::client::structs::Back;
    use crate::ui::mpv::menu_actions::MenuActions;
    use crate::ui::mpv::mpvglarea::MPVGLArea;
    use crate::ui::mpv::video_scale::VideoScale;
    use crate::ui::provider::tu_item::TuItem;
    use crate::ui::widgets::action_row::AActionRow;

    // Object holding the state
    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsukimi/mpvpage.ui")]
    #[properties(wrapper_type = super::MPVPage)]
    pub struct MPVPage {
        #[property(get, set, nullable)]
        pub url: RefCell<Option<String>>,
        #[property(get, set = Self::set_fullscreened, explicit_notify)]
        pub fullscreened: Cell<bool>,
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
        pub spinner: TemplateChild<adw::Spinner>,
        #[template_child]
        pub loading_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub network_speed_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub network_speed_label_2: TemplateChild<gtk::Label>,
        #[template_child]
        pub menu_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub menu_popover: TemplateChild<gtk::Popover>,
        #[template_child]
        pub title: TemplateChild<gtk::Label>,
        #[template_child]
        pub speed_spin: TemplateChild<gtk::SpinButton>,
        #[template_child]
        pub volume_spin: TemplateChild<gtk::SpinButton>,
        #[template_child]
        pub sub_listbox: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub audio_listbox: TemplateChild<gtk::ListBox>,
        pub timeout: RefCell<Option<glib::source::SourceId>>,
        pub back_timeout: RefCell<Option<glib::source::SourceId>>,
        pub back: RefCell<Option<Back>>,
        pub x: RefCell<f64>,
        pub y: RefCell<f64>,
        pub last_motion_time: RefCell<i64>,
        pub suburl: RefCell<Option<String>>,
        pub popover: RefCell<Option<PopoverMenu>>,
        pub menu_actions: MenuActions,
        pub shortcuts_window: RefCell<Option<ShortcutsWindow>>,

        #[template_child]
        pub volume_adj: TemplateChild<gtk::Adjustment>,

        pub current_video: RefCell<Option<TuItem>>,
        pub current_episode_list: RefCell<Vec<TuItem>>,
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
            AActionRow::ensure_type();
            klass.bind_template();
            klass.bind_template_instance_callbacks();
            klass.install_action("mpv.play-pause", None, move |mpv, _action, _parameter| {
                mpv.on_play_pause_clicked();
            });
            klass.install_action("mpv.show-info", None, move |mpv, _action, _parameter| {
                mpv.on_info_clicked();
            });
            klass.install_action("mpv.backward", None, move |mpv, _action, _parameter| {
                mpv.on_backward();
            });
            klass.install_action("mpv.forward", None, move |mpv, _action, _parameter| {
                mpv.on_forward();
            });
            klass.install_action("mpv.chapter-prev", None, move |mpv, _action, _parameter| {
                mpv.chapter_prev();
            });
            klass.install_action("mpv.chapter-next", None, move |mpv, _action, _parameter| {
                mpv.chapter_next();
            });
            klass.install_action_async(
                "mpv.next-video",
                None,
                |mpv, _action, _parameter| async move {
                    mpv.on_next_video().await;
                },
            );
            klass.install_action_async(
                "mpv.previous-video",
                None,
                move |mpv, _action, _parameter| async move {
                    mpv.on_previous_video().await;
                },
            );
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

            let obj = self.obj();

            obj.set_popover();

            obj.connect_root_notify(|obj| {
                if let Some(window) = obj.root().and_downcast::<gtk::Window>() {
                    window
                        .bind_property("fullscreened", obj, "fullscreened")
                        .sync_create()
                        .build();
                }
            });

            obj.listen_events();
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for MPVPage {}

    // Trait shared by all windows
    impl WindowImpl for MPVPage {}

    // Trait shared by all application windows
    impl ApplicationWindowImpl for MPVPage {}

    impl adw::subclass::navigation_page::NavigationPageImpl for MPVPage {}

    impl MPVPage {
        fn set_fullscreened(&self, fullscreened: bool) {
            if fullscreened == self.fullscreened.get() {
                return;
            }

            self.fullscreened.set(fullscreened);

            self.obj().notify_fullscreened();
        }
    }
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
        item: TuItem,
        episode_list: Vec<TuItem>,
        back: Option<Back>,
        percentage: f64,
    ) {
        let url = url.to_owned();
        let suburi = suburi.map(|s| s.to_owned());
        let name = if let Some(series_name) = item.series_name() {
            format!(
                "{} - S{}E{}: {}",
                series_name,
                item.parent_index_number(),
                item.index_number(),
                item.name()
            )
        } else {
            item.name()
        };

        self.imp().current_video.replace(Some(item));
        self.imp().current_episode_list.replace(episode_list);
        spawn_g_timeout(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                obj.load_config();
                let imp = obj.imp();
                imp.spinner.set_visible(true);
                imp.loading_box.set_visible(true);
                imp.network_speed_label.set_text("Initializing...");
                imp.title.set_text(&name);
                imp.suburl
                    .replace(suburi.map(|suburi| EMBY_CLIENT.get_streaming_url(&suburi)));
                imp.video.play(&url, percentage);
                imp.back.replace(back);
                obj.handle_callback(BackType::Start);
            }
        ));
    }

    fn set_audio_and_video_tracks_dropdown(&self, value: MpvTracks) {
        let imp = self.imp();
        self.bind_tracks::<true>(value.audio_tracks, &imp.audio_listbox.get());
        self.bind_tracks::<false>(value.sub_tracks, &imp.sub_listbox.get());
    }

    // TODO: Use GAction instead of listening to each button
    fn bind_tracks<const A: bool>(&self, tracks: Vec<MpvTrack>, listbox: &gtk::ListBox) {
        while let Some(row) = listbox.first_child() {
            listbox.remove(&row);
        }

        let track_id = self.imp().video.get_track_id(if A { "aid" } else { "sid" });

        let row = CheckRow::new();
        row.set_title("None");
        if track_id == 0 {
            row.imp().check.get().set_active(true);
        }
        let none_check = &row.imp().check.get();
        row.connect_activated(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            move |_| {
                obj.set_vsid::<A>(0);
            }
        ));
        listbox.append(&row);

        for track in tracks {
            let row = CheckRow::new();
            row.set_title(&track.title.replace('&', "&amp;"));
            row.set_subtitle(&track.lang.replace('&', "&amp;"));
            row.imp().track_id.replace(track.id);
            let check = &row.imp().check.get();
            check.set_group(Some(none_check));
            if track.id == track_id {
                check.set_active(true);
            }
            row.connect_activated(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move |_| {
                    obj.set_vsid::<A>(track.id);
                }
            ));
            listbox.append(&row);
        }
    }

    fn set_vsid<const A: bool>(&self, track_id: i64) {
        let track = if track_id == 0 {
            TrackSelection::None
        } else {
            TrackSelection::Track(track_id)
        };

        if A {
            self.imp().video.set_aid(track);
        } else {
            self.imp().video.set_sid(track);
        }
    }

    async fn load_video(&self, offset: isize) {
        toast!(self, "Loading Video...");

        let Some(current_video) = self.imp().current_video.borrow().clone() else {
            return;
        };

        let video_list = self.imp().current_episode_list.borrow().clone();

        let next_item = video_list.iter().enumerate().find_map(|(i, item)| {
            if item.id() == current_video.id() {
                let new_index = (i as isize + offset) as usize;
                video_list.get(new_index).cloned()
            } else {
                None
            }
        });

        let Some(next_item) = next_item else {
            toast!(self, "No more videos found");
            self.on_stop_clicked();
            return;
        };

        self.in_play_item(next_item).await;
    }

    pub async fn in_play_item(&self, item: TuItem) {
        let item_id = item.id();

        let video_list = self.imp().current_episode_list.borrow().clone();

        let playback =
            match spawn_tokio(async move { EMBY_CLIENT.get_playbackinfo(&item_id).await })
                .await
            {
                Ok(playback) => playback,
                Err(e) => {
                    toast!(self, e.to_user_facing());
                    return;
                }
            };

        let (media_source_id, url) = if let Some(media_source) = playback.media_sources.first() {
            (
                media_source.id.clone(),
                media_source.direct_stream_url.clone(),
            )
        } else {
            toast!(self, "No media sources found");
            return;
        };

        let Some(url) = url else {
            toast!(self, "No media sources found");
            return;
        };

        let back = Back {
            id: item.id(),
            playsessionid: playback.play_session_id,
            mediasourceid: media_source_id.to_string(),
            tick: 0,
        };

        self.play(&url, None, item.clone(), video_list, Some(back), 0.0);
    }

    pub async fn on_next_video(&self) {
        self.load_video(1).await;
    }

    pub async fn on_previous_video(&self) {
        self.load_video(-1).await;
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
                        ListenEvent::Seek | ListenEvent::PausedForCache(true) => {
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
                        ListenEvent::PlaybackRestart | ListenEvent::PausedForCache(false) => {
                            obj.update_seeking(false);
                        }
                        ListenEvent::StartFile => {
                            obj.on_start_file();
                        }
                        ListenEvent::TrackList(value) => {
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
        imp.video_scale.set_range(0.0, value);
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
        self.update_timeout();
    }

    fn update_seeking(&self, seeking: bool) {
        let spinner = &self.imp().spinner;
        let loading_box = &self.imp().loading_box;
        if seeking {
            loading_box.set_visible(true);
            spinner.set_visible(true);
        } else {
            loading_box.set_visible(false);
            spinner.set_visible(false);
        }
    }

    fn on_end_file(&self, value: u32) {
        spawn(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                if value == 0 {
                    match SETTINGS.mpv_action_after_video_end() {
                        0 => obj.on_next_video().await,
                        2 => obj.on_stop_clicked(),
                        _ => {}
                    }
                }
            }
        ));
    }

    fn on_error(&self, value: &str) {
        toast!(self, value);
    }

    fn on_pause_update(&self, value: bool) {
        self.pause_icon_set(value);
    }

    fn on_cache_speed_update(&self, value: i64) {
        let label = &self.imp().network_speed_label;
        if value >= 2 * 1024 * 1024 {
            label.set_text(&format!("{:.2} MiB/s", value as f64 / (1024.0 * 1024.0)));
        } else {
            label.set_text(&format!("{} KiB/s", value / 1024));
        }
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
        let video = &self.imp().video;
        let paused = video.paused();
        video.pause(!paused);

        if paused {
            self.update_timeout();
        } else {
            self.remove_timeout();
        }
    }

    fn pause_icon_set(&self, paused: bool) {
        let imp = self.imp();
        let play_pause_image = imp.play_pause_image.get();
        let menu_actions_play_pause_button = imp.menu_actions.imp().play_pause_button.get();
        if paused {
            play_pause_image.set_icon_name(Some("media-playback-start-symbolic"));
            play_pause_image.set_tooltip_text(Some(&gettext("Play")));
            menu_actions_play_pause_button.set_icon_name("media-playback-start-symbolic");
            menu_actions_play_pause_button.set_tooltip_text(Some(&gettext("Play")));
        } else {
            play_pause_image.set_icon_name(Some("media-playback-pause-symbolic"));
            play_pause_image.set_tooltip_text(Some(&gettext("Pause")));
            menu_actions_play_pause_button.set_icon_name("media-playback-pause-symbolic");
            menu_actions_play_pause_button.set_tooltip_text(Some(&gettext("Pause")));
        }
    }

    #[template_callback]
    fn on_stop_clicked(&self) {
        self.handle_callback(BackType::Stop);
        self.remove_timeout();
        self.imp().video_scale.remove_timeout();
        let mpv = &self.imp().video.imp().mpv;
        mpv.pause(true);
        mpv.stop();
        mpv.event_thread_alive
            .store(PAUSED, std::sync::atomic::Ordering::SeqCst);
        let root = self.root();
        let window = root
            .and_downcast_ref::<crate::ui::widgets::window::Window>()
            .unwrap();
        window.imp().stack.set_visible_child_name("main");
    }

    pub fn update_position_callback(&self) -> glib::ControlFlow {
        self.handle_callback(BackType::Back);
        glib::ControlFlow::Continue
    }

    fn handle_callback(&self, backtype: BackType) {
        let position = &self.imp().video.position();
        let back = self.imp().back.borrow();

        // close window when vo=gpu-next will set position to 0, so we need to ignore it
        if position < &9.0 && backtype != BackType::Start {
            return;
        }

        if let Some(back) = back.as_ref() {
            let duration = *position as u64 * 10000000;
            let mut back = back.clone();
            back.tick = duration;
            spawn(spawn_tokio(async move {
                let _ = EMBY_CLIENT.position_back(&back, backtype).await;
            }))
        }
    }

    pub fn update_timeout(&self) {
        self.remove_timeout();
        let closure = glib::clone!(
            #[weak(rename_to = obj)]
            self,
            move || {
                self.imp()
                    .back_timeout
                    .replace(Some(glib::timeout_add_seconds_local(10, move || {
                        obj.update_position_callback()
                    })));
            }
        );
        closure();
    }

    pub fn remove_timeout(&self) {
        if let Some(timeout) = self.imp().back_timeout.borrow_mut().take() {
            glib::source::SourceId::remove(timeout);
        }
    }

    #[template_callback]
    fn right_click_cb(&self, _n: i32, x: f64, y: f64) {
        if let Some(popover) = self.imp().popover.borrow().as_ref() {
            popover.set_pointing_to(Some(&Rectangle::new(x as i32, y as i32, 0, 0)));
            popover.popup();
        };
    }

    #[template_callback]
    fn left_click_cb(&self) {
        let video = &self.imp().video;
        video.pause(!video.paused());
    }

    #[template_callback]
    fn on_playlist_clicked(&self) {
        let binding = self.ancestor(adw::OverlaySplitView::static_type());
        let Some(view) = binding.and_downcast_ref::<adw::OverlaySplitView>() else {
            return;
        };
        view.set_show_sidebar(!view.shows_sidebar());
    }

    pub fn key_pressed_cb(&self, key: u32, state: gtk::gdk::ModifierType) {
        self.imp().video.press_key(key, state)
    }

    pub fn key_released_cb(&self, key: u32, state: gtk::gdk::ModifierType) {
        self.imp().video.release_key(key, state)
    }

    pub fn set_popover(&self) {
        let imp = self.imp();
        let builder = Builder::from_resource("/moe/tsukimi/mpv_menu.ui");
        let menu = builder.object::<gio::MenuModel>("mpv-menu");
        match menu {
            Some(popover) => {
                let popover = PopoverMenu::builder()
                    .menu_model(&popover)
                    .halign(gtk::Align::Start)
                    .has_arrow(false)
                    .build();
                popover.set_parent(self);
                popover.add_child(&imp.menu_actions, "menu-actions");
                let _ = imp.popover.replace(Some(popover));
            }
            None => eprintln!("Failed to load popover"),
        }
    }

    pub fn on_backward(&self) {
        let video = &self.imp().video;
        video.seek_backward(SETTINGS.mpv_seek_backward_step() as i64)
    }

    pub fn on_forward(&self) {
        let video = &self.imp().video;
        video.seek_forward(SETTINGS.mpv_seek_forward_step() as i64)
    }

    pub fn chapter_prev(&self) {
        self.key_pressed_cb(PREV_CHAPTER_KEYVAL, gtk::gdk::ModifierType::empty());
    }

    pub fn chapter_next(&self) {
        self.key_pressed_cb(NEXT_CHAPTER_KEYVAL, gtk::gdk::ModifierType::empty());
    }

    pub fn load_config(&self) {
        let imp = self.imp();
        imp.network_speed_label_2
            .set_visible(SETTINGS.mpv_show_buffer_speed());
        imp.volume_adj
            .set_value(SETTINGS.mpv_default_volume() as f64);
        let mpv = &imp.video.imp().mpv;
        if !SETTINGS.proxy().is_empty() {
            mpv.set_property("http-proxy", SETTINGS.proxy());
        }
        match SETTINGS.mpv_hwdec() {
            0 => mpv.set_property("hwdec", "no"),
            1 => mpv.set_property("hwdec", "auto-safe"),
            2 => mpv.set_property("hwdec", "vaapi"),
            _ => unreachable!(),
        }
        if SETTINGS.mpv_estimate() {
            let fps = SETTINGS.mpv_estimate_target_frame();
            mpv.set_property("vf", format!("lavfi=\"fps=fps={fps}:round=down\""));
        } else {
            // clear vf
            mpv.set_property("vf", "");
        }
        mpv.set_property(
            "demuxer-max-bytes",
            format!("{}MiB", SETTINGS.mpv_cache_size()),
        );
        mpv.set_property("cache-secs", (SETTINGS.mpv_cache_time()) as i64);
        mpv.set_property("volume", SETTINGS.mpv_default_volume() as i64);
        mpv.set_property("sub-font-size", SETTINGS.mpv_subtitle_size() as i64);
        mpv.set_property("sub-font", SETTINGS.mpv_subtitle_font());
        if SETTINGS.mpv_force_stereo() {
            mpv.set_property("audio-channels", "stereo");
        } else {
            mpv.set_property("audio-channels", "auto");
        }
        match SETTINGS.mpv_video_output() {
            0 => mpv.set_property("vo", "libmpv"),
            1 => mpv.set_property("vo", "gpu-next"),
            2 => mpv.set_property("vo", "dmabuf-wayland"),
            _ => unreachable!(),
        }
        match SETTINGS.mpv_audio_preferred_lang() {
            0 => mpv.set_property("alang", ""), // clear alang
            1 => mpv.set_property("alang", "eng"),
            2 => mpv.set_property("alang", "chs"),
            3 => mpv.set_property("alang", "jpn"),
            _ => unreachable!(),
        }
        match SETTINGS.mpv_subtitle_preferred_lang() {
            0 => mpv.set_property("slang", ""), // clear slang
            1 => mpv.set_property("slang", "eng"),
            2 => mpv.set_property("slang", "chs"),
            3 => mpv.set_property("slang", "jpn"),
            _ => unreachable!(),
        }
        if SETTINGS.mpv_action_after_video_end() == 1 {
            mpv.set_property("loop", "inf");
        } else {
            mpv.set_property("loop", "no");
        }
    }
}
