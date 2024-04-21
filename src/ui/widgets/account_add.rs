use adw::prelude::AdwDialogExt;
use adw::Toast;
use glib::Object;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use crate::ui::network::{loginv2, RUNTIME};
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
        pub spinner: TemplateChild<gtk::Spinner>,
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

impl AccountWindow {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub async fn add(&self) {
        let imp = self.imp();
        imp.spinner.set_visible(true);
        let servername = imp.servername_entry.text();
        let server = imp.server_entry.text();
        let username = imp.username_entry.text();
        let password = imp.password_entry.text();
        let port = imp.port_entry.text();
        if servername.is_empty()
            || server.is_empty()
            || username.is_empty()
            || password.is_empty()
            || port.is_empty()
        {
            imp.toast.add_toast(
                Toast::builder()
                    .timeout(3)
                    .title("All fields must be filled in")
                    .build(),
            );
        } else {
            let (sender, receiver) = async_channel::bounded::<Result<bool, reqwest::Error>>(1);
            RUNTIME.spawn(async move {
                match loginv2(
                    servername.to_string(),
                    server.to_string(),
                    username.to_string(),
                    password.to_string(),
                    port.to_string(),
                )
                .await
                {
                    Ok(_) => {
                        sender.send(Ok(true)).await.unwrap();
                    }
                    Err(e) => {
                        sender.send(Err(e)).await.unwrap();
                    }
                }
            });
            glib::spawn_future_local(glib::clone!(@weak self as obj=>async move {
                while let Ok(item) = receiver.recv().await {
                    match item {
                        Ok(_) => {
                            obj.imp().spinner.set_visible(false);
                            obj.close();
                            let window = obj.root().and_downcast::<super::window::Window>().unwrap();
                            window.toast("Account added successfully");
                            window.set_servers();
                        }
                        Err(e) => {
                            obj.imp().spinner.set_visible(false);
                            obj.imp().toast.add_toast(Toast::builder().timeout(3).title(&format!("Failed to login: {}", e)).build());
                        }
                    }
                }
            }));
        }
    }
}
