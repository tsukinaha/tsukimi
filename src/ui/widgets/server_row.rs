use adw::{
    prelude::*,
    subclass::prelude::*,
};
use gtk::{
    CompositeTemplate,
    glib,
};

use crate::{
    client::Account,
    ui::{
        models::SETTINGS,
        provider::account_item::AccountItem,
        widgets::window::Window,
    },
};

mod imp {
    use std::cell::OnceCell;

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
    #[template(resource = "/moe/tsuna/tsukimi/ui/server_row.ui")]
    #[properties(wrapper_type = super::ServerRow)]
    pub struct ServerRow {
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
            
            // Add the base serverrow CSS class
            obj.add_css_class("serverrow");
            
            // Check if this is the current preferred server and add highlighting
            obj.update_current_server_highlighting();
        }
    }

    impl WidgetImpl for ServerRow {}
    impl ListBoxRowImpl for ServerRow {
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
                        w.reset();
                        // Update highlighting after server change
                        w.update_server_highlighting();
                    }
                }
            ));
        }
    }
}

glib::wrapper! {
    pub struct ServerRow(ObjectSubclass<imp::ServerRow>)
        @extends gtk::Widget, gtk::ListBoxRow, @implements gtk::Accessible;
}

impl ServerRow {
    pub fn new(account: Account) -> Self {
        glib::Object::builder()
            .property("item", AccountItem::from_simple(&account))
            .build()
    }

    pub fn update_current_server_highlighting(&self) {
        let account = self.item().account();
        let preferred_server = SETTINGS.preferred_server();
        
        if account.servername == preferred_server {
            self.add_css_class("current-server");
        } else {
            self.remove_css_class("current-server");
        }
    }
}
