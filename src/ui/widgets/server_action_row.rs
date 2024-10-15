use crate::{
    config::Account,
    ui::{models::SETTINGS, provider::account_item::AccountItem},
};
use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{glib, template_callbacks, CompositeTemplate};

use super::{account_add::imp::ActionType, window::Window};

mod imp {
    use std::cell::OnceCell;

    use glib::subclass::InitializingObject;

    use crate::{
        client::client::EMBY_CLIENT,
        ui::{models::SETTINGS, provider::account_item::AccountItem, widgets::window::Window},
    };

    use adw::prelude::*;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate, glib::Properties)]
    #[template(resource = "/moe/tsukimi/server_action_row.ui")]
    #[properties(wrapper_type = super::ServerActionRow)]
    pub struct ServerActionRow {
        #[property(get, set, construct_only)]
        pub item: OnceCell<AccountItem>,
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
        }
    }

    impl ListBoxRowImpl for ServerActionRow {}
    impl PreferencesRowImpl for ServerActionRow {}
    impl WidgetImpl for ServerActionRow {}
    impl ActionRowImpl for ServerActionRow {
        fn activate(&self) {
            let account = self.obj().item().account();
            SETTINGS.set_preferred_server(&account.servername).unwrap();
            let _ = EMBY_CLIENT.init(&account);
            let window = self.obj().root().and_downcast::<Window>().unwrap();
            window.reset();
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
            .replace(Some(account.clone()));
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
        account_window.present(self.root().as_ref());
    }

    #[template_callback]
    fn on_delete_clicked(&self) {
        let account = self.item().account();
        SETTINGS
            .remove_account(account)
            .expect("Failed to remove server");
        let root = self.root();
        let window = root
            .and_downcast_ref::<Window>()
            .expect("Failed to get Window");
        window.set_servers();
        window.set_nav_servers();
    }
}
