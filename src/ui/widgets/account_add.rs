use adw::prelude::AdwDialogExt;
use adw::Toast;
use gettextrs::gettext;
use glib::Object;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::template_callbacks;

use crate::client::client::EMBY_CLIENT;
use crate::client::error::UserFacingError;
use crate::config::save_cfg;
use crate::config::Account;
use crate::toast;
use crate::utils::spawn_tokio;

mod imp {

    use adw::subclass::dialog::AdwDialogImpl;
    use glib::subclass::InitializingObject;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate};

    // Object holding the state
    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/moe/tsukimi/account.ui")]
    pub struct AccountWindow {
        #[template_child]
        pub servername_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub server_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub username_entry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub password_entry: TemplateChild<adw::PasswordEntryRow>,
        #[template_child]
        pub port_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub toast: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub spinner: TemplateChild<adw::Spinner>,

        #[template_child]
        pub protocol: TemplateChild<gtk::DropDown>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for AccountWindow {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "AccountWindow";
        type Type = super::AccountWindow;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
            klass.install_action_async("account.add", None, |account, _, _| async move {
                account.add().await;
            });
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // Trait shared by all GObjects
    impl ObjectImpl for AccountWindow {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for AccountWindow {}
    impl AdwDialogImpl for AccountWindow {}
}

glib::wrapper! {
    pub struct AccountWindow(ObjectSubclass<imp::AccountWindow>)
    @extends gtk::Widget, adw::Dialog;
}

impl Default for AccountWindow {
    fn default() -> Self {
        Self::new()
    }
}

#[template_callbacks]
impl AccountWindow {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub async fn add(&self) {
        let imp = self.imp();
        let mut servername = imp.servername_entry.text().to_string();
        let scheme = imp.protocol.selected();
        let protocol = if scheme == 0 { "http://" } else { "https://" };
        let server = imp.server_entry.text();
        let username = imp.username_entry.text();
        let password = imp.password_entry.text();
        let port = imp.port_entry.text();
        if server.is_empty() || username.is_empty() || port.is_empty() {
            toast!(imp.spinner, gettext("Fields must be filled in"));
            return;
        }

        imp.spinner.set_visible(true);

        let server = format!("{protocol}{server}");

        let _ = EMBY_CLIENT.header_change_url(&server, &port);
        let _ = EMBY_CLIENT.header_change_token(&servername);
        let un = username.to_string();
        let pw = password.to_string();
        let res =
            match spawn_tokio(async move { EMBY_CLIENT.login(&username, &password).await }).await {
                Ok(res) => res,
                Err(e) => {
                    toast!(imp.spinner, e.to_user_facing());
                    imp.spinner.set_visible(false);
                    return;
                }
            };

        if servername.is_empty() {
            let res = match spawn_tokio(async move { EMBY_CLIENT.get_server_info_public().await })
                .await
            {
                Ok(res) => res,
                Err(e) => {
                    toast!(imp.spinner, e.to_user_facing());
                    imp.spinner.set_visible(false);
                    return;
                }
            };

            servername = res.server_name;
        }

        let account = Account {
            servername: servername.to_string(),
            server: server.to_string(),
            username: un,
            password: pw,
            port: port.to_string(),
            user_id: res.user.id,
            access_token: res.access_token,
        };

        match save_cfg(account).await {
            Ok(_) => (),
            Err(e) => {
                imp.toast.add_toast(
                    Toast::builder()
                        .timeout(3)
                        .title(e.to_user_facing())
                        .build(),
                );
                imp.spinner.set_visible(false);
                return;
            }
        };

        imp.spinner.set_visible(false);
        self.close();
        let window = self.root().and_downcast::<super::window::Window>().unwrap();
        toast!(self, gettext("Account added successfully"));
        window.set_servers();
        window.set_nav_servers();
    }

    #[template_callback]
    fn on_server_entry_changed(&self, entry: &adw::EntryRow) {
        let text = entry.text().to_string();

        let Some(url) = url::Url::parse(&text).ok() else {
            return;
        };

        match url.scheme() {
            "http" => {
                self.imp().protocol.set_selected(0);
            }
            "https" => {
                self.imp().protocol.set_selected(1);
                if url.port().is_none() {
                    self.imp().port_entry.set_text("443");
                }
            }
            _ => {}
        }

        if let Some(port) = url.port() {
            self.imp().port_entry.set_text(&port.to_string());
        }

        if let Some(host) = url.host_str() {
            self.imp().server_entry.set_text(host);
        }
    }
}
