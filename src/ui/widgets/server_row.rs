use adw::{prelude::*, subclass::prelude::*};
use gtk::{glib, CompositeTemplate};

use crate::{config::Account, ui::provider::account_item::AccountItem};

mod imp {
    use std::cell::OnceCell;

    use glib::subclass::InitializingObject;

    use crate::{
        client::client::EMBY_CLIENT,
        ui::{models::SETTINGS, provider::account_item::AccountItem, widgets::window::Window},
    };

    use super::*;

    #[derive(Debug, Default, CompositeTemplate, glib::Properties)]
    #[template(resource = "/moe/tsukimi/server_row.ui")]
    #[properties(wrapper_type = super::ServerRow)]
    pub struct ServerRow {
        /// The room represented by this row.
        #[property(get, set, construct_only)]
        pub item: OnceCell<AccountItem>,
        #[template_child]
        pub title_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ServerRow {
        const NAME: &'static str = "SidebarServerRow";
        type Type = super::ServerRow;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.set_accessible_role(gtk::AccessibleRole::Group);
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for ServerRow {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            self.title_label.set_text(&obj.item().servername());
        }
    }

    impl WidgetImpl for ServerRow {}
    impl ListBoxRowImpl for ServerRow {
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
    /// A sidebar row representing a room.
    pub struct ServerRow(ObjectSubclass<imp::ServerRow>)
        @extends gtk::Widget, gtk::ListBoxRow, @implements gtk::Accessible;
}

impl ServerRow {
    pub fn new(account: Account) -> Self {
        glib::Object::builder()
            .property("item", AccountItem::from_simple(&account))
            .build()
    }
}
