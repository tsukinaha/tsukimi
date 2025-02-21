use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::{
    CompositeTemplate,
    glib,
    prelude::*,
    template_callbacks,
};

use super::song_widget::SongWidget;
use crate::ui::provider::tu_item::TuItem;

mod imp {
    use std::sync::OnceLock;

    use glib::subclass::{
        InitializingObject,
        Signal,
    };

    use super::*;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/disc_box.ui")]
    pub struct DiscBox {
        #[template_child]
        pub disc_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub listbox: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DiscBox {
        const NAME: &'static str = "DiscBox";
        type Type = super::DiscBox;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for DiscBox {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("song-activated")
                        .param_types([SongWidget::static_type()])
                        .build(),
                ]
            })
        }
    }

    impl WidgetImpl for DiscBox {}
    impl BoxImpl for DiscBox {}
}

glib::wrapper! {

    pub struct DiscBox(ObjectSubclass<imp::DiscBox>)
        @extends gtk::Widget, adw::Dialog, adw::NavigationPage, @implements gtk::Accessible;
}

impl Default for DiscBox {
    fn default() -> Self {
        Self::new()
    }
}

#[template_callbacks]
impl DiscBox {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn set_disc(&self, disc: u32) {
        let disc_label = self.imp().disc_label.get();
        disc_label.set_text(&format!("{} {}", &gettext("Disc"), disc));
    }

    pub fn add_song(&self, item: TuItem) {
        let listbox = self.imp().listbox.get();
        let song_widget = SongWidget::new(item);
        listbox.append(&song_widget);
    }

    #[template_callback]
    pub fn song_activated(&self, song_widget: &SongWidget) {
        self.emit_by_name::<()>("song-activated", &[&song_widget]);
    }
}
