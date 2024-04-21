use adw::prelude::ActionRowExt;
use adw::prelude::AdwDialogExt;
use adw::prelude::NavigationPageExt;
use gio::Settings;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use std::env;
use std::path::PathBuf;

mod imp {
    use adw::subclass::application_window::AdwApplicationWindowImpl;
    use glib::subclass::InitializingObject;
    use gtk::gio::Settings;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate};
    use std::cell::OnceCell;

    // Object holding the state
    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/moe/tsukimi/window.ui")]
    pub struct Window {
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub selectlist: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub insidestack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub backgroundstack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub popbutton: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub settingspage: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub searchpage: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub historypage: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub homepage: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub split_view: TemplateChild<adw::OverlaySplitView>,
        #[template_child]
        pub homeview: TemplateChild<adw::NavigationView>,
        #[template_child]
        pub historyview: TemplateChild<adw::NavigationView>,
        #[template_child]
        pub searchview: TemplateChild<adw::NavigationView>,
        #[template_child]
        pub navipage: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub toast: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub rootpic: TemplateChild<gtk::Picture>,
        #[template_child]
        pub serversbox: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub login_stack: TemplateChild<gtk::Stack>,
        pub selection: gtk::SingleSelection,
        pub settings: OnceCell<Settings>,
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
            klass.install_action("win.home", None, move |window, _action, _parameter| {
                window.freshhomepage();
            });
            klass.install_action("win.history", None, move |window, _action, _parameter| {
                window.freshhistorypage();
            });
            klass.install_action("win.search", None, move |window, _action, _parameter| {
                window.freshsearchpage();
            });
            klass.install_action("win.relogin", None, move |window, _action, _parameter| {
                window.placeholder();
            });
            klass.install_action("win.sidebar", None, move |window, _action, _parameter| {
                window.sidebar();
            });
            klass.install_action(
                "win.new-account",
                None,
                move |window, _action, _parameter| {
                    window.new_account();
                },
            );
            klass.install_action_async("win.pop", None, |window, _action, _parameter| async move {
                window.pop().await;
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
            let obj = self.obj();
            obj.setup_rootpic();
            obj.setup_settings();
            obj.load_window_size();
            obj.set_servers();
            self.selectlist
                .connect_row_selected(glib::clone!(@weak obj => move |_, row| {
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
                }));
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for Window {}

    // Trait shared by all windows
    impl WindowImpl for Window {
        // Save window state right before the window will be closed
        fn close_request(&self) -> glib::Propagation {
            // Save window size
            self.obj()
                .save_window_size()
                .expect("Failed to save window state");
            // Allow to invoke other event handlers
            glib::Propagation::Proceed
        }
    }

    // Trait shared by all application windows
    impl ApplicationWindowImpl for Window {}
    impl AdwApplicationWindowImpl for Window {}
}

use crate::config::Account;
use crate::config::{load_cfgv2, load_env};
use crate::ui::models::SETTINGS;
use crate::APP_ID;
use glib::Object;
use gtk::{gio, glib};

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends adw::ApplicationWindow, gtk::Window, gtk::Widget, gtk::HeaderBar,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl Window {
    pub fn set_servers(&self) {
        let imp = self.imp();
        let listbox = imp.serversbox.get();
        listbox.remove_all();
        let accounts = load_cfgv2().unwrap();
        for account in &accounts.accounts {
            if SETTINGS.auto_select_server() && account.servername == SETTINGS.preferred_server() {
                load_env(account);
                imp.historypage.set_child(None::<&gtk::Widget>);
                imp.searchpage.set_child(None::<&gtk::Widget>);
                self.mainpage();
                self.freshhomepage();
            }
        }
        if accounts.accounts.is_empty() {
            imp.login_stack.set_visible_child_name("no-server");
            return;
        } else {
            imp.login_stack.set_visible_child_name("servers");
        }
        for account in accounts.accounts {
            let account_clone = account.clone();
            let row = adw::ActionRow::builder()
                .title(&account.servername)
                .subtitle(&account.username)
                .height_request(80)
                .activatable(true)
                .build();
            unsafe {
                row.set_data("account", account);
            }
            row.add_suffix(&{
                let button = gtk::Button::builder()
                    .icon_name("user-trash-symbolic")
                    .valign(gtk::Align::Center)
                    .build();
                button.add_css_class("flat");
                button.connect_clicked(glib::clone!(@weak self as obj=> move |_| {
                    crate::config::remove(&account_clone).unwrap();
                    obj.set_servers();
                }));
                button
            });
            row.add_css_class("serverrow");

            listbox.append(&row);
        }
        listbox.connect_row_activated(glib::clone!(@weak self as obj => move |_, row| {
            unsafe {
                let account_ptr: std::ptr::NonNull<Account>  = row.data("account").unwrap();
                let account: &Account = &*account_ptr.as_ptr();
                load_env(account);
                SETTINGS.set_preferred_server(&account.servername).unwrap();
            }
            obj.imp().historypage.set_child(None::<&gtk::Widget>);
            obj.imp().searchpage.set_child(None::<&gtk::Widget>);
            obj.mainpage();
            obj.freshhomepage();
        }));
    }

    async fn homeviewpop(&self) {
        let imp = self.imp();
        imp.homeview.pop();
        if let Some(tag) = imp.homeview.visible_page().unwrap().tag() {
            if tag.as_str() == "homepage" {
                imp.navipage
                    .set_title(&env::var("EMBY_NAME").unwrap_or_else(|_| "Home".to_string()));
                self.change_pop_visibility();
            } else {
                imp.navipage.set_title(&tag);
            }
        }
    }

    async fn historyviewpop(&self) {
        let imp = self.imp();
        imp.historyview.pop();
        if let Some(tag) = imp.historyview.visible_page().unwrap().tag() {
            if tag.as_str() == "historypage" {
                imp.navipage.set_title("History & Liked");
                self.change_pop_visibility();
            } else {
                imp.navipage.set_title(&tag);
            }
        }
    }

    async fn searchviewpop(&self) {
        let imp = self.imp();
        imp.searchview.pop();
        if let Some(tag) = imp.searchview.visible_page().unwrap().tag() {
            if tag.as_str() == "searchpage" {
                imp.navipage.set_title("Search");
                self.change_pop_visibility();
            } else {
                imp.navipage.set_title(&tag);
            }
        }
    }

    async fn pop(&self) {
        let imp = self.imp();
        if imp.insidestack.visible_child_name().unwrap().as_str() == "homepage" {
            self.homeviewpop().await;
        } else if imp.insidestack.visible_child_name().unwrap().as_str() == "historypage" {
            self.historyviewpop().await;
        } else if imp.insidestack.visible_child_name().unwrap().as_str() == "searchpage" {
            self.searchviewpop().await;
        }
    }

    pub fn change_pop_visibility(&self) {
        let imp = self.imp();
        imp.popbutton.set_visible(!imp.popbutton.is_visible());
    }

    pub fn set_pop_visibility(&self, visible: bool) {
        let imp = self.imp();
        imp.popbutton.set_visible(visible);
    }

    fn setup_settings(&self) {
        let settings = Settings::new(APP_ID);
        let is_overlay = settings.boolean("is-overlay");
        self.overlay_sidebar(is_overlay);
        self.imp()
            .settings
            .set(settings)
            .expect("`settings` should not be set before calling `setup_settings`.");
    }

    fn settings(&self) -> &Settings {
        self.imp()
            .settings
            .get()
            .expect("`settings` should be set in `setup_settings`.")
    }

    pub fn save_window_size(&self) -> Result<(), glib::BoolError> {
        // Get the size of the window
        let size = self.default_size();

        // Set the window state in `settings`
        self.settings().set_int("window-width", size.0)?;
        self.settings().set_int("window-height", size.1)?;
        self.settings()
            .set_boolean("is-maximized", self.is_maximized())?;

        Ok(())
    }

    fn load_window_size(&self) {
        // Get the window state from `settings`
        let width = self.settings().int("window-width");
        let height = self.settings().int("window-height");
        let is_maximized = self.settings().boolean("is-maximized");

        // Set the size of the window
        self.set_default_size(width, height);

        // If the window was maximized when it was closed, maximize it again
        if is_maximized {
            self.maximize();
        }
    }

    pub fn new(app: &adw::Application) -> Self {
        // Create new window
        Object::builder().property("application", app).build()
    }

    pub fn set_title(&self, title: &str) {
        let imp = self.imp();
        imp.navipage.set_title(title);
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
        imp.insidestack.set_visible_child_name("homepage");
        if imp.homepage.child().is_none() {
            imp.homepage
                .set_child(Some(&crate::ui::widgets::home::HomePage::new()));
            imp.navipage
                .set_title(&env::var("EMBY_NAME").unwrap_or_else(|_| "Home".to_string()));
        }
        if let Some(tag) = imp.homeview.visible_page().unwrap().tag() {
            if tag.as_str() == "homepage" {
                imp.navipage
                    .set_title(&env::var("EMBY_NAME").unwrap_or_else(|_| "Home".to_string()));
                self.set_pop_visibility(false);
            } else {
                imp.navipage
                    .set_title(&env::var("HOME_TITLE").unwrap_or_else(|_| "Home".to_string()));
                self.set_pop_visibility(true);
            }
        } else {
            imp.navipage
                .set_title(&env::var("HOME_TITLE").unwrap_or_else(|_| "Home".to_string()));
            self.set_pop_visibility(true);
        }
    }

    fn freshhomepage(&self) {
        let imp = self.imp();
        imp.selectlist
            .select_row(imp.selectlist.row_at_index(0).as_ref());
        imp.homeview
            .pop_to_page(&imp.homeview.find_page("homepage").unwrap());
        imp.homepage
            .set_child(Some(&crate::ui::widgets::home::HomePage::new()));
        imp.navipage
            .set_title(&env::var("EMBY_NAME").unwrap_or_else(|_| "Home".to_string()));
        self.set_pop_visibility(false);
    }

    fn freshhistorypage(&self) {
        let imp = self.imp();
        imp.selectlist
            .select_row(imp.selectlist.row_at_index(1).as_ref());
        imp.insidestack.set_visible_child_name("historypage");
        imp.historyview
            .pop_to_page(&imp.historyview.find_page("historypage").unwrap());
        imp.historypage
            .set_child(Some(&crate::ui::widgets::history::HistoryPage::new()));
        imp.navipage.set_title("History");
        self.set_pop_visibility(false);
    }

    fn freshsearchpage(&self) {
        let imp = self.imp();
        imp.selectlist
            .select_row(imp.selectlist.row_at_index(2).as_ref());
        imp.insidestack.set_visible_child_name("searchpage");
        imp.searchview
            .pop_to_page(&imp.searchview.find_page("searchpage").unwrap());
        imp.searchpage
            .set_child(Some(&crate::ui::widgets::search::SearchPage::new()));
        imp.navipage.set_title("Search");
        self.set_pop_visibility(false);
    }

    fn historypage(&self) {
        let imp = self.imp();
        imp.insidestack.set_visible_child_name("historypage");
        if imp.historypage.child().is_none() {
            imp.historypage
                .set_child(Some(&crate::ui::widgets::history::HistoryPage::new()));
            imp.navipage.set_title("History & Liked");
        }
        if let Some(tag) = imp.historyview.visible_page().unwrap().tag() {
            if tag.as_str() == "historypage" {
                imp.navipage.set_title("History & Liked");
                self.set_pop_visibility(false);
            } else {
                self.set_pop_visibility(true);
                imp.navipage.set_title(
                    &env::var("HISTORY_TITLE").unwrap_or_else(|_| "History & Liked".to_string()),
                );
            }
        } else {
            self.set_pop_visibility(true);
            imp.navipage
                .set_title(&env::var("HISTORY_TITLE").unwrap_or_else(|_| "History".to_string()));
        }
    }

    fn searchpage(&self) {
        let imp = self.imp();
        imp.insidestack.set_visible_child_name("searchpage");
        if imp.searchpage.child().is_none() {
            imp.searchpage
                .set_child(Some(&crate::ui::widgets::search::SearchPage::new()));
            imp.navipage.set_title("Search");
        }
        if let Some(tag) = imp.searchview.visible_page().unwrap().tag() {
            if tag.as_str() == "searchpage" {
                imp.navipage.set_title("Search");
                self.set_pop_visibility(false);
            } else {
                self.set_pop_visibility(true);
                imp.navipage
                    .set_title(&env::var("SEARCH_TITLE").unwrap_or_else(|_| "Search".to_string()));
            }
        } else {
            self.set_pop_visibility(true);
            imp.navipage
                .set_title(&env::var("SEARCH_TITLE").unwrap_or_else(|_| "Search".to_string()));
        }
    }

    fn settingspage(&self) {
        let imp = self.imp();
        if imp.settingspage.child().is_none() {
            imp.settingspage
                .set_child(Some(&crate::ui::widgets::settings::SettingsPage::new()));
        }
        imp.insidestack.set_visible_child_name("settingspage");
        imp.navipage.set_title("Preferences");
        self.set_pop_visibility(false);
    }

    fn sidebar(&self) {
        let imp = self.imp();
        imp.split_view
            .set_show_sidebar(!imp.split_view.shows_sidebar());
    }

    pub fn overlay_sidebar(&self, overlay: bool) {
        let imp = self.imp();
        imp.split_view.set_collapsed(overlay);
    }

    pub fn toast(&self, message: &str) {
        let imp = self.imp();
        let toast = adw::Toast::builder()
            .title(message.to_string())
            .timeout(3)
            .build();
        imp.toast.add_toast(toast);
    }

    pub fn current_view_name(&self) -> String {
        let imp = self.imp();
        imp.insidestack.visible_child_name().unwrap().to_string()
    }

    pub fn set_rootpic(&self, file: gio::File) {
        let imp = self.imp();
        let settings = Settings::new(APP_ID);
        if settings.boolean("is-backgroundenabled") {
            let backgroundstack = imp.backgroundstack.get();
            let pic: gtk::Picture = if settings.boolean("is-blurenabled") {
                let paintbale =
                    crate::ui::provider::background_paintable::BackgroundPaintable::default();
                paintbale.set_pic(file);
                gtk::Picture::builder()
                    .paintable(&paintbale)
                    .halign(gtk::Align::Fill)
                    .valign(gtk::Align::Fill)
                    .hexpand(true)
                    .vexpand(true)
                    .content_fit(gtk::ContentFit::Cover)
                    .build()
            } else {
                gtk::Picture::builder()
                    .halign(gtk::Align::Fill)
                    .valign(gtk::Align::Fill)
                    .hexpand(true)
                    .vexpand(true)
                    .content_fit(gtk::ContentFit::Cover)
                    .file(&file)
                    .build()
            };
            let opacity = settings.int("pic-opacity");
            pic.set_opacity(opacity as f64 / 100.0);
            backgroundstack.add_child(&pic);
            backgroundstack.set_visible_child(&pic);
            if backgroundstack.observe_children().n_items() > 2 {
                if let Some(child) = backgroundstack.first_child() {
                    backgroundstack.remove(&child);
                }
            }
        }
    }

    pub fn setup_rootpic(&self) {
        let pic = SETTINGS.root_pic();
        let pathbuf = PathBuf::from(pic);
        if pathbuf.exists() {
            let file = gio::File::for_path(&pathbuf);
            self.set_rootpic(file);
        }
    }

    pub fn set_picopacity(&self, opacity: i32) {
        let imp = self.imp();
        let backgroundstack = imp.backgroundstack.get();
        if let Some(child) = backgroundstack.last_child() {
            let pic = child.downcast::<gtk::Picture>().unwrap();
            pic.set_opacity(opacity as f64 / 100.0);
        }
    }

    pub fn clear_pic(&self) {
        let imp = self.imp();
        let backgroundstack = imp.backgroundstack.get();
        if let Some(child) = backgroundstack.last_child() {
            backgroundstack.remove(&child);
        }
    }

    pub fn new_account(&self) {
        let dialog = crate::ui::widgets::account_add::AccountWindow::new();
        dialog.present(self);
    }
}
