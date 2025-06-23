use adw::{
    prelude::*,
    subclass::prelude::*,
};
use gettextrs::gettext;
use gtk::{
    SpinButton,
    glib,
    template_callbacks,
};

use super::utils::GlobalToast;
use crate::{
    client::{
        error::UserFacingError,
        jellyfin_client::JELLYFIN_CLIENT,
        structs::SimpleListItem,
    },
    utils::{
        spawn,
        spawn_tokio,
    },
};
mod imp {
    use std::cell::{
        OnceCell,
        RefCell,
    };

    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        glib,
    };
    use serde_json::Value;

    use super::*;
    use crate::{
        client::structs::SimpleListItem,
        ui::{
            provider::IS_ADMIN,
            widgets::item::dt,
        },
        utils::spawn,
    };

    #[derive(Debug, Default, CompositeTemplate, glib::Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/metadata_dialog.ui")]
    #[properties(wrapper_type = super::MetadataDialog)]
    pub struct MetadataDialog {
        #[property(get, set, construct_only)]
        pub id: OnceCell<String>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,

        #[template_child]
        pub page: TemplateChild<adw::NavigationPage>,

        #[template_child]
        pub hint: TemplateChild<adw::ActionRow>,

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
        #[template_child]
        pub apply_button: TemplateChild<adw::ButtonRow>,

        pub timezone: RefCell<Option<glib::DateTime>>,
        pub value: OnceCell<Value>,
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
            if IS_ADMIN.load(std::sync::atomic::Ordering::Relaxed) {
                self.hint.set_visible(false);
                self.apply_button.set_sensitive(true);
            }

            spawn(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                async move { imp.obj().get_data().await }
            ));
        }

        pub fn load_data(&self, metadata: SimpleListItem) {
            self.path_entry.set_subtitle(
                &metadata
                    .path
                    .unwrap_or("No Data".to_string())
                    .replace('&', "&amp;"),
            );
            self.title_entry.set_text(&metadata.name);
            self.sorttitle_entry
                .set_text(&metadata.sort_name.unwrap_or_default());
            self.timezone
                .replace(metadata.date_created.as_ref().map(|x| {
                    glib::DateTime::from_iso8601(&x.to_rfc3339(), None)
                        .unwrap()
                        .to_local()
                        .unwrap()
                }));
            self.date_entry.set_subtitle(&dt(metadata.date_created));
            self.overview_entry
                .buffer()
                .set_text(&metadata.overview.unwrap_or_default());
            self.lock_check
                .set_active(metadata.lock_data.unwrap_or_default());

            if metadata.item_type == "MusicAlbum" || metadata.item_type == "Audio" {
                return;
            }

            if let Some(provider_ids) = metadata.provider_ids {
                self.moviedb_entry
                    .set_text(&provider_ids.tmdb.unwrap_or_default());
                self.tvdb_entry
                    .set_text(&provider_ids.tvdb.unwrap_or_default());
                self.imdb_entry
                    .set_text(&provider_ids.imdb.unwrap_or_default());
                self.ids_group.set_visible(true);
            }
        }

        pub fn get_edit_data(&self) -> Option<Value> {
            let value = self.value.get()?;

            let mut value = value.to_owned();

            let title = self.title_entry.text().to_string();
            if !title.is_empty() {
                value["Name"] = Value::String(title);
            }

            let sort_title = self.sorttitle_entry.text().to_string();
            if !sort_title.is_empty() {
                value["SortName"] = Value::String(sort_title);
            }

            let buffer = self.overview_entry.buffer();
            let overview = buffer
                .text(&buffer.start_iter(), &buffer.end_iter(), false)
                .to_string();
            if !overview.is_empty() {
                value["Overview"] = Value::String(overview);
            }

            let lock_data = self.lock_check.is_active();
            value["LockData"] = Value::Bool(lock_data);

            if let Some(date) = self.timezone.borrow().as_ref() {
                let Ok(date) = date.to_utc().and_then(|x| x.format_iso8601()) else {
                    println!("Error converting date");
                    return None;
                };
                value["DateCreated"] = Value::String(date.to_string());
            }

            let tmdb = self.moviedb_entry.text().to_string();
            if !tmdb.is_empty() {
                value["ProviderIds"]["Tmdb"] = Value::String(tmdb);
            }

            let tvdb = self.tvdb_entry.text().to_string();
            if !tvdb.is_empty() {
                value["ProviderIds"]["Tvdb"] = Value::String(tvdb);
            }

            let imdb = self.imdb_entry.text().to_string();
            if !imdb.is_empty() {
                value["ProviderIds"]["IMDB"] = Value::String(imdb);
            }

            Some(value)
        }
    }
}

glib::wrapper! {

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
        match spawn_tokio(async move { JELLYFIN_CLIENT.get_edit_info(&id).await }).await {
            Ok(metadata) => {
                self.imp().stack.set_visible_child_name("page");
                let value = metadata.to_owned();
                let _ = self.imp().value.set(value);
                let Ok(item) = serde_json::from_value::<SimpleListItem>(metadata.to_owned()) else {
                    return;
                };
                self.imp().load_data(item);
            }
            Err(e) => {
                self.toast(e.to_user_facing());
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
        )
        .unwrap();

        self.set_time(date)
    }

    #[template_callback]
    fn on_apply_button_clicked(&self) {
        let Some(value) = self.imp().get_edit_data() else {
            return;
        };

        let id = self.id();

        let alert_dialog = adw::AlertDialog::builder()
            .heading(gettext("Apply"))
            .title("Apply")
            .body(gettext("Are you sure you wish to continue?"))
            .build();

        alert_dialog.add_response("close", &gettext("Cancel"));
        alert_dialog.add_response("ok", &gettext("Ok"));
        alert_dialog.set_response_appearance("ok", adw::ResponseAppearance::Suggested);

        alert_dialog.connect_response(
            Some("ok"),
            glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move |_, _| {
                    spawn(glib::clone!(
                        #[weak]
                        obj,
                        #[strong]
                        id,
                        #[strong]
                        value,
                        async move {
                            match spawn_tokio(
                                async move { JELLYFIN_CLIENT.post_item(&id, value).await },
                            )
                            .await
                            {
                                Ok(_) => {
                                    obj.toast(gettext("Success"));
                                    obj.close();
                                }
                                Err(e) => {
                                    obj.toast(e.to_user_facing());
                                }
                            }
                        }
                    ))
                }
            ),
        );

        alert_dialog.present(Some(self));
    }

    fn get_time(&self) -> glib::DateTime {
        let timezone = self.imp().timezone.borrow();
        let now_local = glib::DateTime::now_local().unwrap();
        timezone.as_ref().unwrap_or(&now_local).to_owned()
    }

    fn set_time(&self, date: glib::DateTime) {
        self.imp().date_entry.set_subtitle(&format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            date.year(),
            date.month(),
            date.day_of_month(),
            date.hour(),
            date.minute(),
            date.second()
        ));
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
        if let Ok(date) = glib::DateTime::from_local(
            date_time.year(),
            date_time.month(),
            date_time.day_of_month(),
            hour,
            minute,
            second,
        ) {
            self.set_time(date);
        }
    }
}
