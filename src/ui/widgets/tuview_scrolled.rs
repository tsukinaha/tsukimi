use std::sync::{
    Arc,
    atomic::{
        AtomicBool,
        Ordering,
    },
};

use adw::{
    prelude::*,
    subclass::prelude::*,
};
use gtk::{
    CompositeTemplate,
    SignalListItemFactory,
    gio,
    glib::{
        self,
    },
    template_callbacks,
};

use super::{
    hortu_scrolled::{
        UnifySize,
        resolve_prefer_size,
    },
    single_grid::imp::ViewType,
    tu_list_item::imp::PosterType,
    tu_overview_item::imp::ViewGroup,
    utils::TuItemBuildExt,
};
use crate::{
    client::structs::SimpleListItem,
    ui::provider::{
        tu_item::{
            PreferPoster,
            PreferSize,
            TuItem,
        },
        tu_object::TuObject,
    },
};

// #region agent log
fn agent_log(hypothesis_id: &str, location: &str, message: &str, data: serde_json::Value) {
    use std::io::Write;
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let line = serde_json::json!({
        "sessionId": "ef5d72",
        "hypothesisId": hypothesis_id,
        "location": location,
        "message": message,
        "data": data,
        "timestamp": ts,
            "runId": "post-fix"
    });
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/var/mnt/SSD/Atlas Commons/technitiumdns-api/.cursor/debug-ef5d72.log")
    {
        let _ = writeln!(f, "{line}");
    }
}
// #endregion

pub(crate) mod imp {

    use std::sync::{
        Arc,
        atomic::AtomicBool,
    };

    use std::{
        cell::{
            Cell,
            RefCell,
        },
        collections::HashMap,
    };

    use glib::subclass::InitializingObject;
    use gtk::glib::Properties;

    use super::*;
    use crate::ui::provider::tu_object::TuObject;

    type LoadNearEndCb = std::rc::Rc<dyn Fn(&super::TuViewScrolled)>;

    pub struct SelectionWrap(pub gtk::SingleSelection);

    impl Default for SelectionWrap {
        fn default() -> Self {
            Self(gtk::SingleSelection::new(Some(gio::ListStore::new::<
                TuObject,
            >())))
        }
    }

    impl std::ops::Deref for SelectionWrap {
        type Target = gtk::SingleSelection;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    #[derive(CompositeTemplate, Default, Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/tuview_scrolled.ui")]
    #[properties(wrapper_type = super::TuViewScrolled)]
    pub struct TuViewScrolled {
        #[template_child]
        pub scrolled_window: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub grid: TemplateChild<gtk::GridView>,
        #[template_child]
        pub list: TemplateChild<gtk::ListView>,
        #[template_child]
        pub spinner_revealer: TemplateChild<gtk::Revealer>,

        pub selection: SelectionWrap,
        pub lock: Arc<AtomicBool>,

        #[property(get, set, builder(UnifySize::default()))]
        pub unify_size: RefCell<UnifySize>,
        #[property(get, set, builder(PreferPoster::default()))]
        pub prefer_poster: RefCell<PreferPoster>,
        #[property(get, set, default = false)]
        pub is_resume: Cell<bool>,
        pub prefer_size_cache: RefCell<PreferSize>,
        pub cached_columns: RefCell<i32>,
        pub measured_item_width: RefCell<Option<i32>>,
        pub bound_items: RefCell<HashMap<u32, gtk::ListItem>>,
        pub last_pagination_at: Cell<u32>,
        pub load_near_end: RefCell<Option<LoadNearEndCb>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TuViewScrolled {
        const NAME: &'static str = "TuViewScrolled";
        type Type = super::TuViewScrolled;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for TuViewScrolled {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.set_view_type(ViewType::GridView);
            let weak = obj.downgrade();
            self.scrolled_window
                .connect_notify_local(Some("width"), move |_, _| {
                    if let Some(obj) = weak.upgrade() {
                        obj.update_grid_columns();
                    }
                });
            let weak = obj.downgrade();
            obj.connect_map(move |_| {
                if let Some(obj) = weak.upgrade() {
                    obj.schedule_grid_layout_refresh();
                }
            });
            let weak = obj.downgrade();
            self.selection.connect_selected_notify(move |_| {
                if let Some(obj) = weak.upgrade() {
                    obj.refresh_poster_focus_state();
                    if crate::tv::cursor::suppress_pointer_hover() {
                        crate::ui::widgets::hover_scale::request_pointer_targeting_sync();
                    }
                }
            });
        }
    }

    impl WidgetImpl for TuViewScrolled {}
    impl BinImpl for TuViewScrolled {}
}

glib::wrapper! {
    pub struct TuViewScrolled(ObjectSubclass<imp::TuViewScrolled>)
        @extends gtk::Widget, adw::Bin, @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for TuViewScrolled {
    fn default() -> Self {
        Self::new()
    }
}

#[template_callbacks]
impl TuViewScrolled {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_store<const C: bool>(&self, items: Vec<SimpleListItem>) {
        let imp = self.imp();
        let Some(store) = imp.selection.model().and_downcast::<gio::ListStore>() else {
            return;
        };

        let prefer_size = if C {
            *imp.measured_item_width.borrow_mut() = None;
            let size = resolve_prefer_size(self.unify_size(), &items);
            self.imp().prefer_size_cache.replace(size);
            size
        } else {
            *self.imp().prefer_size_cache.borrow()
        };

        let prefer_poster = self.prefer_poster();
        let is_resume = self.is_resume();

        let items = items
            .into_iter()
            .map(|item| {
                let tu_item = TuItem::from_simple(item);
                tu_item.set_is_resume(is_resume);
                tu_item.set_prefer_poster(prefer_poster);
                tu_item.set_prefer_size(prefer_size);
                TuObject::new(tu_item)
            })
            .collect::<Vec<_>>();

        if C {
            store.splice(0, store.n_items(), &items);
            imp.last_pagination_at.set(0);
        } else {
            store.extend_from_slice(&items);
        }
        self.update_grid_columns();
        self.schedule_grid_layout_refresh();
        if crate::tv::cursor::suppress_pointer_hover() {
            crate::ui::widgets::hover_scale::request_pointer_targeting_sync();
        }
    }

    fn schedule_grid_layout_refresh(&self) {
        let obj = self.clone();
        glib::idle_add_local_once(move || obj.refresh_grid_layout(5));
    }

    fn refresh_grid_layout(&self, retries: u32) {
        *self.imp().measured_item_width.borrow_mut() = None;
        self.update_grid_columns();

        let width = self.imp().scrolled_window.width();
        let cols = *self.imp().cached_columns.borrow();
        if retries > 0 && width >= 100 && cols == 1 && width > self.default_item_width() * 2 {
            let obj = self.clone();
            glib::idle_add_local_once(move || obj.refresh_grid_layout(retries - 1));
        }
    }

    pub fn set_view_type(&self, view_type: ViewType) {
        let imp = self.imp();
        let factory = SignalListItemFactory::new();
        match view_type {
            ViewType::GridView => {
                imp.scrolled_window.set_child(Some(&imp.grid.get()));
                let grid_factory = factory.tu_item(PosterType::default());
                grid_factory.connect_bind(glib::clone!(
                    #[weak(rename_to = obj)]
                    self,
                    move |_factory, item| {
                        let Some(list_item) = item.downcast_ref::<gtk::ListItem>() else {
                            return;
                        };
                        let position = list_item.position();
                        obj.imp()
                            .bound_items
                            .borrow_mut()
                            .insert(position, list_item.clone());
                        let selected = obj.imp().selection.selected();
                        if let Some(child) = list_item
                            .child()
                            .and_downcast::<super::tu_list_item::TuListItem>()
                        {
                            child.set_poster_focused(
                                selected != gtk::INVALID_LIST_POSITION && position == selected,
                            );
                            if crate::tv::cursor::suppress_pointer_hover() {
                                crate::ui::widgets::hover_scale::request_pointer_targeting_sync();
                            }
                        }
                        Self::attach_grid_pointer_activate(&obj, list_item);
                        if let Some(child) = list_item.child() {
                            let width = child.width();
                            if width > 0 {
                                let viewport = obj.imp().scrolled_window.width().max(1);
                                // GridView stretches items in a single column — ignore that width.
                                if width >= viewport.saturating_sub(72) {
                                    return;
                                }
                                let imp = obj.imp();
                                let changed = imp
                                    .measured_item_width
                                    .borrow()
                                    .is_none_or(|current| current != width);
                                if changed {
                                    *imp.measured_item_width.borrow_mut() = Some(width);
                                    let obj = obj.clone();
                                    glib::idle_add_local_once(move || {
                                        obj.update_grid_columns();
                                    });
                                }
                            }
                        }
                    }
                ));
                grid_factory.connect_unbind(glib::clone!(
                    #[weak(rename_to = obj)]
                    self,
                    move |_factory, item| {
                        let Some(list_item) = item.downcast_ref::<gtk::ListItem>() else {
                            return;
                        };
                        obj.imp()
                            .bound_items
                            .borrow_mut()
                            .remove(&list_item.position());
                    }
                ));
                imp.grid.set_factory(Some(grid_factory));
                imp.grid.set_model(Some(&imp.selection.0));
            }
            ViewType::ListView => {
                imp.scrolled_window.set_child(Some(&imp.list.get()));
                imp.list
                    .set_factory(Some(factory.tu_overview_item(ViewGroup::ListView)));
                imp.list.set_model(Some(&imp.selection.0));
            }
        }
    }

    #[template_callback]
    fn on_gridview_item_activated(&self, position: u32, view: &gtk::GridView) {
        let Some(model) = view.model() else {
            return;
        };
        let Some(tu_obj) = model.item(position).and_downcast::<TuObject>() else {
            return;
        };
        tu_obj.activate(view);
    }

    #[template_callback]
    fn on_listview_item_activated(&self, position: u32, view: &gtk::ListView) {
        let Some(model) = view.model() else {
            return;
        };
        let Some(tu_obj) = model.item(position).and_downcast::<TuObject>() else {
            return;
        };
        tu_obj.activate(view);
    }

    pub fn connect_end_edge_reached<F>(&self, cb: F)
    where
        F: Fn(&Self, Arc<AtomicBool>) + 'static,
    {
        let cb = std::rc::Rc::new(cb);
        let try_load: std::rc::Rc<dyn Fn(&Self)> = std::rc::Rc::new({
            let cb = std::rc::Rc::clone(&cb);
            move |obj: &Self| {
                let is_running = Arc::clone(&obj.imp().lock);
                if is_running
                    .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                    .is_err()
                {
                    return;
                }
                cb(obj, is_running);
            }
        });
        *self.imp().load_near_end.borrow_mut() = Some(std::rc::Rc::clone(&try_load));

        let edge_load = std::rc::Rc::clone(&try_load);
        let weak = self.downgrade();
        self.imp()
            .scrolled_window
            .connect_edge_reached(move |_scrolled, pos| {
                if pos == gtk::PositionType::Bottom
                    && let Some(obj) = weak.upgrade()
                {
                    edge_load(&obj);
                }
            });

        let overshot_load = std::rc::Rc::clone(&try_load);
        let weak = self.downgrade();
        self.imp()
            .scrolled_window
            .connect_edge_overshot(move |_scrolled, pos| {
                if pos == gtk::PositionType::Bottom
                    && let Some(obj) = weak.upgrade()
                {
                    overshot_load(&obj);
                }
            });
    }

    fn maybe_load_near_end(&self) {
        let total = self.imp().selection.n_items();
        if total == 0 {
            return;
        }
        let selected = self.selected_index();
        let cols = self.grid_column_count().max(1) as u32;
        let last_row = total.saturating_sub(1) / cols;
        let selected_row = selected / cols;
        let near_end = selected_row >= last_row.saturating_sub(1);
        let blocked_by_pagination = total <= self.imp().last_pagination_at.get();
        // #region agent log
        if near_end {
            agent_log(
                "E",
                "tuview_scrolled.rs:maybe_load_near_end",
                "pagination check",
                serde_json::json!({
                    "selected": selected,
                    "selectedRow": selected_row,
                    "total": total,
                    "cols": cols,
                    "lastRow": last_row,
                    "nearEnd": near_end,
                    "blockedByPagination": blocked_by_pagination,
                    "lastPaginationAt": self.imp().last_pagination_at.get()
                }),
            );
        }
        // #endregion
        if selected_row < last_row.saturating_sub(1) {
            return;
        }
        if blocked_by_pagination {
            return;
        }
        self.imp().last_pagination_at.set(total);
        if let Some(load) = self.imp().load_near_end.borrow().as_ref() {
            // #region agent log
            agent_log(
                "E",
                "tuview_scrolled.rs:maybe_load_near_end",
                "triggering load_near_end",
                serde_json::json!({ "total": total, "selected": selected }),
            );
            // #endregion
            load(self);
        }
    }

    fn scroll_metrics(&self) -> serde_json::Value {
        let adj = self.imp().scrolled_window.vadjustment();
        let bound = self.imp().bound_items.borrow();
        serde_json::json!({
            "adjValue": adj.value(),
            "adjUpper": adj.upper(),
            "adjPage": adj.page_size(),
            "adjMax": (adj.upper() - adj.page_size()).max(0.0),
            "boundCount": bound.len(),
            "selectedBound": bound.contains_key(&self.selected_index())
        })
    }

    pub fn n_items(&self) -> u32 {
        let imp = self.imp();
        let Some(store) = imp.selection.model().and_downcast::<gio::ListStore>() else {
            return 0;
        };
        store.n_items()
    }

    pub fn reveal_spinner(&self, reveal: bool) {
        self.imp().spinner_revealer.set_reveal_child(reveal);
    }

    pub fn ensure_selection(&self) {
        let imp = self.imp();
        if imp.selection.n_items() == 0 {
            return;
        }
        if imp.selection.selected() == gtk::INVALID_LIST_POSITION {
            imp.selection.set_selected(0);
            self.refresh_poster_focus_state();
        }
    }

    pub fn clear_selection(&self) {
        self.imp()
            .selection
            .set_selected(gtk::INVALID_LIST_POSITION);
        self.clear_tv_focus();
    }

    pub fn clear_tv_focus(&self) {
        self.imp()
            .selection
            .set_selected(gtk::INVALID_LIST_POSITION);
    }

    fn default_item_width(&self) -> i32 {
        match *self.imp().prefer_size_cache.borrow() {
            crate::ui::provider::tu_item::PreferSize::Video => 280,
            crate::ui::provider::tu_item::PreferSize::Post => 185,
            crate::ui::provider::tu_item::PreferSize::Auto => 210,
        }
    }

    pub fn update_grid_columns(&self) {
        let imp = self.imp();
        let width = imp.scrolled_window.width().max(100);
        let item_width = self.default_item_width().max(1);
        let cols = ((width.saturating_sub(36)) / item_width).max(1);
        let prev = *imp.cached_columns.borrow();
        *imp.cached_columns.borrow_mut() = cols;
        imp.grid.set_max_columns(cols as u32);
        if prev != cols {
            imp.grid.queue_resize();
        }
    }

    pub fn selected_index(&self) -> u32 {
        let imp = self.imp();
        if imp.selection.selected() == gtk::INVALID_LIST_POSITION {
            0
        } else {
            imp.selection.selected()
        }
    }

    pub fn is_at_top_row(&self) -> bool {
        let imp = self.imp();
        let current = self.selected_index();
        let grid = imp.grid.get();
        let Some(current_item) = imp.bound_items.borrow().get(&current).cloned() else {
            return current == 0;
        };
        let Some((_, cur_y)) = Self::list_item_center(&current_item, &grid) else {
            return current == 0;
        };
        for (&pos, item) in imp.bound_items.borrow().iter() {
            if pos == current {
                continue;
            }
            let Some((_, y)) = Self::list_item_center(item, &grid) else {
                continue;
            };
            if cur_y - y > 8.0 {
                return false;
            }
        }
        true
    }

    pub fn move_selection(&self, delta: i32) {
        let imp = self.imp();
        let count = imp.selection.n_items() as i32;
        if count == 0 {
            return;
        }
        let current = self.selected_index() as i32;
        let next = (current + delta).clamp(0, count - 1) as u32;
        imp.selection.set_selected(next);
        self.scroll_to_selected(next, delta, 0);
    }

    pub fn grid_column_count(&self) -> i32 {
        self.update_grid_columns();
        if let Some(visible_cols) = self.infer_columns_from_visible_items() {
            return visible_cols.max(1);
        }
        (*self.imp().cached_columns.borrow()).max(1)
    }

    fn infer_columns_from_visible_items(&self) -> Option<i32> {
        let grid = self.imp().grid.get();
        let mut buckets = std::collections::BTreeSet::new();
        for item in self.imp().bound_items.borrow().values() {
            let Some((x, _)) = Self::list_item_center(item, &grid) else {
                continue;
            };
            buckets.insert((x / 48.0).round() as i32);
        }
        (!buckets.is_empty()).then_some(buckets.len() as i32)
    }

    fn list_item_center(list_item: &gtk::ListItem, grid: &gtk::GridView) -> Option<(f64, f64)> {
        let widget = list_item.child()?.upcast::<gtk::Widget>();
        let width = widget.width();
        let height = widget.height();
        if width <= 0 || height <= 0 {
            return None;
        }
        let point = gtk::graphene::Point::new(width as f32 / 2.0, height as f32 / 2.0);
        let translated = widget.compute_point(grid.upcast_ref::<gtk::Widget>(), &point)?;
        Some((f64::from(translated.x()), f64::from(translated.y())))
    }

    fn move_grid_selection_spatial(&self, row_delta: i32) -> bool {
        self.move_grid_selection_spatial_axis(row_delta, 0)
    }

    fn move_grid_selection_spatial_horizontal(&self, col_delta: i32) -> bool {
        self.move_grid_selection_spatial_axis(0, col_delta)
    }

    fn move_grid_selection_spatial_axis(&self, row_delta: i32, col_delta: i32) -> bool {
        let imp = self.imp();
        let current = self.selected_index();
        let grid = imp.grid.get();
        let Some(current_item) = imp.bound_items.borrow().get(&current).cloned() else {
            return false;
        };
        let Some((cur_x, cur_y)) = Self::list_item_center(&current_item, &grid) else {
            return false;
        };

        let mut best: Option<(u32, f64)> = None;
        for (&pos, item) in imp.bound_items.borrow().iter() {
            if pos == current {
                continue;
            }
            let Some((x, y)) = Self::list_item_center(item, &grid) else {
                continue;
            };
            if row_delta != 0 {
                let dy = y - cur_y;
                if row_delta > 0 && dy <= 8.0 {
                    continue;
                }
                if row_delta < 0 && dy >= -8.0 {
                    continue;
                }
                let score = dy.abs() * 10_000.0 + (x - cur_x).abs();
                if best.is_none_or(|(_, best_score)| score < best_score) {
                    best = Some((pos, score));
                }
            } else if col_delta != 0 {
                let dx = x - cur_x;
                if col_delta > 0 && dx <= 8.0 {
                    continue;
                }
                if col_delta < 0 && dx >= -8.0 {
                    continue;
                }
                let score = dx.abs() * 10_000.0 + (y - cur_y).abs();
                if best.is_none_or(|(_, best_score)| score < best_score) {
                    best = Some((pos, score));
                }
            }
        }

        let Some((next, _)) = best else {
            return false;
        };
        if row_delta > 0 {
            self.maybe_load_near_end();
        }
        imp.selection.set_selected(next);
        self.scroll_to_selected(next, row_delta, 0);
        self.refresh_poster_focus_state();
        true
    }

    fn move_grid_selection_by_index(&self, row_delta: i32, col_delta: i32) -> bool {
        let imp = self.imp();
        let count = imp.selection.n_items() as i32;
        if count == 0 {
            return false;
        }
        let cols = self.grid_column_count().max(1);
        let current = if imp.selection.selected() == gtk::INVALID_LIST_POSITION {
            0
        } else {
            imp.selection.selected() as i32
        };
        let row = current / cols;
        let col = current % cols;
        let max_row = (count - 1) / cols;
        let next_row = (row + row_delta).clamp(0, max_row);
        let next_col = if row_delta != 0 {
            col
        } else {
            (col + col_delta).clamp(0, cols - 1)
        };
        let mut next = next_row * cols + next_col;
        if next >= count {
            next = ((count - 1) / cols) * cols + next_col;
            if next >= count {
                next = count - 1;
            }
        }
        if next == current {
            return false;
        }
        if row_delta > 0 {
            self.maybe_load_near_end();
        }
        imp.selection.set_selected(next as u32);
        self.scroll_to_selected(next as u32, row_delta, col_delta);
        self.refresh_poster_focus_state();
        self.maybe_load_near_end();
        true
    }

    pub fn move_grid_selection(&self, row_delta: i32, col_delta: i32) {
        let imp = self.imp();
        let count = imp.selection.n_items() as i32;
        if count == 0 {
            return;
        }
        if row_delta != 0 && col_delta == 0 && self.move_grid_selection_by_index(row_delta, 0) {
            // #region agent log
            agent_log(
                "A",
                "tuview_scrolled.rs:move_grid_selection",
                "moved by index",
                serde_json::json!({
                    "rowDelta": row_delta,
                    "selected": self.selected_index(),
                    "cols": self.grid_column_count(),
                    "total": count,
                    "metrics": self.scroll_metrics()
                }),
            );
            // #endregion
            return;
        }
        if col_delta != 0 && row_delta == 0 && self.move_grid_selection_by_index(0, col_delta) {
            return;
        }
        if row_delta != 0 && col_delta == 0 && self.move_grid_selection_spatial(row_delta) {
            self.maybe_load_near_end();
            return;
        }
        if col_delta != 0
            && row_delta == 0
            && self.move_grid_selection_spatial_horizontal(col_delta)
        {
            return;
        }
        let cols = self.grid_column_count();
        let current = if imp.selection.selected() == gtk::INVALID_LIST_POSITION {
            0
        } else {
            imp.selection.selected() as i32
        };
        let row = current / cols;
        let col = current % cols;
        let max_row = (count - 1) / cols;
        let next_row = (row + row_delta).clamp(0, max_row);
        let next_col = if row_delta != 0 {
            col
        } else {
            (col + col_delta).clamp(0, cols - 1)
        };
        let mut next = next_row * cols + next_col;
        if next >= count {
            next = ((count - 1) / cols) * cols + next_col;
            if next >= count {
                next = count - 1;
            }
        }
        imp.selection.set_selected(next as u32);
        self.scroll_to_selected(next as u32, row_delta, col_delta);
        self.refresh_poster_focus_state();
        self.maybe_load_near_end();
    }

    fn refresh_poster_focus_state(&self) {
        let selected = self.imp().selection.selected();
        for item in self.imp().bound_items.borrow().values() {
            if let Some(child) = item
                .child()
                .and_downcast::<super::tu_list_item::TuListItem>()
            {
                child.set_poster_focused(
                    selected != gtk::INVALID_LIST_POSITION && item.position() == selected,
                );
            }
        }
    }

    fn attach_grid_pointer_activate(scrolled: &TuViewScrolled, list_item: &gtk::ListItem) {
        if !crate::tv::controller_navigation_enabled() {
            return;
        }
        let Some(child) = list_item.child() else {
            return;
        };
        if child.widget_name() == "tv-grid-pointer-bound" {
            return;
        }
        child.set_widget_name("tv-grid-pointer-bound");
        let gesture = gtk::GestureClick::new();
        gesture.set_button(1);
        let scrolled = scrolled.clone();
        let weak_item = list_item.downgrade();
        gesture.connect_released(move |gesture, _, _, _| {
            crate::tv::osk::mark_pointer_input();
            let Some(list_item) = weak_item.upgrade() else {
                return;
            };
            let position = list_item.position();
            gesture.set_state(gtk::EventSequenceState::Claimed);
            scrolled.imp().selection.set_selected(position);
            scrolled.refresh_poster_focus_state();
            if let Some(window) = scrolled.root().and_downcast::<crate::Window>() {
                scrolled.activate_selected(&window);
            }
        });
        child.add_controller(gesture);
    }

    pub fn activate_selected(&self, window: &crate::Window) {
        let imp = self.imp();
        let index = imp.selection.selected();
        if index == gtk::INVALID_LIST_POSITION {
            return;
        }
        if let Some(tu_obj) = imp
            .selection
            .item(index)
            .and_downcast::<crate::ui::provider::tu_object::TuObject>()
        {
            tu_obj.item().activate(window);
        }
    }

    fn scroll_to_selected(&self, index: u32, row_delta: i32, _col_delta: i32) {
        let imp = self.imp();
        let flags = gtk::ListScrollFlags::all();
        if imp
            .scrolled_window
            .child()
            .is_some_and(|child| child.is::<gtk::GridView>())
        {
            // #region agent log
            if row_delta != 0 {
                agent_log(
                    "C",
                    "tuview_scrolled.rs:scroll_to_selected",
                    "grid.scroll_to only",
                    serde_json::json!({
                        "index": index,
                        "rowDelta": row_delta,
                        "total": imp.selection.n_items(),
                        "metrics": self.scroll_metrics()
                    }),
                );
            }
            // #endregion
            imp.grid.scroll_to(index, flags, None);
            if row_delta > 0 {
                self.maybe_load_near_end();
            }
        } else {
            imp.list.scroll_to(index, flags, None);
        }
    }
}
