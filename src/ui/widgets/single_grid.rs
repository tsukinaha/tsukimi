use std::{
    future::Future,
    sync::atomic::Ordering,
};

use adw::prelude::*;
use anyhow::Result;
use glib::Object;
use gtk::{
    SignalListItemFactory,
    gio,
    glib,
    subclass::prelude::*,
};
use imp::{
    ListType,
    ViewType,
};

use super::{
    filter_panel::{
        FilterPanelDialog,
        FiltersList,
    },
    hortu_scrolled::UnifySize,
    tu_list_item::imp::PosterType,
    tu_overview_item::imp::ViewGroup,
    utils::{
        GlobalToast,
        TuItemBuildExt,
    },
};
use crate::{
    client::{
        error::UserFacingError,
        structs::{
            List,
            SimpleListItem,
        },
    },
    tv::set_tv_focused,
    ui::provider::tu_item::PreferPoster,
    utils::{
        spawn,
        spawn_tokio,
    },
};

pub mod imp {

    use std::{
        cell::{
            Cell,
            RefCell,
        },
        fmt::Display,
        sync::{
            Arc,
            OnceLock,
            atomic::AtomicBool,
        },
    };

    use glib::subclass::{
        InitializingObject,
        Signal,
    };
    use gtk::{
        CompositeTemplate,
        glib,
        prelude::*,
        subclass::prelude::*,
    };
    use std::cell::OnceCell;

    use crate::ui::{
        models::SETTINGS,
        widgets::{
            filter_panel::FilterPanelDialog,
            tu_list_item::imp::PosterType,
            tuview_scrolled::TuViewScrolled,
        },
    };

    #[derive(Default, Hash, Eq, PartialEq, Clone, Copy, glib::Enum, Debug)]
    #[repr(u32)]
    #[enum_type(name = "ListType")]
    pub enum ListType {
        All,
        Resume,
        NextUp,
        BoxSet,
        Tags,
        Genres,
        Liked,
        Folder,
        #[default]
        None,
    }

    #[derive(Default, Hash, Eq, PartialEq, Clone, Copy, glib::Enum, Debug)]
    #[repr(u32)]
    #[enum_type(name = "SortOrder")]
    pub enum SortOrder {
        Ascending = 0,
        #[default]
        Descending = 1,
    }

    impl From<SortOrder> for u32 {
        fn from(val: SortOrder) -> Self {
            val as u32
        }
    }

    impl From<SortOrder> for i32 {
        fn from(val: SortOrder) -> Self {
            val as i32
        }
    }

    impl From<i32> for SortOrder {
        fn from(value: i32) -> Self {
            match value {
                0 => SortOrder::Ascending,
                _ => SortOrder::Descending,
            }
        }
    }

    impl From<u32> for SortOrder {
        fn from(value: u32) -> Self {
            Self::from(value as i32)
        }
    }

    impl Display for SortOrder {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                SortOrder::Ascending => f.write_str("Ascending"),
                SortOrder::Descending => f.write_str("Descending"),
            }
        }
    }

    #[derive(Default, Hash, Eq, PartialEq, Clone, Copy, glib::Enum, Debug)]
    #[repr(u32)]
    #[enum_type(name = "ViewType")]
    pub enum ViewType {
        ListView,
        #[default]
        GridView,
    }

    #[derive(Default, Hash, Eq, PartialEq, Clone, Copy, glib::Enum, Debug)]
    #[repr(u32)]
    #[enum_type(name = "SortBy")]
    pub enum SortBy {
        Title = 0,
        Bitrate = 1,
        DateCreated = 2,
        ImdbRate = 3,
        CriticRating = 4,
        #[default]
        PremiereDate = 5,
        OfficialRating = 6,
        ProductionYear = 7,
        DatePlayed = 8,
        Runtime = 9,
        UpdatedAt = 10,
    }

    impl From<SortBy> for u32 {
        fn from(val: SortBy) -> Self {
            val as u32
        }
    }

    impl From<SortBy> for i32 {
        fn from(val: SortBy) -> Self {
            val as i32
        }
    }

    impl From<i32> for SortBy {
        fn from(value: i32) -> Self {
            match value {
                0 => SortBy::Title,
                1 => SortBy::Bitrate,
                2 => SortBy::DateCreated,
                3 => SortBy::ImdbRate,
                4 => SortBy::CriticRating,
                5 => SortBy::PremiereDate,
                6 => SortBy::OfficialRating,
                7 => SortBy::ProductionYear,
                8 => SortBy::DatePlayed,
                9 => SortBy::Runtime,
                _ => SortBy::UpdatedAt,
            }
        }
    }

    impl From<u32> for SortBy {
        fn from(value: u32) -> Self {
            Self::from(value as i32)
        }
    }

    impl Display for SortBy {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                SortBy::Title => f.write_str("SortName"),
                SortBy::Bitrate => f.write_str("TotalBitrate,SortName"),
                SortBy::DateCreated => f.write_str("DateCreated,SortName"),
                SortBy::ImdbRate => f.write_str("CommunityRating,SortName"),
                SortBy::CriticRating => f.write_str("CriticRating,SortName"),
                SortBy::PremiereDate => f.write_str("ProductionYear,PremiereDate,SortName"),
                SortBy::OfficialRating => f.write_str("OfficialRating,SortName"),
                SortBy::ProductionYear => f.write_str("ProductionYear,SortName"),
                SortBy::DatePlayed => f.write_str("DatePlayed,SortName"),
                SortBy::Runtime => f.write_str("Runtime,SortName"),
                SortBy::UpdatedAt => f.write_str("DateLastContentAdded,SortName"),
            }
        }
    }

    // Object holding the state
    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/single_grid.ui")]
    #[properties(wrapper_type = super::SingleGrid)]
    pub struct SingleGrid {
        #[template_child]
        pub count: TemplateChild<gtk::Label>,
        #[template_child]
        pub postmenu: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub dropdown: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub adgroup: TemplateChild<adw::ToggleGroup>,
        #[template_child]
        pub glgroup: TemplateChild<adw::ToggleGroup>,
        #[template_child]
        pub filter: TemplateChild<gtk::Button>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub scrolled: TemplateChild<TuViewScrolled>,

        #[property(get, set = Self::set_list_type, builder(ListType::default()))]
        pub list_type: Cell<ListType>,
        #[property(get, set = Self::set_view_type, builder(ViewType::default()))]
        pub view_type: Cell<ViewType>,

        pub popovermenu: RefCell<Option<gtk::PopoverMenu>>,
        #[property(get, set = Self::set_sort_order, builder(SortOrder::default()))]
        pub sort_order: Cell<SortOrder>,
        #[property(get, set = Self::set_sort_by, builder(SortBy::default()))]
        pub sort_by: Cell<SortBy>,
        pub total_item_count: Cell<Option<u32>>,
        pub lock: Arc<AtomicBool>,

        pub filter_panel: OnceCell<FilterPanelDialog>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SingleGrid {
        const NAME: &'static str = "SingleGrid";
        type Type = super::SingleGrid;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            TuViewScrolled::ensure_type();
            klass.bind_template();
            klass.bind_template_instance_callbacks();
            klass.install_action_async("poster", None, |window, _action, _parameter| async move {
                window.poster(PosterType::Poster).await;
            });
            klass.install_action_async(
                "backdrop",
                None,
                |window, _action, _parameter| async move {
                    window.poster(PosterType::Backdrop).await;
                },
            );
            klass.install_action_async("banner", None, |window, _action, _parameter| async move {
                window.poster(PosterType::Banner).await;
            });
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for SingleGrid {
        fn constructed(&self) {
            self.set_sort_by_and_order(
                SortBy::from(SETTINGS.list_sort_by()),
                SortOrder::from(SETTINGS.list_sort_order()),
            );

            self.obj()
                .bind_property("sort-by", &self.dropdown.get(), "selected")
                .flags(glib::BindingFlags::BIDIRECTIONAL | glib::BindingFlags::SYNC_CREATE)
                .build();

            self.obj()
                .bind_property("sort-order", &self.adgroup.get(), "active-name")
                .flags(glib::BindingFlags::BIDIRECTIONAL | glib::BindingFlags::SYNC_CREATE)
                .transform_from(|_, value: &str| {
                    if value == "asce" {
                        Some(SortOrder::Ascending)
                    } else {
                        Some(SortOrder::Descending)
                    }
                })
                .transform_to(|_, value: SortOrder| {
                    if value == SortOrder::Ascending {
                        Some("asce")
                    } else {
                        Some("desc")
                    }
                })
                .build();

            self.parent_constructed();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| vec![Signal::builder("sort-changed").build()])
        }
    }

    impl WidgetImpl for SingleGrid {
        fn realize(&self) {
            self.parent_realize();
            self.obj().emit_by_name::<()>("sort-changed", &[]);
        }
    }

    impl WindowImpl for SingleGrid {}

    impl ApplicationWindowImpl for SingleGrid {}

    impl adw::subclass::navigation_page::NavigationPageImpl for SingleGrid {}

    impl SingleGrid {
        fn set_sort_by(&self, sort_by: SortBy) {
            self.sort_by.set(sort_by);
            let _ = SETTINGS.set_list_sort_by(sort_by.into());
            self.obj().emit_by_name::<()>("sort-changed", &[]);
        }

        fn set_sort_order(&self, sort_order: SortOrder) {
            self.sort_order.set(sort_order);
            let _ = SETTINGS.set_list_sort_order(sort_order.into());
            self.obj().emit_by_name::<()>("sort-changed", &[]);
        }

        pub fn set_sort_by_and_order(&self, sort_by: SortBy, sort_order: SortOrder) {
            self.sort_by.set(sort_by);
            self.sort_order.set(sort_order);
        }

        fn set_view_type(&self, view_type: ViewType) {
            self.view_type.set(view_type);
            self.scrolled.set_view_type(view_type);
        }

        pub fn set_list_type(&self, list_type: ListType) {
            self.list_type.set(list_type);
            self.obj().handle_type();
        }
    }
}

glib::wrapper! {
    pub struct SingleGrid(ObjectSubclass<imp::SingleGrid>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget ,adw::NavigationPage,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl Default for SingleGrid {
    fn default() -> Self {
        Self::new()
    }
}

#[gtk::template_callbacks]
impl SingleGrid {
    pub fn new() -> Self {
        Object::new()
    }

    pub fn tuview_scrolled(&self) -> super::tuview_scrolled::TuViewScrolled {
        self.imp().scrolled.get()
    }

    #[template_callback]
    fn on_view_changed_cb(&self, _param: Option<glib::ParamSpec>, group: &adw::ToggleGroup) {
        let view_type = if group.active_name() == Some(glib::GString::from("grid")) {
            ViewType::GridView
        } else {
            ViewType::ListView
        };
        self.set_view_type(view_type);
    }

    #[template_callback]
    fn filter_panel_cb(&self, _btn: &gtk::Button) {
        let panel = self.imp().filter_panel.get_or_init(|| {
            let dialog = FilterPanelDialog::new();
            dialog.connect_applied(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                #[weak]
                dialog,
                move |_| {
                    dialog.close();
                    obj.emit_by_name::<()>("sort-changed", &[]);
                }
            ));
            dialog
        });
        panel.present(Some(self));
    }

    pub fn handle_type(&self) {
        let imp = self.imp();
        match self.list_type() {
            ListType::All => {
                imp.postmenu.set_visible(true);
            }
            ListType::Resume | ListType::NextUp => {
                imp.postmenu.set_visible(false);
                imp.dropdown.set_visible(false);
                imp.adgroup.set_visible(false);
                imp.glgroup.set_visible(false);
                imp.filter.set_visible(false);
            }
            ListType::BoxSet => {
                imp.postmenu.set_visible(false);
                imp.filter.set_visible(false);
            }
            ListType::Tags => {
                imp.postmenu.set_visible(false);
                imp.dropdown.set_visible(false);
                imp.adgroup.set_visible(false);
                imp.glgroup.set_visible(false);
                imp.filter.set_visible(false);
            }
            ListType::Genres => {
                imp.postmenu.set_visible(false);
                imp.dropdown.set_visible(false);
                imp.adgroup.set_visible(false);
                imp.glgroup.set_visible(false);
                imp.filter.set_visible(false);
            }
            ListType::Liked => {
                imp.postmenu.set_visible(false);
            }
            ListType::Folder => {
                imp.postmenu.set_visible(false);
            }
            ListType::None => {
                imp.postmenu.set_visible(false);
            }
        }
    }

    pub async fn poster(&self, poster_type: PosterType) {
        let scrolled = self.imp().scrolled.get();
        let factory = SignalListItemFactory::new();
        match self.view_type() {
            ViewType::GridView => {
                scrolled
                    .imp()
                    .grid
                    .set_factory(Some(factory.tu_item(poster_type)));
            }
            ViewType::ListView => {
                scrolled
                    .imp()
                    .list
                    .set_factory(Some(factory.tu_overview_item(ViewGroup::ListView)));
            }
        };
    }

    pub fn add_items<const C: bool>(&self, items: Vec<SimpleListItem>) {
        let imp = self.imp();
        let scrolled = imp.scrolled.get();
        scrolled.set_store::<C>(items);
        if scrolled.n_items() == 0 {
            imp.stack.set_visible_child_name("fallback");
        } else {
            imp.stack.set_visible_child_name("result");
        }
    }

    pub fn set_item_number(&self, n: u32) {
        self.update_total_item_count(n);
        self.imp()
            .count
            .set_text(&format!("{} {}", n, gettextrs::gettext("Items")));
    }

    fn update_total_item_count(&self, n: u32) {
        self.imp().total_item_count.set(Some(n));
    }

    fn has_loaded_all_items(&self, n_items: u32) -> bool {
        self.imp()
            .total_item_count
            .get()
            .is_some_and(|total_item_count| n_items >= total_item_count)
    }

    pub fn set_unify_size(&self, unify_size: UnifySize) {
        self.imp().scrolled.get().set_unify_size(unify_size);
    }

    pub fn set_prefer_poster(&self, prefer_poster: PreferPoster) {
        self.imp().scrolled.get().set_prefer_poster(prefer_poster);
    }

    pub fn set_is_resume(&self, is_resume: bool) {
        self.imp().scrolled.get().set_is_resume(is_resume);
    }

    pub fn connect_sort_changed<F>(&self, f: F)
    where
        F: Fn(&Self) + 'static,
    {
        self.connect_closure(
            "sort-changed",
            true,
            glib::closure_local!(move |obj: Self| {
                f(&obj);
            }),
        );
    }

    pub fn connect_sort_changed_tokio<F, Fut>(&self, f: F)
    where
        F: Fn(String, String, FiltersList) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<List>> + Send + 'static,
    {
        self.connect_sort_changed(move |obj| {
            let sort_by = obj.sort_by().to_string();
            let sort_order = obj.sort_order().to_string();
            let filters_list = obj
                .imp()
                .filter_panel
                .get()
                .map(|f| f.filters_list())
                .unwrap_or_default();
            if !filters_list.is_empty() {
                obj.imp().filter.add_css_class("accent");
            } else {
                obj.imp().filter.remove_css_class("accent");
            }
            let future = f(sort_by.to_owned(), sort_order.to_owned(), filters_list);
            spawn(glib::clone!(
                #[weak(rename_to = obj)]
                obj,
                async move {
                    obj.imp().stack.set_visible_child_name("loading");
                    match spawn_tokio(future).await {
                        Ok(item) => {
                            let total_record_count = item.total_record_count;
                            obj.add_items::<true>(item.items);
                            let item_number =
                                if matches!(obj.list_type(), ListType::Resume | ListType::NextUp) {
                                    // Scroll loading is disabled for Resume and NextUp, so we use the number of items instead of total_record_count
                                    obj.imp().scrolled.get().n_items()
                                } else {
                                    total_record_count
                                };
                            obj.set_item_number(item_number);
                        }
                        Err(e) => {
                            obj.toast(e.to_user_facing());
                        }
                    }
                }
            ));
        });
    }

    pub fn connect_end_edge_overshot_tokio<F, Fut>(&self, f: F)
    where
        F: Fn(String, String, u32, FiltersList) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<List>> + Send + 'static,
    {
        self.imp().scrolled.connect_end_edge_reached(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            move |scrolled, lock| {
                let sort_by = obj.sort_by().to_string();
                let sort_order = obj.sort_order().to_string();
                let n_items = scrolled.n_items();
                if obj.has_loaded_all_items(n_items) {
                    lock.store(false, Ordering::Relaxed);
                    return;
                }
                let filters_list = obj
                    .imp()
                    .filter_panel
                    .get()
                    .map(|f| f.filters_list())
                    .unwrap_or_default();
                let future = f(
                    sort_by.to_owned(),
                    sort_order.to_owned(),
                    n_items,
                    filters_list,
                );
                spawn(glib::clone!(
                    #[weak]
                    obj,
                    #[weak]
                    scrolled,
                    async move {
                        scrolled.reveal_spinner(true);

                        match spawn_tokio(future).await {
                            Ok(item) => {
                                obj.update_total_item_count(item.total_record_count);
                                obj.add_items::<false>(item.items);
                            }
                            Err(e) => {
                                obj.toast(e.to_user_facing());
                            }
                        }

                        scrolled.reveal_spinner(false);

                        lock.store(false, Ordering::Relaxed);
                    }
                ));
            }
        ));
    }

    pub fn toolbar_widget_count(&self) -> usize {
        7
    }

    pub fn clear_toolbar_focus(&self) {
        let imp = self.imp();
        set_tv_focused(&imp.postmenu.get(), false);
        set_tv_focused(&imp.filter.get(), false);
        set_tv_focused(&imp.glgroup.get(), false);
        set_tv_focused(&imp.adgroup.get(), false);
        set_tv_focused(&imp.dropdown.get(), false);
    }

    pub fn focus_toolbar_index(&self, index: usize) {
        self.clear_toolbar_focus();
        let imp = self.imp();
        match index {
            0 => {
                let widget = imp.postmenu.get();
                set_tv_focused(&widget, true);
                widget.grab_focus();
            }
            1 => {
                let widget = imp.filter.get();
                set_tv_focused(&widget, true);
                widget.grab_focus();
            }
            2 | 3 => {
                let widget = imp.glgroup.get();
                set_tv_focused(&widget, true);
                widget.grab_focus();
            }
            4 | 5 => {
                let widget = imp.adgroup.get();
                set_tv_focused(&widget, true);
                widget.grab_focus();
            }
            6 => {
                let widget = imp.dropdown.get();
                set_tv_focused(&widget, true);
                widget.grab_focus();
            }
            _ => {}
        }
    }

    pub fn activate_toolbar_index(&self, index: usize) -> bool {
        let imp = self.imp();
        match index {
            0 => {
                imp.postmenu.get().popup();
                true
            }
            1 => {
                imp.filter.get().emit_clicked();
                true
            }
            2 => {
                imp.glgroup.get().set_active_name(Some("grid"));
                true
            }
            3 => {
                imp.glgroup.get().set_active_name(Some("list"));
                true
            }
            4 => {
                imp.adgroup.get().set_active_name(Some("asce"));
                true
            }
            5 => {
                imp.adgroup.get().set_active_name(Some("desc"));
                true
            }
            6 => {
                gtk::prelude::WidgetExt::activate(&imp.dropdown.get());
                true
            }
            _ => false,
        }
    }
}
