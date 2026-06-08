use adw::prelude::*;
use glib::Object;
use gtk::{
    Builder,
    PopoverMenu,
    gdk::Rectangle,
    gio,
    glib,
    subclass::prelude::*,
};


const MIN_MOTION_TIME: i64 = 100000;
const PREV_CHAPTER_KEYVAL: u32 = 65366;
const NEXT_CHAPTER_KEYVAL: u32 = 65365;

mod imp {

    use std::cell::{
        Cell,
        RefCell,
    };

    use adw::prelude::*;
    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        PopoverMenu,
        ShortcutsWindow,
        glib,
        subclass::prelude::*,
    };
    #[cfg(target_os = "linux")]
    use mpris_server::LocalServer;
    use once_cell::sync::OnceCell;

    use super::*;

    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/mpvpage.ui")]
    #[properties(wrapper_type = super::MPVPage)]
    pub struct MPVPage {
        #[property(get, set, nullable)]
        pub url: RefCell<Option<String>>,
        #[property(get, set = Self::set_fullscreened, explicit_notify)]
        pub fullscreened: Cell<bool>,
        #[property(get, set = Self::set_paused)]
        pub paused: Cell<bool>,
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
        pub danmaku_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub menu_popover: TemplateChild<gtk::Popover>,
        #[template_child]
        pub title_label1: TemplateChild<gtk::Label>,
        #[template_child]
        pub title_label2: TemplateChild<gtk::Label>,
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
        #[cfg(target_os = "linux")]
        pub mpris_server: OnceCell<LocalServer<super::MPVPage>>,

        #[template_child]
        pub volume_adj: TemplateChild<gtk::Adjustment>,

        #[template_child]
        pub volume_bar: TemplateChild<VolumeBar>,

        #[template_child]
        pub video_overlay: TemplateChild<gtk::Overlay>,

        #[template_child]
        pub danmaku_area: TemplateChild<DanmakwArea>,

        #[template_child]
        pub danmaku_page: TemplateChild<adw::PreferencesPage>,

        #[template_child]
        pub danmaku_popover: TemplateChild<gtk::Popover>,

        #[template_child]
        pub danmaku_switch: TemplateChild<gtk::Switch>,

        #[template_child]
        pub danmaku_top_padding_adj: TemplateChild<gtk::Adjustment>,

        #[template_child]
        pub danmaku_font_size_adj: TemplateChild<gtk::Adjustment>,

        #[template_child]
        pub topcenter_danmaku_max_lines_adj: TemplateChild<gtk::Adjustment>,

        #[template_child]
        pub scroll_danmaku_max_lines_adj: TemplateChild<gtk::Adjustment>,

        #[template_child]
        pub bottomcenter_danmaku_max_lines_adj: TemplateChild<gtk::Adjustment>,

        #[template_child]
        pub danmaku_speed_adj: TemplateChild<gtk::Adjustment>,

        #[template_child]
        pub danmaku_row_spacing_adj: TemplateChild<gtk::Adjustment>,

        #[template_child]
        pub danmaku_opacity_adj: TemplateChild<gtk::Adjustment>,

        #[property(get, set, nullable)]
        pub current_video: RefCell<Option<TuItem>>,
        pub current_episode_list: RefCell<Vec<TuItem>>,

        #[property(get, set, default_value = true)]
        pub key_vaild: RefCell<bool>,

        pub video_version_matcher: RefCell<Option<String>>,

        pub danmaku_client: OnceCell<dandanapi::DanDanClient>,

        pub danmaku_list: RefCell<Option<Vec<danmakw::Danmaku>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MPVPage {
        const NAME: &'static str = "MPVPage";
        type Type = super::MPVPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            MPVGLArea::ensure_type();
            VideoScale::ensure_type();
            AActionRow::ensure_type();
            VolumeBar::ensure_type();
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
            klass.install_action(
                "mpv.show-settings",
                None,
                move |mpv, _action, _parameter| {
                    mpv.on_sidebar_clicked();
                },
            );
            klass.install_action(
                "mpv.show-playlist",
                None,
                move |mpv, _action, _parameter| {
                    mpv.on_playlist_clicked();
                },
            );
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

    #[glib::derived_properties]
    impl ObjectImpl for MPVPage {
        fn constructed(&self) {
            self.parent_constructed();

            SETTINGS
                .bind(
                    "mpv-show-buffer-speed",
                    &self.network_speed_label_2.get(),
                    "visible",
                )
                .build();

            SETTINGS
                .bind("mpv-default-volume", &self.volume_adj.get(), "value")
                .build();

            SETTINGS
                .bind("is-danmaku-enabled", &self.danmaku_switch.get(), "active")
                .build();

            self.danmaku_area
                .set_enable_danmaku(SETTINGS.is_danmaku_enabled());

            self.danmaku_font_size_adj
                .bind_property("value", &self.danmaku_area.get(), "font-size")
                .flags(glib::BindingFlags::BIDIRECTIONAL)
                .build();

            self.danmaku_top_padding_adj
                .bind_property("value", &self.danmaku_area.get(), "top-padding")
                .flags(glib::BindingFlags::BIDIRECTIONAL)
                .build();

            self.scroll_danmaku_max_lines_adj
                .bind_property("value", &self.danmaku_area.get(), "max-lines")
                .flags(glib::BindingFlags::BIDIRECTIONAL)
                .build();

            self.bottomcenter_danmaku_max_lines_adj
                .bind_property("value", &self.danmaku_area.get(), "bottom-center-max-lines")
                .flags(glib::BindingFlags::BIDIRECTIONAL)
                .build();

            self.topcenter_danmaku_max_lines_adj
                .bind_property("value", &self.danmaku_area.get(), "top-center-max-lines")
                .flags(glib::BindingFlags::BIDIRECTIONAL)
                .build();

            self.danmaku_speed_adj
                .bind_property("value", &self.danmaku_area.get(), "speed-factor")
                .flags(glib::BindingFlags::BIDIRECTIONAL)
                .build();

            self.danmaku_row_spacing_adj
                .bind_property("value", &self.danmaku_area.get(), "row-spacing")
                .flags(glib::BindingFlags::BIDIRECTIONAL)
                .build();

            self.danmaku_opacity_adj
                .bind_property("value", &self.danmaku_area.get(), "opacity")
                .flags(glib::BindingFlags::BIDIRECTIONAL)
                .build();

            SETTINGS
                .bind("danmaku-opacity", &self.danmaku_opacity_adj.get(), "value")
                .build();

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

            self.init_dandanapi_client();

            // Initialize MPRIS server
            #[cfg(target_os = "linux")]
            glib::spawn_future_local(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                async move {
                    let app_id = format!("{}.{}", APP_ID, "mpv");
                    if let Err(e) = imp.obj().initialize_mpris(&app_id).await {
                        tracing::warn!("Failed to initialize mpris server: {}", e);
                    }
                }
            ));
        }
    }

    impl WidgetImpl for MPVPage {
        fn unrealize(&self) {
            self.video_overlay
                .get()
                .remove_overlay(&self.danmaku_area.get());
            self.parent_unrealize();
        }
    }

    impl WindowImpl for MPVPage {}

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

        fn set_paused(&self, paused: bool) {
            let play_pause_image = self.play_pause_image.get();
            let menu_actions_play_pause_button = self.menu_actions.imp().play_pause_button.get();
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
            self.paused.set(paused);
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
        Object::new()
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


    fn on_cache_time_update(&self, value: i64) {
        self.imp().video_scale.set_cache_end_time(value);
    }

    fn update_duration(&self, value: f64) {
        let imp = self.imp();
        imp.video_scale.set_range(0.0, value);
        imp.duration_label.set_text(&format_duration(value as i64));
    }

    fn speed_cb(&self, value: f64) {
        self.imp().speed_spin.set_value(value);
    }

    fn volume_cb(&self, value: i64) {
        self.imp().volume_spin.set_value(value as f64);
        self.imp().volume_bar.set_level(value as f64 / 100.0);
    }

    fn scale_cb(&self, value: i64) {
        self.imp().video_scale.set_value(value as f64);
    }

    #[template_callback]
    fn video_scroll_cb(&self, _: f64, dy: f64) -> bool {
        self.imp().video.volume_scroll(-dy as i64 * 5);
        true
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
        self.handle_callback(BackType::Start);
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



    pub fn on_pause_update(&self, value: bool) {
        if !value {
            self.update_timeout();
            self.notify_playing();
        } else {
            self.remove_timeout();
            self.notify_player_paused();
        }

        self.set_paused(value);
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

        let is_threshold = (old_x - x).abs() > 3.0 || (old_y - y).abs() > 3.0;

        if is_threshold {
            if !self.toolbar_revealed() {
                self.set_reveal_overlay(true);
            }

            self.reset_fade_timeout();

            *imp.last_motion_time.borrow_mut() = now;
        }
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
        if let Some(timeout) = imp.timeout.take() {
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
        self.imp().bottom_revealer.is_child_revealed()
    }

    fn fade_overlay_delay_cb(&self) {
        *self.imp().timeout.borrow_mut() = None;

        let binding = self.ancestor(adw::OverlaySplitView::static_type());
        let Some(view) = binding.and_downcast_ref::<adw::OverlaySplitView>() else {
            return;
        };

        if view.shows_sidebar() {
            return;
        }

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
                if !widget.is::<MPVGLArea>() && !widget.is::<DanmakwArea>() {
                    return false;
                }
            }
        }

        if self.imp().menu_button.is_active() {
            return false;
        }

        if self.imp().danmaku_button.is_active() {
            return false;
        }

        let binding = self.ancestor(gtk::Stack::static_type());
        let Some(view) = binding.and_downcast_ref::<gtk::Stack>() else {
            return false;
        };

        if view.visible_child_name() != Some("mpv".into()) {
            return false;
        }

        true
    }

    fn set_reveal_overlay(&self, reveal: bool) {
        let imp = self.imp();
        imp.bottom_revealer.set_reveal_child(reveal);
        imp.top_revealer.set_reveal_child(reveal);
        let Some(surface) = self.native().and_then(|f| f.surface()) else {
            return;
        };
        let cursor = if reveal {
            gtk::gdk::Cursor::from_name("default", None)
        } else {
            let Some(pixbuf) =
                gtk::gdk_pixbuf::Pixbuf::new(gtk::gdk_pixbuf::Colorspace::Rgb, true, 8, 1, 1)
            else {
                return;
            };
            pixbuf.fill(0);
            let texture = gtk::gdk::Texture::for_pixbuf(&pixbuf);
            Some(gtk::gdk::Cursor::from_texture(&texture, 0, 0, None))
        };

        surface.set_cursor(cursor.as_ref());
    }

    #[template_callback]
    fn on_play_pause_clicked(&self) {
        let video = &self.imp().video;
        video.pause();
    }

    #[template_callback]
    fn on_stop_clicked(&self) {
        self.handle_callback(BackType::Stop);
        self.remove_timeout();
        self.imp().pause_danmaku();

        let mpv = self.mpv();
        mpv.pause(true);
        mpv.stop();
        mpv.event_thread_alive
            .store(PAUSED, std::sync::atomic::Ordering::SeqCst);
        let root = self.root();
        let window = root
            .and_downcast_ref::<crate::ui::widgets::window::Window>()
            .unwrap();
        window.imp().stack.set_visible_child_name("main");
        window.allow_suspend();

        spawn_g_timeout(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            #[weak]
            window,
            async move {
                if let Some(timeout) = obj.imp().timeout.take() {
                    glib::source::SourceId::remove(timeout);
                }
                obj.set_reveal_overlay(true);
                window.update_item_page().await;
            }
        ));
        self.notify_stopped();
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
        self.imp().video.pause();
    }

    #[template_callback]
    fn on_playlist_clicked(&self) {
        let binding = self.root();
        let Some(window) = binding.and_downcast_ref::<Window>() else {
            return;
        };
        window.view_playlist();
    }

    fn on_sidebar_clicked(&self) {
        let binding = self.root();
        let Some(window) = binding.and_downcast_ref::<Window>() else {
            return;
        };
        window.view_control_sidebar();
    }

    pub fn key_pressed_cb(&self, key: u32, state: gtk::gdk::ModifierType) {
        let binding = self.ancestor(adw::OverlaySplitView::static_type());
        let Some(view) = binding.and_downcast_ref::<adw::OverlaySplitView>() else {
            return;
        };

        if view.shows_sidebar() {
            return;
        }

        self.imp().video.press_key(key, state)
    }

    pub fn key_released_cb(&self, key: u32, state: gtk::gdk::ModifierType) {
        let binding = self.ancestor(adw::OverlaySplitView::static_type());
        let Some(view) = binding.and_downcast_ref::<adw::OverlaySplitView>() else {
            return;
        };

        if view.shows_sidebar() {
            return;
        }

        self.imp().video.release_key(key, state)
    }

    pub fn set_popover(&self) {
        let imp = self.imp();
        let builder = Builder::from_resource("/moe/tsuna/tsukimi/ui/mpv_menu.ui");
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
        let step = SETTINGS.mpv_seek_backward_step() as i64;
        video.seek_backward(step);
        self.notify_seeked(step);
    }

    pub fn on_forward(&self) {
        let video = &self.imp().video;
        let step = SETTINGS.mpv_seek_forward_step() as i64;
        video.seek_forward(step);
        self.notify_seeked(step);
    }

    pub fn chapter_prev(&self) {
        self.key_pressed_cb(PREV_CHAPTER_KEYVAL, gtk::gdk::ModifierType::empty());
    }

    pub fn chapter_next(&self) {
        self.key_pressed_cb(NEXT_CHAPTER_KEYVAL, gtk::gdk::ModifierType::empty());
    }

    pub async fn load_danmaku(&self) {
        if !self.key_vaild() {
            return;
        }

        let Some(item) = self.current_video() else {
            return;
        };

        let (anime, episode, time_ticks) = {
            if let Some(series_name) = item.series_name() {
                (series_name, item.name(), item.playback_position_ticks())
            } else {
                (
                    item.name(),
                    "movie".to_string(),
                    item.playback_position_ticks(),
                )
            }
        };

        let time_milis = (time_ticks / 10000) as f64;

        let imp = self.imp();
        let danmaku = self
            .request_danmaku(dandanapi::RequestEpisodes {
                anime,
                episode,
                tmdb_id: None,
            })
            .await;

        match danmaku {
            Ok(danmaku) => {
                self.imp().danmaku_page.set_description(&format!(
                    "{} {}",
                    danmaku.len(),
                    gettext("Danmaku Loaded")
                ));
                imp.init_danmaku(danmaku, time_milis);
            }
            Err(e) => {
                tracing::error!("Loading danmaku error: {}", e);
                self.imp()
                    .danmaku_page
                    .set_description(&gettext("No Danmaku Loaded"));
            }
        }
    }

    pub async fn request_danmaku(
        &self, request_episodes: dandanapi::RequestEpisodes,
    ) -> Result<Vec<danmakw::Danmaku>> {
        let client = self
            .imp()
            .danmaku_client
            .get()
            .ok_or_else(|| anyhow::anyhow!("Danmaku client not initialized"))?;

        let route = client.route(dandanapi::Episodes(request_episodes));

        let response = spawn_tokio(async move {
            let response = route.await?;
            Ok::<dandanapi::SearchEpisodesResponse, anyhow::Error>(response)
        })
        .await?;

        let episode_id = response
            .animes
            .and_then(|anims| anims.first()?.episodes.first().map(|ep| ep.episode_id))
            .ok_or_else(|| anyhow::anyhow!("No episode found"))?;

        let route = client.route(dandanapi::Comments {
            episode_id,
            request_comments: dandanapi::RequestComments {
                from: 0,
                with_related: true,
                ch_convert: dandanapi::ChConvert::NONE,
            },
        });

        let danmaku = spawn_tokio(async move {
            let comments = route
                .await?
                .comments
                .ok_or_else(|| anyhow::anyhow!("No comment found"))?;

            let danmaku = comments
                .into_iter()
                .map(|comment| comment.into_danmaku())
                .collect::<Vec<_>>();

            Ok::<Vec<danmakw::Danmaku>, anyhow::Error>(danmaku)
        })
        .await?;

        Ok(danmaku)
    }

    #[template_callback]
    pub fn on_danmaku_switch_state_set(&self, state: bool) -> bool {
        self.imp().danmaku_area.set_enable_danmaku(state);

        if state {
            if let Some(danmaku) = self.imp().danmaku_list.borrow().as_ref() {
                self.imp().danmaku_area.set_danmaku(danmaku.to_owned());
                self.imp().resume_danmaku();
            } else {
                spawn(glib::clone!(
                    #[weak(rename_to = obj)]
                    self,
                    async move {
                        obj.load_danmaku().await;
                    }
                ));
            }
        } else {
            self.imp().danmaku_area.clear_danmaku();
            self.imp().pause_danmaku();
        }

        let _ = SETTINGS.set_danmaku_enabled(state);

        false
    }

    pub fn notify_has_chapters(&self, has_chapters: bool) {
        #[cfg(target_os = "linux")]
        self.notify_mpris_has_chapters(has_chapters);
    }

    pub fn notify_playing(&self) {
        #[cfg(target_os = "linux")]
        self.notify_mpris_playing();
    }

    pub fn notify_player_paused(&self) {
        #[cfg(target_os = "linux")]
        self.notify_mpris_paused();
    }

    pub fn notify_stopped(&self) {
        #[cfg(target_os = "linux")]
        self.notify_mpris_stopped();
    }

    pub fn notify_seeked(&self, position: i64) {
        #[cfg(target_os = "linux")]
        self.notify_mpris_seeked(position);
    }
}

pub async fn direct_stream_url(source: &MediaSource) -> Option<String> {
    let container = source.container.to_owned()?;
    let etag = source.etag.to_owned()?;
    Some(
        JELLYFIN_CLIENT
            .get_direct_stream_url(&container, &source.id.to_owned(), &etag)
            .await,
    )
}

pub async fn extract_url(source: &MediaSource) -> Option<String> {
    source
        .direct_stream_url
        .as_ref()
        .or(source
            .transcoding_url
            .as_ref()
            .or(direct_stream_url(source).await.as_ref()))
        .map(|url| url.to_string())
}
