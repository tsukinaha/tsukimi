use crate::ui::provider::actions::HasLikeAction;
use crate::{
    ui::provider::{core_song::CoreSong, tu_item::TuItem},
    utils::spawn,
};
use adw::prelude::*;
use adw::subclass::prelude::*;
use chrono::Duration;
use gtk::{glib, CompositeTemplate};

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
    use super::*;
    use crate::ui::provider::core_song::CoreSong;
    use crate::ui::provider::tu_item::TuItem;
    use crate::ui::widgets::star_toggle::StarToggle;
    use crate::ui::widgets::window::Window;
    use crate::utils::spawn;
    use glib::subclass::InitializingObject;
    use std::cell::{Cell, OnceCell};

    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsukimi/song_widget.ui")]
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
            obj.bind(&core_song);
        }
    }
    impl WidgetImpl for SongWidget {}
    impl ListBoxRowImpl for SongWidget {
        fn activate(&self) {
            let core_song = self.obj().coresong();
            self.set_state(State::Playing);
            let window = self.obj().root().and_downcast::<Window>().unwrap();
            let player_toolbar = window.imp().player_toolbar_box.get();
            player_toolbar.play(core_song);
            spawn(glib::clone!(@weak self as obj => async move {
                player_toolbar.set_item(obj.item.get().unwrap()).await;
            }));
        }
    }

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
        let id = item.id();
        imp.number_label.set_text(&item.index_number().to_string());
        imp.title_label.set_text(&item.name());
        imp.artist_label
            .set_text(&item.artists().unwrap_or("".to_string()));
        let duration = item.run_time_ticks() / 10000000;
        imp.duration_label
            .set_text(&format_duration(duration as i64));
        imp.favourite_button.set_active(item.is_favorite());

        spawn(glib::clone!(@weak self as obj, @strong id => async move {
            obj.bind_actions(&id).await;
        }));
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
