use adw::{
    prelude::*,
    subclass::prelude::*,
};
use chrono::{
    DateTime,
    Datelike,
    Timelike,
    Utc,
};
use gettextrs::gettext;
use gtk::{
    Button,
    CompositeTemplate,
    Image,
    glib,
    template_callbacks,
};

use crate::{
    client::{
        error::UserFacingError,
        jellyfin_client::JELLYFIN_CLIENT,
    },
    fraction,
    fraction_reset,
    utils::{
        spawn,
        spawn_tokio,
    },
};

use super::utils::GlobalToast;

#[derive(Clone)]
pub struct ServerFocusGroup {
    pub rows: Vec<gtk::Widget>,
}

pub(crate) mod imp {
    use glib::subclass::InitializingObject;

    use super::*;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/server_panel.ui")]
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
        @extends gtk::Widget, adw::Dialog, adw::NavigationPage, @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
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
        dialog.add_responses(&[
            ("revert", &gettext("Revert")),
            ("confirm", &gettext("Confirm")),
        ]);
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
        match spawn_tokio(JELLYFIN_CLIENT.shut_down()).await {
            Ok(_) => (),
            Err(e) => {
                self.toast(e.to_user_facing());
                return;
            }
        };

        self.toast(gettext("Server is shutting down"));
    }

    async fn restart(&self) {
        match spawn_tokio(JELLYFIN_CLIENT.restart()).await {
            Ok(_) => (),
            Err(e) => {
                self.toast(e.to_user_facing());
                return;
            }
        };

        self.toast(gettext("Server is restarting"));
    }

    async fn set_server_info(&self) {
        let server_info = match spawn_tokio(JELLYFIN_CLIENT.get_server_info()).await {
            Ok(server_info) => server_info,
            Err(e) => {
                self.toast(e.to_user_facing());
                return;
            }
        };

        let imp = self.imp();

        imp.server_title.set_text(&server_info.server_name);
        imp.server_version.set_text(&server_info.version);
    }

    async fn set_server_logs(&self) {
        let logs = match spawn_tokio(JELLYFIN_CLIENT.get_activity_log(false)).await {
            Ok(logs) => logs,
            Err(e) => {
                self.toast(e.to_user_facing());
                return;
            }
        };

        let imp = self.imp();

        for log in logs.item {
            let row = adw::ActionRow::builder()
                .title(&log.name)
                .subtitle(utc_to_localstring(&log.date))
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
        let logs = match spawn_tokio(JELLYFIN_CLIENT.get_activity_log(true)).await {
            Ok(logs) => logs,
            Err(e) => {
                self.toast(e.to_user_facing());
                return;
            }
        };

        let imp = self.imp();

        for log in logs.item {
            let row = adw::ActionRow::builder()
                .title(&log.name)
                .subtitle(utc_to_localstring(&log.date))
                .build();

            let avator = adw::Avatar::new(32, None, false);

            row.add_prefix(&avator);

            imp.activity_log_group.add(&row);
        }
    }

    async fn set_tasks(&self) {
        let tasks = match spawn_tokio(JELLYFIN_CLIENT.get_scheduled_tasks()).await {
            Ok(tasks) => tasks,
            Err(e) => {
                self.toast(e.to_user_facing());
                return;
            }
        };

        let imp = self.imp();

        for task in tasks {
            let mut subtitle = String::new();
            if let Some(last) = task.last_execution_result {
                subtitle.push_str(&format!(
                    "{}{} \n",
                    gettext("Last execute: "),
                    utc_to_localstring(&last.start_time_utc)
                ));
                subtitle.push_str(&format!(
                    "{}{}m\n",
                    gettext("Duration: "),
                    last.end_time_utc
                        .signed_duration_since(last.start_time_utc)
                        .num_minutes()
                ));
                subtitle.push_str(&format!("{}{} \n", gettext("Result: "), last.status));
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
                    let id = task.id.to_owned();
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
        match spawn_tokio(JELLYFIN_CLIENT.run_scheduled_task(id)).await {
            Ok(result) => result,
            Err(e) => {
                self.toast(e.to_user_facing());
                return;
            }
        };

        self.toast(gettext("Task started"));
    }

    pub fn focus_groups(&self) -> Vec<ServerFocusGroup> {
        let imp = self.imp();
        [
            imp.system_log_group.get(),
            imp.activity_log_group.get(),
            imp.task_group.get(),
        ]
        .into_iter()
        .map(|group| {
            let mut rows = Vec::new();
            let mut child = group.first_child();
            while let Some(row) = child {
                let next = row.next_sibling();
                rows.push(row.upcast());
                child = next;
            }
            ServerFocusGroup { rows }
        })
        .filter(|group| !group.rows.is_empty())
        .collect()
    }

    pub fn activate_focused_row(
        &self, groups: &[ServerFocusGroup], group_index: usize, row_index: usize,
    ) -> bool {
        let Some(group) = groups.get(group_index) else {
            return false;
        };
        let Some(row) = group.rows.get(row_index) else {
            return false;
        };
        if let Ok(action_row) = row.clone().downcast::<adw::ActionRow>()
            && let Some(button) = action_row
                .last_child()
                .and_then(|child| child.downcast::<gtk::Button>().ok())
        {
            button.emit_clicked();
            return true;
        }
        false
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
