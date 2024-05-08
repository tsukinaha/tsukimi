
use gtk::{glib, prelude::*, subclass::prelude::*, template_callbacks};

use crate::{ui::provider::{core_song::{self, CoreSong}, tu_item::TuItem}, utils::get_image_with_cache};

use super::song_widget::State;

mod imp {
    use std::cell::OnceCell;

    use adw::subclass::bin::BinImpl;
    use gtk::{glib::subclass::InitializingObject, CompositeTemplate};

    use crate::{gstl::list, ui::{provider::core_song::CoreSong, widgets::smooth_scale::SmoothScale}};

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
        self.imp().player.play(core_song)
    }

    pub async fn set_item(&self, item: &TuItem) {
        let imp = self.imp();
        imp.title_label.set_text(&item.name());
        imp.artist_label.set_text(&item.album_artist().unwrap_or_default());
        let path = get_image_with_cache(&item.id(), "Primary", None).await.unwrap();
        imp.cover_image.set_file(Some(&path));
    }

    #[template_callback]
    fn on_play_button_clicked(&self) {
        let player = &self.imp().player;
        let play_pause_image = &self.imp().play_pause_image.get();
        if player.state() == State::PLAYING {
            player.pause();
            play_pause_image.set_icon_name(Some("media-playback-start-symbolic"));
        } else {
            player.unpause();
            play_pause_image.set_icon_name(Some("media-playback-pause-symbolic"));
        }
    }
}
