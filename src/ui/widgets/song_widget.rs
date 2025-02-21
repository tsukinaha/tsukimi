use adw::{
    prelude::*,
    subclass::prelude::*,
};
use chrono::Duration;
use gtk::{
    CompositeTemplate,
    glib,
};

use crate::{
    ui::provider::{
        actions::HasLikeAction,
        core_song::CoreSong,
        tu_item::TuItem,
    },
    utils::spawn,
};

#[derive(Default, Hash, Eq, PartialEq, Clone, Copy, glib::Enum, Debug)]
#[repr(u32)]
#[enum_type(name = "State")]
pub enum State {
    Played,
    Playing,
    #[default]
    Unplayed,
}

pub(crate) mod imp {
    use std::cell::{
        Cell,
        OnceCell,
    };

    use glib::subclass::InitializingObject;

    use super::*;
    use crate::{
        insert_editm_dialog,
        ui::{
            provider::{
                core_song::CoreSong,
                tu_item::TuItem,
            },
            widgets::{
                image_dialog::ImageDialog,
                metadata_dialog::MetadataDialog,
                star_toggle::StarToggle,
            },
        },
    };

    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/song_widget.ui")]
    #[properties(wrapper_type = super::SongWidget)]
    pub struct SongWidget {
        #[property(get, set, construct_only)]
        pub item: OnceCell<TuItem>,
        #[property(get, set = Self::set_state, explicit_notify, builder(State::default()))]
        pub state: Cell<State>,
        #[template_child]
        pub number_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub title_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub artist_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub duration_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub favourite_button: TemplateChild<StarToggle>,
        #[template_child]
        pub play_icon: TemplateChild<gtk::Image>,
        #[property(get, set, construct_only)]
        pub coresong: OnceCell<CoreSong>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SongWidget {
        const NAME: &'static str = "SongWidget";
        type Type = super::SongWidget;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            StarToggle::ensure_type();
            klass.bind_template();
            klass.install_action_async(
                "song.editm",
                None,
                |window, _action, _parameter| async move {
                    let id = window.item().id();
                    let dialog = MetadataDialog::new(&id);
                    insert_editm_dialog!(window, dialog);
                },
            );
            klass.install_action_async(
                "song.editi",
                None,
                |window, _action, _parameter| async move {
                    let id = window.item().id();
                    let dialog = ImageDialog::new(&id);
                    insert_editm_dialog!(window, dialog);
                },
            );
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for SongWidget {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            obj.set_up();

            let core_song = self.obj().coresong();

            let item = obj.item();
            if let Some(album_id) = item.album_id() {
                core_song.set_album_id(album_id);
            }
            core_song.set_artist(item.albumartist_name());
            core_song.set_name(item.name());
            core_song.set_id(item.id());
            if let Some(image_tags) = item.image_tags() {
                if image_tags.primary().is_some() {
                    core_song.set_have_single_track_image(true);
                }
            }
            core_song.set_duration(item.run_time_ticks() / 10000000);
            obj.bind(&core_song);
        }
    }
    impl WidgetImpl for SongWidget {}
    impl ListBoxRowImpl for SongWidget {}

    impl SongWidget {
        fn set_state(&self, state: State) {
            if self.state.get() == state {
                return;
            }
            let obj = self.obj();
            let ctx = self.title_label.get();
            ctx.remove_css_class("dim-label");
            ctx.remove_css_class("playing-song-label");
            self.play_icon.set_visible(false);
            match state {
                State::Played => {
                    ctx.add_css_class("dim-label");
                }
                State::Playing => {
                    ctx.add_css_class("playing-song-label");
                    self.play_icon.set_visible(true);
                }
                _ => {}
            }
            self.state.set(state);
            obj.notify_state();
        }
    }
}

glib::wrapper! {
    pub struct SongWidget(ObjectSubclass<imp::SongWidget>)
        @extends gtk::Widget, adw::Dialog, adw::NavigationPage, @implements gtk::Accessible;
}

impl SongWidget {
    pub fn new(item: TuItem) -> Self {
        glib::Object::builder()
            .property("coresong", CoreSong::new(&item.id()))
            .property("item", item)
            .build()
    }

    pub fn set_up(&self) {
        let imp = self.imp();
        let item = imp.item.get().unwrap();
        imp.number_label.set_text(&item.index_number().to_string());
        imp.title_label.set_text(&item.name());
        imp.artist_label
            .set_text(&item.artists().unwrap_or("".to_string()));
        let duration = item.run_time_ticks() / 10000000;
        imp.duration_label
            .set_text(&format_duration(duration as i64));
        imp.favourite_button.set_active(item.is_favorite());

        let id = item.id();
        spawn(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                obj.bind_like(&id).await;
            }
        ));
    }

    fn bind(&self, core_song: &CoreSong) {
        self.bind_property("state", core_song, "state")
            .sync_create()
            .bidirectional()
            .build();
    }
}

pub fn format_duration(seconds: i64) -> String {
    let duration = Duration::seconds(seconds);
    let minutes = duration.num_minutes();
    let seconds = duration.num_seconds() % 60;
    format!("{}:{:02}", minutes, seconds)
}
