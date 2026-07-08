use std::path::PathBuf;

use adw::prelude::*;
use gettextrs::gettext;
use gio::Settings;
use gtk::{
    Widget,
    subclass::prelude::*,
};

mod imp {
    use std::cell::{
        OnceCell,
        RefCell,
    };

    use adw::subclass::application_window::AdwApplicationWindowImpl;
    use glib::subclass::InitializingObject;
    use gtk::{
        CompositeTemplate,
        glib,
        prelude::*,
        subclass::prelude::*,
    };

    use crate::{
        ui::{
            SETTINGS,
            input::{
                FocusManager,
                GamepadManager,
                GridNavigator,
                ItemPageNavigator,
                LikedNavigator,
                MediaViewerNavigator,
                MpvNavigator,
                PlaceholderNavigator,
                PushedNavigator,
                SearchNavigator,
                SettingsNavigator,
            },
            mpv::{
                control_sidebar::MPVControlSidebar,
                page::MPVPage,
            },
            provider::tu_object::TuObject,
            widgets::{
                content_viewer::MediaContentViewer,
                home::HomePage,
                image_dialog::ImageDialog,
                item_actionbox::ItemActionsBox,
                liked::LikedPage,
                media_viewer::MediaViewer,
                player_toolbar::PlayerToolbarBox,
                search::SearchPage,
                theme_switcher::ThemeSwitcher,
                tu_overview_item::imp::ViewGroup,
                utils::TuItemBuildExt,
            },
        },
        utils::spawn,
    };

    // Object holding the state
    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/window.ui")]
    pub struct Window {
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub selectlist: TemplateChild<adw::Sidebar>,
        #[template_child]
        pub insidestack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub backgroundstack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub popbutton: TemplateChild<gtk::Button>,
        #[template_child]
        pub split_view: TemplateChild<adw::OverlaySplitView>,
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
        pub add_server: TemplateChild<gtk::Button>,
        #[template_child]
        pub namerow: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub player_toolbar_box: TemplateChild<PlayerToolbarBox>,
        #[template_child]
        pub progressbar: TemplateChild<gtk::ProgressBar>,
        #[template_child]
        pub mpvpage: TemplateChild<gtk::StackPage>,
        #[template_child]
        pub mpvnav: TemplateChild<MPVPage>,
        #[template_child]
        pub media_viewer: TemplateChild<MediaViewer>,
        #[template_child]
        pub servers_section: TemplateChild<adw::SidebarSection>,
        pub selection: gtk::SingleSelection,

        #[template_child]
        pub mainpage: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub mainview: TemplateChild<adw::NavigationView>,
        #[template_child]
        pub homepage: TemplateChild<adw::Bin>,
        #[template_child]
        pub likedpage: TemplateChild<adw::Bin>,
        #[template_child]
        pub searchpage: TemplateChild<adw::Bin>,
        #[template_child]
        pub mpv_playlist: TemplateChild<gtk::ListView>,
        #[template_child]
        pub mpv_control_sidebar: TemplateChild<MPVControlSidebar>,

        #[template_child]
        pub mpv_view: TemplateChild<adw::OverlaySplitView>,
        #[template_child]
        pub mpv_view_stack: TemplateChild<adw::ViewStack>,

        #[template_child]
        pub avatar: TemplateChild<adw::Avatar>,

        pub progress_bar_animation: OnceCell<adw::TimedAnimation>,
        pub progress_bar_fade_animation: OnceCell<adw::TimedAnimation>,

        pub last_content_list_selection: RefCell<Option<i32>>,

        pub mpv_playlist_selection: gtk::SingleSelection,

        pub suspend_cookie: RefCell<Option<u32>>,

        pub focus_manager: RefCell<FocusManager>,
        pub gamepad_manager: RefCell<GamepadManager>,
        pub placeholder_navigator: RefCell<PlaceholderNavigator>,
        pub liked_navigator: RefCell<LikedNavigator>,
        pub search_navigator: RefCell<SearchNavigator>,
        pub mpv_navigator: RefCell<MpvNavigator>,
        pub settings_navigator: RefCell<SettingsNavigator>,
        pub item_navigator: RefCell<ItemPageNavigator>,
        pub grid_navigator: RefCell<GridNavigator>,
        pub pushed_navigator: RefCell<PushedNavigator>,
        pub media_viewer_navigator: RefCell<MediaViewerNavigator>,
        pub home_focus_snapshot: RefCell<Option<crate::ui::input::HomeFocusSnapshot>>,
        pub active_settings: RefCell<Option<crate::ui::widgets::account_settings::AccountSettings>>,
        pub active_account_dialog: RefCell<Option<crate::ui::widgets::account_add::AccountWindow>>,
        pub tv_hints_revealer: RefCell<Option<gtk::Revealer>>,
        pub tv_hints_label: RefCell<Option<gtk::Label>>,
        pub tv_hints_hide_source: RefCell<Option<glib::SourceId>>,
        pub tv_preferences_section: RefCell<Option<adw::SidebarSection>>,
        pub tv_session_section: RefCell<Option<adw::SidebarSection>>,

        #[template_child]
        pub sidebar_breakpoint: TemplateChild<adw::Breakpoint>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Window {
        const NAME: &'static str = "AppWindow";
        type Type = super::Window;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            PlayerToolbarBox::ensure_type();
            ItemActionsBox::ensure_type();
            MediaContentViewer::ensure_type();
            MediaViewer::ensure_type();
            ImageDialog::ensure_type();
            HomePage::ensure_type();
            SearchPage::ensure_type();
            LikedPage::ensure_type();
            MPVPage::ensure_type();
            MPVControlSidebar::ensure_type();
            ThemeSwitcher::ensure_type();
            klass.bind_template();
            klass.bind_template_instance_callbacks();
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
            klass.install_action("win.toggle-fullscreen", None, |obj, _, _| {
                if obj.is_fullscreen() {
                    obj.unfullscreen();
                } else {
                    obj.fullscreen();
                }
            });
            klass.install_action("win.search", None, |obj, _, _| {
                obj.searchpage();
            });
            klass.install_action("win.add-server", None, |obj, _, _| {
                obj.new_account();
            });
            klass.install_action("win.quit", None, |window, _, _| {
                if let Some(app) = window.application() {
                    app.quit();
                }
            });
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Window {
        fn constructed(&self) {
            // Call "constructed" on parent
            self.parent_constructed();

            let store = gtk::gio::ListStore::new::<TuObject>();
            self.mpv_playlist_selection.set_model(Some(&store));
            self.mpv_playlist
                .set_model(Some(&self.mpv_playlist_selection));
            self.mpv_playlist.set_factory(Some(
                gtk::SignalListItemFactory::new().tu_overview_item(ViewGroup::EpisodesView),
            ));
            self.mpv_control_sidebar
                .set_player(Some(&self.mpvnav.imp().video.get()));

            let obj = self.obj();

            self.sidebar_breakpoint.connect_apply(glib::clone!(
                #[weak]
                obj,
                move |_breakpoint| {
                    obj.imp().split_view.set_collapsed(true);
                }
            ));
            self.sidebar_breakpoint.connect_unapply(glib::clone!(
                #[weak]
                obj,
                move |_breakpoint| {
                    if crate::tv::is_tv_mode_active() && SETTINGS.tv_hide_sidebar() {
                        let split_view = obj.imp().split_view.get();
                        split_view.set_collapsed(true);
                        split_view.set_show_sidebar(false);
                        return;
                    }
                    if !SETTINGS.is_overlay() {
                        obj.imp().split_view.set_collapsed(false);
                    }
                }
            ));

            obj.bind_about_action();

            spawn(glib::clone!(
                #[weak(rename_to = obj)]
                obj,
                async move {
                    obj.setup_rootpic();
                    obj.set_servers().await;
                    obj.set_nav_servers();
                    obj.set_shortcuts();
                },
            ));
        }
    }

    impl WidgetImpl for Window {}

    impl WindowImpl for Window {
        // Save window state right before the window will be closed
        fn close_request(&self) -> glib::Propagation {
            // Save window size
            self.obj()
                .save_window_state()
                .expect("Failed to save window state");
            // Allow to invoke other event handlers
            glib::Propagation::Proceed
        }
    }

    impl ApplicationWindowImpl for Window {}
    impl AdwApplicationWindowImpl for Window {}
}

use super::{
    home::HomePage,
    item::{
        ItemPage,
        SelectedVideoSubInfo,
    },
    liked::LikedPage,
    search::SearchPage,
    server_action_row,
    server_panel::ServerPanel,
    single_grid::SingleGrid,
    tu_item::PROGRESSBAR_ANIMATION_DURATION,
    utils::GlobalToast,
};
use crate::{
    APP_ID,
    client::{
        Account,
        jellyfin_client::JELLYFIN_CLIENT,
    },
    ui::{
        input::{
            InputAction,
            MainTab,
            NavigationContext,
            PushedPageKind,
            key_to_action,
        },
        models::SETTINGS,
        provider::{
            IS_ADMIN,
            core_song::CoreSong,
            tu_item::TuItem,
            tu_object::TuObject,
        },
    },
    utils::{
        spawn,
        spawn_tokio,
    },
};
use glib::Object;
use gtk::{
    gio,
    glib,
    template_callbacks,
};

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends adw::ApplicationWindow, gtk::ApplicationWindow, gtk::Window, gtk::Widget, gtk::HeaderBar,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

pub const PROGRESSBAR_FADE_ANIMATION_DURATION: u32 = 500;

#[template_callbacks]
impl Window {
    pub fn homepage(&self) {
        let imp = self.imp();
        if imp.homepage.child().is_none() {
            imp.homepage.set_child(Some(&HomePage::new()));
        }
        imp.navipage.set_title(&gettext("Home"));
        imp.mainview.pop_to_tag("mainpage");
        imp.insidestack.set_visible_child_name("homepage");
        imp.popbutton.set_visible(false);
        imp.last_content_list_selection.replace(Some(0));
        if let Some(home) = imp.homepage.child().and_downcast::<HomePage>() {
            imp.focus_manager.borrow().register_home(&home);
        }
    }

    pub fn likedpage(&self) {
        let imp = self.imp();
        if imp.likedpage.child().is_none() {
            imp.likedpage.set_child(Some(&LikedPage::new()));
        }
        imp.navipage.set_title(&gettext("Liked"));
        imp.mainview.pop_to_tag("mainpage");
        imp.insidestack.set_visible_child_name("likedpage");
        imp.popbutton.set_visible(false);
        imp.last_content_list_selection.replace(Some(1));
        if let Some(liked) = imp.likedpage.child().and_downcast::<LikedPage>() {
            imp.liked_navigator.borrow().register(&liked);
        }
    }

    pub fn searchpage(&self) {
        let imp = self.imp();
        if imp.searchpage.child().is_none() {
            imp.searchpage.set_child(Some(&SearchPage::new()));
        }
        imp.navipage.set_title(&gettext("Search"));
        imp.mainview.pop_to_tag("mainpage");
        imp.insidestack.set_visible_child_name("searchpage");
        imp.popbutton.set_visible(false);
        imp.last_content_list_selection.replace(Some(2));
    }

    #[template_callback]
    pub fn on_pop(&self) {
        let imp = self.imp();
        imp.mainview.pop();
        let Some(now_page) = imp.mainview.visible_page() else {
            return;
        };
        let Some(tag) = now_page.tag() else {
            return;
        };
        if tag != "mainpage" {
            imp.navipage.set_title(&now_page.title());
            return;
        }

        imp.popbutton.set_visible(false);
        imp.navipage.set_title("");
        if imp.insidestack.visible_child_name().as_deref() == Some("homepage")
            && let Some(home) = imp.homepage.child().and_downcast::<HomePage>()
        {
            let fm = imp.focus_manager.borrow();
            if let Some(snapshot) = imp.home_focus_snapshot.borrow_mut().take() {
                fm.restore_home_focus(&home, snapshot);
            } else {
                fm.refresh_home_rows(&home);
            }
        }
        self.refresh_homepage_if_needed();
    }

    pub fn now_page_tag(&self) -> Option<String> {
        let now_page = self.imp().mainview.visible_page()?;

        now_page.tag().map(|s| s.to_string())
    }

    pub async fn set_servers(&self) {
        let imp = self.imp();
        let listbox = &imp.serversbox;
        listbox.remove_all();
        let accounts = SETTINGS.accounts();
        for account in &accounts {
            if SETTINGS.auto_select_server()
                && account.servername == SETTINGS.preferred_server()
                && JELLYFIN_CLIENT.session().account.user_id.is_empty()
            {
                let _ = JELLYFIN_CLIENT.init(account).await;
                self.reset();
            }
        }
        if accounts.is_empty() {
            imp.login_stack.set_visible_child_name("no-server");
            imp.placeholder_navigator.borrow().reset();
            if crate::tv::focus::tv_focus_enabled() {
                let add_button = self.imp().add_server.get();
                self.imp()
                    .placeholder_navigator
                    .borrow()
                    .focus_add_server(&add_button);
            }
            return;
        } else {
            imp.login_stack.set_visible_child_name("servers");
        }
        imp.placeholder_navigator.borrow().reset();
        for (index, account) in accounts.iter().enumerate() {
            let server_action_row = server_action_row::ServerActionRow::new(account.to_owned());

            let drag_source = gtk::DragSource::builder()
                .name("descriptor-drag-format")
                .actions(gtk::gdk::DragAction::MOVE)
                .build();

            drag_source.connect_prepare(glib::clone!(
                #[weak(rename_to = widget)]
                server_action_row,
                #[weak]
                listbox,
                #[strong]
                account,
                #[upgrade_or]
                None,
                move |drag_context, _x, _y| {
                    listbox.drag_highlight_row(&widget);
                    let icon = gtk::WidgetPaintable::new(Some(&widget));
                    drag_context.set_icon(Some(&icon), 0, 0);
                    let object = glib::BoxedAnyObject::new(account.to_owned());
                    Some(gtk::gdk::ContentProvider::for_value(&object.to_value()))
                }
            ));

            let drop_target = gtk::DropTarget::builder()
                .name("descriptor-drag-format")
                .propagation_phase(gtk::PropagationPhase::Capture)
                .actions(gtk::gdk::DragAction::MOVE)
                .build();

            drop_target.set_types(&[glib::BoxedAnyObject::static_type()]);

            drop_target.connect_drop(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                #[strong]
                account,
                #[upgrade_or]
                false,
                move |_drop_target, value, _y, _data| {
                    let lr_account = value
                        .get::<glib::BoxedAnyObject>()
                        .expect("Failed to get descriptor from drop data");
                    let lr_account: std::cell::Ref<Account> = lr_account.borrow();

                    if account == *lr_account {
                        return false;
                    }

                    let mut accounts = SETTINGS.accounts();
                    let lr_index = accounts.iter().position(|d| *d == *lr_account).unwrap();
                    accounts.remove(lr_index);
                    accounts.insert(index, lr_account.to_owned());
                    SETTINGS
                        .set_accounts(accounts)
                        .expect("Failed to set accounts");

                    spawn(glib::clone!(
                        #[weak]
                        obj,
                        async move {
                            obj.set_servers().await;
                            obj.set_nav_servers();
                        }
                    ));

                    true
                }
            ));

            server_action_row.add_controller(drag_source);
            server_action_row.add_controller(drop_target);

            listbox.append(&server_action_row);
        }

        if accounts.len() == 1 {
            if let Some(row) = listbox.row_at_index(0) {
                listbox.select_row(Some(&row));
            }
        } else if crate::tv::focus::tv_focus_enabled() {
            self.imp()
                .placeholder_navigator
                .borrow()
                .select_initial(listbox);
        }
    }

    pub fn set_nav_servers(&self) {
        let imp = self.imp();
        imp.servers_section.remove_all();
        let accounts = SETTINGS.accounts();
        for account in accounts {
            let item = adw::SidebarItem::new(&account.servername);
            item.set_icon_name(Some("network-server-symbolic"));
            imp.servers_section.append(item);
        }
    }

    pub fn reset(&self) {
        self.mainpage();
        self.imp().selectlist.set_selected(0);

        spawn(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                obj.account_setup().await;
                obj.remove_all();
                obj.homepage();

                let avatar =
                    match spawn_tokio(async move { JELLYFIN_CLIENT.get_user_avatar().await }).await
                    {
                        Ok(avatar) => avatar,
                        Err(e) => {
                            obj.toast(e.to_string());
                            return;
                        }
                    };

                let Some(texture) = gtk::gdk::Texture::from_file(&gio::File::for_path(avatar)).ok()
                else {
                    obj.imp()
                        .avatar
                        .set_custom_image(None::<&gtk::gdk::Paintable>);
                    return;
                };

                obj.imp().avatar.set_custom_image(Some(&texture));
            }
        ));
    }

    pub fn hard_set_fraction(&self, to_value: f64) {
        let progressbar = &self.imp().progressbar;
        self.progressbar_animation().pause();
        progressbar.set_fraction(to_value);
    }

    pub async fn account_setup(&self) {
        let imp = self.imp();
        let s = JELLYFIN_CLIENT.session();
        imp.namerow.set_title(&s.account.username);
        imp.namerow.set_subtitle(&s.account.servername);
    }

    pub fn account_settings(&self) {
        let window_clone = self.clone();
        let ac = crate::ui::widgets::account_settings::AccountSettings::new(window_clone);
        ac.set_transient_for(Some(self));
        ac.set_application(Some(&self.application().unwrap()));
        if crate::tv::is_tv_mode_active() {
            ac.set_decorated(false);
        }
        *self.imp().active_settings.borrow_mut() = Some(ac.clone());
        self.imp()
            .settings_navigator
            .borrow()
            .reset_for_preferences(&ac);
        ac.present();
    }

    pub fn change_pop_visibility(&self) {
        let imp = self.imp();
        imp.popbutton.set_visible(!imp.popbutton.is_visible());
    }

    pub fn set_pop_visibility(&self, visible: bool) {
        self.imp().popbutton.set_visible(visible);
    }

    pub fn save_window_state(&self) -> Result<(), glib::BoolError> {
        let (width, height) = self.default_size();
        SETTINGS.set_window_dismension(width, height)?;

        SETTINGS.set_is_maximized(self.is_maximized())?;
        SETTINGS.set_is_fullscreen(self.is_fullscreen())?;

        Ok(())
    }

    pub fn load_window_state(&self) {
        let (width, height) = SETTINGS.window_dismension();
        self.set_default_size(width, height);

        if SETTINGS.is_maximized() {
            self.maximize();
        }

        if SETTINGS.is_fullscreen() {
            self.fullscreen();
        }

        self.overlay_sidebar(SETTINGS.is_overlay());
    }

    pub fn new(app: &crate::Application) -> Self {
        Object::builder().property("application", app).build()
    }

    pub fn set_title(&self, title: &str) {
        self.imp().navipage.set_title(title);
    }

    pub fn mainpage(&self) {
        self.imp().stack.set_visible_child_name("main");
        self.sync_tv_button_hints();
    }

    pub fn refresh_homepage_if_needed(&self) {
        if self.now_page_tag() == Some("mainpage".into())
            && SETTINGS.is_refresh()
            && let Some(homepage) = self.imp().homepage.child().and_downcast_ref::<HomePage>()
        {
            homepage.update(false);
        }
    }

    fn placeholder(&self) {
        self.imp().stack.set_visible_child_name("placeholder");
    }

    fn sidebar(&self) {
        let imp = self.imp();
        imp.split_view
            .set_show_sidebar(!imp.split_view.shows_sidebar());
    }

    pub fn overlay_sidebar(&self, overlay: bool) {
        self.imp().split_view.set_collapsed(overlay);
    }

    /// In TV mode with a collapsed sidebar, show or hide the overlay panel.
    pub fn set_sidebar_panel_visible(&self, visible: bool) {
        let split_view = self.imp().split_view.get();
        if crate::tv::is_tv_mode_active() && SETTINGS.tv_hide_sidebar() {
            split_view.set_collapsed(true);
            split_view.set_show_sidebar(visible);
            return;
        }
        if visible != split_view.shows_sidebar() {
            gtk::prelude::ActionGroupExt::activate_action(self, "win.sidebar", None);
        }
    }

    pub fn tv_sidebar_collapsed(&self) -> bool {
        crate::tv::is_tv_mode_active() && SETTINGS.tv_hide_sidebar()
    }

    pub fn add_toast(&self, toast: adw::Toast) {
        self.imp().toast.add_toast(toast);
    }

    pub fn current_view_name(&self) -> String {
        self.imp()
            .insidestack
            .visible_child_name()
            .unwrap()
            .to_string()
    }

    pub fn set_progressbar_opacity(&self, opacity: f64) {
        self.imp().progressbar.set_opacity(opacity);
    }

    pub fn set_rootpic(&self, file: gio::File) {
        let settings = Settings::new(APP_ID);

        if !settings.boolean("is-backgroundenabled") {
            return;
        }

        let backgroundstack = &self.imp().backgroundstack;
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

        if backgroundstack.observe_children().n_items() > 2
            && let Some(child) = backgroundstack.first_child()
        {
            backgroundstack.remove(&child);
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
        if let Some(child) = self.imp().backgroundstack.last_child() {
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
        *self.imp().active_account_dialog.borrow_mut() = Some(dialog.clone());
        dialog.present(Some(self));
    }

    pub fn set_fraction(&self, to_value: f64) {
        let progressbar = &self.imp().progressbar;
        self.progressbar_animation()
            .set_value_from(progressbar.fraction());
        self.progressbar_animation().set_value_to(to_value);
        self.progressbar_animation().play();
    }

    pub fn set_progressbar_fade(&self) {
        let progressbar = &self.imp().progressbar;
        self.progressbar_fade_animation()
            .set_value_from(progressbar.opacity());
        self.progressbar_fade_animation().play();
    }

    fn progressbar_animation(&self) -> &adw::TimedAnimation {
        self.imp().progress_bar_animation.get_or_init(|| {
            let target = adw::CallbackAnimationTarget::new(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move |fraction| obj.imp().progressbar.set_fraction(fraction)
            ));

            adw::TimedAnimation::builder()
                .duration(PROGRESSBAR_ANIMATION_DURATION)
                .widget(&self.imp().progressbar.get())
                .target(&target)
                .build()
        })
    }

    fn progressbar_fade_animation(&self) -> &adw::TimedAnimation {
        self.imp().progress_bar_fade_animation.get_or_init(|| {
            let target = adw::CallbackAnimationTarget::new(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move |opacity| obj.imp().progressbar.set_opacity(opacity)
            ));

            adw::TimedAnimation::builder()
                .duration(PROGRESSBAR_FADE_ANIMATION_DURATION)
                .widget(&self.imp().progressbar.get())
                .target(&target)
                .value_to(0.)
                .build()
        })
    }

    pub fn reveal_image(&self, source_widget: &impl IsA<gtk::Widget>) {
        let imp = self.imp();
        imp.media_viewer.reveal(source_widget);
    }

    pub fn media_viewer_show_paintable(&self, paintable: Option<gtk::gdk::Paintable>) {
        let Some(paintable) = paintable else {
            return;
        };

        self.imp().media_viewer.view_image(paintable);
    }

    #[template_callback]
    fn on_add_server(&self) {
        self.new_account();
    }

    pub async fn bind_song_model(&self, active_model: gio::ListStore, active_core_song: CoreSong) {
        self.imp()
            .player_toolbar_box
            .bind_song_model(active_model, active_core_song)
            .await;
    }

    pub fn play_media(
        &self, selected: Option<SelectedVideoSubInfo>, item: TuItem, episode_list: Vec<TuItem>,
        matcher: Option<String>, start_seconds: f64, external_sub: Option<String>,
    ) {
        let imp = self.imp();
        imp.stack.set_visible_child_name("mpv");
        self.sync_tv_button_hints();
        self.prevent_suspend();
        self.set_mpv_playlist(&episode_list);
        imp.mpvnav.set_external_sub_override(external_sub);
        imp.mpvnav
            .play(selected, item, episode_list, matcher, start_seconds);
    }

    pub fn push_page<T>(&self, page: &T, tag: &str, name: &str)
    where
        T: NavigationPageExt,
    {
        if self.now_page_tag().as_deref() == Some("mainpage")
            && self.imp().insidestack.visible_child_name().as_deref() == Some("homepage")
            && let Some(home) = self.imp().homepage.child().and_downcast::<HomePage>()
        {
            let snapshot = self.imp().focus_manager.borrow().snapshot_home_focus();
            *self.imp().home_focus_snapshot.borrow_mut() = Some(snapshot);
            self.imp().focus_manager.borrow().clear_all_row_selections();
            let _ = home;
        }

        let imp = self.imp();
        page.set_title(name);
        imp.navipage.set_title(name);
        if imp.mainview.find_page(tag).is_some() {
            imp.mainview.pop_to_tag(tag);
            return;
        }
        page.set_tag(Some(tag));
        imp.mainview.push(page);
        imp.popbutton.set_visible(true);
    }

    #[template_callback]
    pub fn on_home_update(&self) {
        if let Some(homepage) = self.imp().homepage.child().and_downcast::<HomePage>() {
            homepage.update(false);
        }
        self.homepage();
    }

    #[template_callback]
    pub fn on_liked_update(&self) {
        if let Some(likedpage) = self.imp().likedpage.child().and_downcast::<LikedPage>() {
            likedpage.update();
        }
        self.likedpage();
    }

    pub fn remove_all(&self) {
        self.imp().homepage.set_child(None::<&Widget>);
        self.imp().likedpage.set_child(None::<&Widget>);
        self.imp().searchpage.set_child(None::<&Widget>);
        self.imp().player_toolbar_box.on_stop_button_clicked();
    }

    #[template_callback]
    fn avatar_pressed_cb(&self) {
        if !IS_ADMIN.load(std::sync::atomic::Ordering::Relaxed) {
            return;
        }

        let page = ServerPanel::new();
        let tag = gettext("Server Panel");
        page.set_tag(Some(&tag));
        self.push_page(&page, &tag, &tag);
    }

    fn is_on_mpv_stack(&self) -> bool {
        self.imp().stack.visible_child_name() == Some("mpv".into())
    }

    pub fn is_on_mpv_stack_pub(&self) -> bool {
        self.is_on_mpv_stack()
    }

    #[template_callback]
    fn key_pressed_cb(&self, key: u32, _code: u32, state: gtk::gdk::ModifierType) -> bool {
        if state.contains(gtk::gdk::ModifierType::CONTROL_MASK)
            || state.contains(gtk::gdk::ModifierType::ALT_MASK)
        {
            return false;
        }

        if self.is_on_mpv_stack() {
            self.imp().mpvnav.key_pressed_cb(key, state);
            if self.imp().mpv_view.shows_sidebar() {
                return false;
            }
            return true;
        }

        if crate::tv::controller_navigation_enabled()
            && let Some(action) = key_to_action(key)
        {
            return self.handle_input_action(action);
        }

        false
    }

    #[template_callback]
    fn key_released_cb(&self, key: u32, _code: u32, state: gtk::gdk::ModifierType) {
        if self.is_on_mpv_stack() {
            self.imp().mpvnav.key_released_cb(key, state);
        }
    }

    #[template_callback]
    fn on_sidebar_activated(&self, index: u32) {
        let imp = self.imp();
        let sidebar = imp.selectlist.get();
        let Some(item) = sidebar.item(index) else {
            return;
        };
        let Some(section) = item.section() else {
            return;
        };

        if let Some(prefs) = imp.tv_preferences_section.borrow().as_ref()
            && prefs == &section
        {
            self.account_settings();
            return;
        }

        if let Some(session) = imp.tv_session_section.borrow().as_ref()
            && session == &section
        {
            match item.section_index() {
                0 => {
                    let _ = gtk::prelude::WidgetExt::activate_action(self, "win.relogin", None);
                }
                1 => {
                    let _ = gtk::prelude::WidgetExt::activate_action(self, "win.quit", None);
                }
                _ => {}
            }
            return;
        }

        if section == *imp.servers_section {
            let section_idx = item.section_index() as usize;
            let accounts = SETTINGS.accounts();
            if let Some(account) = accounts.get(section_idx).cloned() {
                spawn(glib::clone!(
                    #[weak(rename_to = obj)]
                    self,
                    async move {
                        SETTINGS.set_preferred_server(&account.servername).unwrap();
                        let _ = JELLYFIN_CLIENT.init(&account).await;
                        obj.reset();
                    }
                ));
            }
            return;
        }

        let pos = item.section_index() as i32;
        let last_pos = *imp.last_content_list_selection.borrow();
        if last_pos == Some(pos) {
            self.update_view(pos);
            return;
        }
        self.select_view(pos);
    }

    fn select_view(&self, pos: i32) {
        match pos {
            0 => self.homepage(),
            1 => self.likedpage(),
            2 => self.searchpage(),
            _ => {}
        }
    }

    fn update_view(&self, pos: i32) {
        match pos {
            0 => self.on_home_update(),
            1 => self.on_liked_update(),
            _ => {}
        }
    }

    pub fn setup_input(&self) {
        crate::tv::cursor::register_window(self);
        let window = self.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(16), move || {
            let actions = {
                let mut mgr = window.imp().gamepad_manager.borrow_mut();
                mgr.poll(&window)
            };
            if !actions.is_empty() {
                crate::tv::cursor::on_gamepad_activity();
                glib::idle_add_local_once(crate::tv::cursor::on_gamepad_activity);
            }
            for action in actions {
                window.handle_input_action(action);
            }
            glib::ControlFlow::Continue
        });

        let pointer = gtk::GestureClick::new();
        pointer.connect_pressed(|_, _, _, _| crate::tv::osk::mark_pointer_input());
        self.add_controller(pointer);

        let motion = gtk::EventControllerMotion::new();
        motion.connect_motion(|_, _, _| crate::tv::osk::mark_pointer_input());
        self.add_controller(motion);

        let keys = gtk::EventControllerKey::new();
        keys.connect_key_pressed(|_, _, _, _| {
            crate::tv::osk::mark_keyboard_input();
            glib::Propagation::Proceed
        });
        self.add_controller(keys);

        self.sync_tv_button_hints();
    }

    pub fn refresh_focus_manager(&self) {
        if let Some(home) = self.imp().homepage.child().and_downcast::<HomePage>() {
            self.imp().focus_manager.borrow().refresh_home_rows(&home);
        }
    }

    pub fn activate_sidebar_selection(&self) {
        let sidebar = self.imp().selectlist.get();
        self.on_sidebar_activated(sidebar.selected());
    }

    pub fn is_on_placeholder(&self) -> bool {
        self.imp().stack.visible_child_name().as_deref() == Some("placeholder")
    }

    pub fn enable_tv_mode_ui(&self, cli_fullscreen: bool) {
        self.add_css_class("tv-mode");
        self.set_decorated(false);

        if self.imp().tv_hints_revealer.borrow().is_none() {
            self.setup_tv_hints();
        }
        self.sync_tv_button_hints();

        let start_fullscreen = cli_fullscreen
            || SETTINGS.tv_start_fullscreen()
            || crate::steam::is_steam_big_picture();
        if start_fullscreen {
            self.fullscreen();
        }

        if SETTINGS.tv_hide_sidebar() {
            self.set_sidebar_panel_visible(false);
            let window = self.clone();
            glib::idle_add_local_once(move || {
                window.set_sidebar_panel_visible(false);
            });
        }

        self.setup_tv_preferences_sidebar();
        self.setup_tv_session_sidebar();
    }

    fn setup_tv_preferences_sidebar(&self) {
        if self.imp().tv_preferences_section.borrow().is_some() {
            return;
        }
        let section = adw::SidebarSection::new();
        section.set_title(Some(&gettext("Settings")));
        let item = adw::SidebarItem::new(&gettext("Preferences"));
        item.set_icon_name(Some("applications-system-symbolic"));
        section.append(item);
        let stored = section.clone();
        self.imp().selectlist.get().append(section);
        *self.imp().tv_preferences_section.borrow_mut() = Some(stored);
    }

    fn setup_tv_session_sidebar(&self) {
        if self.imp().tv_session_section.borrow().is_some() {
            return;
        }
        let section = adw::SidebarSection::new();
        section.set_title(Some(&gettext("Session")));
        let logout = adw::SidebarItem::new(&gettext("Log Out"));
        logout.set_icon_name(Some("system-log-out-symbolic"));
        let quit = adw::SidebarItem::new(&gettext("Quit"));
        quit.set_icon_name(Some("application-exit-symbolic"));
        section.append(logout);
        section.append(quit);
        let stored = section.clone();
        self.imp().selectlist.get().append(section);
        *self.imp().tv_session_section.borrow_mut() = Some(stored);
    }

    fn remove_tv_session_sidebar(&self) {
        if let Some(section) = self.imp().tv_session_section.borrow_mut().take() {
            self.imp().selectlist.get().remove(&section);
        }
    }

    fn remove_tv_preferences_sidebar(&self) {
        if let Some(section) = self.imp().tv_preferences_section.borrow_mut().take() {
            self.imp().selectlist.get().remove(&section);
        }
    }

    pub fn disable_tv_mode_ui(&self) {
        self.remove_tv_preferences_sidebar();
        self.remove_tv_session_sidebar();
        self.remove_css_class("tv-mode");
        crate::tv::cursor::restore();
        self.set_decorated(true);
        if let Some(revealer) = self.imp().tv_hints_revealer.borrow().as_ref() {
            revealer.set_visible(false);
            revealer.set_reveal_child(false);
        }
        if self.is_fullscreen() && !SETTINGS.is_fullscreen() && !SETTINGS.tv_start_fullscreen() {
            self.unfullscreen();
        }
        self.overlay_sidebar(SETTINGS.overlay());
    }

    pub fn handle_input_action(&self, action: InputAction) -> bool {
        if crate::tv::osk::handle_input(action) {
            return true;
        }
        if self.is_on_mpv_stack() && SETTINGS.tv_show_button_hints() {
            self.flash_tv_button_hints();
        }
        if crate::subtitles::dialog::handle_active_input(action) {
            return true;
        }
        if crate::ui::input::popover_navigator::handle(self, action) {
            return true;
        }
        if crate::playback::rule_editor::handle_active_input(action) {
            return true;
        }
        if crate::ui::input::dialog_navigator::handle(self, action) {
            return true;
        }

        match self.resolve_navigation_context() {
            NavigationContext::Mpv => {
                self.imp()
                    .mpv_navigator
                    .borrow()
                    .handle(self, &self.imp().mpvnav, action)
            }
            NavigationContext::Placeholder => {
                let imp = self.imp();
                imp.placeholder_navigator.borrow().handle(
                    self,
                    &imp.login_stack.get(),
                    &imp.serversbox.get(),
                    action,
                )
            }
            NavigationContext::MediaViewer => self.imp().media_viewer_navigator.borrow().handle(
                self,
                &self.imp().media_viewer.get(),
                action,
            ),
            NavigationContext::Modal => self.handle_modal_input(action),
            NavigationContext::Pushed(PushedPageKind::Item) => {
                if let Some(page) = self
                    .imp()
                    .mainview
                    .visible_page()
                    .and_downcast::<ItemPage>()
                {
                    return self
                        .imp()
                        .item_navigator
                        .borrow()
                        .handle(self, &page, action);
                }
                false
            }
            NavigationContext::Pushed(PushedPageKind::Grid) => {
                if let Some(page) = self
                    .imp()
                    .mainview
                    .visible_page()
                    .and_downcast::<SingleGrid>()
                {
                    return self
                        .imp()
                        .grid_navigator
                        .borrow()
                        .handle(self, &page, action);
                }
                false
            }
            NavigationContext::Pushed(PushedPageKind::Other) => {
                self.imp().pushed_navigator.borrow().handle(self, action)
            }
            NavigationContext::Main(MainTab::Home) => match action {
                InputAction::ToggleHints => {
                    self.toggle_tv_hints();
                    true
                }
                InputAction::SwitchGamepad => {
                    let enabled = !crate::ui::SETTINGS.gamepad_enabled();
                    let _ = crate::ui::SETTINGS.set_gamepad_enabled(enabled);
                    if !enabled {
                        crate::tv::cursor::restore();
                    }
                    let mgr = self.imp().gamepad_manager.borrow();
                    self.toast(if enabled {
                        format!(
                            "{} — {} ({})",
                            gettext("Controller navigation enabled"),
                            mgr.active_name(),
                            match mgr.active_profile() {
                                crate::ui::input::GamepadProfile::Xbox => "Xbox",
                                crate::ui::input::GamepadProfile::PlayStation => "PlayStation",
                                crate::ui::input::GamepadProfile::SteamDeck => "Steam Deck",
                                crate::ui::input::GamepadProfile::Nintendo => "Nintendo",
                                crate::ui::input::GamepadProfile::Generic => "Generic",
                            }
                        )
                    } else {
                        gettext("Controller navigation disabled").to_string()
                    });
                    true
                }
                InputAction::PlayPause => {
                    self.imp().player_toolbar_box.toggle_playback();
                    true
                }
                _ => self.imp().focus_manager.borrow().handle(self, action),
            },
            NavigationContext::Main(MainTab::Liked) => {
                if let Some(liked) = self.imp().likedpage.child().and_downcast::<LikedPage>() {
                    self.imp()
                        .liked_navigator
                        .borrow()
                        .handle(self, &liked, action)
                } else {
                    false
                }
            }
            NavigationContext::Main(MainTab::Search) => {
                if let Some(search) = self.imp().searchpage.child().and_downcast::<SearchPage>() {
                    self.imp()
                        .search_navigator
                        .borrow()
                        .handle(self, &search, action)
                } else {
                    false
                }
            }
        }
    }

    fn handle_modal_input(&self, action: InputAction) -> bool {
        if let Some(settings) = self.imp().active_settings.borrow().clone() {
            if settings.is_visible() {
                return self
                    .imp()
                    .settings_navigator
                    .borrow()
                    .handle_window(&settings, action);
            }
            *self.imp().active_settings.borrow_mut() = None;
        }
        if let Some(account) = self.imp().active_account_dialog.borrow().clone() {
            if account.is_visible() {
                return self
                    .imp()
                    .settings_navigator
                    .borrow()
                    .handle_account_window(&account, action);
            }
            *self.imp().active_account_dialog.borrow_mut() = None;
        }
        false
    }

    fn setup_tv_hints(&self) {
        if !crate::tv::is_tv_mode_active() {
            return;
        }
        if self.imp().tv_hints_revealer.borrow().is_some() {
            return;
        }
        let revealer = gtk::Revealer::builder()
            .valign(gtk::Align::End)
            .halign(gtk::Align::Center)
            .transition_type(gtk::RevealerTransitionType::SlideUp)
            .reveal_child(false)
            .visible(false)
            .build();

        let bar = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .build();
        bar.add_css_class("tv-hints-bar");

        let label = gtk::Label::new(None);
        label.set_wrap(false);
        bar.append(&label);
        revealer.set_child(Some(&bar));

        let toast = self.imp().toast.get();
        if let Some(overlay) = toast.parent().and_downcast::<gtk::Overlay>() {
            overlay.add_overlay(&revealer);
        }

        *self.imp().tv_hints_revealer.borrow_mut() = Some(revealer);
        *self.imp().tv_hints_label.borrow_mut() = Some(label);
    }

    pub fn sync_tv_button_hints(&self) {
        if !crate::tv::is_tv_mode_active() {
            if let Some(revealer) = self.imp().tv_hints_revealer.borrow().as_ref() {
                revealer.set_visible(false);
                revealer.set_reveal_child(false);
            }
            return;
        }

        if self.imp().tv_hints_revealer.borrow().is_none() {
            self.setup_tv_hints();
        }

        let Some(revealer) = self.imp().tv_hints_revealer.borrow().clone() else {
            return;
        };

        if !SETTINGS.tv_show_button_hints() {
            if let Some(id) = self.imp().tv_hints_hide_source.borrow_mut().take() {
                id.remove();
            }
            revealer.set_visible(false);
            revealer.set_reveal_child(false);
            return;
        }

        revealer.set_visible(true);
        self.update_tv_hints_label();

        let show = self.imp().stack.visible_child_name().as_deref() == Some("main")
            && !self.is_on_mpv_stack();

        revealer.set_reveal_child(show);
    }

    pub fn flash_tv_button_hints(&self) {
        if !crate::tv::is_tv_mode_active() || !SETTINGS.tv_show_button_hints() {
            return;
        }

        if self.imp().tv_hints_revealer.borrow().is_none() {
            self.setup_tv_hints();
        }
        let Some(revealer) = self.imp().tv_hints_revealer.borrow().clone() else {
            return;
        };

        revealer.set_visible(true);
        self.update_tv_hints_label();
        revealer.set_reveal_child(true);

        if let Some(id) = self.imp().tv_hints_hide_source.borrow_mut().take() {
            id.remove();
        }
        let window = self.clone();
        let id = glib::timeout_add_local(std::time::Duration::from_secs(4), move || {
            if window.is_on_mpv_stack() {
                if let Some(revealer) = window.imp().tv_hints_revealer.borrow().as_ref() {
                    revealer.set_reveal_child(false);
                }
            } else {
                window.sync_tv_button_hints();
            }
            window.imp().tv_hints_hide_source.borrow_mut().take();
            glib::ControlFlow::Break
        });
        *self.imp().tv_hints_hide_source.borrow_mut() = Some(id);
    }

    fn update_tv_hints_label(&self) {
        let Some(label) = self.imp().tv_hints_label.borrow().clone() else {
            return;
        };
        let profile = self.imp().gamepad_manager.borrow().active_profile();
        label.set_text(&format!(
            "{}: Navigate   {}: Select   {}: Back   Menu: Sidebar",
            "D-pad",
            profile.activate_label(),
            profile.back_label(),
        ));
    }

    fn toggle_tv_hints(&self) {
        let enabled = !SETTINGS.tv_show_button_hints();
        let _ = SETTINGS.set_tv_show_button_hints(enabled);
        self.sync_tv_button_hints();
    }

    pub fn set_shortcuts(&self) {
        let shortcuts_action = gtk::gio::ActionEntry::builder("show-help-overlay")
            .activate(|window: &Window, _, _| {
                window.imp().mpvnav.set_can_fade_cursor_set(false);
                let Some(dialog) =
                    gtk::Builder::from_resource("/moe/tsuna/tsukimi/ui/mpv_shortcuts_window.ui")
                        .object::<adw::ShortcutsDialog>("shortcuts_dialog")
                else {
                    eprintln!("Failed to load shortcuts dialog");
                    return;
                };
                dialog.connect_closed(glib::clone!(
                    #[weak]
                    window,
                    move |_| {
                        window.imp().mpvnav.set_can_fade_cursor_set(true);
                    }
                ));
                dialog.present(Some(window));
            })
            .build();
        self.add_action_entries([shortcuts_action]);
    }

    pub fn set_mpv_playlist(&self, episode_list: &[TuItem]) {
        let model = self.imp().mpv_playlist_selection.model();
        let Some(store) = model.and_downcast_ref::<gio::ListStore>() else {
            return;
        };
        let items = episode_list
            .iter()
            .map(|item| TuObject::new(item.to_owned()))
            .collect::<Vec<_>>();

        store.splice(0, store.n_items(), &items);
    }

    pub fn view_playlist(&self) {
        let imp = self.imp();
        imp.mpv_view.set_show_sidebar(!imp.mpv_view.shows_sidebar());
        imp.mpv_view_stack.set_visible_child_name("playlist");
    }

    pub fn view_control_sidebar(&self) {
        let imp = self.imp();
        imp.mpv_view.set_show_sidebar(!imp.mpv_view.shows_sidebar());
        imp.mpv_view_stack.set_visible_child_name("control-bar");
    }

    #[template_callback]
    async fn on_playlist_item_activated(&self, position: u32, view: &gtk::ListView) {
        let Some(model) = view.model() else {
            return;
        };

        let Some(item) = model.item(position).and_downcast::<TuObject>() else {
            return;
        };

        self.imp().mpvnav.in_play_item(item.item()).await;
    }

    fn prevent_suspend(&self) {
        let app = self.application().expect("No application found");
        let cookie = app.inhibit(
            Some(self),
            gtk::ApplicationInhibitFlags::LOGOUT
                | gtk::ApplicationInhibitFlags::IDLE
                | gtk::ApplicationInhibitFlags::SUSPEND,
            Some("Playing media"),
        );
        self.imp().suspend_cookie.replace(Some(cookie));
    }

    pub fn allow_suspend(&self) {
        let app = self.application().expect("No application found");
        if let Some(cookie) = self.imp().suspend_cookie.take() {
            app.uninhibit(cookie);
        }
    }

    pub async fn update_item_page(&self, current_item: TuItem) {
        let nav = self.imp().mainview.visible_page();
        let Some(now_page) = nav.and_downcast_ref::<ItemPage>() else {
            return;
        };
        now_page.update_intro(current_item).await;
    }

    pub fn close_on_error(&self, description: String) {
        let alert_dialog = adw::AlertDialog::builder()
            .heading(gettext("Fatal Error"))
            .body(gettext(&description))
            .build();
        alert_dialog.add_response("close", &gettext("Copy Error & Close"));
        alert_dialog.set_response_appearance("close", adw::ResponseAppearance::Destructive);
        alert_dialog.connect_response(
            Some("close"),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |_, _| {
                    let clipboard = window.clipboard();
                    clipboard.set_text(&description);
                    window.close();
                }
            ),
        );
        alert_dialog.present(Some(self));
    }

    pub fn alert_dialog(&self, alert_dialog: adw::AlertDialog) {
        alert_dialog.present(Some(self));
    }

    pub fn bind_about_action(&self) {
        let about_action = gtk::gio::ActionEntry::builder("about")
            .activate(|window, _, _| {
                let about = adw::AboutDialog::builder()
                    .application_name("Tsukimi")
                    .version(crate::config::version())
                    .comments("A simple third-party Jellyfin client.")
                    // TRANSLATORS: 'Name <email@domain.com>' or 'Name https://website.example'
                    .translator_credits(gettext("translator-credits"))
                    .website("https://github.com/tsukinaha/tsukimi")
                    .application_icon("moe.tsuna.tsukimi")
                    .license_type(gtk::License::Gpl30)
                    .build();
                about.set_debug_info(&format!(
                    "Version: {}\nArchitecture: {}\nGTK Version: {}.{}.{}\nADW Version: {}.{}.{}\nOS: {}\n",
                    crate::config::version(),
                    std::env::consts::ARCH,
                    gtk::major_version(),
                    gtk::minor_version(),
                    gtk::micro_version(),
                    adw::major_version(),
                    adw::minor_version(),
                    adw::micro_version(),
                    std::env::consts::OS
                ));
                about.add_acknowledgement_section(Some("Code"), &["Inaha", "amtoaer", "Kosette"]);
                about.add_acknowledgement_section(
                    Some("Special Thanks"),
                    &["Qound", "Eikano"],
                );
                about.present(Some(window));
            })
            .build();

        self.add_action_entries([about_action]);
    }
}
