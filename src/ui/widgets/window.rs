use gtk::subclass::prelude::*;
use gtk::prelude::EditableExt;
mod imp{
    use adw::subclass::application_window::AdwApplicationWindowImpl;
    use glib::subclass::InitializingObject;
    use gtk::{prelude::*, HeaderBar, ToggleButton};
    use gtk::subclass::prelude::*;
    use gtk::{glib, Button, CompositeTemplate, Stack};

    // Object holding the state
    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/moe/tsukimi/window.ui")]
    pub struct Window {
        #[template_child]
        pub serverentry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub portentry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub nameentry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub passwordentry: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub selectlist: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub inwindow: TemplateChild<gtk::ScrolledWindow>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for Window {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "AppWindow";
        type Type = super::Window;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.install_action_async(
                "win.login",
                None,
                |window, _, _| async move {
                    window.login().await;
                    window.mainpage();
                },
            );
            klass.install_action(
                "win.home",
                None,
                move |window, _action, _parameter| {
                    window.homepage();
                },
            );
            klass.install_action(
                "win.search",
                None,
                move |window, _action, _parameter| {
                    window.searchpage();
                },
            );
            
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // Trait shared by all GObjects
    impl ObjectImpl for Window {
        fn constructed(&self) {
            // Call "constructed" on parent
            self.parent_constructed();
            let obj = self.obj().clone();
            self.selectlist.connect_row_selected(move |_, row| {
                if let Some(row) = row {
                    let num = row.index();
                    match num {
                        0 => {
                            obj.homepage();
                        }
                        1 => {
                            obj.homepage();
                        }
                        2 => {
                            obj.searchpage();
                        }
                        3 => {
                            println!("Settings");
                        }
                        _ => {}
                    }
                }
            });
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for Window {}

    // Trait shared by all windows
    impl WindowImpl for Window {}

    // Trait shared by all application windows
    impl ApplicationWindowImpl for Window {}
    impl AdwApplicationWindowImpl for Window {}

}

use glib::Object;
use gtk::{gio, glib};

use crate::ui::network::runtime;

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends adw::ApplicationWindow, gtk::Window, gtk::Widget, gtk::HeaderBar,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl Window {
    pub fn new(app: &adw::Application) -> Self {
        // Create new window
        Object::builder().property("application", app).build()
    }

    fn imp(&self) -> &imp::Window {
        imp::Window::from_obj(self)
    }

    fn mainpage(&self) {
        let imp = self.imp();
        imp.stack.set_visible_child_name("main");
    }

    fn homepage(&self) {
        let imp = self.imp();
        let stack = crate::ui::home_page::create_page();
        imp.inwindow.set_child(Some(&stack));
    }

    fn searchpage(&self) {
        let imp = self.imp();
        let stack = crate::ui::search_page::create_page1();
        imp.inwindow.set_child(Some(&stack));
    }


    async fn login(&self) {
        let imp = self.imp();
        let server = imp.serverentry.text().to_string();
        let port = imp.portentry.text().to_string();
        let name = imp.nameentry.text().to_string();
        let password = imp.passwordentry.text().to_string();
        let (sender, receiver) = async_channel::bounded::<String>(1);

        runtime().spawn(async move {
            match crate::ui::network::login(server, name, password, port).await {
                Ok(_) => {
                    sender.send("1".to_string()).await.expect("The channel needs to be open.");
                },
                Err(e) => eprintln!("Error: {}", e),
            }
        });
        glib::MainContext::default().spawn_local(async move {
            while let Ok(_) = receiver.recv().await {
                println!("Login successful");
            }
        });
    }
}