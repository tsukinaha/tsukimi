use gtk::{glib, prelude::*, subclass::prelude::*, template_callbacks};

use crate::{
    gstl::player::imp::ListRepeatMode,
    ui::{models::SETTINGS, provider::core_song::CoreSong},
    utils::{get_image_with_cache, spawn},
};

use super::{smooth_scale::SmoothScale, song_widget::format_duration};

mod imp {

    use adw::subclass::bin::BinImpl;
    use gtk::{glib::subclass::InitializingObject, CompositeTemplate};

    use crate::{
        gstl::player::{imp::ListRepeatMode, MusicPlayer},
        ui::widgets::smooth_scale::SmoothScale,
    };

    use super::*;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/com/github/inaha/tsukimi/ui/player_toolbar.ui")]
    pub struct PlayerToolbarBox {
        #[template_child]
        pub toolbar: TemplateChild<gtk::ActionBar>,
        pub player: MusicPlayer,
        #[template_child]
        pub cover_image: TemplateChild<gtk::Image>,
        #[template_child]
        pub title_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub artist_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub play_pause_image: TemplateChild<gtk::Image>,
        #[template_child]
        pub progress_scale: TemplateChild<SmoothScale>,
        #[template_child]
        pub progress_time_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub duration_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub repeat_image: TemplateChild<gtk::Image>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PlayerToolbarBox {
        const NAME: &'static str = "PlayerToolbarBox";
        type Type = super::PlayerToolbarBox;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            SmoothScale::ensure_type();
            klass.bind_template();
            klass.bind_template_instance_callbacks();

            klass.install_action(
                "repeat.none",
                None,
                move |player_toolbar_box, _action, _target| {
                    player_toolbar_box.set_repeat_mode(ListRepeatMode::None);
                },
            );

            klass.install_action(
                "repeat.one",
                None,
                move |player_toolbar_box, _action, _target| {
                    player_toolbar_box.set_repeat_mode(ListRepeatMode::RepeatOne);
                },
            );

            klass.install_action(
                "repeat.all",
                None,
                move |player_toolbar_box, _action, _target| {
                    player_toolbar_box.set_repeat_mode(ListRepeatMode::Repeat);
                },
            );
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PlayerToolbarBox {
        fn constructed(&self) {
            self.parent_constructed();
            self.progress_scale.set_player(Some(&self.player));
            let obj = self.obj().clone();
            self.player.connect_local("stream-start", true, move |_| {
                obj.change_view();
                None
            });
            self.obj()
                .set_repeat_mode(ListRepeatMode::from_string(&SETTINGS.music_repeat_mode()));
        }
    }

    impl WidgetImpl for PlayerToolbarBox {}
    impl BinImpl for PlayerToolbarBox {}
}

glib::wrapper! {
    /// Preference Window to display and update room details.
    pub struct PlayerToolbarBox(ObjectSubclass<imp::PlayerToolbarBox>)
        @extends gtk::Widget, gtk::ToggleButton, gtk::Button;
}

impl Default for PlayerToolbarBox {
    fn default() -> Self {
        Self::new()
    }
}

#[template_callbacks]
impl PlayerToolbarBox {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn toolbar_reveal(&self) {
        self.imp().toolbar.set_revealed(true)
    }

    pub fn set_repeat_mode(&self, mode: ListRepeatMode) {
        let player = &self.imp().player;
        player.set_repeat_mode(mode);
        SETTINGS.set_music_repeat_mode(mode.to_string()).unwrap();
        let i = &self.imp().repeat_image;
        match mode {
            ListRepeatMode::None => {
                i.set_icon_name(Some("media-playlist-consecutive-symbolic"));
            }
            ListRepeatMode::RepeatOne => {
                i.set_icon_name(Some("media-playlist-repeat-song-symbolic"));
            }
            ListRepeatMode::Repeat => {
                i.set_icon_name(Some("media-playlist-repeat-symbolic"));
            }
        }
    }

    pub fn update_play_state(&self) {
        self.imp().progress_scale.update_timeout();
        let play_pause_image = &self.imp().play_pause_image.get();
        play_pause_image.set_icon_name(Some("media-playback-pause-symbolic"));
        self.imp().toolbar.set_revealed(true);
    }

    pub fn change_view(&self) {
        let Some(core_song) = self.imp().player.active_core_song() else {
            return;
        };
        let imp = self.imp();
        imp.title_label.set_text(&core_song.name());
        imp.artist_label.set_text(&core_song.artist());
        imp.duration_label
            .set_text(&format_duration(core_song.duration() as i64));
        imp.progress_scale
            .set_range(0.0, core_song.duration() as f64);
        spawn(glib::clone!(
            #[weak]
            imp,
            async move {
                if core_song.have_single_track_image() {
                    let path = get_image_with_cache(&core_song.id(), "Primary", None)
                        .await
                        .unwrap();
                    imp.cover_image.set_from_file(Some(&path));
                } else {
                    let path = get_image_with_cache(&core_song.album_id(), "Primary", None)
                        .await
                        .unwrap();
                    imp.cover_image.set_from_file(Some(&path));
                }
            }
        ));
    }

    #[template_callback]
    fn on_progress_value_changed(&self, progress_scale: &SmoothScale) {
        let label = &self.imp().progress_time_label.get();
        let position = progress_scale.value();
        label.set_text(&format_duration(position as i64));
    }

    #[template_callback]
    pub fn on_stop_button_clicked(&self) {
        let imp = self.imp();
        imp.player.imp().stop();
        imp.progress_scale.remove_timeout();
        imp.toolbar.set_revealed(false);
    }

    #[template_callback]
    fn on_play_button_clicked(&self) {
        let player = &self.imp().player;
        let play_pause_image = &self.imp().play_pause_image.get();
        if player.imp().state() == gst::State::Playing {
            player.imp().pause();
            play_pause_image.set_icon_name(Some("media-playback-start-symbolic"));
        } else {
            player.imp().unpause();
            play_pause_image.set_icon_name(Some("media-playback-pause-symbolic"));
        }
    }

    pub fn bind_song_model(&self, active_model: gtk::gio::ListStore, active_core_song: CoreSong) {
        let player = &self.imp().player.imp();
        player.load_model(active_model, active_core_song);
        self.update_play_state();
    }

    #[template_callback]
    fn on_next_button_clicked(&self) {
        self.imp().player.imp().next();
    }

    #[template_callback]
    fn on_prev_button_clicked(&self) {
        self.imp().player.imp().prev();
    }
}
