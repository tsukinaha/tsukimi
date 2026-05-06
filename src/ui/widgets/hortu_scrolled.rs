use std::{
    cell::{
        Cell,
        RefCell,
    },
    collections::{
        HashMap,
        HashSet,
    },
    rc::Rc,
    time::Duration,
};

use adw::{
    CallbackAnimationTarget,
    Easing,
    TimedAnimation,
    prelude::*,
    subclass::prelude::*,
};
use gtk::{
    CompositeTemplate,
    GestureClick,
    Orientation,
    Revealer,
    RevealerTransitionType,
    glib,
    template_callbacks,
};

use crate::{
    client::structs::SimpleListItem,
    ui::{
        provider::{
            tu_item::{
                PreferPoster,
                PreferSize,
            },
            tu_object::TuObject,
        },
        widgets::{
            animated_bin::{
                AnimatedBin,
                animate,
                lerp_f64,
                set_axis_offset,
            },
            fix::ScrolledWindowFixExt,
            tu_list_item::{
                TuListItem,
                imp::PosterType,
            },
        },
    },
};

pub const SHOW_BUTTON_ANIMATION_DURATION: u32 = 500;

// ── Diff animation timing constants ──
const DIFF_ANIMATION_QUEUE_DELAY: Duration = Duration::from_millis(3200);
const INSERT_ANIMATION_DURATION_MS: u32 = 3000;
const REMOVE_ANIMATION_DURATION_MS: u32 = 3000;
const REORDER_ANIMATION_DURATION_MS: u32 = 3000;
const REVEALER_TRANSITION_DURATION_MS: u32 = 3000;
const INSERT_MOTION_OFFSET: f64 = 16.0;

// ═══════════════════════════════════════════════════════════════
// UnifySize
// ═══════════════════════════════════════════════════════════════

#[derive(Default, Hash, Eq, PartialEq, Clone, Copy, glib::Enum, Debug)]
#[repr(u32)]
#[enum_type(name = "UnifySize")]
pub enum UnifySize {
    #[default]
    Disable,
    Majority,
    ForceVideo,
    ForcePost,
}

pub fn resolve_prefer_size(unify_size: UnifySize, items: &[SimpleListItem]) -> PreferSize {
    match unify_size {
        UnifySize::Disable => PreferSize::Auto,
        UnifySize::ForceVideo => PreferSize::Video,
        UnifySize::ForcePost => PreferSize::Post,
        UnifySize::Majority => {
            let primary_ratio: Vec<_> = items
                .iter()
                .filter(|i| i.item_type != "Episode")
                .filter_map(|i| i.primary_image_aspect_ratio)
                .collect();
            if primary_ratio.is_empty() {
                return PreferSize::Auto;
            }
            let video_percentage = primary_ratio.iter().filter(|i| **i > 1.0).count() as f64
                / primary_ratio.len() as f64;
            match video_percentage {
                p if p > 0.8 => PreferSize::Video,
                p if p < 0.2 => PreferSize::Post,
                _ => PreferSize::Auto,
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// LazyRow – a single row in the diffing list
// ═══════════════════════════════════════════════════════════════

#[derive(Debug)]
pub(crate) struct LazyRow {
    key: String,
    item: RefCell<TuObject>,
    container: AnimatedBin,
    revealer: Revealer,
    content: gtk::Box,
    child: RefCell<Option<TuListItem>>,
    initialized: Cell<bool>,
}

impl LazyRow {
    fn new(key: String, item: TuObject) -> Rc<Self> {
        let placeholder = gtk::Label::new(None);
        placeholder.set_width_request(1);
        placeholder.set_height_request(1);

        let content = gtk::Box::new(Orientation::Horizontal, 0);
        content.set_overflow(gtk::Overflow::Visible);
        content.append(&placeholder);

        let revealer = Revealer::builder()
            .transition_type(RevealerTransitionType::SlideRight)
            .transition_duration(REVEALER_TRANSITION_DURATION_MS)
            .reveal_child(false)
            .child(&content)
            .build();
        revealer.set_overflow(gtk::Overflow::Visible);

        let container = AnimatedBin::new(&revealer);
        container.set_overflow(gtk::Overflow::Visible);
        container.set_margin_end(6);

        let row = Rc::new(Self {
            key,
            item: RefCell::new(item),
            container,
            revealer,
            content,
            child: RefCell::new(None),
            initialized: Cell::new(false),
        });

        let weak_row = Rc::downgrade(&row);
        row.container.connect_map(move |_| {
            if let Some(row) = weak_row.upgrade() {
                row.ensure_initialized();
            }
        });

        row
    }

    fn container_widget(&self) -> &AnimatedBin {
        &self.container
    }

    fn bind_item(&self, item: TuObject) {
        *self.item.borrow_mut() = item;

        if let Some(child) = self.child.borrow().as_ref() {
            child.set_item(self.item.borrow().item());
        }
    }

    fn ensure_initialized(&self) {
        if self.initialized.replace(true) {
            return;
        }

        // Remove placeholder
        while let Some(child) = self.content.first_child() {
            self.content.remove(&child);
        }

        let tu_list_item = TuListItem::default();
        tu_list_item.set_poster_type(PosterType::default());
        tu_list_item.set_item(self.item.borrow().item());

        self.content.append(&tu_list_item);
        *self.child.borrow_mut() = Some(tu_list_item);
    }

    // ── Animations ──

    fn animate_insert(&self) {
        let in_tree = self.container.root().is_some();
        eprintln!(
            "[HortuScrolled] animate_insert: key={}, in_tree={}",
            self.key, in_tree
        );

        if in_tree {
            self.ensure_initialized();
            let transition_duration = self.revealer.transition_duration();
            self.revealer.set_transition_duration(0);
            self.revealer.set_reveal_child(true);
            self.revealer.set_transition_duration(transition_duration);

            let start_offset = -INSERT_MOTION_OFFSET;
            self.container.set_opacity(0.0);
            set_axis_offset(&self.container, Orientation::Horizontal, start_offset);

            animate(
                &self.container,
                0.0,
                1.0,
                INSERT_ANIMATION_DURATION_MS,
                Easing::EaseOutCubic,
                {
                    let container = self.container.clone();
                    move |value| {
                        container.set_opacity(value);
                        set_axis_offset(
                            &container,
                            Orientation::Horizontal,
                            lerp_f64(start_offset, 0.0, value),
                        );
                    }
                },
            );
        } else {
            // Not in widget tree yet – show immediately, animation will
            // happen when the parent revealer reveals the whole section.
            self.revealer.set_reveal_child(true);
            self.container.set_opacity(1.0);
            set_axis_offset(&self.container, Orientation::Horizontal, 0.0);
        }
    }

    fn animate_remove(&self, list: gtk::Box) {
        self.container.set_sensitive(false);
        self.revealer.set_reveal_child(false);

        let container_to_remove = self.container.clone();
        let revealer = self.revealer.clone();
        revealer.connect_child_revealed_notify(move |revealer| {
            if !revealer.reveals_child() && !revealer.is_child_revealed() {
                list.remove(&container_to_remove);
            }
        });

        animate(
            &self.container,
            1.0,
            0.0,
            REMOVE_ANIMATION_DURATION_MS,
            Easing::EaseInCubic,
            {
                let container = self.container.clone();
                move |value| {
                    container.set_opacity(value);
                }
            },
        );
    }

    fn reorder_start_offset(&self, old_index: usize, new_index: usize) -> f64 {
        let slot_size = self.container.width().max(1) as f64;
        (old_index as f64 - new_index as f64) * slot_size
    }

    fn set_reorder_offset(&self, offset: f64) {
        set_axis_offset(&self.container, Orientation::Horizontal, offset);
    }

    fn animate_reorder_to_rest(&self, start_offset: f64) {
        eprintln!(
            "[HortuScrolled] animate_reorder: key={}, start_offset={:.0}",
            self.key, start_offset
        );
        animate(
            &self.container,
            start_offset,
            0.0,
            REORDER_ANIMATION_DURATION_MS,
            Easing::EaseOutCubic,
            {
                let container = self.container.clone();
                move |value| set_axis_offset(&container, Orientation::Horizontal, value)
            },
        );
    }
}

// ═══════════════════════════════════════════════════════════════
// Diff helpers
// ═══════════════════════════════════════════════════════════════

fn rows_with_removals_at_old_positions(
    kept_rows: &[Rc<LazyRow>], removed_rows: &[Rc<LazyRow>], old_indices: &HashMap<String, usize>,
) -> Vec<Rc<LazyRow>> {
    let mut rows = kept_rows.to_vec();
    let mut removed_rows = removed_rows.to_vec();
    removed_rows.sort_by_key(|row| old_indices.get(&row.key).copied().unwrap_or(usize::MAX));

    for row in removed_rows {
        let old_index = old_indices.get(&row.key).copied().unwrap_or(rows.len());
        rows.insert(old_index.min(rows.len()), row);
    }

    rows
}

fn rows_to_animate_from_old_positions(
    kept_rows: &[Rc<LazyRow>], transient_rows: &[Rc<LazyRow>], old_indices: &HashMap<String, usize>,
) -> Vec<(Rc<LazyRow>, f64)> {
    let transient_indices: HashMap<String, usize> = transient_rows
        .iter()
        .enumerate()
        .map(|(index, row)| (row.key.clone(), index))
        .collect();

    kept_rows
        .iter()
        .filter_map(|row| {
            let old_index = *old_indices.get(&row.key)?;
            let transient_index = *transient_indices.get(&row.key)?;

            if old_index == transient_index {
                return None;
            }

            let start_offset = row.reorder_start_offset(old_index, transient_index);
            row.set_reorder_offset(start_offset);
            Some((row.clone(), start_offset))
        })
        .collect()
}

fn previous_sibling(rows: &[Rc<LazyRow>], index: usize) -> Option<gtk::Widget> {
    if index == 0 {
        None
    } else {
        Some(rows[index - 1].container_widget().clone().upcast())
    }
}

// ═══════════════════════════════════════════════════════════════
// Widget implementation
// ═══════════════════════════════════════════════════════════════

mod imp {
    use std::cell::{
        OnceCell,
        RefCell,
    };

    use glib::subclass::InitializingObject;

    use super::*;

    #[derive(Debug, CompositeTemplate, glib::Properties)]
    #[template(resource = "/moe/tsuna/tsukimi/ui/hortu_scrolled.ui")]
    #[properties(wrapper_type = super::HortuScrolled)]
    pub struct HortuScrolled {
        #[property(get, set, construct_only, default_value = false)]
        pub isresume: OnceCell<bool>,
        #[template_child]
        pub label: TemplateChild<gtk::Label>,
        #[template_child]
        pub scrolled: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub list: TemplateChild<gtk::Box>,
        #[template_child]
        pub revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub morebutton: TemplateChild<gtk::Button>,
        #[template_child]
        pub left_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub right_button: TemplateChild<gtk::Button>,

        #[property(get, set, default_value = false)]
        pub moreview: RefCell<bool>,
        #[property(get, set)]
        pub title: RefCell<String>,

        #[property(get, set, builder(UnifySize::default()))]
        pub unify_size: RefCell<UnifySize>,

        #[property(get, set, builder(PreferPoster::default()))]
        pub prefer_poster: RefCell<PreferPoster>,

        pub show_button_animation: OnceCell<adw::TimedAnimation>,
        pub hide_button_animation: OnceCell<adw::TimedAnimation>,

        // ── Diff state ──
        #[allow(private_interfaces)]
        pub rows: RefCell<Vec<Rc<LazyRow>>>,
        pub pending_items: RefCell<Option<Vec<TuObject>>>,
        pub is_animating: Cell<bool>,
        pub has_been_shown: Cell<bool>,
    }

    impl Default for HortuScrolled {
        fn default() -> Self {
            Self {
                isresume: OnceCell::default(),
                label: TemplateChild::default(),
                scrolled: TemplateChild::default(),
                list: TemplateChild::default(),
                revealer: TemplateChild::default(),
                morebutton: TemplateChild::default(),
                left_button: TemplateChild::default(),
                right_button: TemplateChild::default(),
                moreview: RefCell::new(false),
                title: RefCell::default(),
                unify_size: RefCell::new(UnifySize::default()),
                prefer_poster: RefCell::new(PreferPoster::default()),
                show_button_animation: OnceCell::default(),
                hide_button_animation: OnceCell::default(),
                rows: RefCell::new(Vec::new()),
                pending_items: RefCell::new(None),
                is_animating: Cell::new(false),
                has_been_shown: Cell::new(false),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for HortuScrolled {
        const NAME: &'static str = "HortuScrolled";
        type Type = super::HortuScrolled;
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
    impl ObjectImpl for HortuScrolled {
        fn constructed(&self) {
            self.parent_constructed();

            self.scrolled.fix();

            // The GtkBox (list) is already in the template as a child of scrolled.
            // We manage its children (LazyRow containers) manually.
            self.list.set_hexpand(true);
            self.list.set_overflow(gtk::Overflow::Visible);
        }
    }

    impl WidgetImpl for HortuScrolled {}

    impl BinImpl for HortuScrolled {}
}

glib::wrapper! {
    /// A scrolled list of items with diff animations.
    pub struct HortuScrolled(ObjectSubclass<imp::HortuScrolled>)
        @extends gtk::Widget, adw::Bin, @implements gtk::Accessible;
}

impl Default for HortuScrolled {
    fn default() -> Self {
        Self::new()
    }
}

#[template_callbacks]
impl HortuScrolled {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_morebutton(&self) {
        let imp = self.imp();
        imp.morebutton.set_visible(true);
    }

    /// Applies a new item snapshot with diff animations.
    ///
    /// Calls made while an animation batch is still running are coalesced so the
    /// latest snapshot wins.
    pub fn set_items(&self, items: &[SimpleListItem]) {
        if items.is_empty() {
            self.set_visible(false);
            {
                let imp = self.imp();
                let removed_rows = imp.rows.borrow().clone();
                for row in removed_rows {
                    row.animate_remove(imp.list.clone());
                }
                imp.rows.borrow_mut().clear();
            }
            return;
        }

        self.set_visible(true);

        let prefer_size = resolve_prefer_size(self.unify_size(), items);
        let prefer_poster = self.prefer_poster();
        let is_resume = self.isresume();

        let new_items: Vec<TuObject> = items
            .iter()
            .map(|item| {
                let object = TuObject::from_simple(item, None);
                object.item().set_is_resume(is_resume);
                object.item().set_prefer_size(prefer_size);
                object.item().set_prefer_poster(prefer_poster);
                object
            })
            .collect();

        let imp = self.imp();

        // Coalesce: if an animation batch is running, store pending items.
        if imp.is_animating.get() {
            *imp.pending_items.borrow_mut() = Some(new_items);
            return;
        }

        imp.is_animating.set(true);
        self.apply_items(new_items);

        let obj = self.clone();
        gtk::glib::timeout_add_local_once(DIFF_ANIMATION_QUEUE_DELAY, move || {
            let next_items = {
                let imp = obj.imp();
                imp.is_animating.set(false);
                imp.pending_items.borrow_mut().take()
            };

            if let Some(items) = next_items {
                obj.apply_items(items);
            }
        });
    }

    fn apply_items(&self, items: Vec<TuObject>) {
        let imp = self.imp();
        let is_cold = !imp.has_been_shown.get();

        eprintln!(
            "[HortuScrolled] apply_items: {} items, is_cold={}, old_rows={}, mapped={}",
            items.len(),
            is_cold,
            imp.rows.borrow().len(),
            imp.list.is_mapped(),
        );

        let old_indices: HashMap<String, usize> = imp
            .rows
            .borrow()
            .iter()
            .enumerate()
            .map(|(index, row)| (row.key.clone(), index))
            .collect();

        let mut existing_rows: HashMap<String, Rc<LazyRow>> = imp
            .rows
            .borrow()
            .iter()
            .map(|row| (row.key.clone(), row.clone()))
            .collect();

        let mut new_keys = HashSet::with_capacity(items.len());
        let mut kept_rows: Vec<Rc<LazyRow>> = Vec::with_capacity(items.len());
        let mut matched_count = 0usize;
        let mut inserted_count = 0usize;

        for item in items {
            let key = item.item().id();
            let is_unique = new_keys.insert(key.clone());
            debug_assert!(
                is_unique,
                "HortuScrolled item keys must be unique within one update"
            );

            if let Some(row) = existing_rows.remove(&key) {
                row.bind_item(item);
                kept_rows.push(row);
                matched_count += 1;
            } else {
                let row = LazyRow::new(key, item);
                imp.list.append(row.container_widget());

                // Set up activation (click to play) via weak ref so rebinds stay current
                let row_weak = Rc::downgrade(&row);
                let click = GestureClick::new();
                let container = row.container_widget().clone();
                click.connect_released(move |_, _, _, _| {
                    if let Some(row) = row_weak.upgrade() {
                        row.item.borrow().activate(&container);
                    }
                });
                row.container_widget().add_controller(click);

                if is_cold {
                    // Cold start: build the widget now so layout is correct,
                    // then show immediately without animation.
                    row.ensure_initialized();
                    row.revealer.set_reveal_child(true);
                    row.container_widget().set_opacity(1.0);
                } else {
                    row.animate_insert();
                }
                kept_rows.push(row);
                inserted_count += 1;
            }
        }

        let removed_rows: Vec<Rc<LazyRow>> = imp
            .rows
            .borrow()
            .iter()
            .filter(|row| !new_keys.contains(&row.key))
            .cloned()
            .collect();

        let transient_rows =
            rows_with_removals_at_old_positions(&kept_rows, &removed_rows, &old_indices);
        let moved_rows =
            rows_to_animate_from_old_positions(&kept_rows, &transient_rows, &old_indices);

        eprintln!(
            "[HortuScrolled] diff: kept={}, inserted={}, removed={}, moved={}",
            matched_count,
            inserted_count,
            removed_rows.len(),
            moved_rows.len(),
        );

        // Reorder children in the box to match the new order
        if is_cold {
            for (new_index, row) in kept_rows.iter().enumerate() {
                let sibling: Option<gtk::Widget> = previous_sibling(&kept_rows, new_index);
                imp.list
                    .reorder_child_after(row.container_widget(), sibling.as_ref());
            }
        } else {
            for (new_index, row) in transient_rows.iter().enumerate() {
                let sibling = previous_sibling(&transient_rows, new_index);
                imp.list
                    .reorder_child_after(row.container_widget(), sibling.as_ref());
            }
        }

        if is_cold {
            // Cold start: remove old rows instantly
            for row in removed_rows {
                imp.list.remove(row.container_widget());
            }
        } else {
            // Animate removals
            for row in removed_rows {
                row.animate_remove(imp.list.clone());
            }

            // Animate reorders
            for (row, start_offset) in moved_rows {
                row.animate_reorder_to_rest(start_offset);
            }
        }

        *imp.rows.borrow_mut() = kept_rows;

        // Force visible rows to initialize their widgets
        if !is_cold {
            for row in imp.rows.borrow().iter() {
                if row.container_widget().is_mapped() {
                    row.ensure_initialized();
                }
            }
        }

        // Reveal the whole section if it was hidden
        imp.revealer.set_reveal_child(true);

        // Mark as shown so the next update uses animated diff
        imp.has_been_shown.set(true);
    }

    // ── Control button animations (unchanged) ──

    fn set_control_opacity(&self, opacity: f64) {
        let imp = self.imp();
        imp.left_button.set_opacity(opacity);
        imp.right_button.set_opacity(opacity);
    }

    fn are_controls_visible(&self) -> bool {
        if self.hide_controls_animation().state() == adw::AnimationState::Playing {
            return false;
        }

        self.imp().left_button.opacity() >= 0.68
            || self.show_controls_animation().state() == adw::AnimationState::Playing
    }

    fn show_controls_animation(&self) -> &adw::TimedAnimation {
        self.imp().show_button_animation.get_or_init(|| {
            let target = CallbackAnimationTarget::new(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move |opacity| obj.set_control_opacity(opacity)
            ));

            TimedAnimation::builder()
                .duration(SHOW_BUTTON_ANIMATION_DURATION)
                .widget(&self.imp().scrolled.get())
                .target(&target)
                .value_to(0.7)
                .build()
        })
    }

    fn hide_controls_animation(&self) -> &adw::TimedAnimation {
        self.imp().hide_button_animation.get_or_init(|| {
            let target = CallbackAnimationTarget::new(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move |opacity| obj.set_control_opacity(opacity)
            ));

            TimedAnimation::builder()
                .duration(SHOW_BUTTON_ANIMATION_DURATION)
                .widget(&self.imp().scrolled.get())
                .target(&target)
                .value_to(0.)
                .build()
        })
    }

    #[template_callback]
    fn on_rightbutton_clicked(&self) {
        self.anime::<true>();
    }

    fn controls_opacity(&self) -> f64 {
        self.imp().left_button.opacity()
    }

    #[template_callback]
    fn on_enter_focus(&self) {
        if !self.are_controls_visible() {
            self.hide_controls_animation().pause();
            self.show_controls_animation()
                .set_value_from(self.controls_opacity());
            self.show_controls_animation().play();
        }
    }

    #[template_callback]
    fn on_leave_focus(&self) {
        if self.are_controls_visible() {
            self.show_controls_animation().pause();
            self.hide_controls_animation()
                .set_value_from(self.controls_opacity());
            self.hide_controls_animation().play();
        }
    }

    #[template_callback]
    fn on_leftbutton_clicked(&self) {
        self.anime::<false>();
    }

    pub fn connect_morebutton<F>(&self, cb: F)
    where
        F: Fn(&gtk::Button) + 'static,
    {
        self.imp().morebutton.connect_clicked(cb);
    }

    fn anime<const R: bool>(&self) {
        let scrolled = self.imp().scrolled.get();
        let adj = scrolled.hadjustment();

        let Some(clock) = scrolled.frame_clock() else {
            return;
        };

        let start = adj.value();
        let end = if R { start + 800.0 } else { start - 800.0 };

        let start_time = clock.frame_time();
        let end_time = start_time + 1000 * 400;

        scrolled.add_tick_callback(move |_view, clock| {
            let now = clock.frame_time();
            if now < end_time && adj.value() != end {
                let mut t = (now - start_time) as f64 / (end_time - start_time) as f64;
                t = Self::ease_in_out_cubic(t);
                adj.set_value(start + t * (end - start));
                glib::ControlFlow::Continue
            } else {
                adj.set_value(end);
                glib::ControlFlow::Break
            }
        });
    }

    fn ease_in_out_cubic(t: f64) -> f64 {
        if t < 0.5 {
            4.0 * t * t * t
        } else {
            let t = 2.0 * t - 2.0;
            0.5 * t * t * t + 1.0
        }
    }
}
