use std::{
    cell::{
        Cell,
        RefCell,
    },
    sync::{
        Arc,
        OnceLock,
    },
    time::Duration,
};

use gtk::{
    gdk,
    glib,
    prelude::*,
    subclass::prelude::*,
};

use crate::{
    model::LyricsDocument,
    render::{
        cache::{
            BatchPrepare,
            CacheBuild,
            CacheStamp,
            RenderCache,
        },
        layout::{
            LaneVisibility,
            ViewportAnchor,
            gap_before_range,
            slot_for_kind,
        },
        scheduler::{
            RenderScheduler,
            WakeupKey,
            frame_delay,
            media_delay,
        },
        snapshot::{
            LineMotion,
            LineMotionLeg,
            SnapshotFrame,
            snapshot_lyrics,
        },
        state::{
            AnimationReasons,
            BatchPlan,
            DirtyFlags,
            FrameStatus,
            PlaybackAnchor,
            PlaybackState,
            RenderEffects,
            RenderEvent,
            RenderState,
            SourceIntents,
            Wakeup,
            reduce,
        },
        style::{
            LaneSlot,
            LyricsStyle,
        },
        visual::{
            GapPhase,
            VisualSignature,
        },
    },
    time::{
        LyricTime,
        TimeRange,
    },
    timeline::{
        Timeline,
        TimelineFrame,
    },
};

const MANUAL_SCROLL_HOLD_US: i64 = 3_000_000;

const LARGE_SEEK_US: i64 = 750_000;
const SCROLL_STEP: f32 = 56.0;
const DRAG_THRESHOLD: f64 = 5.0;
const LAYOUT_WIDTH_BUCKET: i32 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LineTransitionKind {
    Playback,
    Click,
}

#[derive(Clone, Debug)]
struct LineTransition {
    kind: LineTransitionKind,
    to_scroll: f32,
    legs: Arc<[LineMotionLeg]>,
}

impl LineTransition {
    fn new(
        started_us: i64, from_scroll: f32, to_scroll: f32, _from_focus: Option<usize>,
        to_focus: Option<usize>, style: &LyricsStyle,
    ) -> Self {
        Self {
            kind: LineTransitionKind::Playback,
            to_scroll,
            legs: Arc::from([Self::leg(
                started_us,
                to_scroll - from_scroll,
                to_focus,
                style,
            )]),
        }
    }

    fn for_click(
        started_us: i64, to_scroll: f32, to_focus: Option<usize>, entry_offset: f32,
        style: &LyricsStyle,
    ) -> Self {
        let entry_offset = if entry_offset.is_finite() {
            entry_offset
        } else {
            0.0
        };
        Self {
            kind: LineTransitionKind::Click,
            to_scroll,
            legs: Arc::from([LineMotionLeg::new(
                started_us,
                style.transition.duration_us,
                0,
                0,
                entry_offset,
                to_focus,
                style.transition.easing,
            )]),
        }
    }

    fn is_click(&self) -> bool {
        self.kind == LineTransitionKind::Click
    }

    fn leg(
        started_us: i64, scroll_delta: f32, focus_line: Option<usize>, style: &LyricsStyle,
    ) -> LineMotionLeg {
        LineMotionLeg::new(
            started_us,
            style.transition.duration_us,
            style.interaction.line_stagger_us,
            style.overscan.total().max(1),
            scroll_delta,
            focus_line,
            style.transition.easing,
        )
    }

    fn retarget(
        &mut self, now_us: i64, to_scroll: f32, to_focus: Option<usize>, style: &LyricsStyle,
    ) {
        let mut legs: Vec<_> = self
            .legs
            .iter()
            .copied()
            .filter(|leg| !leg.finished(now_us))
            .collect();
        let scroll_delta = to_scroll - self.to_scroll;
        if scroll_delta != 0.0 {
            legs.push(Self::leg(now_us, scroll_delta, to_focus, style));
        }
        self.to_scroll = to_scroll;
        self.legs = legs.into();
    }

    fn finished(&self, now_us: i64) -> bool {
        self.legs.iter().all(|leg| leg.finished(now_us))
    }

    fn retain_active(&mut self, now_us: i64) {
        if self.legs.iter().all(|leg| !leg.finished(now_us)) {
            return;
        }
        let active: Vec<_> = self
            .legs
            .iter()
            .copied()
            .filter(|leg| !leg.finished(now_us))
            .collect();
        self.legs = active.into();
    }

    fn visual_progress(&self, now_us: i64) -> f32 {
        self.legs
            .last()
            .map(|leg| leg.overall_progress(now_us))
            .unwrap_or(1.0)
    }

    fn shift(&mut self, delta: f32) {
        self.to_scroll += delta;
    }

    fn targets(&self, scroll: f32) -> bool {
        (self.to_scroll - scroll).abs() <= 0.5
    }

    fn motion(&self, now_us: i64) -> LineMotion {
        LineMotion::new(now_us, self.legs.clone())
    }
}

fn discard_playback_transition(transition: &mut Option<LineTransition>) {
    if transition
        .as_ref()
        .is_some_and(|transition| !transition.is_click())
    {
        transition.take();
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PendingClickTransition {
    line: usize,
    entry_offset: f32,
}

fn click_entry_for_focus(
    pending: Option<PendingClickTransition>, focus_line: Option<usize>,
) -> Option<f32> {
    pending
        .filter(|pending| Some(pending.line) == focus_line)
        .map(|pending| pending.entry_offset)
}

fn visual_line_offset_from_center(
    line_center: f32, scroll: f32, line_motion: f32, viewport_height: f32,
) -> f32 {
    line_center - scroll + line_motion - viewport_height * 0.5
}

#[derive(Clone, Copy, Debug)]
struct HoverAnimation {
    line: usize,
    started_us: i64,
    from: f32,
    to: f32,
}

impl HoverAnimation {
    fn value(self, duration_us: i64, now_us: i64) -> f32 {
        let progress = animation_progress(self.started_us, duration_us, now_us);
        self.from + (self.to - self.from) * ease_out_cubic(progress)
    }

    fn finished(self, duration_us: i64, now_us: i64) -> bool {
        now_us.saturating_sub(self.started_us) >= duration_us.max(0)
    }
}

#[derive(Clone, Copy, Debug)]
struct ActivationAnimation {
    line: usize,
    started_us: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DrawState {
    signature: VisualSignature,
    cache_generation: u64,
    scroll_bits: u32,
    transition_bits: u32,
    gap_phase: Option<GapPhase>,
    hover_line: Option<usize>,
    hover_opacity_bits: u32,
    activation_line: Option<usize>,
    activation_progress_bits: u32,
}

mod imp {
    use super::*;

    #[derive(glib::Properties)]
    #[properties(wrapper_type = super::LyricsView)]
    pub struct LyricsView {
        pub document: RefCell<Option<Arc<LyricsDocument>>>,
        pub track_index: Cell<Option<usize>>,
        pub timeline: RefCell<Timeline>,
        pub anchor: Cell<PlaybackAnchor>,
        pub style: RefCell<LyricsStyle>,
        pub visibility: Cell<LaneVisibility>,
        pub cache: RefCell<RenderCache>,
        pub scheduler: RefCell<RenderScheduler>,
        pub render_state: Cell<RenderState>,
        pub allocation: Cell<(i32, i32)>,
        pub last_allocation_change_us: Cell<i64>,
        pub position: Cell<LyricTime>,
        pub next_event: Cell<Option<LyricTime>>,
        pub current_line: Cell<Option<usize>>,
        pub focus_line: Cell<Option<usize>>,
        pub visible_range: Cell<(usize, usize)>,
        pub word_animation: Cell<bool>,
        pub gap: Cell<Option<TimeRange>>,
        pub gap_line: Cell<Option<usize>>,
        pub gap_origin_scroll: Cell<Option<f32>>,
        #[property(get, set = Self::set_reduced_motion, explicit_notify)]
        pub reduced_motion: Cell<bool>,
        #[property(get, set = Self::set_max_animation_fps, explicit_notify)]
        pub max_animation_fps: Cell<u32>,
        #[property(get)]
        pub manual_scrolling: Cell<bool>,
        pub last_animation_frame_us: Cell<i64>,
        pub(super) transition: RefCell<Option<LineTransition>>,
        pub(super) hover: Cell<Option<HoverAnimation>>,
        pub(super) activation: Cell<Option<ActivationAnimation>>,
        pub(super) pending_click: Cell<Option<PendingClickTransition>>,
        pub pointer_position: Cell<Option<(f64, f64)>>,
        pub scroll_offset: Cell<f32>,
        pub drag_start_scroll: Cell<f32>,
        pub suppress_click: Cell<bool>,
        pub document_generation: Cell<u64>,
        pub width_generation: Cell<u64>,
        pub style_generation: Cell<u64>,
        pub visibility_generation: Cell<u64>,
        pub scale_generation: Cell<u64>,
        pub environment_generation: Cell<u64>,
        pub settings: RefCell<Option<gtk::Settings>>,
        pub settings_handlers: RefCell<Vec<glib::SignalHandlerId>>,
    }

    impl Default for LyricsView {
        fn default() -> Self {
            Self {
                document: RefCell::new(None),
                track_index: Cell::new(None),
                timeline: RefCell::new(Timeline::default()),
                anchor: Cell::new(PlaybackAnchor::new(
                    LyricTime::ZERO,
                    0,
                    1.0,
                    PlaybackState::Paused,
                    0,
                )),
                style: RefCell::new(LyricsStyle::default()),
                visibility: Cell::new(LaneVisibility::default()),
                cache: RefCell::new(RenderCache::default()),
                scheduler: RefCell::new(RenderScheduler::default()),
                render_state: Cell::new(RenderState::Detached),
                allocation: Cell::new((0, 0)),
                last_allocation_change_us: Cell::new(0),
                position: Cell::new(LyricTime::ZERO),
                next_event: Cell::new(None),
                current_line: Cell::new(None),
                focus_line: Cell::new(None),
                visible_range: Cell::new((0, 0)),
                word_animation: Cell::new(false),
                gap: Cell::new(None),
                gap_line: Cell::new(None),
                gap_origin_scroll: Cell::new(None),
                reduced_motion: Cell::new(false),
                max_animation_fps: Cell::new(0),
                manual_scrolling: Cell::new(false),
                last_animation_frame_us: Cell::new(0),
                transition: RefCell::new(None),
                hover: Cell::new(None),
                activation: Cell::new(None),
                pending_click: Cell::new(None),
                pointer_position: Cell::new(None),
                scroll_offset: Cell::new(0.0),
                drag_start_scroll: Cell::new(0.0),
                suppress_click: Cell::new(false),
                document_generation: Cell::new(0),
                width_generation: Cell::new(0),
                style_generation: Cell::new(0),
                visibility_generation: Cell::new(0),
                scale_generation: Cell::new(0),
                environment_generation: Cell::new(0),
                settings: RefCell::new(None),
                settings_handlers: RefCell::new(Vec::new()),
            }
        }
    }

    impl LyricsView {
        fn set_max_animation_fps(&self, frames_per_second: u32) {
            let frames_per_second = frames_per_second.min(240);
            if self.max_animation_fps.replace(frames_per_second) == frames_per_second {
                return;
            }
            self.last_animation_frame_us.set(0);
            self.obj().notify_max_animation_fps();
        }

        fn set_reduced_motion(&self, reduced_motion: bool) {
            if self.reduced_motion.replace(reduced_motion) == reduced_motion {
                return;
            }
            let obj = self.obj();
            let now_us = obj.frame_time_us();
            let before = obj.draw_state(now_us);
            if reduced_motion {
                obj.imp().transition.borrow_mut().take();
                if !obj.is_manual_scroll() {
                    let scroll = obj.auto_scroll_for(obj.imp().focus_line.get());
                    obj.imp().scroll_offset.set(scroll);
                }
            }
            obj.ensure_scene(now_us);
            obj.apply_event(
                RenderEvent::AnimationsChanged {
                    playback: effective_playback(obj.imp().anchor.get()),
                    frame: obj.frame_status(now_us),
                },
                now_us,
                &before,
                true,
                false,
            );
            if obj.draw_state(now_us) != before {
                obj.queue_draw();
            }
            obj.notify_reduced_motion();
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LyricsView {
        const NAME: &'static str = "LycoricLyricsView";
        type Type = super::LyricsView;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("lycoric-lyrics-view");
            klass.set_accessible_role(gtk::AccessibleRole::Group);
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for LyricsView {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.set_overflow(gtk::Overflow::Hidden);
            obj.set_focusable(true);
            obj.install_controllers();
            gtk::prelude::WidgetExt::connect_scale_factor_notify(&*obj, |obj| {
                obj.handle_scale_change();
            });
            obj.install_settings_observers();
        }

        fn dispose(&self) {
            let reduction = reduce(self.render_state.get(), RenderEvent::Disposed);
            self.render_state.set(reduction.state);
            self.scheduler.borrow_mut().stop_all();
            if let Some(settings) = self.settings.borrow_mut().take() {
                for handler in self.settings_handlers.take() {
                    settings.disconnect(handler);
                }
            } else {
                self.settings_handlers.borrow_mut().clear();
            }
            self.cache.borrow_mut().clear();
            self.document.borrow_mut().take();
        }

        fn signals() -> &'static [glib::subclass::Signal] {
            static SIGNALS: OnceLock<Vec<glib::subclass::Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    glib::subclass::Signal::builder("line-activated")
                        .param_types([i64::static_type()])
                        .build(),
                ]
            })
        }
    }

    impl WidgetImpl for LyricsView {
        fn map(&self) {
            self.parent_map();
            self.obj().handle_map();
        }

        fn unmap(&self) {
            self.obj().handle_unmap();
            self.parent_unmap();
        }

        fn unrealize(&self) {
            self.scheduler.borrow_mut().stop_all();
            self.cache.borrow_mut().invalidate_scale();
            self.parent_unrealize();
        }

        fn direction_changed(&self, previous_direction: gtk::TextDirection) {
            self.parent_direction_changed(previous_direction);
            self.obj().handle_text_environment_change();
        }

        fn system_setting_changed(&self, setting: &gtk::SystemSetting) {
            self.parent_system_setting_changed(setting);
            match setting {
                gtk::SystemSetting::Dpi
                | gtk::SystemSetting::FontName
                | gtk::SystemSetting::FontConfig
                | gtk::SystemSetting::Display => self.obj().handle_text_environment_change(),
                _ => {}
            }
        }

        fn measure(&self, orientation: gtk::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            match orientation {
                gtk::Orientation::Horizontal => (160, 480, -1, -1),
                gtk::Orientation::Vertical => (160, 600, -1, -1),
                _ => (0, 0, -1, -1),
            }
        }

        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            self.parent_size_allocate(width, height, baseline);
            self.obj().handle_allocation(width, height);
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            self.obj().render_snapshot(snapshot);
        }
    }
}

glib::wrapper! {
    pub struct LyricsView(ObjectSubclass<imp::LyricsView>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl LyricsView {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_document(&self, document: Option<Arc<LyricsDocument>>) {
        if same_document(self.imp().document.borrow().as_ref(), document.as_ref()) {
            return;
        }
        let now_us = self.frame_time_us();
        let before = self.draw_state(now_us);
        self.imp().document.replace(document.clone());
        self.imp().timeline.replace(
            document
                .as_deref()
                .map(|document| selected_timeline(document, self.imp().track_index.get()))
                .unwrap_or_default(),
        );
        bump(&self.imp().document_generation);
        self.reset_navigation();
        self.sample_frame(now_us, false);
        let generation = document
            .as_ref()
            .map(|_| self.imp().document_generation.get());
        self.apply_event(
            RenderEvent::DocumentChanged { generation },
            now_us,
            &before,
            true,
            false,
        );
        if generation.is_none() {
            self.imp().cache.borrow_mut().clear();
        }
    }

    pub fn document(&self) -> Option<Arc<LyricsDocument>> {
        self.imp().document.borrow().clone()
    }

    pub fn set_track_index(&self, track_index: Option<usize>) {
        if self.imp().track_index.replace(track_index) == track_index {
            return;
        }
        let now_us = self.frame_time_us();
        let before = self.draw_state(now_us);
        let document = self.imp().document.borrow().clone();
        self.imp().timeline.replace(
            document
                .as_deref()
                .map(|document| selected_timeline(document, track_index))
                .unwrap_or_default(),
        );
        bump(&self.imp().document_generation);
        self.reset_navigation();
        self.sample_frame(now_us, false);
        let generation = document
            .as_ref()
            .map(|_| self.imp().document_generation.get());
        self.apply_event(
            RenderEvent::DocumentChanged { generation },
            now_us,
            &before,
            true,
            false,
        );
        if generation.is_none() {
            self.imp().cache.borrow_mut().clear();
        }
    }

    pub fn track_index(&self) -> Option<usize> {
        self.imp().track_index.get()
    }

    pub fn set_playback_anchor(&self, anchor: PlaybackAnchor) {
        let old_anchor = self.imp().anchor.get();
        if old_anchor == anchor {
            return;
        }
        let now_us = self.frame_time_us();
        let before = self.draw_state(now_us);
        self.refresh_pending_click(now_us);
        let manual_deadline = match self.imp().render_state.get() {
            RenderState::ManualScroll {
                return_deadline_us, ..
            } => Some(return_deadline_us),
            _ => None,
        };
        let predicted = old_anchor.position_at(now_us);

        let large_seek = anchor
            .media_position
            .as_micros()
            .abs_diff(predicted.as_micros())
            > LARGE_SEEK_US as u64;

        if large_seek && self.imp().render_state.get().has_document() {
            self.apply_event(
                RenderEvent::Seek {
                    generation: self.imp().document_generation.get(),
                    target: anchor.media_position,
                },
                now_us,
                &before,
                false,
                false,
            );
        }

        self.imp().anchor.set(anchor);
        self.sample_frame(now_us, anchor.is_advancing());
        let event =
            if large_seek && matches!(self.imp().render_state.get(), RenderState::Seeking { .. }) {
                RenderEvent::SeekCommitted {
                    generation: self.imp().document_generation.get(),
                }
            } else {
                RenderEvent::AnchorChanged {
                    anchor,
                    frame_time_us: now_us,
                    next_event: self.imp().next_event.get(),
                    animations: self.animation_reasons(now_us),
                }
            };
        self.apply_event(event, now_us, &before, true, false);
        if let Some(return_deadline_us) = manual_deadline
            && !anchor.is_advancing()
            && !large_seek
            && !self.is_manual_scroll()
        {
            self.apply_event(
                RenderEvent::UserScroll {
                    return_deadline_us,
                    advancing: anchor.is_advancing(),
                },
                now_us,
                &before,
                false,
                false,
            );
        }
    }

    pub fn playback_anchor(&self) -> PlaybackAnchor {
        self.imp().anchor.get()
    }

    pub fn set_style(&self, style: LyricsStyle) {
        if *self.imp().style.borrow() == style {
            return;
        }
        let now_us = self.frame_time_us();
        let before = self.draw_state(now_us);
        self.imp().style.replace(style);
        bump(&self.imp().style_generation);
        self.imp().transition.borrow_mut().take();
        self.sample_frame(now_us, false);
        self.apply_event(
            RenderEvent::StyleChanged {
                generation: self.imp().style_generation.get(),
            },
            now_us,
            &before,
            true,
            false,
        );
    }

    pub fn style(&self) -> LyricsStyle {
        self.imp().style.borrow().clone()
    }

    pub fn set_lane_visible(&self, lane: LaneSlot, visible: bool) {
        let mut visibility = self.imp().visibility.get();
        if visibility.is_visible(lane) == visible {
            return;
        }
        let now_us = self.frame_time_us();
        let before = self.draw_state(now_us);
        visibility.set_visible(lane, visible);
        self.imp().visibility.set(visibility);
        bump(&self.imp().visibility_generation);
        self.sample_frame(now_us, false);
        self.apply_event(
            RenderEvent::StyleChanged {
                generation: self.imp().visibility_generation.get(),
            },
            now_us,
            &before,
            true,
            false,
        );
    }

    pub fn lane_visible(&self, lane: LaneSlot) -> bool {
        self.imp().visibility.get().is_visible(lane)
    }

    pub fn render_state(&self) -> RenderState {
        self.imp().render_state.get()
    }

    fn handle_map(&self) {
        let now_us = self.frame_time_us();
        let before = self.draw_state(now_us);
        self.sample_frame(now_us, false);
        self.apply_event(
            RenderEvent::Mapped {
                document_generation: self
                    .imp()
                    .document
                    .borrow()
                    .as_ref()
                    .map(|_| self.imp().document_generation.get()),
            },
            now_us,
            &before,
            true,
            false,
        );
    }

    fn handle_unmap(&self) {
        let now_us = self.frame_time_us();
        let before = self.draw_state(now_us);
        self.apply_event(RenderEvent::Unmapped, now_us, &before, false, false);
    }

    fn handle_allocation(&self, width: i32, height: i32) {
        let old = self.imp().allocation.replace((width, height));
        if old == (width, height) {
            return;
        }
        let now_us = self.frame_time_us();
        self.imp().last_allocation_change_us.set(now_us);
        let before = self.draw_state(now_us);
        let valid = width > 0 && height > 0;
        let relayout = allocation_requires_relayout(old, (width, height));
        if relayout {
            bump(&self.imp().width_generation);
        }
        let preserve_click = self.has_click_transition();
        discard_playback_transition(&mut self.imp().transition.borrow_mut());
        if !relayout && !self.is_manual_scroll() && !preserve_click {
            self.center_focus();
        }
        self.apply_event(
            RenderEvent::AllocationChanged {
                generation: self.imp().width_generation.get(),
                valid,
                relayout,
            },
            now_us,
            &before,
            true,
            false,
        );
        if preserve_click {
            self.realign_click_transition();
        }
    }

    fn handle_scale_change(&self) {
        let now_us = self.frame_time_us();
        let before = self.draw_state(now_us);
        bump(&self.imp().scale_generation);
        self.apply_event(
            RenderEvent::ScaleChanged {
                generation: self.imp().scale_generation.get(),
            },
            now_us,
            &before,
            true,
            false,
        );
    }

    fn handle_text_environment_change(&self) {
        let now_us = self.frame_time_us();
        let before = self.draw_state(now_us);
        bump(&self.imp().environment_generation);
        self.imp().transition.borrow_mut().take();
        self.sample_frame(now_us, false);
        self.apply_event(
            RenderEvent::StyleChanged {
                generation: self.imp().environment_generation.get(),
            },
            now_us,
            &before,
            true,
            false,
        );
    }

    fn handle_theme_change(&self) {
        let now_us = self.frame_time_us();
        let before = self.draw_state(now_us);
        bump(&self.imp().environment_generation);
        self.apply_event(
            RenderEvent::ThemeChanged {
                generation: self.imp().environment_generation.get(),
            },
            now_us,
            &before,
            true,
            false,
        );
    }

    fn install_settings_observers(&self) {
        let Some(settings) = gtk::Settings::default() else {
            return;
        };
        let mut handlers = Vec::with_capacity(3);
        let weak = self.downgrade();
        handlers.push(settings.connect_gtk_theme_name_notify(move |_| {
            if let Some(obj) = weak.upgrade() {
                obj.handle_theme_change();
            }
        }));
        let weak = self.downgrade();
        handlers.push(settings.connect_gtk_font_name_notify(move |_| {
            if let Some(obj) = weak.upgrade() {
                obj.handle_text_environment_change();
            }
        }));
        let weak = self.downgrade();
        handlers.push(settings.connect_gtk_xft_dpi_notify(move |_| {
            if let Some(obj) = weak.upgrade() {
                obj.handle_text_environment_change();
            }
        }));
        self.imp().settings_handlers.replace(handlers);
        self.imp().settings.replace(Some(settings));
    }

    fn apply_event(
        &self, event: RenderEvent, now_us: i64, before: &DrawState, allow_queue: bool,
        in_tick: bool,
    ) {
        let previous_state = self.imp().render_state.get();
        let reduction = reduce(previous_state, event);
        self.imp().render_state.set(reduction.state);
        let manual_scrolling = matches!(reduction.state, RenderState::ManualScroll { .. });
        if self.imp().manual_scrolling.replace(manual_scrolling) != manual_scrolling {
            self.notify_manual_scrolling();
        }
        self.apply_dirty(reduction.effects.dirty);
        let prepared = self.execute_batch(reduction.effects.batch, now_us);
        if !matches!(
            reduction.effects.batch,
            BatchPlan::Prepare { .. } | BatchPlan::Rebuild { .. }
        ) && !(reduction.effects.dirty & (DirtyFlags::POSITION | DirtyFlags::VIEWPORT))
            .is_empty()
        {
            self.ensure_scene(now_us);
        }
        if in_tick && !reduction.effects.sources.frame_tick {
            self.imp().scheduler.borrow_mut().mark_tick_finished();
        }
        self.reconcile_sources(reduction.effects.sources, now_us);
        self.queue_from_effects(reduction.effects, before, now_us, allow_queue, in_tick);

        if let Some(generation) = prepared {
            let frame = self.frame_status(now_us);
            self.apply_event(
                RenderEvent::Prepared {
                    generation,
                    playback: effective_playback(self.imp().anchor.get()),
                    frame,
                },
                now_us,
                before,
                allow_queue,
                false,
            );
        }
    }

    fn apply_dirty(&self, dirty: DirtyFlags) {
        if !(dirty & DirtyFlags::STATIC_SCENE).is_empty() {
            self.imp().cache.borrow_mut().invalidate_batch();
        }
    }

    fn execute_batch(&self, plan: BatchPlan, now_us: i64) -> Option<u64> {
        match plan {
            BatchPlan::None => None,
            BatchPlan::Cancel => {
                self.imp().cache.borrow_mut().invalidate_batch();
                None
            }
            BatchPlan::DeferRebuild { .. } => None,
            BatchPlan::Prepare { generation } | BatchPlan::Rebuild { generation } => {
                let (width, height) = self.imp().allocation.get();
                if width <= 0 || height <= 0 || self.imp().document.borrow().is_none() {
                    return None;
                }
                let preserve_transition = self.imp().transition.borrow().is_some();
                self.rebuild_cache();
                if !preserve_transition {
                    self.sample_frame(now_us, false);
                }
                self.ensure_scene(now_us);
                match self.imp().render_state.get() {
                    RenderState::Preparing {
                        generation: preparing,
                    } => Some(preparing),
                    _ if matches!(plan, BatchPlan::Prepare { .. }) => Some(generation),
                    _ => None,
                }
            }
        }
    }

    fn rebuild_cache(&self) {
        let (width, _) = self.imp().allocation.get();
        let document = self.imp().document.borrow().clone();
        let Some(document) = document.filter(|_| width > 0) else {
            self.imp().cache.borrow_mut().clear();
            return;
        };
        let style = self.imp().style.borrow().clone();
        let context = self.pango_context();
        let timeline = self.imp().timeline.borrow();
        self.imp().cache.borrow_mut().rebuild(CacheBuild {
            document: &document,
            timeline: &timeline,
            context: &context,
            width: layout_width(width),
            style: &style,
            visibility: self.imp().visibility.get(),
            stamp: self.cache_stamp(),
        });
    }

    fn cache_stamp(&self) -> CacheStamp {
        CacheStamp {
            document_generation: self.imp().document_generation.get(),
            width_generation: self.imp().width_generation.get(),
            style_generation: self.imp().style_generation.get(),
            visibility_generation: self.imp().visibility_generation.get(),
            environment_generation: self.imp().environment_generation.get(),
        }
    }

    fn ensure_scene(&self, now_us: i64) {
        let (width, height) = self.imp().allocation.get();
        let Some(document) = self
            .imp()
            .document
            .borrow()
            .clone()
            .filter(|_| width > 0 && height > 0)
        else {
            return;
        };
        if self.imp().cache.borrow().layout().is_none() {
            return;
        }
        let style = self.imp().style.borrow().clone();
        let context = self.pango_context();
        let timeline = self.imp().timeline.borrow();
        let scroll = self.display_scroll(now_us);
        let gap_phase = self.gap_phase();
        let gap_expansion = gap_phase.map(GapPhase::expansion).unwrap_or(0.0);
        let gap_layout_animating = gap_phase.is_some() && gap_expansion < 0.999;
        let mut cache = self.imp().cache.borrow_mut();
        cache.set_gap_expansion(self.imp().gap_line.get(), gap_expansion);
        let update = cache.ensure_viewport(
            CacheBuild {
                document: &document,
                timeline: &timeline,
                context: &context,
                width: layout_width(width),
                style: &style,
                visibility: self.imp().visibility.get(),
                stamp: self.cache_stamp(),
            },
            scroll,
            height as f32,
            style.overscan,
            self.viewport_anchor(scroll),
        );
        drop(timeline);
        drop(cache);
        self.apply_scroll_correction(update.scroll_correction);
        self.update_gap_scroll(gap_expansion);
        self.imp()
            .visible_range
            .set((update.visible.start, update.visible.end));
        let mut cache = self.imp().cache.borrow_mut();
        let visible = update.visible;
        if gap_layout_animating {
            cache.invalidate_batch();
            drop(cache);
            if self.has_click_transition() {
                self.realign_click_transition();
            }
            return;
        }
        let viewport = gtk::graphene::Rect::new(
            0.0,
            self.display_scroll(now_us),
            width as f32,
            height as f32,
        );
        let renderer = self.native().and_then(|native| native.renderer());
        let allow_texture_bake = self.imp().transition.borrow().is_none()
            && now_us.saturating_sub(self.imp().last_allocation_change_us.get()) >= 150_000;
        cache.prepare_batch(BatchPrepare {
            visible,
            current_line: self.imp().current_line.get(),
            scale_generation: self.imp().scale_generation.get(),
            scale_factor: gtk::prelude::WidgetExt::scale_factor(self) as f64,
            manual_scroll: self.is_manual_scroll(),
            allow_texture_bake,
            viewport: &viewport,
            renderer: renderer.as_ref(),
        });
        drop(cache);
        if self.has_click_transition() {
            self.realign_click_transition();
        }
    }

    fn apply_scroll_correction(&self, correction: f32) {
        if !correction.is_finite() || correction == 0.0 {
            return;
        }
        if let Some(transition) = self.imp().transition.borrow_mut().as_mut() {
            transition.shift(correction);
        }
        if let Some(origin) = self.imp().gap_origin_scroll.get() {
            self.imp()
                .gap_origin_scroll
                .set(Some(self.clamp_scroll(origin + correction)));
        }
        self.imp()
            .scroll_offset
            .set(self.clamp_scroll(self.imp().scroll_offset.get() + correction));
    }

    fn update_gap_scroll(&self, expansion: f32) {
        let Some(line) = self.imp().gap_line.get() else {
            return;
        };
        let target = self.auto_scroll_for(Some(line));
        let origin = self
            .imp()
            .gap_origin_scroll
            .get()
            .unwrap_or_else(|| self.imp().scroll_offset.get());
        let progress = smoothstep01(expansion);
        self.imp()
            .scroll_offset
            .set(self.clamp_scroll(origin + (target - origin) * progress));
        if expansion >= 0.999 {
            self.imp().gap_origin_scroll.set(None);
            self.imp().scroll_offset.set(target);
        }
    }

    fn reset_navigation(&self) {
        self.imp().position.set(LyricTime::ZERO);
        self.imp().next_event.set(None);
        self.imp().current_line.set(None);
        self.imp().focus_line.set(None);
        self.imp().visible_range.set((0, 0));
        self.imp().word_animation.set(false);
        self.imp().gap.set(None);
        self.imp().gap_line.set(None);
        self.imp().gap_origin_scroll.set(None);
        self.imp().transition.borrow_mut().take();
        self.imp().scroll_offset.set(0.0);
    }

    fn sample_frame(&self, now_us: i64, animate_line_change: bool) {
        self.finish_interaction_animations(now_us);
        self.finish_transition(now_us);
        if !animate_line_change {
            discard_playback_transition(&mut self.imp().transition.borrow_mut());
        }
        let anchor = self.imp().anchor.get();
        let position = anchor.position_at(now_us);
        let timeline = self.imp().timeline.borrow();
        let frame = timeline.frame_at(position);
        drop(timeline);
        let base_current = frame
            .active_line
            .as_ref()
            .map(|active| active.line.timeline_index);
        let gap_display = self.display_gap(&frame);
        let gap_range = gap_display.map(|(_, range)| range);
        let gap = gap_range.filter(|range| GapPhase::at(*range, position, false).is_some());
        let gap_line = gap.and_then(|_| gap_display.map(|(line, _)| line));
        let current = gap.is_none().then_some(base_current).flatten();
        let gap_focus = gap_line
            .or(base_current)
            .or_else(|| frame.previous_line.as_ref().map(|line| line.timeline_index));
        let focus = if gap.is_some() {
            gap_focus
        } else {
            current
                .or_else(|| frame.previous_line.as_ref().map(|line| line.timeline_index))
                .or_else(|| frame.next_line.as_ref().map(|line| line.timeline_index))
        };
        let word_animation = gap.is_none()
            && frame.active_line.as_ref().is_some_and(|active| {
                active.active_segments.iter().any(|segment| {
                    self.lane_is_visible(active.line.timeline_index, segment.lane_index)
                        && segment
                            .range
                            .end
                            .is_some_and(|end| end > position && end > segment.range.start)
                })
            });
        let next_gap_event = gap_range
            .filter(|range| range.start > position)
            .map(|range| range.start);

        let entering_gap = self.imp().gap.get().is_none() && gap.is_some();
        let leaving_gap = self.imp().gap.get().is_some() && gap.is_none();
        if entering_gap {
            let displayed = self.display_scroll(now_us);
            discard_playback_transition(&mut self.imp().transition.borrow_mut());
            self.imp().scroll_offset.set(displayed);
            self.imp().gap_origin_scroll.set(Some(displayed));
        } else if leaving_gap {
            self.imp().gap_origin_scroll.set(None);
        }

        self.imp().position.set(position);
        self.imp().next_event.set(earlier_time(
            self.next_media_event(position),
            next_gap_event,
        ));
        self.imp().current_line.set(current);
        self.imp().gap.set(gap);
        self.imp().gap_line.set(gap_line);
        self.imp().word_animation.set(word_animation);
        let click_entry = click_entry_for_focus(self.imp().pending_click.get(), focus);
        let click_handled = self.change_focus(focus, now_us, animate_line_change, click_entry);
        if click_entry.is_some() && click_handled {
            self.imp().pending_click.set(None);
        }
    }

    fn display_gap(&self, frame: &TimelineFrame) -> Option<(usize, TimeRange)> {
        let next = frame.next_line.as_ref()?;
        let previous = frame
            .active_line
            .as_ref()
            .map(|active| &active.line)
            .or(frame.previous_line.as_ref());
        let range = gap_before_range(previous, next)?;
        Some((next.timeline_index, range))
    }

    fn lane_is_visible(&self, line_index: usize, lane_index: usize) -> bool {
        let source = self
            .imp()
            .timeline
            .borrow()
            .lines()
            .nth(line_index)
            .cloned();
        let Some(source) = source else {
            return false;
        };
        self.imp()
            .document
            .borrow()
            .as_ref()
            .and_then(|document| document.tracks.get(source.track_index))
            .and_then(|track| track.lines.get(source.line_index))
            .and_then(|line| line.lanes.get(lane_index))
            .is_some_and(|lane| {
                self.imp()
                    .visibility
                    .get()
                    .is_visible(slot_for_kind(&lane.kind))
                    && !lane.text.is_empty()
            })
    }

    fn change_focus(
        &self, focus: Option<usize>, now_us: i64, animate: bool, click_entry_offset: Option<f32>,
    ) -> bool {
        let old = self.imp().focus_line.replace(focus);
        if let Some(entry_offset) = click_entry_offset {
            let to_scroll = self.auto_scroll_for_line(focus);
            let style = self.imp().style.borrow();
            let transition = (!self.imp().reduced_motion.get()
                && style.transition.duration_us > 0
                && entry_offset.is_finite()
                && entry_offset.abs() > 0.5)
                .then(|| LineTransition::for_click(now_us, to_scroll, focus, entry_offset, &style));
            self.imp().scroll_offset.set(to_scroll);
            self.imp().transition.replace(transition);
            return true;
        }
        if self.imp().gap.get().is_some() {
            return false;
        }
        if self
            .imp()
            .transition
            .borrow()
            .as_ref()
            .is_some_and(LineTransition::is_click)
        {
            return false;
        }
        if self.is_manual_scroll() {
            return false;
        }

        let from_scroll = self.display_scroll(now_us);
        let to_scroll = self.auto_scroll_for(focus);
        if old == focus {
            if !animate {
                self.imp().transition.borrow_mut().take();
                self.imp().scroll_offset.set(to_scroll);
                return false;
            }
            if self
                .imp()
                .transition
                .borrow()
                .as_ref()
                .is_some_and(|transition| transition.targets(to_scroll))
            {
                return false;
            }
            if self.can_animate_scroll(from_scroll, to_scroll) {
                self.imp().scroll_offset.set(from_scroll);
                self.start_or_retarget_transition(now_us, from_scroll, to_scroll, focus, focus);
            } else {
                self.imp().transition.borrow_mut().take();
                self.imp().scroll_offset.set(to_scroll);
            }
            return false;
        }

        let adjacent = old
            .zip(focus)
            .is_some_and(|(old, next)| old.abs_diff(next) <= 1);
        let continues_existing = animate
            && adjacent
            && self
                .imp()
                .transition
                .borrow()
                .as_ref()
                .is_some_and(|transition| transition.targets(to_scroll));
        if continues_existing {
            return false;
        }
        if animate && adjacent && self.can_animate_scroll(from_scroll, to_scroll) {
            self.imp().scroll_offset.set(from_scroll);
            self.start_or_retarget_transition(now_us, from_scroll, to_scroll, old, focus);
        } else {
            self.imp().transition.borrow_mut().take();
            self.imp().scroll_offset.set(to_scroll);
        }
        false
    }

    fn start_or_retarget_transition(
        &self, now_us: i64, from_scroll: f32, to_scroll: f32, from_focus: Option<usize>,
        to_focus: Option<usize>,
    ) {
        let style = self.imp().style.borrow();
        let mut transition = self.imp().transition.borrow_mut();
        if let Some(transition) = transition.as_mut() {
            if transition.is_click() {
                return;
            }
            transition.retarget(now_us, to_scroll, to_focus, &style);
        } else {
            *transition = Some(LineTransition::new(
                now_us,
                from_scroll,
                to_scroll,
                from_focus,
                to_focus,
                &style,
            ));
        }
    }

    fn center_focus(&self) {
        let scroll = self.auto_scroll_for(self.imp().focus_line.get());
        self.imp().scroll_offset.set(scroll);
    }

    fn viewport_anchor(&self, scroll: f32) -> ViewportAnchor {
        if self.is_manual_scroll() {
            ViewportAnchor::ScrollOffset(scroll)
        } else if let Some(line) = self.imp().gap_line.get() {
            ViewportAnchor::FocusGap(line)
        } else if let Some(line) = self.imp().focus_line.get() {
            ViewportAnchor::FocusLine(line)
        } else {
            ViewportAnchor::ScrollOffset(scroll)
        }
    }

    fn line_offset_from_viewport_center(&self, line: usize, now_us: i64) -> Option<f32> {
        let (_, height) = self.imp().allocation.get();
        if height <= 0 {
            return None;
        }
        let line_center = self
            .imp()
            .cache
            .borrow()
            .layout()
            .and_then(|layout| layout.line_center(line))?;
        let scroll = self.display_scroll(now_us);
        let line_motion = self
            .imp()
            .transition
            .borrow()
            .as_ref()
            .map(|transition| transition.motion(now_us).line_offset(line))
            .unwrap_or(0.0);
        let offset =
            visual_line_offset_from_center(line_center, scroll, line_motion, height as f32);
        offset.is_finite().then_some(offset)
    }

    fn refresh_pending_click(&self, now_us: i64) {
        let Some(mut pending) = self.imp().pending_click.get() else {
            return;
        };
        let Some(entry_offset) = self.line_offset_from_viewport_center(pending.line, now_us) else {
            return;
        };
        pending.entry_offset = entry_offset;
        self.imp().pending_click.set(Some(pending));
    }

    fn has_click_transition(&self) -> bool {
        self.imp()
            .transition
            .borrow()
            .as_ref()
            .is_some_and(LineTransition::is_click)
    }

    fn realign_click_transition(&self) {
        let target = self.auto_scroll_for_line(self.imp().focus_line.get());
        let mut transition = self.imp().transition.borrow_mut();
        let Some(transition) = transition
            .as_mut()
            .filter(|transition| transition.is_click())
        else {
            return;
        };
        transition.shift(target - transition.to_scroll);
        self.imp().scroll_offset.set(target);
    }

    fn auto_scroll_for_line(&self, line: Option<usize>) -> f32 {
        let (_, height) = self.imp().allocation.get();
        let target = self
            .imp()
            .cache
            .borrow()
            .layout()
            .and_then(|layout| line.and_then(|index| layout.line_center(index)))
            .map(|center| center - height.max(0) as f32 * 0.5)
            .unwrap_or(0.0);
        self.clamp_scroll(target)
    }

    fn auto_scroll_for(&self, line: Option<usize>) -> f32 {
        let (_, height) = self.imp().allocation.get();
        let target = self
            .imp()
            .cache
            .borrow()
            .layout()
            .and_then(|layout| {
                line.and_then(|index| {
                    if self.imp().gap_line.get() == Some(index) && self.imp().gap.get().is_some() {
                        layout.gap_center(index)
                    } else {
                        layout.line_center(index)
                    }
                })
            })
            .map(|center| center - height.max(0) as f32 * 0.5)
            .unwrap_or(0.0);
        self.clamp_scroll(target)
    }

    fn clamp_scroll(&self, value: f32) -> f32 {
        if !value.is_finite() {
            return 0.0;
        }
        let (_, height) = self.imp().allocation.get();
        let half = height.max(0) as f32 * 0.5;
        let total = self
            .imp()
            .cache
            .borrow()
            .layout()
            .map(|layout| layout.total_height())
            .unwrap_or(0.0);
        let minimum = -half;
        let maximum = (total - half).max(minimum);
        value.clamp(minimum, maximum)
    }

    fn can_animate_scroll(&self, from: f32, to: f32) -> bool {
        let (_, height) = self.imp().allocation.get();
        self.imp().anchor.get().is_advancing()
            && !self.imp().reduced_motion.get()
            && self.imp().style.borrow().transition.duration_us > 0
            && (to - from).abs() > 0.5
            && (to - from).abs() <= height.max(1) as f32 * 1.5
    }

    fn display_scroll(&self, _now_us: i64) -> f32 {
        if self.is_manual_scroll() {
            return self.imp().scroll_offset.get();
        }
        self.imp()
            .transition
            .borrow()
            .as_ref()
            .map(|transition| transition.to_scroll)
            .unwrap_or_else(|| self.imp().scroll_offset.get())
    }

    fn transition_progress(&self, now_us: i64) -> f32 {
        self.imp()
            .transition
            .borrow()
            .as_ref()
            .map(|transition| transition.visual_progress(now_us))
            .unwrap_or(1.0)
    }

    fn finish_transition(&self, now_us: i64) {
        let target = {
            let mut slot = self.imp().transition.borrow_mut();
            let Some(transition) = slot.as_mut() else {
                return;
            };
            if !transition.finished(now_us) {
                transition.retain_active(now_us);
                return;
            }
            let target = transition.to_scroll;
            slot.take();
            target
        };
        self.imp().scroll_offset.set(target);
    }

    fn animation_reasons(&self, now_us: i64) -> AnimationReasons {
        let mut reasons = AnimationReasons::NONE;
        if self.imp().word_animation.get() {
            reasons.insert(AnimationReasons::WORD_PROGRESS);
        }
        if self
            .imp()
            .transition
            .borrow()
            .as_ref()
            .is_some_and(|transition| !transition.finished(now_us))
        {
            reasons.insert(AnimationReasons::LINE_TRANSITION);
        }
        if self.imp().gap.get().is_some() && !self.imp().reduced_motion.get() {
            reasons.insert(AnimationReasons::GAP_PULSE);
        }
        if self.interaction_animating(now_us) {
            reasons.insert(AnimationReasons::INTERACTION);
        }
        reasons
    }

    fn interaction_animating(&self, now_us: i64) -> bool {
        let style = self.imp().style.borrow();
        let hover = self
            .imp()
            .hover
            .get()
            .is_some_and(|hover| !hover.finished(style.interaction.hover_duration_us, now_us));
        let activation = self.imp().activation.get().is_some_and(|activation| {
            animation_progress(
                activation.started_us,
                style.interaction.activation_duration_us,
                now_us,
            ) < 1.0
        });
        hover || activation
    }

    fn gap_phase(&self) -> Option<GapPhase> {
        GapPhase::at(
            self.imp().gap.get()?,
            self.imp().position.get(),
            self.imp().reduced_motion.get(),
        )
    }

    fn frame_status(&self, now_us: i64) -> FrameStatus {
        FrameStatus::new(
            self.imp().position.get(),
            self.imp().next_event.get(),
            self.animation_reasons(now_us),
        )
    }

    fn next_media_event(&self, position: LyricTime) -> Option<LyricTime> {
        let rate = self.imp().anchor.get().rate;
        let timeline = self.imp().timeline.borrow();
        if rate > 0.0 {
            timeline.next_event_after(position)
        } else if rate < 0.0 {
            let before = LyricTime::from_micros(position.as_micros().saturating_sub(1));
            timeline.previous_event_at_or_before(before)
        } else {
            None
        }
    }

    fn reconcile_sources(&self, intents: SourceIntents, now_us: i64) {
        if intents.frame_tick {
            self.imp()
                .scheduler
                .borrow_mut()
                .ensure_tick(self, |obj, clock| obj.on_tick(clock));
        } else {
            self.imp().scheduler.borrow_mut().stop_tick();
        }
        match intents.wakeup {
            Some(Wakeup::MediaTime(target)) => self.schedule_media_wakeup(target, now_us),
            Some(Wakeup::FrameTime(deadline_us)) => self.schedule_frame_wakeup(deadline_us, now_us),
            None => self.imp().scheduler.borrow_mut().cancel_wakeup(),
        }
    }

    fn schedule_media_wakeup(&self, target: LyricTime, now_us: i64) {
        let anchor = self.imp().anchor.get();
        let Some(delay) = media_delay(&anchor, now_us, target) else {
            self.imp().scheduler.borrow_mut().cancel_wakeup();
            return;
        };
        let key = WakeupKey::Media {
            target,
            serial: anchor.serial,
        };
        self.schedule_wakeup(key, delay);
    }

    fn schedule_frame_wakeup(&self, deadline_us: i64, now_us: i64) {
        let delay = frame_delay(now_us, deadline_us);
        self.schedule_wakeup(WakeupKey::ManualScroll { deadline_us }, delay);
    }

    fn schedule_wakeup(&self, key: WakeupKey, delay: Duration) {
        let weak = self.downgrade();
        self.imp()
            .scheduler
            .borrow_mut()
            .schedule(key, delay, move || {
                if let Some(obj) = weak.upgrade() {
                    obj.on_wakeup(key);
                }
            });
    }

    fn on_tick(&self, clock: &gdk::FrameClock) -> glib::ControlFlow {
        let now_us = clock.frame_time();
        if !self.animation_frame_due(now_us) {
            return glib::ControlFlow::Continue;
        }
        let before = self.draw_state(now_us);
        self.sample_frame(now_us, true);
        self.apply_event(
            RenderEvent::Tick {
                anchor: self.imp().anchor.get(),
                frame_time_us: now_us,
                next_event: self.imp().next_event.get(),
                animations: self.animation_reasons(now_us),
            },
            now_us,
            &before,
            true,
            true,
        );
        if self.imp().render_state.get().needs_frame_tick() {
            glib::ControlFlow::Continue
        } else {
            glib::ControlFlow::Break
        }
    }

    fn animation_frame_due(&self, now_us: i64) -> bool {
        let frames_per_second = self.max_animation_fps();
        if frames_per_second == 0 {
            return true;
        }
        let interval = 1_000_000 / i64::from(frames_per_second);
        let previous = self.imp().last_animation_frame_us.get();
        if now_us.saturating_sub(previous) < interval {
            return false;
        }
        self.imp().last_animation_frame_us.set(now_us);
        true
    }

    fn on_wakeup(&self, key: WakeupKey) {
        self.imp().scheduler.borrow_mut().mark_wakeup_fired(key);
        let now_us = self.frame_time_us();
        match key {
            WakeupKey::Media { serial, .. } if serial == self.imp().anchor.get().serial => {
                let before = self.draw_state(now_us);
                self.sample_frame(now_us, true);
                self.apply_event(
                    RenderEvent::Wakeup {
                        anchor: self.imp().anchor.get(),
                        frame_time_us: now_us,
                        next_event: self.imp().next_event.get(),
                        animations: self.animation_reasons(now_us),
                    },
                    now_us,
                    &before,
                    true,
                    false,
                );
            }
            WakeupKey::ManualScroll { .. } => {
                self.finish_manual_scroll(now_us);
            }
            WakeupKey::Media { .. } => {
                self.reconcile_sources(self.imp().render_state.get().source_intents(), now_us);
            }
        }
    }

    fn begin_manual_scroll(&self, now_us: i64) {
        if !self.is_manual_scroll() {
            let current = self.display_scroll(now_us);
            self.imp().transition.borrow_mut().take();
            self.imp().scroll_offset.set(current);
        }
    }

    fn update_manual_deadline(&self, now_us: i64, before: &DrawState) {
        let deadline = now_us.saturating_add(MANUAL_SCROLL_HOLD_US);
        self.apply_event(
            RenderEvent::UserScroll {
                return_deadline_us: deadline,
                advancing: self.imp().anchor.get().is_advancing(),
            },
            now_us,
            before,
            true,
            false,
        );
    }

    fn finish_manual_scroll(&self, now_us: i64) {
        let before = self.draw_state(now_us);
        self.sample_frame(now_us, false);
        self.apply_event(
            RenderEvent::ManualScrollTimeout {
                frame_time_us: now_us,
                playback: effective_playback(self.imp().anchor.get()),
                frame: self.frame_status(now_us),
            },
            now_us,
            &before,
            false,
            false,
        );
        if self.is_manual_scroll() {
            return;
        }
        self.resume_auto_scroll(now_us);
        self.ensure_scene(now_us);
        self.apply_event(
            RenderEvent::AnimationsChanged {
                playback: effective_playback(self.imp().anchor.get()),
                frame: self.frame_status(now_us),
            },
            now_us,
            &before,
            false,
            false,
        );
        if self.draw_state(now_us) != before {
            self.queue_draw();
        }
    }

    fn resume_auto_scroll(&self, now_us: i64) {
        if self.has_click_transition() {
            return;
        }
        let from_scroll = self.imp().scroll_offset.get();
        let to_scroll = self.auto_scroll_for(self.imp().focus_line.get());
        if self.can_animate_scroll(from_scroll, to_scroll) {
            let focus = self.imp().focus_line.get();
            self.start_or_retarget_transition(now_us, from_scroll, to_scroll, focus, focus);
        } else {
            self.imp().scroll_offset.set(to_scroll);
            self.imp().transition.borrow_mut().take();
        }
    }

    fn scroll_by(&self, delta: f32) {
        if !delta.is_finite() || delta == 0.0 {
            return;
        }
        let now_us = self.frame_time_us();
        let before = self.draw_state(now_us);
        self.begin_manual_scroll(now_us);
        self.imp()
            .scroll_offset
            .set(self.clamp_scroll(self.imp().scroll_offset.get() + delta));
        self.update_manual_deadline(now_us, &before);
        self.ensure_scene(now_us);
        self.refresh_hover_from_pointer();
        if self.draw_state(now_us) != before {
            self.queue_draw();
        }
    }

    fn drag_begin(&self) {
        let now_us = self.frame_time_us();
        self.imp()
            .drag_start_scroll
            .set(self.display_scroll(now_us));
        self.imp().suppress_click.set(false);
    }

    fn drag_update(&self, offset_y: f64) {
        if offset_y.abs() < DRAG_THRESHOLD {
            return;
        }
        let now_us = self.frame_time_us();
        let before = self.draw_state(now_us);
        self.imp().suppress_click.set(true);
        self.begin_manual_scroll(now_us);
        self.imp()
            .scroll_offset
            .set(self.clamp_scroll(self.imp().drag_start_scroll.get() - offset_y as f32));
        self.update_manual_deadline(now_us, &before);
        self.ensure_scene(now_us);
        self.refresh_hover_from_pointer();
        if self.draw_state(now_us) != before {
            self.queue_draw();
        }
    }

    fn interaction_end(&self) {
        if !self.is_manual_scroll() {
            return;
        }
        let now_us = self.frame_time_us();
        let before = self.draw_state(now_us);
        self.update_manual_deadline(now_us, &before);
        self.refresh_hover_from_pointer();
    }

    fn is_manual_scroll(&self) -> bool {
        matches!(
            self.imp().render_state.get(),
            RenderState::ManualScroll { .. }
        )
    }

    fn line_hit_at(&self, x: f64, y: f64) -> Option<(usize, i64)> {
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        let now_us = self.frame_time_us();
        let content_y = y as f32 + self.display_scroll(now_us);
        let line_motion = self
            .imp()
            .transition
            .borrow()
            .as_ref()
            .map(|transition| transition.motion(now_us));
        let cache = self.imp().cache.borrow();
        let layout = cache.layout()?;
        let hit = if let Some(motion) = line_motion {
            let (start, end) = self.imp().visible_range.get();
            (start..end)
                .find_map(|index| {
                    layout.hit_test_line(index, content_y - motion.line_offset(index))
                })
                .or_else(|| layout.hit_test(content_y))
        } else {
            layout.hit_test(content_y)
        }?;
        Some((hit.timeline_index, hit.start.as_micros()))
    }

    fn update_hover_at(&self, x: f64, y: f64) {
        self.imp().pointer_position.set(Some((x, y)));
        let line = self.line_hit_at(x, y).map(|(line, _)| line);
        self.set_hover_line(line);
    }

    fn clear_hover(&self) {
        self.imp().pointer_position.set(None);
        self.set_hover_line(None);
    }

    fn refresh_hover_from_pointer(&self) {
        if let Some((x, y)) = self.imp().pointer_position.get() {
            let line = self.line_hit_at(x, y).map(|(line, _)| line);
            self.set_hover_line(line);
        }
    }

    fn set_hover_line(&self, line: Option<usize>) {
        let now_us = self.frame_time_us();
        let before = self.draw_state(now_us);
        if let Some(line) = line
            && self
                .imp()
                .hover
                .get()
                .is_some_and(|hover| hover.line == line && hover.to >= 1.0)
        {
            return;
        }
        if line.is_none() && self.imp().hover.get().is_some_and(|hover| hover.to <= 0.0) {
            return;
        }
        let current = self.hover_visual(now_us);
        let next = match line {
            Some(line) => HoverAnimation {
                line,
                started_us: now_us,
                from: current
                    .filter(|(current, _)| *current == line)
                    .map(|(_, value)| value)
                    .unwrap_or(0.0),
                to: 1.0,
            },
            None => {
                let Some((line, value)) = current else {
                    return;
                };
                HoverAnimation {
                    line,
                    started_us: now_us,
                    from: value,
                    to: 0.0,
                }
            }
        };
        self.imp().hover.set(Some(next));
        self.refresh_interaction_animation(now_us, &before);
    }

    fn start_activation(&self, line: usize) {
        let now_us = self.frame_time_us();
        let before = self.draw_state(now_us);
        self.imp().activation.set(Some(ActivationAnimation {
            line,
            started_us: now_us,
        }));
        let pending_click = (self.imp().focus_line.get() != Some(line))
            .then(|| {
                self.line_offset_from_viewport_center(line, now_us)
                    .map(|entry_offset| PendingClickTransition { line, entry_offset })
            })
            .flatten();
        self.imp().pending_click.set(pending_click);
        self.refresh_interaction_animation(now_us, &before);
    }

    fn refresh_interaction_animation(&self, now_us: i64, before: &DrawState) {
        self.apply_event(
            RenderEvent::AnimationsChanged {
                playback: effective_playback(self.imp().anchor.get()),
                frame: self.frame_status(now_us),
            },
            now_us,
            before,
            true,
            false,
        );
        if self.draw_state(now_us) != *before {
            self.queue_draw();
        }
    }

    fn hover_visual(&self, now_us: i64) -> Option<(usize, f32)> {
        let hover = self.imp().hover.get()?;
        let duration = self.imp().style.borrow().interaction.hover_duration_us;
        Some((hover.line, hover.value(duration, now_us).clamp(0.0, 1.0)))
    }

    fn activation_visual(&self, now_us: i64) -> Option<(usize, f32)> {
        let activation = self.imp().activation.get()?;
        let duration = self.imp().style.borrow().interaction.activation_duration_us;
        Some((
            activation.line,
            animation_progress(activation.started_us, duration, now_us),
        ))
    }

    fn finish_interaction_animations(&self, now_us: i64) {
        let style = self.imp().style.borrow();
        if self.imp().hover.get().is_some_and(|hover| {
            hover.to <= 0.0 && hover.finished(style.interaction.hover_duration_us, now_us)
        }) {
            self.imp().hover.set(None);
        }
        if self.imp().activation.get().is_some_and(|activation| {
            animation_progress(
                activation.started_us,
                style.interaction.activation_duration_us,
                now_us,
            ) >= 1.0
        }) {
            self.imp().activation.set(None);
        }
    }

    fn activate_at(&self, x: f64, y: f64) {
        if self.imp().suppress_click.replace(false) || !x.is_finite() || !y.is_finite() {
            return;
        }
        let Some((timeline_index, start_us)) = self.line_hit_at(x, y) else {
            return;
        };
        self.start_activation(timeline_index);
        self.emit_by_name::<()>("line-activated", &[&start_us]);
    }

    fn render_snapshot(&self, snapshot: &gtk::Snapshot) {
        let (width, height) = self.imp().allocation.get();
        if width <= 0 || height <= 0 || self.imp().cache.borrow().layout().is_none() {
            return;
        }
        let now_us = self.frame_time_us();
        let (start, end) = self.imp().visible_range.get();
        let gap_phase = self.gap_phase();
        let gap_layout_animating = gap_phase.is_some_and(|phase| phase.expansion() < 0.999);
        let style = self.imp().style.borrow();
        let hover = self.hover_visual(now_us);
        let activation = self.activation_visual(now_us);
        let line_motion = self
            .imp()
            .transition
            .borrow()
            .as_ref()
            .map(|transition| transition.motion(now_us));
        let frame = SnapshotFrame {
            position: self.imp().position.get(),
            visible: start..end,
            current_line: self.imp().current_line.get(),
            scroll_offset: self.display_scroll(now_us),

            gap_phase,
            gap_line: self.imp().gap_line.get(),
            gap_layout_animating,
            viewport_width: width as f32,
            viewport_height: height as f32,
            scale_factor: gtk::prelude::WidgetExt::scale_factor(self) as f64,
            scale_generation: self.imp().scale_generation.get(),
            manual_scroll: self.is_manual_scroll(),
            line_motion,
            hover_line: hover.map(|(line, _)| line),
            hover_opacity: hover.map(|(_, opacity)| opacity).unwrap_or(0.0),
            activation_line: activation.map(|(line, _)| line),
            activation_progress: activation.map(|(_, progress)| progress).unwrap_or(1.0),
        };
        snapshot_lyrics(snapshot, &self.imp().cache.borrow(), &style, &frame);
    }

    fn install_controllers(&self) {
        let motion = gtk::EventControllerMotion::new();
        let weak = self.downgrade();
        motion.connect_motion(move |_, x, y| {
            if let Some(obj) = weak.upgrade() {
                obj.update_hover_at(x, y);
            }
        });
        let weak = self.downgrade();
        motion.connect_leave(move |_| {
            if let Some(obj) = weak.upgrade() {
                obj.clear_hover();
            }
        });
        self.add_controller(motion);

        let scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
        let weak = self.downgrade();
        scroll.connect_scroll(move |_, _, dy| {
            if let Some(obj) = weak.upgrade() {
                obj.scroll_by(dy as f32 * SCROLL_STEP);
            }
            glib::Propagation::Stop
        });
        let weak = self.downgrade();
        scroll.connect_scroll_end(move |_| {
            if let Some(obj) = weak.upgrade() {
                obj.interaction_end();
            }
        });
        self.add_controller(scroll);

        let drag = gtk::GestureDrag::new();
        let weak = self.downgrade();
        drag.connect_drag_begin(move |_, _, _| {
            if let Some(obj) = weak.upgrade() {
                obj.drag_begin();
            }
        });
        let weak = self.downgrade();
        drag.connect_drag_update(move |_, _, offset_y| {
            if let Some(obj) = weak.upgrade() {
                obj.drag_update(offset_y);
            }
        });
        let weak = self.downgrade();
        drag.connect_drag_end(move |_, _, _| {
            if let Some(obj) = weak.upgrade() {
                obj.interaction_end();
            }
        });
        self.add_controller(drag);

        let click = gtk::GestureClick::new();
        let weak = self.downgrade();
        click.connect_released(move |_, _, x, y| {
            if let Some(obj) = weak.upgrade() {
                obj.activate_at(x, y);
            }
        });
        self.add_controller(click);
    }

    fn queue_from_effects(
        &self, effects: RenderEffects, before: &DrawState, now_us: i64, allow_queue: bool,
        in_tick: bool,
    ) {
        if !allow_queue || !effects.queue_draw {
            return;
        }
        if in_tick {
            self.queue_draw();
            return;
        }
        let structural = !(effects.dirty
            & (DirtyFlags::LAYOUT
                | DirtyFlags::TEXT_VISUALS
                | DirtyFlags::STATIC_SCENE
                | DirtyFlags::VIEWPORT))
            .is_empty();
        if structural || self.draw_state(now_us) != *before {
            self.queue_draw();
        }
    }

    fn draw_state(&self, now_us: i64) -> DrawState {
        let cache = self.imp().cache.borrow();
        let hover = self.hover_visual(now_us);
        let activation = self.activation_visual(now_us);
        DrawState {
            signature: cache
                .visual_signature(self.imp().current_line.get(), self.imp().position.get()),
            cache_generation: cache.generation(),
            scroll_bits: self.display_scroll(now_us).to_bits(),
            transition_bits: self.transition_progress(now_us).to_bits(),
            gap_phase: self.gap_phase(),
            hover_line: hover.map(|(line, _)| line),
            hover_opacity_bits: hover.map(|(_, opacity)| opacity).unwrap_or(0.0).to_bits(),
            activation_line: activation.map(|(line, _)| line),
            activation_progress_bits: activation
                .map(|(_, progress)| progress)
                .unwrap_or(1.0)
                .to_bits(),
        }
    }

    fn frame_time_us(&self) -> i64 {
        glib::monotonic_time()
    }
}

impl Default for LyricsView {
    fn default() -> Self {
        Self::new()
    }
}

fn animation_progress(started_us: i64, duration_us: i64, now_us: i64) -> f32 {
    if duration_us <= 0 {
        return 1.0;
    }
    (now_us.saturating_sub(started_us) as f32 / duration_us as f32).clamp(0.0, 1.0)
}

fn ease_out_cubic(progress: f32) -> f32 {
    1.0 - (1.0 - progress.clamp(0.0, 1.0)).powi(3)
}

fn selected_timeline(document: &LyricsDocument, track_index: Option<usize>) -> Timeline {
    match track_index {
        Some(track_index) => {
            Timeline::from_document_track(document, track_index).unwrap_or_default()
        }
        None => Timeline::new(document),
    }
}

fn effective_playback(anchor: PlaybackAnchor) -> PlaybackState {
    if anchor.is_advancing() {
        PlaybackState::Playing
    } else {
        PlaybackState::Paused
    }
}

fn same_document(left: Option<&Arc<LyricsDocument>>, right: Option<&Arc<LyricsDocument>>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => Arc::ptr_eq(left, right),
        (None, None) => true,
        _ => false,
    }
}

fn layout_width(width: i32) -> f32 {
    if width <= 0 {
        return 0.0;
    }
    let rounded = width
        .saturating_add(LAYOUT_WIDTH_BUCKET / 2)
        .div_euclid(LAYOUT_WIDTH_BUCKET)
        .max(1)
        .saturating_mul(LAYOUT_WIDTH_BUCKET);
    rounded as f32
}

fn allocation_requires_relayout(old: (i32, i32), new: (i32, i32)) -> bool {
    let old_valid = old.0 > 0 && old.1 > 0;
    let new_valid = new.0 > 0 && new.1 > 0;
    old_valid != new_valid || layout_width(old.0) != layout_width(new.0)
}

fn smoothstep01(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    value * value * (3.0 - 2.0 * value)
}

fn earlier_time(left: Option<LyricTime>, right: Option<LyricTime>) -> Option<LyricTime> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

fn bump(generation: &Cell<u64>) {
    generation.set(generation.get().wrapping_add(1));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hover_and_activation_use_ease_out_timelines() {
        let hover = HoverAnimation {
            line: 3,
            started_us: 0,
            from: 0.0,
            to: 1.0,
        };
        assert!(hover.value(200, 100) > 0.5);
        assert!(hover.finished(200, 200));
        assert_eq!(animation_progress(0, 400, 200), 0.5);
        assert!(ease_out_cubic(0.5) > 0.5);
    }

    #[test]
    fn stable_focus_target_does_not_restart_transition() {
        let transition =
            LineTransition::new(100, 20.0, 80.0, Some(0), Some(1), &LyricsStyle::default());
        assert!(transition.targets(80.4));
        assert!(!transition.targets(81.0));
        assert_eq!(transition.legs.len(), 1);
        assert_eq!(transition.visual_progress(100), 0.0);
        assert!(!transition.finished(800_100));
        assert!(transition.finished(1_700_100));
    }

    #[test]
    fn click_transition_starts_at_the_sampled_position_and_discards_current_motion() {
        let style = LyricsStyle::default();
        let mut current = LineTransition::new(0, 0.0, 100.0, Some(0), Some(1), &style);
        current.retarget(200_000, 180.0, Some(2), &style);
        assert_eq!(current.legs.len(), 2);
        assert!(!current.is_click());

        let entry_offset = visual_line_offset_from_center(
            720.0,
            current.to_scroll,
            current.motion(200_000).line_offset(8),
            600.0,
        );
        let clicked = LineTransition::for_click(200_000, 500.0, Some(8), entry_offset, &style);
        assert!(clicked.is_click());
        assert_eq!(clicked.legs.len(), 1);
        assert_eq!(clicked.to_scroll, 500.0);
        assert_eq!(clicked.motion(200_000).line_offset(8), entry_offset);
        assert_eq!(clicked.motion(200_000).line_offset(0), entry_offset);
        assert_eq!(
            clicked.motion(600_000).line_offset(0),
            clicked.motion(600_000).line_offset(8),
            "click scrolling must move the lyric column rigidly without stagger"
        );
        assert_eq!(clicked.motion(1_000_000).line_offset(8), 0.0);

        let clicked_above = LineTransition::for_click(200_000, 500.0, Some(8), -80.0, &style);
        assert_eq!(clicked_above.motion(200_000).line_offset(8), -80.0);
        assert_eq!(clicked_above.motion(1_000_000).line_offset(8), 0.0);

        let mut active = Some(clicked);
        discard_playback_transition(&mut active);
        discard_playback_transition(&mut active);
        assert!(active.is_some_and(|transition| transition.is_click()));
    }

    #[test]
    fn repeated_paused_seek_anchors_do_not_clear_the_click_transition() {
        let style = LyricsStyle::default();
        let mut pending = Some(PendingClickTransition {
            line: 8,
            entry_offset: 132.0,
        });
        let mut transition = Some(LineTransition::new(0, 0.0, 100.0, Some(0), Some(1), &style));

        discard_playback_transition(&mut transition);
        if let Some(entry_offset) = click_entry_for_focus(pending, Some(8)) {
            transition = Some(LineTransition::for_click(
                100_000,
                500.0,
                Some(8),
                entry_offset,
                &style,
            ));
            pending = None;
        }
        assert!(pending.is_none());
        assert!(transition.as_ref().is_some_and(LineTransition::is_click));

        discard_playback_transition(&mut transition);
        assert!(transition.as_ref().is_some_and(LineTransition::is_click));
        assert!(
            transition
                .as_ref()
                .is_some_and(|transition| transition.motion(200_000).line_offset(8) > 0.0)
        );
    }

    #[test]
    fn pending_click_is_consumed_only_when_the_timeline_focus_matches() {
        let pending = Some(PendingClickTransition {
            line: 8,
            entry_offset: 132.0,
        });
        assert_eq!(click_entry_for_focus(pending, Some(8)), Some(132.0));
        assert_eq!(click_entry_for_focus(pending, Some(7)), None);
        assert_eq!(click_entry_for_focus(pending, None), None);
    }

    #[test]
    fn rapid_line_transitions_preserve_every_lines_visual_position() {
        let style = LyricsStyle::default();
        let mut transition = LineTransition::new(0, 0.0, 100.0, Some(0), Some(1), &style);

        assert_transition_handoff_is_continuous(&mut transition, 400_000, 180.0, Some(2), &style);
        assert_eq!(transition.legs.len(), 2);

        assert_transition_handoff_is_continuous(&mut transition, 650_000, 40.0, Some(1), &style);
        assert_eq!(transition.legs.len(), 3);
        assert!(!transition.finished(2_349_999));
        assert!(transition.finished(2_350_000));
    }

    fn assert_transition_handoff_is_continuous(
        transition: &mut LineTransition, now_us: i64, to_scroll: f32, to_focus: Option<usize>,
        style: &LyricsStyle,
    ) {
        let before: Vec<_> = (0..=12)
            .map(|line| transition.motion(now_us).line_offset(line) - transition.to_scroll)
            .collect();
        transition.retarget(now_us, to_scroll, to_focus, style);
        for (line, before) in before.into_iter().enumerate() {
            let after = transition.motion(now_us).line_offset(line) - transition.to_scroll;
            assert!(
                (after - before).abs() <= 0.001,
                "line {line} jumped from {before} to {after} during transition handoff"
            );
        }
    }

    #[test]
    fn reverse_playback_remains_an_advancing_animation_clock() {
        let anchor = PlaybackAnchor::new(
            LyricTime::from_secs(10),
            1_000,
            -1.0,
            PlaybackState::Playing,
            1,
        );
        assert!(anchor.is_advancing());
        assert!(anchor.position_at(501_000) < anchor.media_position);
    }

    #[test]
    fn gap_scroll_progress_is_smooth_and_monotonic() {
        assert_eq!(smoothstep01(0.0), 0.0);
        assert_eq!(smoothstep01(1.0), 1.0);
        assert!(smoothstep01(0.25) < smoothstep01(0.5));
        assert!(smoothstep01(0.5) < smoothstep01(0.75));
    }

    #[test]
    fn height_only_resize_keeps_the_layout_generation_path() {
        assert!(!allocation_requires_relayout((640, 480), (640, 720)));
        assert!(!allocation_requires_relayout((641, 480), (642, 720)));
        assert!(allocation_requires_relayout((640, 0), (640, 720)));
    }

    #[test]
    fn width_bucket_coalesces_per_pixel_resize_churn() {
        assert_eq!(layout_width(640), 640.0);
        assert_eq!(layout_width(641), 640.0);
        assert_eq!(layout_width(643), 640.0);
        assert_eq!(layout_width(644), 648.0);
        assert!(allocation_requires_relayout((643, 480), (644, 480)));
    }

    #[test]
    fn large_seek_threshold_ignores_small_anchor_resync_drift() {
        let expected = LyricTime::from_micros(1_000_000);
        let close = LyricTime::from_micros(1_100_000);
        let far = LyricTime::from_micros(5_000_000);
        assert!(close.as_micros().abs_diff(expected.as_micros()) <= LARGE_SEEK_US as u64);
        assert!(far.as_micros().abs_diff(expected.as_micros()) > LARGE_SEEK_US as u64);
    }

    #[test]
    fn manual_deadline_is_based_on_the_last_interaction() {
        let first = 10_i64.saturating_add(MANUAL_SCROLL_HOLD_US);
        let drag_end = 500_i64.saturating_add(MANUAL_SCROLL_HOLD_US);
        assert!(drag_end > first);
    }
}
