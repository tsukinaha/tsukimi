use std::{
    any::Any,
    cell::{
        Cell,
        RefCell,
    },
    collections::{
        HashMap,
        HashSet,
    },
    ops::Range,
    rc::Rc,
    time::Duration,
};

use adw::{
    CallbackAnimationTarget,
    Easing,
    TimedAnimation,
    prelude::*,
};
use gtk::{
    Orientation,
    ScrolledWindow,
    Widget,
    glib,
    subclass::prelude::*,
};

mod animated_bin;
mod virtual_viewport;

use crate::ui::widgets::tu_list_item::TuListItem;

use animated_bin::AnimatedBin;
use virtual_viewport::VirtualViewport;

const DIFF_ANIMATION_QUEUE_DELAY: Duration = Duration::from_millis(320);
const INSERT_ANIMATION_DURATION_MS: u32 = 260;
const REMOVE_ANIMATION_DURATION_MS: u32 = 220;
const REORDER_ANIMATION_DURATION_MS: u32 = 260;
const INSERT_MOTION_OFFSET: f64 = 16.0;
const HORIZONTAL_MIN_CONTENT_HEIGHT: i32 = 128;
const VERTICAL_MIN_CONTENT_HEIGHT: i32 = 240;
const VIRTUAL_ROW_EXTENT: i32 = 84;
const VIRTUAL_COLUMN_EXTENT: i32 = 186;
const VIRTUAL_ROW_BUFFER: usize = 4;
const VIRTUAL_RECYCLE_LIMIT: usize = 64;

pub trait OnSameKey {
    fn on_same_key(&self, _widget: &Widget) {}
}

type KeyFactory = Rc<dyn Fn(&dyn Any) -> String>;
type WidgetFactory = Rc<dyn Fn(&dyn Any) -> Widget>;
type WidgetBinder = Rc<dyn Fn(&Widget, &dyn Any)>;

#[derive(Clone)]
struct RowData {
    key: String,
    item: Rc<dyn Any>,
}

#[derive(Clone, Copy, Debug, Default)]
struct VisibleAnchor {
    position: usize,
    before: usize,
    after: usize,
}

struct VirtualRow {
    item: RefCell<Rc<dyn Any>>,
    container: AnimatedBin,
    child: Widget,
    orientation: Cell<Orientation>,
    animations: RefCell<Vec<TimedAnimation>>,
    widget_binder: WidgetBinder,
}

mod imp {
    use crate::ui::widgets::fix::ScrolledWindowFixExt;

    use super::*;

    pub struct LazyDiffView {
        pub(super) items: RefCell<Vec<RowData>>,
        pub(super) rows: RefCell<HashMap<String, Rc<VirtualRow>>>,
        pub(super) removing_rows: RefCell<Vec<Rc<VirtualRow>>>,
        pub(super) recycled_rows: RefCell<Vec<Rc<VirtualRow>>>,
        pub(super) visible_anchor: Cell<VisibleAnchor>,
        pub(super) pending_items: RefCell<Option<Vec<RowData>>>,

        pub scroll: RefCell<Option<ScrolledWindow>>,
        pub viewport: RefCell<Option<VirtualViewport>>,

        pub is_animating: Cell<bool>,
        pub item_extent: Cell<i32>,
        pub item_cross_extent: Cell<i32>,
        pub spacing: Cell<i32>,
        pub item_extent_measured: Cell<bool>,
        pub allocated_width: Cell<i32>,
        pub allocated_height: Cell<i32>,
        pub orientation: Cell<Orientation>,
        pub key_factory: RefCell<Option<KeyFactory>>,
        pub widget_factory: RefCell<Option<WidgetFactory>>,
        pub widget_binder: RefCell<Option<WidgetBinder>>,
        pub same_key_binder: RefCell<Option<WidgetBinder>>,
    }

    impl Default for LazyDiffView {
        fn default() -> Self {
            Self {
                scroll: RefCell::new(None),
                viewport: RefCell::new(None),
                items: RefCell::new(Vec::new()),
                rows: RefCell::new(HashMap::new()),
                removing_rows: RefCell::new(Vec::new()),
                recycled_rows: RefCell::new(Vec::new()),
                visible_anchor: Cell::new(VisibleAnchor::default()),
                pending_items: RefCell::new(None),
                is_animating: Cell::new(false),
                item_extent: Cell::new(VIRTUAL_ROW_EXTENT),
                item_cross_extent: Cell::new(0),
                spacing: Cell::new(0),
                item_extent_measured: Cell::new(false),
                allocated_width: Cell::new(0),
                allocated_height: Cell::new(0),
                orientation: Cell::new(Orientation::Vertical),
                key_factory: RefCell::new(None),
                widget_factory: RefCell::new(None),
                widget_binder: RefCell::new(None),
                same_key_binder: RefCell::new(None),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LazyDiffView {
        const NAME: &'static str = "LazyDiffView";
        type Type = super::LazyDiffView;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for LazyDiffView {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            let viewport = VirtualViewport::new();
            viewport.set_hexpand(true);
            viewport.set_overflow(gtk::Overflow::Visible);
            viewport.set_size_request(0, 0);
            let scroll = ScrolledWindow::builder().child(&viewport).build();
            scroll.set_parent(&*obj);
            scroll.add_css_class("boxed-list");
            scroll.fix();
            *self.scroll.borrow_mut() = Some(scroll);
            *self.viewport.borrow_mut() = Some(viewport);
            obj.apply_orientation();

            let weak_obj = obj.downgrade();
            obj.scroll().hadjustment().connect_value_changed(move |_| {
                if let Some(obj) = weak_obj.upgrade() {
                    obj.sync_visible_rows(HashMap::new(), HashSet::new());
                }
            });

            let weak_obj = obj.downgrade();
            obj.scroll().vadjustment().connect_value_changed(move |_| {
                if let Some(obj) = weak_obj.upgrade() {
                    obj.sync_visible_rows(HashMap::new(), HashSet::new());
                }
            });
        }

        fn dispose(&self) {
            if let Some(scroll) = self.scroll.borrow_mut().take() {
                scroll.unparent();
            }
        }

        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: std::sync::OnceLock<Vec<glib::ParamSpec>> =
                std::sync::OnceLock::new();
            PROPERTIES.get_or_init(|| {
                vec![
                    glib::ParamSpecEnum::builder_with_default("orientation", Orientation::Vertical)
                        .build(),
                    glib::ParamSpecInt::builder("spacing")
                        .minimum(0)
                        .default_value(0)
                        .build(),
                ]
            })
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "orientation" => self.obj().set_orientation(value.get().unwrap()),
                "spacing" => self.obj().set_spacing(value.get().unwrap()),
                name => unreachable!("unknown property {name}"),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "orientation" => self.orientation.get().to_value(),
                "spacing" => self.spacing.get().to_value(),
                name => unreachable!("unknown property {name}"),
            }
        }
    }

    impl WidgetImpl for LazyDiffView {
        fn compute_expand(&self, hexpand: &mut bool, vexpand: &mut bool) {
            if let Some(scroll) = self.scroll.borrow().as_ref() {
                *hexpand = scroll.compute_expand(Orientation::Horizontal);
                *vexpand = scroll.compute_expand(Orientation::Vertical);
            }
        }

        fn measure(&self, orientation: Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            self.scroll
                .borrow()
                .as_ref()
                .map(|scroll| scroll.measure(orientation, for_size))
                .unwrap_or((0, 0, -1, -1))
        }

        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            let previous_cross_size = match self.orientation.get() {
                Orientation::Horizontal => self.allocated_height.get(),
                _ => self.allocated_width.get(),
            };
            self.allocated_width.set(width);
            self.allocated_height.set(height);
            if let Some(scroll) = self.scroll.borrow().as_ref() {
                scroll.allocate(width, height, baseline, None);
            }

            let cross_size = match self.orientation.get() {
                Orientation::Horizontal => height,
                _ => width,
            };
            if previous_cross_size != cross_size {
                let obj = self.obj();
                obj.reset_item_extent();
                obj.sync_visible_rows(HashMap::new(), HashSet::new());
            }
        }
    }
}

glib::wrapper! {
    pub struct LazyDiffView(ObjectSubclass<imp::LazyDiffView>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for LazyDiffView {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl LazyDiffView {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn scroll(&self) -> ScrolledWindow {
        self.imp()
            .scroll
            .borrow()
            .as_ref()
            .expect("LazyDiffView is not constructed")
            .clone()
    }

    pub fn with_factories<T: Clone + OnSameKey + 'static>(
        orientation: Orientation, key_factory: impl Fn(&T) -> String + 'static,
        widget_factory: impl Fn(&T) -> Widget + 'static,
        widget_binder: impl Fn(&Widget, &T) + 'static,
    ) -> Self {
        let view = Self::new();
        view.set_orientation(orientation);
        view.configure(key_factory, widget_factory, widget_binder);
        view
    }

    pub fn configure<T: Clone + OnSameKey + 'static>(
        &self, key_factory: impl Fn(&T) -> String + 'static,
        widget_factory: impl Fn(&T) -> Widget + 'static,
        widget_binder: impl Fn(&Widget, &T) + 'static,
    ) {
        let imp = self.imp();
        *imp.key_factory.borrow_mut() = Some(Rc::new(move |item| {
            key_factory(
                item.downcast_ref::<T>()
                    .expect("LazyDiffView item type mismatch"),
            )
        }));
        *imp.widget_factory.borrow_mut() = Some(Rc::new(move |item| {
            widget_factory(
                item.downcast_ref::<T>()
                    .expect("LazyDiffView item type mismatch"),
            )
        }));
        *imp.widget_binder.borrow_mut() = Some(Rc::new(move |widget, item| {
            widget_binder(
                widget,
                item.downcast_ref::<T>()
                    .expect("LazyDiffView item type mismatch"),
            )
        }));
        *imp.same_key_binder.borrow_mut() = Some(Rc::new(move |widget, item| {
            item.downcast_ref::<T>()
                .expect("LazyDiffView item type mismatch")
                .on_same_key(widget);
        }));
    }

    pub fn orientation(&self) -> Orientation {
        self.imp().orientation.get()
    }

    pub fn spacing(&self) -> i32 {
        self.imp().spacing.get()
    }

    pub fn set_spacing(&self, spacing: i32) {
        let spacing = spacing.max(0);
        let imp = self.imp();
        if imp.spacing.replace(spacing) == spacing {
            return;
        }

        self.resize_viewport();
        self.reposition_active_rows();
        self.sync_visible_rows(HashMap::new(), HashSet::new());
        self.notify("spacing");
    }

    pub fn set_orientation(&self, orientation: Orientation) {
        let imp = self.imp();
        if imp.orientation.replace(orientation) == orientation {
            return;
        }

        self.apply_orientation();
        self.reset_item_extent();
        for row in imp.rows.borrow().values() {
            row.set_orientation(orientation);
        }
        self.resize_viewport();
        self.sync_visible_rows(HashMap::new(), HashSet::new());
        self.notify("orientation");
    }

    pub fn len(&self) -> usize {
        self.imp().items.borrow().len()
    }

    pub fn key_at(&self, index: usize) -> Option<String> {
        self.imp()
            .items
            .borrow()
            .get(index)
            .map(|row| row.key.clone())
    }

    pub fn scroll_to_index(&self, index: usize) {
        let len = self.len();
        if len == 0 {
            return;
        }
        let index = index.min(len - 1);
        let adj = match self.orientation() {
            Orientation::Horizontal => self.scroll().hadjustment(),
            _ => self.scroll().vadjustment(),
        };
        let pitch = self.item_pitch() as f64;
        let target = index as f64 * pitch;
        let page = adj.page_size();
        let upper = (adj.upper() - page).max(0.0);
        let value = (target + pitch * 0.5 - page * 0.5).clamp(0.0, upper);
        adj.set_value(value);
    }

    pub fn row_widget_for_key(&self, key: &str) -> Option<gtk::Widget> {
        self.imp()
            .rows
            .borrow()
            .get(key)
            .map(|row| row.child.clone())
    }

    pub fn rebind_active_rows(&self) {
        let binder = self.imp().widget_binder.borrow().clone();
        let Some(binder) = binder else {
            return;
        };
        for row in self.imp().rows.borrow().values() {
            row.rebind(&binder);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn set_items<T: Clone + 'static>(&self, items: Vec<T>) {
        let row_data = self.row_data(items);
        if self.imp().is_animating.get() {
            *self.imp().pending_items.borrow_mut() = Some(row_data);
            return;
        }

        self.imp().is_animating.set(true);
        self.apply_items(row_data);

        let weak_self = self.downgrade();
        gtk::glib::timeout_add_local_once(DIFF_ANIMATION_QUEUE_DELAY, move || {
            let Some(view) = weak_self.upgrade() else {
                return;
            };

            view.finish_animation_batch();
            view.imp().is_animating.set(false);
            let pending_items = view.imp().pending_items.borrow_mut().take();
            if let Some(items) = pending_items {
                view.set_row_data(items);
            }
        });
    }

    pub fn insert_item<T: Clone + 'static>(&self, index: usize, item: T) {
        let imp = self.imp();
        let mut items = imp
            .items
            .borrow()
            .iter()
            .map(|row| row.item.clone())
            .collect::<Vec<_>>();
        items.insert(index.min(items.len()), Rc::new(item));
        self.set_erased_items(items);
    }

    pub fn remove_item(&self, index: usize) -> Option<Rc<dyn Any>> {
        let imp = self.imp();
        let mut items = imp
            .items
            .borrow()
            .iter()
            .map(|row| row.item.clone())
            .collect::<Vec<_>>();
        if index >= items.len() {
            return None;
        }

        let removed = items.remove(index);
        self.set_erased_items(items);
        Some(removed)
    }

    pub fn reverse_items(&self) {
        let imp = self.imp();
        let mut items = imp
            .items
            .borrow()
            .iter()
            .map(|row| row.item.clone())
            .collect::<Vec<_>>();
        items.reverse();
        self.set_erased_items(items);
    }

    fn set_row_data(&self, items: Vec<RowData>) {
        if self.imp().is_animating.get() {
            *self.imp().pending_items.borrow_mut() = Some(items);
            return;
        }

        self.imp().is_animating.set(true);
        self.apply_items(items);

        let weak_self = self.downgrade();
        gtk::glib::timeout_add_local_once(DIFF_ANIMATION_QUEUE_DELAY, move || {
            let Some(view) = weak_self.upgrade() else {
                return;
            };

            view.finish_animation_batch();
            view.imp().is_animating.set(false);
            let pending_items = view.imp().pending_items.borrow_mut().take();
            if let Some(items) = pending_items {
                view.set_row_data(items);
            }
        });
    }

    fn set_erased_items(&self, items: Vec<Rc<dyn Any>>) {
        let row_data = self.erased_row_data(items);
        self.set_row_data(row_data);
    }

    fn row_data<T: Clone + 'static>(&self, items: Vec<T>) -> Vec<RowData> {
        self.erased_row_data(
            items
                .into_iter()
                .map(|item| Rc::new(item) as Rc<dyn Any>)
                .collect(),
        )
    }

    fn erased_row_data(&self, items: Vec<Rc<dyn Any>>) -> Vec<RowData> {
        let key_factory = self
            .imp()
            .key_factory
            .borrow()
            .as_ref()
            .expect("LazyDiffView::configure must be called before setting items")
            .clone();
        let mut seen_keys = HashSet::with_capacity(items.len());

        items
            .into_iter()
            .map(|item| {
                let key = key_factory(item.as_ref());
                debug_assert!(
                    seen_keys.insert(key.clone()),
                    "LazyDiffView item keys must be unique within one update"
                );
                RowData { key, item }
            })
            .collect()
    }

    fn apply_items(&self, new_items: Vec<RowData>) {
        let imp = self.imp();
        let active_keys = imp.rows.borrow().keys().cloned().collect::<HashSet<_>>();
        let old_indices = active_indices(&imp.items.borrow(), &active_keys);
        let new_indices = active_indices(&new_items, &active_keys);

        self.animate_removed_rows(&active_keys, &old_indices, &new_indices);
        let reorder_offsets = self.reorder_offsets(&old_indices, &new_indices);
        let old_items = imp.items.replace(new_items);
        self.reset_item_extent();
        self.resize_viewport();

        let inserted_keys = self.visible_inserted_keys(&old_items);
        self.sync_visible_rows(reorder_offsets, inserted_keys);
    }

    fn animate_removed_rows(
        &self, active_keys: &HashSet<String>, old_indices: &HashMap<String, usize>,
        new_indices: &HashMap<String, usize>,
    ) {
        let imp = self.imp();
        let mut rows = imp.rows.borrow_mut();
        let mut removing_rows = imp.removing_rows.borrow_mut();

        for key in active_keys {
            if new_indices.contains_key(key) {
                continue;
            }

            if let Some(row) = rows.remove(key) {
                let old_index = old_indices.get(key).copied().unwrap_or_default();
                self.position_row(&row, old_index);
                row.animate_remove();
                removing_rows.push(row);
            }
        }
    }

    fn reorder_offsets(
        &self, old_indices: &HashMap<String, usize>, new_indices: &HashMap<String, usize>,
    ) -> HashMap<String, f64> {
        old_indices
            .iter()
            .filter_map(|(key, old_index)| {
                let new_index = new_indices.get(key)?;
                let offset = (*old_index as f64 - *new_index as f64) * self.item_pitch() as f64;
                (offset.abs() >= 0.5).then(|| (key.clone(), offset))
            })
            .collect()
    }

    fn visible_inserted_keys(&self, old_items: &[RowData]) -> HashSet<String> {
        let old_keys = old_items
            .iter()
            .map(|row| row.key.as_str())
            .collect::<HashSet<_>>();
        let visiable_range = self.visible_range();
        let items = self.imp().items.borrow();
        visiable_range
            .filter_map(|index| {
                let key = items.get(index)?.key.as_str();
                (!old_keys.contains(key)).then(|| key.to_string())
            })
            .collect()
    }

    fn finish_animation_batch(&self) {
        let imp = self.imp();
        let Some(viewport) = imp.viewport.borrow().clone() else {
            return;
        };

        for row in imp.removing_rows.borrow_mut().drain(..) {
            viewport.remove_child(row.container());
            row.reset_for_reuse();
            if imp.recycled_rows.borrow().len() < VIRTUAL_RECYCLE_LIMIT {
                imp.recycled_rows.borrow_mut().push(row);
            }
        }

        for row in imp.rows.borrow().values() {
            row.reset_animation_state();
        }

        self.sync_visible_rows(HashMap::new(), HashSet::new());
    }

    fn sync_visible_rows(
        &self, start_offsets: HashMap<String, f64>, inserted_keys: HashSet<String>,
    ) {
        let visible_range = self.visible_range();
        let visible_keys = {
            let items = self.imp().items.borrow();
            visible_range
                .clone()
                .map(|index| items[index].key.clone())
                .collect::<HashSet<_>>()
        };

        self.recycle_invisible_rows(&visible_keys);

        let mut extent_changed = false;
        for index in visible_range {
            let row_data = self.imp().items.borrow()[index].clone();
            let row = self.row_for_data(&row_data, index, &inserted_keys);
            extent_changed |= self.update_item_extent_for_row(&row);
            self.position_row(&row, index);

            if let Some(offset) = start_offsets.get(&row_data.key).copied() {
                row.animate_reorder_from(offset);
            }
        }

        self.resize_viewport();
        self.reposition_active_rows();
        if extent_changed {
            self.sync_visible_rows(HashMap::new(), HashSet::new());
        }
    }

    fn recycle_invisible_rows(&self, visible_keys: &HashSet<String>) {
        let imp = self.imp();
        let Some(viewport) = imp.viewport.borrow().clone() else {
            return;
        };
        let stale_keys = imp
            .rows
            .borrow()
            .keys()
            .filter(|key| !visible_keys.contains(*key))
            .cloned()
            .collect::<Vec<_>>();

        for key in stale_keys {
            if let Some(row) = imp.rows.borrow_mut().remove(&key) {
                viewport.remove_child(row.container());
                row.reset_for_reuse();
                if imp.recycled_rows.borrow().len() < VIRTUAL_RECYCLE_LIMIT {
                    imp.recycled_rows.borrow_mut().push(row);
                }
            }
        }
    }

    fn row_for_data(
        &self, row_data: &RowData, index: usize, inserted_keys: &HashSet<String>,
    ) -> Rc<VirtualRow> {
        let imp = self.imp();
        if let Some(row) = imp.rows.borrow().get(&row_data.key) {
            let item_changed = !Rc::ptr_eq(&row.item.borrow(), &row_data.item);
            *row.item.borrow_mut() = row_data.item.clone();
            if item_changed && let Some(cb) = imp.same_key_binder.borrow().as_ref() {
                cb(&row.child, row_data.item.as_ref());
            }
            return row.clone();
        }

        let row = imp
            .recycled_rows
            .borrow_mut()
            .pop()
            .inspect(|row| {
                row.reset_for_reuse();
                row.bind_item(row_data.item.clone());
            })
            .unwrap_or_else(|| {
                VirtualRow::new(
                    row_data.item.clone(),
                    self.orientation(),
                    imp.widget_factory
                        .borrow()
                        .as_ref()
                        .expect("LazyDiffView::configure must be called before rows become visible")
                        .clone(),
                    imp.widget_binder
                        .borrow()
                        .as_ref()
                        .expect("LazyDiffView::configure must be called before rows become visible")
                        .clone(),
                )
            });

        if let Some(viewport) = imp.viewport.borrow().as_ref() {
            viewport.add_child(
                row.container(),
                self.item_x(index),
                self.item_y(index),
                self.item_width(),
                self.item_height(),
            );
        }

        if inserted_keys.contains(&row_data.key) {
            row.animate_insert();
        } else {
            row.reset_animation_state();
        }

        imp.rows
            .borrow_mut()
            .insert(row_data.key.clone(), row.clone());
        row
    }

    fn apply_orientation(&self) {
        let orientation = self.orientation();
        let scroll = self.scroll();
        scroll.set_hscrollbar_policy(match orientation {
            Orientation::Horizontal => gtk::PolicyType::Automatic,
            _ => gtk::PolicyType::Never,
        });
        scroll.set_vscrollbar_policy(match orientation {
            Orientation::Horizontal => gtk::PolicyType::Never,
            _ => gtk::PolicyType::Automatic,
        });
        scroll.set_min_content_height(match orientation {
            Orientation::Horizontal => HORIZONTAL_MIN_CONTENT_HEIGHT,
            _ => VERTICAL_MIN_CONTENT_HEIGHT,
        });
    }

    fn resize_viewport(&self) {
        let imp = self.imp();
        let Some(viewport) = imp.viewport.borrow().clone() else {
            return;
        };
        let item_count = imp.items.borrow().len() as i32;
        let content_extent = self.content_main_extent(item_count);
        match self.orientation() {
            Orientation::Horizontal => {
                viewport.set_content_size(content_extent, self.cross_content_extent())
            }
            _ => viewport.set_content_size(0, content_extent),
        }
    }

    fn visible_range(&self) -> Range<usize> {
        let len = self.imp().items.borrow().len();
        if len == 0 {
            self.imp().visible_anchor.set(VisibleAnchor::default());
            return 0..0;
        }

        let adjustment = match self.orientation() {
            Orientation::Horizontal => self.scroll().hadjustment(),
            _ => self.scroll().vadjustment(),
        };
        let pitch = self.item_pitch() as f64;
        let page_size = adjustment.page_size().max(pitch);
        let center = adjustment.value() + page_size * 0.5;
        let position = ((center / pitch).floor() as usize).min(len - 1);
        let visible_count = (page_size / pitch).ceil() as usize + 1;
        let before = visible_count / 2 + VIRTUAL_ROW_BUFFER;
        let after = visible_count - visible_count / 2 + VIRTUAL_ROW_BUFFER;

        let anchor = VisibleAnchor {
            position,
            before,
            after,
        };
        self.imp().visible_anchor.set(anchor);
        anchor.range(len)
    }

    fn position_row(&self, row: &VirtualRow, index: usize) {
        if let Some(viewport) = self.imp().viewport.borrow().as_ref() {
            viewport.move_child(
                row.container(),
                self.item_x(index),
                self.item_y(index),
                self.item_width(),
                self.item_height(),
            );
        }
    }

    fn item_extent(&self) -> i32 {
        self.imp().item_extent.get()
    }

    fn item_pitch(&self) -> i32 {
        self.item_extent() + self.spacing()
    }

    fn content_main_extent(&self, item_count: i32) -> i32 {
        if item_count <= 0 {
            return 0;
        }

        item_count * self.item_extent() + (item_count - 1) * self.spacing()
    }

    fn fallback_item_extent(&self) -> i32 {
        match self.orientation() {
            Orientation::Horizontal => VIRTUAL_COLUMN_EXTENT,
            _ => VIRTUAL_ROW_EXTENT,
        }
    }

    fn reset_item_extent(&self) {
        self.imp().item_extent.set(self.fallback_item_extent());
        self.imp().item_cross_extent.set(match self.orientation() {
            Orientation::Horizontal => HORIZONTAL_MIN_CONTENT_HEIGHT,
            _ => 0,
        });
        self.imp().item_extent_measured.set(false);
    }

    fn update_item_extent_for_row(&self, row: &VirtualRow) -> bool {
        let mut changed = self.update_cross_extent_for_row(row);
        let measured_extent = row
            .measured_main_extent(self.orientation(), self.cross_axis_size())
            .max(1);
        let imp = self.imp();
        if !imp.item_extent_measured.replace(true) {
            changed |= imp.item_extent.replace(measured_extent) != measured_extent;
            return changed;
        }

        let current_extent = self.item_extent();
        if measured_extent > current_extent {
            imp.item_extent.set(measured_extent);
            return true;
        }

        changed
    }

    fn update_cross_extent_for_row(&self, row: &VirtualRow) -> bool {
        let cross_extent =
            row.measured_cross_extent(self.orientation())
                .max(match self.orientation() {
                    Orientation::Horizontal => HORIZONTAL_MIN_CONTENT_HEIGHT,
                    _ => 0,
                });
        let current_extent = self.imp().item_cross_extent.get();
        if cross_extent > current_extent {
            self.imp().item_cross_extent.set(cross_extent);
            return true;
        }
        false
    }

    fn cross_content_extent(&self) -> i32 {
        match self.orientation() {
            Orientation::Horizontal => self
                .imp()
                .item_cross_extent
                .get()
                .max(HORIZONTAL_MIN_CONTENT_HEIGHT),
            _ => 0,
        }
    }

    fn cross_axis_size(&self) -> i32 {
        let imp = self.imp();
        match self.orientation() {
            Orientation::Horizontal => imp.allocated_height.get().max(self.cross_content_extent()),
            _ => imp.allocated_width.get().max(1),
        }
    }

    fn reposition_active_rows(&self) {
        for (index, row_data) in self.imp().items.borrow().iter().enumerate() {
            let Some(row) = self.imp().rows.borrow().get(&row_data.key).cloned() else {
                continue;
            };
            self.position_row(&row, index);
        }
    }

    fn item_x(&self, index: usize) -> f64 {
        match self.orientation() {
            Orientation::Horizontal => index as f64 * self.item_pitch() as f64,
            _ => 0.0,
        }
    }

    fn item_y(&self, index: usize) -> f64 {
        match self.orientation() {
            Orientation::Horizontal => 0.0,
            _ => index as f64 * self.item_pitch() as f64,
        }
    }

    fn item_width(&self) -> i32 {
        match self.orientation() {
            Orientation::Horizontal => self.item_extent(),
            _ => -1,
        }
    }

    fn item_height(&self) -> i32 {
        match self.orientation() {
            Orientation::Horizontal => -1,
            _ => self.item_extent(),
        }
    }
}

impl VisibleAnchor {
    fn range(self, len: usize) -> Range<usize> {
        if len == 0 {
            return 0..0;
        }

        let position = self.position.min(len - 1);
        let count = (self.before + self.after + 1).min(len);
        let start = position.saturating_sub(self.before).min(len - count);
        start..start + count
    }
}

impl VirtualRow {
    fn new(
        item: Rc<dyn Any>, orientation: Orientation, widget_factory: WidgetFactory,
        widget_binder: WidgetBinder,
    ) -> Rc<Self> {
        let child = widget_factory(item.as_ref());
        widget_binder(&child, item.as_ref());
        let container = AnimatedBin::new(&child);
        container.set_hexpand(true);
        container.set_overflow(gtk::Overflow::Visible);

        let row = Rc::new(Self {
            item: RefCell::new(item),
            container,
            child,
            orientation: Cell::new(orientation),
            animations: RefCell::new(Vec::new()),
            widget_binder,
        });
        row.apply_orientation();
        row
    }

    fn container(&self) -> &AnimatedBin {
        &self.container
    }

    fn bind_item(&self, item: Rc<dyn Any>) {
        if Rc::ptr_eq(&self.item.borrow(), &item) {
            return;
        }

        *self.item.borrow_mut() = item;
        (self.widget_binder)(&self.child, self.item.borrow().as_ref());
    }

    fn rebind(&self, binder: &WidgetBinder) {
        binder(&self.child, self.item.borrow().as_ref());
    }

    fn set_orientation(&self, orientation: Orientation) {
        self.orientation.set(orientation);
        self.apply_orientation();
        self.reset_animation_state();
    }

    fn measured_main_extent(&self, orientation: Orientation, for_size: i32) -> i32 {
        let (minimum, _natural, _, _) = self.container.measure(orientation, for_size.max(1));
        minimum
    }

    fn measured_cross_extent(&self, orientation: Orientation) -> i32 {
        let cross_orientation = match orientation {
            Orientation::Horizontal => Orientation::Vertical,
            _ => Orientation::Horizontal,
        };
        let (minimum, _natural, _, _) = self.container.measure(cross_orientation, -1);
        minimum
    }

    fn apply_orientation(&self) {
        match self.orientation.get() {
            Orientation::Horizontal => self.container.set_size_request(-1, -1),
            _ => self.container.set_size_request(-1, VIRTUAL_ROW_EXTENT),
        }
    }

    fn reset_for_reuse(&self) {
        self.container.set_sensitive(true);
        if let Ok(tu_item) = self.child.clone().downcast::<TuListItem>() {
            tu_item.set_poster_focused(false);
        }
        self.reset_animation_state();
    }

    fn reset_animation_state(&self) {
        self.interrupt_animations();
        self.container.set_opacity(1.0);
        set_axis_offset(&self.container, self.orientation.get(), 0.0);
    }

    fn interrupt_animations(&self) {
        for animation in self.animations.borrow_mut().drain(..) {
            animation.skip();
        }
    }

    fn remember_animation(&self, animation: TimedAnimation) {
        self.animations.borrow_mut().push(animation);
    }

    fn animate_insert(&self) {
        self.interrupt_animations();
        self.container.set_opacity(0.0);
        set_axis_offset(
            &self.container,
            self.orientation.get(),
            INSERT_MOTION_OFFSET,
        );

        let animation = animate(
            &self.container,
            0.0,
            1.0,
            INSERT_ANIMATION_DURATION_MS,
            Easing::EaseOutCubic,
            {
                let container = self.container.clone();
                let orientation = self.orientation.get();
                move |value| {
                    container.set_opacity(value);
                    set_axis_offset(
                        &container,
                        orientation,
                        lerp_f64(INSERT_MOTION_OFFSET, 0.0, value),
                    );
                }
            },
        );
        self.remember_animation(animation);
    }

    fn animate_remove(&self) {
        self.interrupt_animations();
        self.container.set_sensitive(false);

        let animation = animate(
            &self.container,
            1.0,
            0.0,
            REMOVE_ANIMATION_DURATION_MS,
            Easing::EaseOutCubic,
            {
                let container = self.container.clone();
                move |value| container.set_opacity(value)
            },
        );
        self.remember_animation(animation);
    }

    fn animate_reorder_from(&self, offset: f64) {
        self.interrupt_animations();
        set_axis_offset(&self.container, self.orientation.get(), offset);

        let animation = animate(
            &self.container,
            offset,
            0.0,
            REORDER_ANIMATION_DURATION_MS,
            Easing::EaseOutCubic,
            {
                let container = self.container.clone();
                let orientation = self.orientation.get();
                move |value| set_axis_offset(&container, orientation, value)
            },
        );
        self.remember_animation(animation);
    }
}

fn active_indices(items: &[RowData], active_keys: &HashSet<String>) -> HashMap<String, usize> {
    items
        .iter()
        .enumerate()
        .filter(|&(_index, row)| active_keys.contains(&row.key))
        .map(|(index, row)| (row.key.clone(), index))
        .collect()
}

fn set_axis_offset(container: &AnimatedBin, orientation: Orientation, offset: f64) {
    match orientation {
        Orientation::Horizontal => container.set_offset(offset, 0.0),
        Orientation::Vertical => container.set_offset(0.0, offset),
        _ => container.set_offset(0.0, 0.0),
    }
}

fn animate(
    widget: &impl IsA<Widget>, from: f64, to: f64, duration: u32, easing: Easing,
    callback: impl Fn(f64) + 'static,
) -> TimedAnimation {
    let target = CallbackAnimationTarget::new(callback);
    let animation = TimedAnimation::new(widget, from, to, duration, target);
    animation.set_easing(easing);

    let keep_alive = Rc::new(RefCell::new(Some(animation.clone())));
    let keep_alive_on_done = keep_alive.clone();
    animation.connect_done(move |_| {
        keep_alive_on_done.borrow_mut().take();
    });

    animation.play();
    animation
}

fn lerp_f64(from: f64, to: f64, progress: f64) -> f64 {
    from + (to - from) * progress
}
