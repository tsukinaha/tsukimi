use adw::prelude::AdwDialogExt;
use gettextrs::gettext;
use glib::Object;
use gtk::{
    glib,
    prelude::*,
    subclass::prelude::*,
    template_callbacks,
};
use imp::ActionType;

use super::utils::GlobalToast;
use crate::{
    client::{
        Account,
        error::UserFacingError,
        jellyfin_client::JELLYFIN_CLIENT,
    },
    ui::models::SETTINGS,
    utils::spawn_tokio,
};
pub mod imp {

    use std::cell::{
        Cell,
        RefCell,
    };

    use adw::subclass::dialog::AdwDialogImpl;
    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        glib,
        prelude::*,
        subclass::prelude::*,
    };

    use crate::client::Account;

    #[derive(Default, Hash, Eq, PartialEq, Clone, Copy, glib::Enum, Debug)]
    #[repr(u32)]
    #[enum_type(name = "ActionType")]
    pub enum ActionType {
        Edit,
        #[default]
        Add,
    }

    // Object holding the state
    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/account.ui")]
    #[properties(wrapper_type = super::AccountWindow)]
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
        pub stack: TemplateChild<gtk::Stack>,

        #[template_child]
        pub nav: TemplateChild<adw::NavigationPage>,

        #[template_child]
        pub protocol: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub server_type: TemplateChild<gtk::DropDown>,

        #[property(get, set, builder(ActionType::default()))]
        pub action_type: Cell<ActionType>,
        pub old_account: RefCell<Option<Account>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AccountWindow {
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

    #[glib::derived_properties]
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
    @extends gtk::Widget, adw::Dialog, @implements gtk::Accessible;
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

    #[template_callback]
    async fn on_password_entry_activated(&self) {
        self.add().await;
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
            imp.stack.toast(gettext("Fields must be filled in"));
            return;
        }

        imp.stack.set_visible_child_name("loading");

        let server = format!("{protocol}{server}");

        let _ = JELLYFIN_CLIENT.header_change_url(&server, &port).await;
        let _ = JELLYFIN_CLIENT.header_change_token(&servername).await;
        let un = username.to_string();
        let pw = password.to_string();
        let res =
            match spawn_tokio(async move { JELLYFIN_CLIENT.login(&username, &password).await })
                .await
            {
                Ok(res) => res,
                Err(e) => {
                    imp.stack.toast(e.to_user_facing());
                    imp.stack.set_visible_child_name("entry");
                    return;
                }
            };

        if servername.is_empty() {
            let res =
                match spawn_tokio(async move { JELLYFIN_CLIENT.get_server_info_public().await })
                    .await
                {
                    Ok(res) => res,
                    Err(e) => {
                        imp.stack.toast(e.to_user_facing());
                        imp.stack.set_visible_child_name("entry");
                        return;
                    }
                };

            servername = res.server_name;
        }

        let server_type = if imp.server_type.selected() == 0 {
            "Emby"
        } else {
            "Jellyfin"
        };

        let account = Account {
            servername: servername.to_string(),
            server: server.to_string(),
            username: un,
            password: pw,
            port: port.to_string(),
            user_id: res.user.id,
            access_token: res.access_token,
            server_type: Some(server_type.to_string()),
        };

        let action_type = imp.action_type.get();

        match action_type {
            ActionType::Edit => {
                let old_account = imp.old_account.take().expect("No server to edit");
                SETTINGS
                    .edit_account(old_account, account)
                    .expect("Failed to edit server");
                self.close_dialog(&gettext("Server edited successfully"))
                    .await;
            }
            ActionType::Add => {
                SETTINGS.add_account(account).expect("Failed to add server");
                self.close_dialog(&gettext("Server added successfully"))
                    .await;
            }
        }
    }

    async fn close_dialog(&self, msg: &str) {
        self.imp().stack.set_visible_child_name("entry");
        self.close();
        let root = self.root();
        let window = root.and_downcast_ref::<super::window::Window>().unwrap();
        self.toast(msg);
        window.set_servers().await;
        window.set_nav_servers();
    }

    #[template_callback]
    fn on_server_entry_changed(&self, entry: &adw::EntryRow) {
        let text = entry.text().to_string();

        let Some(url) = url::Url::parse(&text).ok() else {
            return;
        };

        // Prevent Gtk-WARNING **: Cannot begin irreversible action while in user action
        glib::idle_add_local_once(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            move || {
                obj.parse_url(&url);
            }
        ));
    }

    fn parse_url(&self, url: &url::Url) {
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
            self.imp().server_entry.set_position(-1);
        }
    }
}
