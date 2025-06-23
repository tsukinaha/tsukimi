#![allow(deprecated)]
// FIXME: replace GtkShortcutsWindow when the replacement is appeared on libadwaita

use std::path::PathBuf;

use adw::prelude::*;
use gettextrs::gettext;
use gio::Settings;
use gtk::{
    ListBoxRow,
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
                listexpand_row::ListExpandRow,
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
        pub selectlist: TemplateChild<gtk::ListBox>,
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
        pub serverselectlist: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub media_viewer: TemplateChild<MediaViewer>,
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
            ListExpandRow::ensure_type();
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

            obj.bind_about_action();

            spawn(glib::clone!(
                #[weak(rename_to = obj)]
                obj,
                async move {
                    obj.setup_rootpic();
                    obj.set_servers().await;
                    obj.set_nav_servers();
                    obj.set_shortcuts();
                    obj.alert_windows();
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

use glib::Object;
use gtk::{
    gio,
    glib,
    template_callbacks,
};
#[cfg(target_os = "windows")]
use windows::Win32::System::Power::{
    EXECUTION_STATE,
    SetThreadExecutionState,
};

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
    server_row::ServerRow,
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
        if imp.insidestack.visible_child_name() == Some("homepage".into()) && SETTINGS.is_refresh()
        {
            let binding = imp.homepage.child();
            let Some(homepage) = binding.and_downcast_ref::<HomePage>() else {
                return;
            };
            homepage.update(false);
        }
    }

    pub async fn set_servers(&self) {
        let imp = self.imp();
        let listbox = &imp.serversbox;
        listbox.remove_all();
        let accounts = SETTINGS.accounts();
        for account in &accounts {
            if SETTINGS.auto_select_server()
                && account.servername == SETTINGS.preferred_server()
                && JELLYFIN_CLIENT.user_id.lock().await.is_empty()
            {
                let _ = JELLYFIN_CLIENT.init(account).await;
                self.reset();
            }
        }
        if accounts.is_empty() {
            imp.login_stack.set_visible_child_name("no-server");
            return;
        } else {
            imp.login_stack.set_visible_child_name("servers");
        }
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
    }

    pub fn set_nav_servers(&self) {
        let listbox = &self.imp().serverselectlist;
        listbox.remove_all();
        let accounts = SETTINGS.accounts();
        for account in accounts {
            listbox.append(&ServerRow::new(account));
        }
    }

    #[template_callback]
    pub fn account_activated(&self, account_row: &ServerRow) {
        account_row.activate();
    }

    pub fn reset(&self) {
        self.mainpage();
        self.imp().selectlist.unselect_all();

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
        imp.namerow
            .set_title(&JELLYFIN_CLIENT.user_name.lock().await);
        imp.namerow
            .set_subtitle(&JELLYFIN_CLIENT.server_name.lock().await);
    }

    pub fn account_settings(&self) {
        let window_clone = self.clone();
        let ac = crate::ui::widgets::account_settings::AccountSettings::new(window_clone);
        ac.set_transient_for(Some(self));
        ac.set_application(Some(&self.application().unwrap()));
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
        // Create new window
        Object::builder().property("application", app).build()
    }

    pub fn set_title(&self, title: &str) {
        self.imp().navipage.set_title(title);
    }

    pub fn mainpage(&self) {
        self.imp().stack.set_visible_child_name("main");
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

        if backgroundstack.observe_children().n_items() > 2 {
            if let Some(child) = backgroundstack.first_child() {
                backgroundstack.remove(&child);
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
        matcher: Option<String>, per: f64,
    ) {
        let imp = self.imp();
        imp.stack.set_visible_child_name("mpv");
        self.prevent_suspend();
        self.set_mpv_playlist(&episode_list);
        imp.mpvnav.play(selected, item, episode_list, matcher, per);
    }

    pub fn push_page<T>(&self, page: &T, tag: &str, name: &str)
    where
        T: NavigationPageExt,
    {
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

    #[template_callback]
    fn key_pressed_cb(&self, key: u32, _code: u32, state: gtk::gdk::ModifierType) -> bool {
        if self.is_on_mpv_stack() {
            self.imp().mpvnav.key_pressed_cb(key, state);
            if self.imp().mpv_view.shows_sidebar() {
                return false;
            }
            return true;
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
    fn on_listboxrow_activated(&self, row: &ListBoxRow) {
        row.activate();
    }

    #[template_callback]
    fn on_contentsrow_selected(&self, row: Option<&ListBoxRow>) {
        let Some(row) = row else {
            return;
        };

        let pos = row.index();

        let last_pos = *self.imp().last_content_list_selection.borrow();

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

    pub fn set_shortcuts(&self) {
        let Some(window) =
            gtk::Builder::from_resource("/moe/tsuna/tsukimi/ui/mpv_shortcuts_window.ui")
                .object::<gtk::ShortcutsWindow>("mpv_shortcuts")
        else {
            eprintln!("Failed to load shortcuts window");
            return;
        };
        self.set_help_overlay(Some(&window));
    }

    pub fn set_mpv_playlist(&self, episode_list: &Vec<TuItem>) {
        let model = self.imp().mpv_playlist_selection.model();
        let Some(store) = model.and_downcast_ref::<gio::ListStore>() else {
            return;
        };

        store.remove_all();

        for item in episode_list {
            let object = TuObject::new(item);
            store.append(&object);
        }
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

    #[cfg(target_os = "windows")]
    fn prevent_suspend(&self) {
        let state = unsafe {
            SetThreadExecutionState(
                EXECUTION_STATE(2u32) | EXECUTION_STATE(1u32) | EXECUTION_STATE(2147483648u32),
            )
        }; // ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED | ES_CONTINUOUS
        if state == EXECUTION_STATE(2147483651u32) {
            println!("System suspend inhibited");
        } else {
            eprintln!("Failed to set thread execution state");
        }
    }

    #[cfg(target_os = "windows")]
    pub fn allow_suspend(&self) {
        let state = unsafe { SetThreadExecutionState(EXECUTION_STATE(2147483648u32)) }; // ES_CONTINUOUS
        if state == EXECUTION_STATE(2147483648u32) {
            println!("System suspend uninhibited");
        } else {
            eprintln!("Failed to reset thread execution state");
        }
    }

    #[cfg(target_os = "linux")]
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

    #[cfg(target_os = "linux")]
    pub fn allow_suspend(&self) {
        let app = self.application().expect("No application found");
        if let Some(cookie) = self.imp().suspend_cookie.take() {
            app.uninhibit(cookie);
        }
    }

    pub async fn update_item_page(&self) {
        let nav = self.imp().mainview.visible_page();
        let Some(now_page) = nav.and_downcast_ref::<ItemPage>() else {
            return;
        };

        now_page.update_intro().await;
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

    pub fn alert_windows(&self) {
        #[cfg(target_os = "windows")]
        {
            if !SETTINGS.is_first_run() {
                return;
            }

            let alert_dialog = adw::AlertDialog::builder()
                .heading(gettext("Windows Alert"))
                .body(gettext("It seems you're using Tsukimi on Windows. Please note that GTK used by this application is designed for Linux, and no functionality is guaranteed to work on Windows, including but not limited to video rendering, audio playback, network connections, and external links. Additionally, there are known issues that cannot be fixed, such as black borders, image rendering, font rendering, DPI scaling, video rendering, GL renderer, fullscreen mode, HDR, and locale detection. \nIf you wish to open an issue, please ensure that it is related to this software and not an upstream issue. Choose the appropriate issue template and fill in all required fields without any omissions."))
                .build();
            alert_dialog.add_response("close", &gettext("Close"));
            alert_dialog.present(Some(self));

            SETTINGS
                .set_is_first_run(false)
                .expect("Failed to set first run");
        }
    }

    pub fn alert_dialog(&self, alert_dialog: adw::AlertDialog) {
        alert_dialog.present(Some(self));
    }

    pub fn bind_about_action(&self) {
        let about_action = gtk::gio::ActionEntry::builder("about")
            .activate(|window, _, _| {
                let about = adw::AboutDialog::builder()
                    .application_name("Tsukimi")
                    .version(crate::config::VERSION)
                    .comments("A simple third-party Jellyfin client.")
                    // TRANSLATORS: 'Name <email@domain.com>' or 'Name https://website.example'
                    .translator_credits(gettext("translator-credits"))
                    .website("https://github.com/tsukinaha/tsukimi")
                    .application_icon("moe.tsuna.tsukimi")
                    .license_type(gtk::License::Gpl30)
                    .build();
                about.set_debug_info(&format!(
                    "Version: {}\nArchitecture: {}\nGTK Version: {}.{}.{}\nADW Version: {}.{}.{}\nOS: {}\n",
                    crate::config::VERSION,
                    std::env::consts::ARCH,
                    gtk::major_version(),
                    gtk::minor_version(),
                    gtk::micro_version(),
                    adw::major_version(),
                    adw::minor_version(),
                    adw::micro_version(),
                    std::env::consts::OS
                ));
                about.add_acknowledgement_section(Some("Code"), &["Inaha", "Kosette"]);
                about.add_acknowledgement_section(
                    Some("Special Thanks"),
                    &["Qound", "Eikano", "amtoaer"],
                );
                about.present(Some(window));
            })
            .build();

        self.add_action_entries([about_action]);
    }
}
