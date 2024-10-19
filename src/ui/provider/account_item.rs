use gtk::glib;
use gtk::glib::prelude::*;
use gtk::glib::subclass::prelude::*;
use std::cell::RefCell;

use crate::client::Account;

pub mod imp {
    use gtk::glib::Properties;

    use super::*;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::AccountItem)]
    pub struct AccountItem {
        #[property(get, set)]
        server: RefCell<String>,
        #[property(get, set)]
        servername: RefCell<String>,
        #[property(get, set)]
        username: RefCell<String>,
        #[property(get, set)]
        password: RefCell<String>,
        #[property(get, set)]
        port: RefCell<String>,
        #[property(get, set)]
        user_id: RefCell<String>,
        #[property(get, set)]
        access_token: RefCell<String>,
    }

    #[glib::derived_properties]
    impl ObjectImpl for AccountItem {}

    #[glib::object_subclass]
    impl ObjectSubclass for AccountItem {
        const NAME: &'static str = "AccountItem";
        type Type = super::AccountItem;
    }
}

glib::wrapper! {
    pub struct AccountItem(ObjectSubclass<imp::AccountItem>);
}

impl AccountItem {
    pub fn from_simple(account: &Account) -> Self {
        let account = account.clone();
        let item: AccountItem = glib::object::Object::new();
        item.set_server(account.server);
        item.set_servername(account.servername);
        item.set_username(account.username);
        item.set_password(account.password);
        item.set_port(account.port);
        item.set_user_id(account.user_id);
        item.set_access_token(account.access_token);
        item
    }

    pub fn account(&self) -> Account {
        Account {
            server: self.server(),
            servername: self.servername(),
            username: self.username(),
            password: self.password(),
            port: self.port(),
            user_id: self.user_id(),
            access_token: self.access_token(),
        }
    }
}
