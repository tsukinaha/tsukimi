#[cfg(unix)]
use dirs::home_dir;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use std::env;
mod imp {
    use adw::subclass::application_window::AdwApplicationWindowImpl;
    use glib::subclass::InitializingObject;
    use gtk::prelude::ListBoxRowExt;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate};

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
        #[template_child]
        pub loginbutton: TemplateChild<gtk::Button>,
        #[template_child]
        pub insidestack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub proxyentry: TemplateChild<adw::EntryRow>,
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
            klass.install_action_async("win.login", None, |window, _, _| async move {
                window.login().await;
            });
            klass.install_action("win.home", None, move |window, _action, _parameter| {
                window.homepage();
            });
            klass.install_action("win.search", None, move |window, _action, _parameter| {
                window.searchpage();
            });
            klass.install_action("win.relogin", None, move |window, _action, _parameter| {
                window.placeholder();
            });
            klass.install_action("win.proxy", None, move |window, _action, _parameter| {
                window.proxy();
            });
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
            obj.loginenter();

            self.selectlist.connect_row_selected(move |_, row| {
                if let Some(row) = row {
                    let num = row.index();
                    match num {
                        0 => {
                            obj.homepage();
                        }
                        1 => {
                            obj.historypage();
                        }
                        2 => {
                            obj.searchpage();
                        }
                        3 => {
                            obj.settingspage();
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

    fn placeholder(&self) {
        let imp = self.imp();
        imp.stack.set_visible_child_name("placeholder");
    }

    fn homepage(&self) {
        let imp = self.imp();
        let stack = crate::ui::home_page::create_page();
        let pagename = format!("homepage");
        if stack.child_by_name(&pagename).is_some() {
            stack.remove(&stack.child_by_name(&pagename).unwrap());
        }
        let pagename = format!("searchpage");
        if stack.child_by_name(&pagename).is_some() {
            stack.remove(&stack.child_by_name(&pagename).unwrap());
        }
        if imp.insidestack.child_by_name("homepage").is_none() {
            imp.insidestack.add_titled(&stack, Some("homepage"), "home");
        }
        imp.insidestack.set_visible_child_name("homepage");
    }

    fn historypage(&self) {
        let imp = self.imp();
        imp.insidestack.set_visible_child_name("title2");
    }

    fn searchpage(&self) {
        let imp = self.imp();
        let stack = crate::ui::search_page::create_page1();
        let pagename = format!("homepage");
        if stack.child_by_name(&pagename).is_some() {
            stack.remove(&stack.child_by_name(&pagename).unwrap());
        }
        let pagename = format!("searchpage");
        if stack.child_by_name(&pagename).is_some() {
            stack.remove(&stack.child_by_name(&pagename).unwrap());
        }
        if imp.insidestack.child_by_name("searchpage").is_none() {
            imp.insidestack
                .add_titled(&stack, Some("searchpage"), "search");
        }
        imp.insidestack.set_visible_child_name("searchpage");
    }

    fn settingspage(&self) {
        let imp = self.imp();
        imp.insidestack.set_visible_child_name("title");
    }

    fn proxy(&self) {
        let imp = self.imp();
        let proxy = imp.proxyentry.text().to_string();
    }

    async fn login(&self) {
        let imp = self.imp();
        imp.loginbutton.set_sensitive(false);
        let loginbutton = imp.loginbutton.clone();
        let server = imp.serverentry.text().to_string();
        let port = imp.portentry.text().to_string();
        let name = imp.nameentry.text().to_string();
        let password = imp.passwordentry.text().to_string();

        let (sender, receiver) = async_channel::bounded::<String>(1);
        let selfc = self.clone();
        runtime().spawn(async move {
            match crate::ui::network::login(server, name, password, port).await {
                Ok(_) => {
                    sender
                        .send("1".to_string())
                        .await
                        .expect("The channel needs to be open.");
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        });
        glib::MainContext::default().spawn_local(async move {
            match receiver.recv().await {
                Ok(_) => {
                    loginbutton.set_sensitive(false);
                    selfc.mainpage();
                }
                Err(_) => {
                    loginbutton.set_sensitive(true);
                    loginbutton.set_label("Link Failed");
                }
            }
        });
    }

    fn loginenter(&self) {
        #[cfg(unix)]
        let path = home_dir().unwrap().join(".config/tsukimi.yaml");
        #[cfg(windows)]
        let path = env::current_dir()
            .unwrap()
            .join("config")
            .join("tsukimi.yaml");
        // path.push(".config");
        // path.push("tsukimi.yaml");
        if path.exists() {
            self.mainpage();
        }
    }
}
