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
        account::ServerType,
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
    @extends gtk::Widget, adw::Dialog, @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Root;
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
        let servername = imp.servername_entry.text().to_string();
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
        let server_type = ServerType::from_index(imp.server_type.selected());
        let account = match spawn_tokio(async move {
            let login = JELLYFIN_CLIENT
                .login(&server, &port, server_type, &username, &password)
                .await?;
            let servername = if servername.is_empty() {
                JELLYFIN_CLIENT
                    .get_server_info_public(&server, &port, server_type)
                    .await?
                    .server_name
            } else {
                servername
            };

            Ok::<_, anyhow::Error>(Account {
                servername,
                server,
                username: username.to_string(),
                password: password.to_string(),
                port: port.to_string(),
                user_id: login.user.id,
                access_token: login.access_token,
                server_type: Some(server_type),
            })
        })
        .await
        {
            Ok(account) => account,
            Err(e) => {
                imp.stack.toast(e.to_user_facing());
                imp.stack.set_visible_child_name("entry");
                return;
            }
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
        let (protocol_idx, default_port) = match url.scheme() {
            "http" => (0, 80),
            "https" => (1, 443),
            _ => return,
        };

        self.imp().protocol.set_selected(protocol_idx);
        self.imp()
            .port_entry
            .set_text(&url.port().unwrap_or(default_port).to_string());

        if let Some(host) = url.host_str() {
            self.imp().server_entry.set_text(host);
            self.imp().server_entry.set_position(-1);
        }
    }
}
