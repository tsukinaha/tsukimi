use crate::utils::spawn_tokio;
use crate::{
    client::{client::EMBY_CLIENT, error::UserFacingError},
    toast,
    utils::spawn,
};
use crate::{fraction, fraction_reset};
use adw::prelude::*;
use adw::subclass::prelude::*;
use chrono::{DateTime, Datelike, Timelike, Utc};
use gettextrs::gettext;
use gtk::{glib, template_callbacks, CompositeTemplate};
use gtk::{Button, Image};

pub(crate) mod imp {
    use super::*;
    use glib::subclass::InitializingObject;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/moe/tsukimi/server_panel.ui")]
    pub struct ServerPanel {
        #[template_child]
        pub server_title: TemplateChild<gtk::Label>,
        #[template_child]
        pub server_version: TemplateChild<gtk::Label>,
        #[template_child]
        pub system_log_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub activity_log_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub task_group: TemplateChild<adw::PreferencesGroup>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ServerPanel {
        const NAME: &'static str = "ServerPanel";
        type Type = super::ServerPanel;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ServerPanel {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.set_up();
        }
    }

    impl WidgetImpl for ServerPanel {}
    impl AdwDialogImpl for ServerPanel {}
    impl NavigationPageImpl for ServerPanel {}
}

glib::wrapper! {
    pub struct ServerPanel(ObjectSubclass<imp::ServerPanel>)
        @extends gtk::Widget, adw::Dialog, adw::NavigationPage, @implements gtk::Accessible;
}

impl Default for ServerPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[template_callbacks]
impl ServerPanel {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_up(&self) {
        spawn(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                fraction_reset!(obj);
                obj.set_server_info().await;
                obj.set_activity_logs().await;
                obj.set_server_logs().await;
                obj.set_tasks().await;
                fraction!(obj);
            }
        ));
    }

    #[template_callback]
    fn on_shutdown(&self) {
        let dialog = adw::AlertDialog::new(
            Some(&gettext("Shut down server")),
            Some(&gettext("Are you sure you want to shut down the server?")),
        );
        dialog.add_responses(&[("revert", "Revert"), ("confirm", "Confirm")]);
        dialog.set_response_appearance("revert", adw::ResponseAppearance::Destructive);
        dialog.set_default_response(Some("revert"));
        dialog.set_close_response("revert");
        dialog.connect_response(
            Some("confirm"),
            glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move |dialog, _| {
                    spawn(glib::clone!(
                        #[weak]
                        obj,
                        #[weak]
                        dialog,
                        async move {
                            obj.shot_down().await;
                            dialog.close();
                        }
                    ));
                }
            ),
        );
        dialog.present(Some(self));
    }

    #[template_callback]
    fn on_restart(&self) {
        let dialog = adw::AlertDialog::new(
            Some(&gettext("Restart server")),
            Some(&gettext("Are you sure you want to restart the server?")),
        );
        dialog.add_responses(&[("revert", "Revert"), ("confirm", "Confirm")]);
        dialog.set_response_appearance("revert", adw::ResponseAppearance::Destructive);
        dialog.set_default_response(Some("revert"));
        dialog.set_close_response("revert");
        dialog.connect_response(
            Some("confirm"),
            glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move |dialog, _| {
                    spawn(glib::clone!(
                        #[weak]
                        obj,
                        #[weak]
                        dialog,
                        async move {
                            obj.restart().await;
                            dialog.close();
                        }
                    ));
                }
            ),
        );
        dialog.present(Some(self));
    }

    async fn shot_down(&self) {
        match spawn_tokio(EMBY_CLIENT.shut_down()).await {
            Ok(_) => (),
            Err(e) => {
                toast!(self, e.to_user_facing());
                return;
            }
        };

        toast!(self, gettext("Server is shutting down"));
    }

    async fn restart(&self) {
        match spawn_tokio(EMBY_CLIENT.restart()).await {
            Ok(_) => (),
            Err(e) => {
                toast!(self, e.to_user_facing());
                return;
            }
        };

        toast!(self, gettext("Server is restarting"));
    }

    async fn set_server_info(&self) {
        let server_info = match spawn_tokio(EMBY_CLIENT.get_server_info()).await {
            Ok(server_info) => server_info,
            Err(e) => {
                toast!(self, e.to_user_facing());
                return;
            }
        };

        let imp = self.imp();

        imp.server_title.set_text(&server_info.server_name);
        imp.server_version.set_text(&server_info.version);
    }

    async fn set_server_logs(&self) {
        let logs = match spawn_tokio(EMBY_CLIENT.get_activity_log(false)).await {
            Ok(logs) => logs,
            Err(e) => {
                toast!(self, e.to_user_facing());
                return;
            }
        };

        let imp = self.imp();

        for log in logs.item {
            let row = adw::ActionRow::builder()
                .title(&log.name)
                .subtitle(&utc_to_localstring(&log.date))
                .build();

            let icon = Image::builder()
                .icon_name("dialog-error-symbolic")
                .icon_size(gtk::IconSize::Large)
                .build();

            row.add_prefix(&icon);

            imp.system_log_group.add(&row);
        }
    }

    async fn set_activity_logs(&self) {
        let logs = match spawn_tokio(EMBY_CLIENT.get_activity_log(true)).await {
            Ok(logs) => logs,
            Err(e) => {
                toast!(self, e.to_user_facing());
                return;
            }
        };

        let imp = self.imp();

        for log in logs.item {
            let row = adw::ActionRow::builder()
                .title(&log.name)
                .subtitle(&utc_to_localstring(&log.date))
                .build();

            let avator = adw::Avatar::new(32, None, false);

            row.add_prefix(&avator);

            imp.activity_log_group.add(&row);
        }
    }

    async fn set_tasks(&self) {
        let tasks = match spawn_tokio(EMBY_CLIENT.get_scheduled_tasks()).await {
            Ok(tasks) => tasks,
            Err(e) => {
                toast!(self, e.to_user_facing());
                return;
            }
        };

        let imp = self.imp();

        for task in tasks {
            let mut subtitle = String::new();
            if let Some(last) = task.last_execution_result {
                subtitle.push_str(&format!(
                    "Last execute: {} \n",
                    utc_to_localstring(&last.start_time_utc)
                ));
                subtitle.push_str(&format!(
                    "Duration: {}m\n",
                    last.end_time_utc
                        .signed_duration_since(last.start_time_utc)
                        .num_minutes()
                ));
                subtitle.push_str(&format!("Result: {} \n", last.status));
            }
            subtitle.push_str(&task.description);
            let row = adw::ActionRow::builder()
                .title(&task.name)
                .subtitle(&subtitle)
                .build();

            let button = Button::builder()
                .icon_name("media-playback-start-symbolic")
                .valign(gtk::Align::Center)
                .build();

            button.add_css_class("accent");

            button.connect_clicked(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move |_| {
                    let id = task.id.clone();
                    spawn(glib::clone!(
                        #[weak]
                        obj,
                        async move {
                            obj.run_task(&id).await;
                        }
                    ));
                }
            ));

            row.add_suffix(&button);

            imp.task_group.add(&row);
        }
    }

    pub async fn run_task(&self, id: &str) {
        let id = id.to_string();
        match spawn_tokio(EMBY_CLIENT.run_scheduled_task(id)).await {
            Ok(result) => result,
            Err(e) => {
                toast!(self, e.to_user_facing());
                return;
            }
        };

        toast!(self, gettext("Task started"));
    }
}

pub fn utc_to_localstring(utc: &DateTime<Utc>) -> String {
    let utc = utc.with_timezone(&chrono::Local);
    format!(
        "{}/{}/{} {:02}:{:02}:{:02}",
        utc.year(),
        utc.month(),
        utc.day(),
        utc.hour(),
        utc.minute(),
        utc.second()
    )
}
