use adw::{
    prelude::*,
    subclass::prelude::*,
};
use gtk::{
    CompositeTemplate,
    glib,
    template_callbacks,
};

use super::{
    account_add::imp::ActionType,
    window::Window,
};
use crate::{
    client::Account,
    ui::{
        models::SETTINGS,
        provider::account_item::AccountItem,
    },
};

mod imp {
    use std::cell::OnceCell;

    use adw::prelude::*;
    use glib::subclass::InitializingObject;

    use super::*;
    use crate::{
        client::jellyfin_client::JELLYFIN_CLIENT,
        ui::{
            models::SETTINGS,
            provider::account_item::AccountItem,
            widgets::window::Window,
        },
        utils::spawn,
    };

    #[derive(Debug, Default, CompositeTemplate, glib::Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/server_action_row.ui")]
    #[properties(wrapper_type = super::ServerActionRow)]
    pub struct ServerActionRow {
        #[property(get, set, construct_only)]
        pub item: OnceCell<AccountItem>,
        #[template_child]
        pub server_image: TemplateChild<gtk::Image>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ServerActionRow {
        const NAME: &'static str = "ServerActionRow";
        type Type = super::ServerActionRow;
        type ParentType = adw::ActionRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
            klass.set_accessible_role(gtk::AccessibleRole::Group);
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ServerActionRow {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.set_title(&obj.item().servername());
            obj.set_subtitle(&obj.item().username());
            if obj.item().server_type() == Some("Jellyfin".to_string()) {
                self.server_image.set_icon_name(Some("jellyfin-symbolic"));
            }
        }
    }

    impl ListBoxRowImpl for ServerActionRow {}
    impl PreferencesRowImpl for ServerActionRow {}
    impl WidgetImpl for ServerActionRow {}
    impl ActionRowImpl for ServerActionRow {
        fn activate(&self) {
            let obj = self.obj();

            spawn(glib::clone!(
                #[weak]
                obj,
                async move {
                    let account = obj.item().account();
                    SETTINGS.set_preferred_server(&account.servername).unwrap();
                    let _ = JELLYFIN_CLIENT.init(&account).await;
                    if let Some(w) = obj.root().and_downcast::<Window>() {
                        w.reset()
                    }
                }
            ));
        }
    }
}

glib::wrapper! {
    pub struct ServerActionRow(ObjectSubclass<imp::ServerActionRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::ActionRow, adw::PreferencesRow, @implements gtk::Accessible;
}

#[template_callbacks]
impl ServerActionRow {
    pub fn new(account: Account) -> Self {
        glib::Object::builder()
            .property("item", AccountItem::from_simple(&account))
            .build()
    }

    #[template_callback]
    fn on_edit_clicked(&self) {
        let account = self.item().account();
        let account_window = crate::ui::widgets::account_add::AccountWindow::new();
        account_window
            .imp()
            .nav
            .set_title(&gettextrs::gettext("Edit Server"));
        account_window.set_action_type(ActionType::Edit);
        account_window
            .imp()
            .old_account
            .replace(Some(account.to_owned()));
        account_window
            .imp()
            .username_entry
            .set_text(&account.username);
        account_window
            .imp()
            .password_entry
            .set_text(&account.password);
        account_window
            .imp()
            .servername_entry
            .set_text(&account.servername);
        account_window.imp().port_entry.set_text(&account.port);
        account_window.imp().server_entry.set_text(&account.server);
        account_window.imp().server_type.set_selected(
            if account.server_type == Some("Jellyfin".to_string()) {
                1
            } else {
                0
            },
        );
        account_window.present(self.root().as_ref());
    }

    #[template_callback]
    async fn on_delete_clicked(&self) {
        let account = self.item().account();
        SETTINGS
            .remove_account(account)
            .expect("Failed to remove server");
        let root = self.root();
        let window = root
            .and_downcast_ref::<Window>()
            .expect("Failed to get Window");
        window.set_servers().await;
        window.set_nav_servers();
    }
}
