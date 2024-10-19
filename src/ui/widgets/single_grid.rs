use std::future::Future;

use super::tu_list_item::imp::PosterType;
use super::tu_overview_item::imp::ViewGroup;
use super::utils::TuItemBuildExt;
use crate::client::error::UserFacingError;
use crate::client::structs::{List, SimpleListItem};
use crate::ui::models::SETTINGS;
use crate::utils::{spawn, spawn_tokio};
use crate::{fraction, fraction_reset, toast};
use adw::prelude::*;
use anyhow::Result;
use glib::Object;
use gtk::subclass::prelude::*;
use gtk::{gio, glib, SignalListItemFactory};
use imp::{ListType, SortBy, SortOrder, ViewType};

pub mod imp {

    use std::cell::{Cell, RefCell};
    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, OnceLock};

    use glib::subclass::InitializingObject;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use gtk::{glib, CompositeTemplate};

    use crate::ui::models::SETTINGS;
    use crate::ui::widgets::tu_list_item::imp::PosterType;
    use crate::ui::widgets::tuview_scrolled::TuViewScrolled;

    use glib::subclass::Signal;

    #[derive(Default, Hash, Eq, PartialEq, Clone, Copy, glib::Enum, Debug)]
    #[repr(u32)]
    #[enum_type(name = "ListType")]

    pub enum ListType {
        All,
        Resume,
        BoxSet,
        Tags,
        Genres,
        Liked,
        #[default]
        None,
    }

    #[derive(Default, Hash, Eq, PartialEq, Clone, Copy, glib::Enum, Debug)]
    #[repr(u32)]
    #[enum_type(name = "SortOrder")]

    pub enum SortOrder {
        Ascending,
        #[default]
        Descending,
    }

    #[derive(Default, Hash, Eq, PartialEq, Clone, Copy, glib::Enum, Debug)]
    #[repr(u32)]
    #[enum_type(name = "ViewType")]

    pub enum ViewType {
        ListView,
        #[default]
        GridView,
    }

    impl From<i32> for SortOrder {
        fn from(value: i32) -> Self {
            match value {
                0 => Self::Descending,
                1 => Self::Ascending,
                _ => Self::Descending,
            }
        }
    }

    impl From<SortOrder> for i32 {
        fn from(value: SortOrder) -> Self {
            match value {
                SortOrder::Ascending => 1,
                SortOrder::Descending => 0,
            }
        }
    }

    #[derive(Default, Hash, Eq, PartialEq, Clone, Copy, glib::Enum, Debug)]
    #[repr(u32)]
    #[enum_type(name = "SortBy")]
    pub enum SortBy {
        Title,
        Bitrate,
        DateCreated,
        ImdbRate,
        CriticRating,
        #[default]
        PremiereDate,
        OfficialRating,
        ProductionYear,
        DatePlayed,
        Runtime,
        UpdatedAt,
    }

    impl From<i32> for SortBy {
        fn from(value: i32) -> Self {
            match value {
                0 => Self::Title,
                1 => Self::Bitrate,
                2 => Self::DateCreated,
                3 => Self::ImdbRate,
                4 => Self::CriticRating,
                5 => Self::PremiereDate,
                6 => Self::OfficialRating,
                7 => Self::ProductionYear,
                8 => Self::DatePlayed,
                9 => Self::Runtime,
                10 => Self::UpdatedAt,
                _ => Self::Title,
            }
        }
    }

    impl From<SortBy> for i32 {
        fn from(value: SortBy) -> Self {
            match value {
                SortBy::Title => 0,
                SortBy::Bitrate => 1,
                SortBy::DateCreated => 2,
                SortBy::ImdbRate => 3,
                SortBy::CriticRating => 4,
                SortBy::PremiereDate => 5,
                SortBy::OfficialRating => 6,
                SortBy::ProductionYear => 7,
                SortBy::DatePlayed => 8,
                SortBy::Runtime => 9,
                SortBy::UpdatedAt => 10,
            }
        }
    }

    // Object holding the state
    #[derive(CompositeTemplate, Default, glib::Properties)]
    #[template(resource = "/com/github/inaha/tsukimi/ui/single_grid.ui")]
    #[properties(wrapper_type = super::SingleGrid)]
    pub struct SingleGrid {
        #[template_child]
        pub count: TemplateChild<gtk::Label>,
        #[template_child]
        pub postmenu: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub dropdown: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub adbutton: TemplateChild<gtk::Box>,
        #[template_child]
        pub glbutton: TemplateChild<gtk::Box>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub scrolled: TemplateChild<TuViewScrolled>,

        #[template_child]
        pub ascending: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub descending: TemplateChild<gtk::ToggleButton>,

        #[property(get, set, builder(ListType::default()))]
        pub list_type: Cell<ListType>,
        #[property(get, set = Self::set_view_type, builder(ViewType::default()))]
        pub view_type: Cell<ViewType>,

        pub popovermenu: RefCell<Option<gtk::PopoverMenu>>,
        #[property(get, set = Self::set_sort_order, builder(SortOrder::default()))]
        pub sort_order: Cell<SortOrder>,
        #[property(get, set = Self::set_sort_by, builder(SortBy::default()))]
        pub sort_by: Cell<SortBy>,
        pub lock: Arc<AtomicBool>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for SingleGrid {
        // `NAME` needs to match `class` attribute of template
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
            self.ascending.set_active(SETTINGS.list_sort_order() == 1);
            self.dropdown.set_selected(SETTINGS.list_sort_by() as u32);
            self.parent_constructed();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| vec![Signal::builder("sort-changed").build()])
        }
    }

    impl WidgetImpl for SingleGrid {}

    impl WindowImpl for SingleGrid {}

    impl ApplicationWindowImpl for SingleGrid {}

    impl adw::subclass::navigation_page::NavigationPageImpl for SingleGrid {}

    impl SingleGrid {
        fn set_sort_by(&self, sort_by: SortBy) {
            self.sort_by.set(sort_by);
            self.obj().emit_by_name::<()>("sort-changed", &[]);
        }

        fn set_sort_order(&self, sort_order: SortOrder) {
            self.sort_order.set(sort_order);
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

    #[template_callback]
    fn sort_order_toggled_cb(&self, btn: &gtk::ToggleButton) {
        let sort_order = if btn.is_active() {
            SortOrder::Ascending
        } else {
            SortOrder::Descending
        };
        self.set_sort_order(sort_order);
        let _ = SETTINGS.set_list_sord_order(sort_order.into());
    }

    #[template_callback]
    fn view_toggled_cb(&self, btn: &gtk::ToggleButton) {
        let view_type = if btn.is_active() {
            ViewType::GridView
        } else {
            ViewType::ListView
        };
        self.set_view_type(view_type);
    }

    #[template_callback]
    fn on_sort_by_selected(&self, _param: Option<glib::ParamSpec>, dropdown: gtk::DropDown) {
        self.set_sort_by(SortBy::from(dropdown.selected() as i32));
        let _ = SETTINGS.set_list_sort_by(dropdown.selected() as i32);
    }

    #[template_callback]
    fn filter_panel_cb(&self, _btn: &gtk::Button) {
        let dialog = adw::Dialog::builder()
            .title("Filter")
            .presentation_mode(adw::DialogPresentationMode::BottomSheet)
            .build();
        dialog.present(Some(self));
    }

    pub fn handle_type(&self) {
        let imp = self.imp();
        match self.list_type() {
            ListType::All => {
                imp.postmenu.set_visible(true);
            }
            ListType::Resume => {
                imp.postmenu.set_visible(false);
                imp.dropdown.set_visible(false);
                imp.adbutton.set_visible(false);
                imp.glbutton.set_visible(false);
            }
            ListType::BoxSet => {
                imp.postmenu.set_visible(false);
            }
            ListType::Tags => {
                imp.postmenu.set_visible(false);
                imp.dropdown.set_visible(false);
                imp.adbutton.set_visible(false);
                imp.glbutton.set_visible(false);
            }
            ListType::Genres => {
                imp.postmenu.set_visible(false);
                imp.dropdown.set_visible(false);
                imp.adbutton.set_visible(false);
                imp.glbutton.set_visible(false);
            }
            ListType::Liked => {
                imp.postmenu.set_visible(false);
            }
            ListType::None => {
                imp.postmenu.set_visible(false);
            }
        }
    }

    pub fn match_sort_by(&self, selected: u32) -> &str {
        match selected {
            0 => "SortName",
            1 => "TotalBitrate,SortName",
            2 => "DateCreated,SortName",
            3 => "CommunityRating,SortName",
            4 => "CriticRating,SortName",
            5 => "ProductionYear,PremiereDate,SortName",
            6 => "OfficialRating,SortName",
            7 => "ProductionYear,SortName",
            8 => "DatePlayed,SortName",
            9 => "Runtime,SortName",
            10 => "DateLastContentAdded,SortName",
            _ => "SortName",
        }
    }

    pub fn match_sort_order(&self, selected: u32) -> &str {
        match selected {
            0 => "Descending",
            1 => "Ascending",
            _ => "Descending",
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

    pub fn add_items<const C: bool>(&self, items: Vec<SimpleListItem>, is_resume: bool) {
        let imp = self.imp();
        let scrolled = imp.scrolled.get();
        scrolled.set_store::<C>(items, is_resume);
        if scrolled.n_items() == 0 {
            imp.stack.set_visible_child_name("fallback");
        } else {
            imp.stack.set_visible_child_name("result");
        }
    }

    pub fn set_item_number(&self, n: u32) {
        self.imp().count.set_text(&format!("{} Items", n));
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

    pub fn connect_sort_changed_tokio<F, Fut>(&self, is_resume: bool, f: F)
    where
        F: Fn(String, String) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<List>> + Send + 'static,
    {
        self.connect_sort_changed(move |obj| {
            let sort_by = obj
                .match_sort_by(i32::from(obj.sort_by()) as u32)
                .to_string();
            let sort_order = obj
                .match_sort_order(i32::from(obj.sort_order()) as u32)
                .to_string();
            let future = f(sort_by.clone(), sort_order.clone());
            spawn(glib::clone!(
                #[weak(rename_to = obj)]
                obj,
                async move {
                    obj.imp().stack.set_visible_child_name("loading");
                    match spawn_tokio(future).await {
                        Ok(item) => {
                            obj.add_items::<true>(item.items, is_resume);
                            obj.imp()
                                .count
                                .set_text(&format!("{} Items", item.total_record_count));
                        }
                        Err(e) => {
                            toast!(obj, e.to_user_facing());
                        }
                    }
                }
            ));
        });
    }

    pub fn connect_end_edge_overshot_tokio<F, Fut>(&self, is_resume: bool, f: F)
    where
        F: Fn(String, String, u32) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<List>> + Send + 'static,
    {
        if is_resume {
            return;
        }

        self.imp().scrolled.connect_end_edge_reached(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            move |scrolled, lock| {
                let sort_by = obj
                    .match_sort_by(i32::from(obj.sort_by()) as u32)
                    .to_string();
                let sort_order = obj
                    .match_sort_order(i32::from(obj.sort_order()) as u32)
                    .to_string();
                let n_items = scrolled.n_items();

                let future = f(sort_by.clone(), sort_order.clone(), n_items);
                spawn(glib::clone!(
                    #[weak]
                    obj,
                    async move {
                        fraction_reset!(obj);
                        match spawn_tokio(future).await {
                            Ok(item) => obj.add_items::<false>(item.items, is_resume),
                            Err(e) => {
                                toast!(obj, e.to_user_facing());
                            }
                        }
                        lock.store(false, std::sync::atomic::Ordering::Relaxed);
                        fraction!(obj);
                    }
                ));
            }
        ));
    }
}
