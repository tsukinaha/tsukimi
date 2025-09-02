use adw::subclass::prelude::*;
use gtk::{
    CompositeTemplate,
    glib,
    prelude::*,
    template_callbacks,
};

use super::playlist_song_widget::PlaylistSongWidget;
use crate::ui::provider::tu_item::TuItem;

mod imp {
    use std::sync::OnceLock;

    use glib::subclass::{
        InitializingObject,
        Signal,
    };

    use super::*;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/playlist_box.ui")]
    pub struct PlaylistBox {
        #[template_child]
        pub listbox: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PlaylistBox {
        const NAME: &'static str = "PlaylistBox";
        type Type = super::PlaylistBox;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PlaylistBox {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("song-activated")
                        .param_types([PlaylistSongWidget::static_type()])
                        .build(),
                ]
            })
        }
    }

    impl WidgetImpl for PlaylistBox {}
    impl BoxImpl for PlaylistBox {}
}

glib::wrapper! {

    pub struct PlaylistBox(ObjectSubclass<imp::PlaylistBox>)
        @extends gtk::Widget, adw::Dialog, adw::NavigationPage, @implements gtk::Accessible;
}

impl Default for PlaylistBox {
    fn default() -> Self {
        Self::new()
    }
}

#[template_callbacks]
impl PlaylistBox {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn add_song(&self, item: TuItem) {
        let listbox = self.imp().listbox.get();
        let song_widget = PlaylistSongWidget::new(item);
        listbox.append(&song_widget);
    }

    #[template_callback]
    pub fn song_activated(&self, song_widget: &PlaylistSongWidget) {
        self.emit_by_name::<()>("song-activated", &[&song_widget]);
    }
}
