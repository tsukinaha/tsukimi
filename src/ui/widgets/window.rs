use std::env;
use std::path::PathBuf;

use adw::prelude::*;
use gio::Settings;
use gtk::subclass::prelude::*;
mod imp {
    use std::cell::OnceCell;

    use adw::subclass::application_window::AdwApplicationWindowImpl;
    use glib::subclass::InitializingObject;
    use gtk::gio::Settings;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate};

    use crate::ui::clapper::page::ClapperPage;
    use crate::ui::widgets::content_viewer::MediaContentViewer;
    use crate::ui::widgets::image_dialog::ImagesDialog;
    use crate::ui::widgets::item_actionbox::ItemActionsBox;
    use crate::ui::widgets::media_viewer::MediaViewer;
    use crate::ui::widgets::player_toolbar::PlayerToolbarBox;

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
        #[template_child]
        pub namerow: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub player_toolbar_bin: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub player_toolbar_box: TemplateChild<PlayerToolbarBox>,
        #[template_child]
        pub progressbar: TemplateChild<gtk::ProgressBar>,
        #[template_child]
        pub clapperpage: TemplateChild<gtk::StackPage>,
        #[template_child]
        pub clappernav: TemplateChild<ClapperPage>,
        #[template_child]
        pub serverselectlist: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub media_viewer: TemplateChild<MediaViewer>,
        pub selection: gtk::SingleSelection,
        pub settings: OnceCell<Settings>,

        pub progress_bar_animation: OnceCell<adw::TimedAnimation>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for Window {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "AppWindow";
        type Type = super::Window;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            PlayerToolbarBox::ensure_type();
            ClapperPage::ensure_type();
            ItemActionsBox::ensure_type();
            MediaContentViewer::ensure_type();
            MediaViewer::ensure_type();
            ImagesDialog::ensure_type();
            klass.bind_template();
            klass.bind_template_instance_callbacks();
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
                "setting.account",
                None,
                move |window, _action, _parameter| {
                    window.account_settings();
                },
            );
            klass.install_action_async("win.pop", None, |window, _action, _parameter| async move {
                window.pop().await;
            });
            klass.install_action("win.toggle-fullscreen", None, |obj, _, _| {
                if obj.is_fullscreened() {
                    obj.unfullscreen();
                } else {
                    obj.fullscreen();
                }
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
            self.clappernav.bind_fullscreen(&obj);
            obj.set_fonts();
            if crate::ui::models::SETTINGS.font_size() != -1 {
                let settings = gtk::Settings::default().unwrap();
                settings.set_property(
                    "gtk-xft-dpi",
                    crate::ui::models::SETTINGS.font_size() * 1024,
                );
            }
            obj.setup_rootpic();
            obj.setup_settings();
            obj.load_window_size();
            obj.set_servers();
            obj.set_nav_servers();
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

use crate::client::client::EMBY_CLIENT;
use crate::client::structs::Back;
use crate::config::load_cfgv2;
use crate::config::Account;
use crate::ui::models::SETTINGS;
use crate::ui::provider::core_song::CoreSong;
use crate::utils::spawn;
use crate::APP_ID;
use glib::Object;
use gtk::{gio, glib, template_callbacks};

use super::server_row::ServerRow;
use super::tu_list_item::PROGRESSBAR_ANIMATION_DURATION;

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends adw::ApplicationWindow, gtk::Window, gtk::Widget, gtk::HeaderBar,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

#[template_callbacks]
impl Window {
    pub fn set_servers(&self) {
        let imp = self.imp();
        let listbox = imp.serversbox.get();
        listbox.remove_all();
        let accounts = load_cfgv2().unwrap();
        for account in &accounts.accounts {
            if SETTINGS.auto_select_server()
                && account.servername == SETTINGS.preferred_server()
                && env::var("EMBY_NAME").is_err()
            {
                EMBY_CLIENT.init(account);
                imp.historypage.set_child(None::<&gtk::Widget>);
                imp.searchpage.set_child(None::<&gtk::Widget>);
                self.mainpage();
                self.freshhomepage();
                self.account_setup();
            }
        }
        if accounts.accounts.is_empty() {
            imp.login_stack.set_visible_child_name("no-server");
            return;
        } else {
            imp.login_stack.set_visible_child_name("servers");
        }
        for account in accounts.accounts {
            listbox.append(&self.set_server_rows(account));
        }
        listbox.connect_row_activated(glib::clone!(@weak self as obj => move |_, row| {
            unsafe {
                let account_ptr: std::ptr::NonNull<Account>  = row.data("account").unwrap();
                let account: &Account = &*account_ptr.as_ptr();
                EMBY_CLIENT.init(account);
                SETTINGS.set_preferred_server(&account.servername).unwrap();
            }
            obj.reset();
        }));
    }

    pub fn set_nav_servers(&self) {
        let imp = self.imp();
        let listbox = imp.serverselectlist.get();
        listbox.remove_all();
        let accounts = load_cfgv2().unwrap();
        for account in accounts.accounts {
            listbox.append(&ServerRow::new(account));
        }
    }

    #[template_callback]
    pub fn account_activated(&self, account_row: &ServerRow) {
        account_row.activate();
    }

    pub fn reset(&self) {
        self.imp().historypage.set_child(None::<&gtk::Widget>);
        self.imp().searchpage.set_child(None::<&gtk::Widget>);

        self.mainpage();
        self.freshhomepage();
        self.account_setup();
    }

    pub fn set_server_rows(&self, account: Account) -> adw::ActionRow {
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
                obj.set_nav_servers();
            }));
            button
        });
        row.add_css_class("serverrow");
        row
    }

    pub fn account_setup(&self) {
        let imp = self.imp();
        imp.namerow
            .set_title(&env::var("EMBY_USERNAME").unwrap_or_else(|_| "Username".to_string()));
        imp.namerow
            .set_subtitle(&env::var("EMBY_NAME").unwrap_or_else(|_| "Server".to_string()));
    }

    pub fn account_settings(&self) {
        let dialog = crate::ui::widgets::account_settings::AccountSettings::new();
        dialog.present(self);
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

    pub fn mainpage(&self) {
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
        self.imp().player_toolbar_box.on_stop_button_clicked();
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
            imp.navipage.set_title("Liked");
        }
        if let Some(tag) = imp.historyview.visible_page().unwrap().tag() {
            if tag.as_str() == "historypage" {
                imp.navipage.set_title("Liked");
                self.set_pop_visibility(false);
            } else {
                self.set_pop_visibility(true);
                imp.navipage
                    .set_title(&env::var("HISTORY_TITLE").unwrap_or_else(|_| "Liked".to_string()));
            }
        } else {
            self.set_pop_visibility(true);
            imp.navipage
                .set_title(&env::var("HISTORY_TITLE").unwrap_or_else(|_| "Liked".to_string()));
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

    pub fn add_toast(&self, toast: adw::Toast) {
        let imp = self.imp();
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

    pub fn set_player_toolbar(&self) {
        spawn(glib::clone!(@weak self as obj=>async move {
            obj.imp().player_toolbar_bin.set_reveal_child(true);
        }));
    }

    pub fn set_fraction(&self, to_value: f64) {
        let progressbar = &self.imp().progressbar;
        progressbar.set_inverted(!progressbar.is_inverted());
        self.progressbar_animation().set_value_from(progressbar.fraction());
        self.progressbar_animation().set_value_to(to_value);
        self.progressbar_animation().play();
    }

    fn progressbar_animation(&self) -> &adw::TimedAnimation {
        self.imp().progress_bar_animation.get_or_init(|| {
            let target = adw::CallbackAnimationTarget::new(glib::clone!(
                @weak self as obj => move |fraction| obj.imp().progressbar.set_fraction(fraction)
            ));

            adw::TimedAnimation::builder()
                .duration(PROGRESSBAR_ANIMATION_DURATION)
                .widget(&self.imp().progressbar.get())
                .target(&target)
                .build()
        })
    }

    pub fn set_fonts(&self) {
        if !SETTINGS.font_name().is_empty() {
            let settings = self.imp().stack.settings();
            settings.set_gtk_font_name(Some(&SETTINGS.font_name()));
        }
    }

    pub fn set_clapperpage(
        &self,
        url: &str,
        suburi: Option<&str>,
        name: Option<&str>,
        line2: Option<&str>,
        back: Option<Back>,
    ) {
        let imp = self.imp();
        imp.stack.set_visible_child_name("clapper");
        imp.clappernav.add_item(url, suburi, name, line2, back);
    }

    pub fn reveal_image(&self, source_widget: &impl IsA<gtk::Widget>) {
        let imp = self.imp();
        imp.media_viewer.reveal(source_widget);
    }

    pub fn media_viewer_show_paintable(&self, paintable: Option<gtk::gdk::Paintable>) {
        let Some(paintable) = paintable else {
            return;
        };
        let imp = self.imp();
        imp.media_viewer.view_image(paintable);
    }

    #[template_callback]
    fn on_add_server(&self) {
        self.new_account();
    }

    pub fn bind_song_model(&self, active_model: gio::ListStore, active_core_song: CoreSong) {
        let imp = self.imp();
        imp.player_toolbar_box.bind_song_model(active_model, active_core_song);
    }

    pub fn play_media(
        &self,
        url: String,
        suburl: Option<String>,
        name: Option<String>,
        back: Option<Back>,
        selected: Option<String>,
        percentage: f64,
    ) {
        if SETTINGS.mpv() {
            gio::spawn_blocking(move || {
                match crate::ui::mpv::event::play(
                    url,
                    suburl,
                    Some(name.unwrap_or("".to_string())),
                    back,
                    Some(percentage),
                ) {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("Failed to play: {}", e);
                    }
                };
            });
        } else {
            self.set_clapperpage(
                &url,
                suburl.as_deref(),
                name.as_deref(),
                selected.as_deref(),
                back,
            );
        }
    }
}
