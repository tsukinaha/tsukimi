use adw::prelude::*;
use adw::subclass::prelude::*;
use chrono::Duration;
use gtk::{glib, CompositeTemplate};

use crate::ui::provider::tu_item::TuItem;

use super::song_widget::SongWidget;

mod imp {
    use std::cell::OnceCell;

    use crate::ui::provider::tu_item::TuItem;

    use super::*;
    use glib::subclass::InitializingObject;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/moe/tsukimi/disc_box.ui")]
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
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for DiscBox {}

    impl WidgetImpl for DiscBox {}
    impl BoxImpl for DiscBox {}
}

glib::wrapper! {
    /// Preference Window to display and update room details.
    pub struct DiscBox(ObjectSubclass<imp::DiscBox>)
        @extends gtk::Widget, adw::Dialog, adw::NavigationPage, @implements gtk::Accessible;
}

impl DiscBox {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn set_disc(&self, disc: u32) {
        let disc_label = self.imp().disc_label.get();
        disc_label.set_text(&format!("Disc {}", disc));
    }

    pub fn add_song(&self, item: TuItem) {
        let listbox = self.imp().listbox.get();
        let song_widget = SongWidget::new(item);
        listbox.append(&song_widget);
    }
}