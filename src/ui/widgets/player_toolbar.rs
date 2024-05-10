use std::path::Path;

use gtk::{glib, prelude::*, subclass::prelude::*, template_callbacks};

use crate::{
    ui::provider::{core_song::CoreSong, tu_item::TuItem},
    utils::get_image_with_cache,
};

use super::{smooth_scale::SmoothScale, song_widget::format_duration};

mod imp {

    use adw::subclass::bin::BinImpl;
    use gtk::{glib::subclass::InitializingObject, CompositeTemplate};

    use crate::{gstl::list, ui::widgets::smooth_scale::SmoothScale};

    use super::*;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/moe/tsukimi/player_toolbar.ui")]
    pub struct PlayerToolbarBox {
        #[template_child]
        pub toolbar: TemplateChild<gtk::ActionBar>,
        pub player: list::Player,
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
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PlayerToolbarBox {
        fn constructed(&self) {
            self.parent_constructed();
            self.toolbar.set_revealed(true);
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

    pub fn play(&self, core_song: CoreSong) {
        self.imp().player.play(core_song);
        self.imp().progress_scale.update_timeout();
        let play_pause_image = &self.imp().play_pause_image.get();
        play_pause_image.set_icon_name(Some("media-playback-pause-symbolic"));
    }

    pub async fn set_item(&self, item: &TuItem) {
        let imp = self.imp();
        imp.title_label.set_text(&item.name());
        imp.artist_label
            .set_text(&item.album_artist().unwrap_or_default());
        let mut path =
            get_image_with_cache(&item.id(), "Primary", None)
                .await
                .unwrap();
        if !Path::new(&path).exists() {
            path = get_image_with_cache(&item.album_id().unwrap(), "Primary", None)
            .await
            .unwrap();
        }
        imp.cover_image.set_file(Some(&path));
        let duration = (item.run_time_ticks() / 10000000) as i64;
        imp.duration_label.set_text(&format_duration(duration));
        imp.progress_scale.set_range(0.0, duration as f64);
    }

    #[template_callback]
    fn on_progress_value_changed(&self, progress_scale: &SmoothScale) {
        let label = &self.imp().progress_time_label.get();
        let position = progress_scale.value();
        label.set_text(&format_duration(position as i64));
    }

    #[template_callback]
    fn on_play_button_clicked(&self) {
        let player = &self.imp().player;
        let play_pause_image = &self.imp().play_pause_image.get();
        if player.state() == gst::State::Playing {
            player.pause();
            play_pause_image.set_icon_name(Some("media-playback-start-symbolic"));
        } else {
            player.unpause();
            play_pause_image.set_icon_name(Some("media-playback-pause-symbolic"));
        }
    }
}
