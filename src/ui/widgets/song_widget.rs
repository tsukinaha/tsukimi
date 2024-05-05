use adw::prelude::*;
use adw::subclass::prelude::*;
use chrono::Duration;
use gtk::{glib, CompositeTemplate};

use crate::ui::provider::tu_item::TuItem;

mod imp {
    use std::cell::OnceCell;

    use crate::ui::provider::tu_item::TuItem;

    use super::*;
    use glib::subclass::InitializingObject;

    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsukimi/song_widget.ui")]
    #[properties(wrapper_type = super::SongWidget)]
    pub struct SongWidget {
        #[property(get, set, construct_only)]
        pub item: OnceCell<TuItem>,
        #[template_child]
        pub number_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub title_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub artist_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub duration_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SongWidget {
        const NAME: &'static str = "SongWidget";
        type Type = super::SongWidget;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
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
        }
    }

    impl WidgetImpl for SongWidget {}
    impl ListBoxRowImpl for SongWidget {}
}

glib::wrapper! {
    /// Preference Window to display and update room details.
    pub struct SongWidget(ObjectSubclass<imp::SongWidget>)
        @extends gtk::Widget, adw::Dialog, adw::NavigationPage, @implements gtk::Accessible;
}

impl SongWidget {
    pub fn new(item: TuItem) -> Self {
        glib::Object::builder().property("item", item).build()
    }

    pub fn set_up(&self) {
        let imp = self.imp();
        let item = imp.item.get().unwrap();
        imp.number_label.set_text(&item.index_number().to_string());
        imp.title_label.set_text(&item.name());
        imp.artist_label.set_text(&item.artists().unwrap_or("".to_string()));
        let duration = item.run_time_ticks() / 10000000;
        imp.duration_label.set_text(&Self::format_duration(duration as i64));
    }
    
    fn format_duration(seconds: i64) -> String {
        let duration = Duration::seconds(seconds);
        let minutes = duration.num_minutes();
        let seconds = duration.num_seconds() % 60;
        format!("{}:{:02}", minutes, seconds)
    }
}
