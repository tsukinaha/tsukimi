use adw::subclass::prelude::*;
use gtk::glib;
use gtk::prelude::*;
use gtk::template_callbacks;
use adw::prelude::*;
use gtk::SpinButton;

use crate::{client::{client::EMBY_CLIENT, error::UserFacingError}, toast, utils::spawn_tokio};

mod imp {
    use std::cell::{OnceCell, RefCell};
    use gtk::prelude::*;
    use adw::prelude::*;
    use crate::{client::structs::Item, ui::{provider::IS_ADMIN, widgets::item::dt}, utils::spawn};
    use gtk::{glib, CompositeTemplate};
    use super::*;
    use glib::subclass::InitializingObject;

    #[derive(Debug, Default, CompositeTemplate, glib::Properties)]
    #[template(resource = "/moe/tsukimi/metadata_dialog.ui")]
    #[properties(wrapper_type = super::MetadataDialog)]
    pub struct MetadataDialog {
        #[property(get, set, construct_only)]
        pub id: OnceCell<String>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,  

        #[template_child]
        pub path_entry: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub title_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub sorttitle_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub date_entry: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub overview_entry: TemplateChild<gtk::TextView>,

        #[template_child]
        pub music_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub artist_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub album_artist_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub album_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub disc_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub track_entry: TemplateChild<adw::EntryRow>,

        #[template_child]
        pub ids_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub moviedb_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub tvdb_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub imdb_entry: TemplateChild<adw::EntryRow>,

        #[template_child]
        pub lock_check: TemplateChild<gtk::CheckButton>,

        #[template_child]
        pub hour_spin: TemplateChild<gtk::SpinButton>,
        #[template_child]
        pub minute_spin: TemplateChild<gtk::SpinButton>,
        #[template_child]
        pub second_spin: TemplateChild<gtk::SpinButton>,

        pub timezone: RefCell<Option<glib::DateTime>>
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MetadataDialog {
        const NAME: &'static str = "MetadataDialog";
        type Type = super::MetadataDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for MetadataDialog {
        fn constructed(&self) {
            self.parent_constructed();
            self.init();
        }
    }

    impl WidgetImpl for MetadataDialog {}
    impl AdwDialogImpl for MetadataDialog {}

    impl MetadataDialog {
        fn init(&self) {
            if !IS_ADMIN.load(std::sync::atomic::Ordering::Relaxed) {
                self.stack.set_visible_child_name("nopermission");
                return;
            }

            spawn(glib::clone!(@weak self as imp => async move {
                imp.obj().get_data().await
            }));
        }

        pub fn load_data(&self, metadata: Item) {
            self.path_entry.set_subtitle(&metadata.path.unwrap_or("No Data".to_string()));
            self.title_entry.set_text(&metadata.name);
            self.sorttitle_entry.set_text(&metadata.sort_name.unwrap_or_default());
            self.timezone.replace(metadata.date_created.as_ref().map(|x| glib::DateTime::from_iso8601(x, None).unwrap().to_local().unwrap()));
            self.date_entry.set_subtitle(&dt(metadata.date_created.as_deref()));
            self.overview_entry.buffer().set_text(&metadata.overview.unwrap_or_default());
            self.lock_check.set_active(metadata.lock_data.unwrap_or_default());

            if metadata.item_type == "Audio" {
                self.artist_entry.set_text(&metadata.artists.unwrap_or_default().join(","));
                self.album_artist_entry.set_text(&metadata.album_artist.unwrap_or_default());
                self.album_entry.set_text(&metadata.album.unwrap_or_default());
                self.disc_entry.set_text(&metadata.parent_index_number.unwrap_or_default().to_string());
                self.track_entry.set_text(&metadata.index_number.unwrap_or_default().to_string());
                self.music_group.set_visible(true);
                return;
            }
            
            if let Some(provider_ids) = metadata.provider_ids {
                self.moviedb_entry.set_text(&provider_ids.tmdb.unwrap_or_default());
                self.tvdb_entry.set_text(&provider_ids.tvdb.unwrap_or_default());
                self.imdb_entry.set_text(&provider_ids.imdb.unwrap_or_default());
                self.ids_group.set_visible(true);
            }
        }
    }
}

glib::wrapper! {
    /// Preference Window to display and update room details.
    pub struct MetadataDialog(ObjectSubclass<imp::MetadataDialog>)
        @extends gtk::Widget, adw::Dialog, adw::PreferencesDialog, @implements gtk::Accessible, gtk::Root;
}

#[template_callbacks]
impl MetadataDialog {
    pub fn new(id: &str) -> Self {
        glib::Object::builder().property("id", id).build()
    }

    async fn get_data(&self) {
        let id = self.id();
        match spawn_tokio(async move { EMBY_CLIENT.get_edit_info(&id).await } ).await {
            Ok(metadata) => {
                self.imp().stack.set_visible_child_name("page");
                self.imp().load_data(metadata);
            }
            Err(e) => {
                toast!(self, e.to_user_facing());
            }
        }
    }

    #[template_callback]
    fn on_day_selected(&self, calender: gtk::Calendar) {
        let date_time = self.get_time();
        
        let date = calender.date();
        let date = glib::DateTime::from_local(
            date.year(),
            date.month(),
            date.day_of_month(),
            date_time.hour(),
            date_time.minute(),
            date_time.second().into(),
        ).unwrap();

        self.set_time(date)
    }

    fn get_time(&self) -> glib::DateTime {
        let timezone = self.imp().timezone.borrow();
        let now_local = glib::DateTime::now_local().unwrap();
        timezone.as_ref().unwrap_or(&now_local).clone()
    }

    fn set_time(&self, date: glib::DateTime) {
        self.imp().date_entry.set_subtitle(&format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", 
            date.year(), date.month(), date.day_of_month(), date.hour(), date.minute(), date.second()));
        {
            self.imp().timezone.replace(Some(date));
        }
    }

    #[template_callback]
    fn on_time_changed(&self, _btn: SpinButton) {
        let date_time = self.get_time();
        let hour = self.imp().hour_spin.value() as i32;
        let minute = self.imp().minute_spin.value() as i32;
        let second = self.imp().second_spin.value();
        let date = glib::DateTime::from_local(
            date_time.year(),
            date_time.month(),
            date_time.day_of_month(),
            hour,
            minute,
            second,
        ).unwrap();
        self.set_time(date);
    }
}
