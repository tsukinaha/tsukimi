use std::ops::{
    BitAnd,
    BitAndAssign,
    BitOr,
    BitOrAssign,
    Not,
};

use crate::LyricTime;

/// Whether media time advances from a playback anchor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlaybackState {
    #[default]
    Paused,
    Playing,
}

impl PlaybackState {
    pub const fn is_playing(self) -> bool {
        matches!(self, Self::Playing)
    }
}

/// An absolute relationship between media time and GTK frame-clock time.
///
/// Both time values are microseconds. `serial` identifies discontinuities such
/// as seeks, track changes, rate changes, and explicit clock resynchronization.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaybackAnchor {
    pub media_position: LyricTime,
    pub frame_time_us: i64,
    pub rate: f64,
    pub state: PlaybackState,
    pub serial: u64,
}

impl PlaybackAnchor {
    pub const fn new(
        media_position: LyricTime, frame_time_us: i64, rate: f64, state: PlaybackState, serial: u64,
    ) -> Self {
        Self {
            media_position,
            frame_time_us,
            rate,
            state,
            serial,
        }
    }

    /// Resolves media time directly from this absolute anchor.
    ///
    /// This deliberately does not integrate previous frames, so dropped frames
    /// cannot accumulate clock error. Paused anchors and non-finite rates stay
    /// fixed at `media_position`.
    pub fn position_at(&self, frame_time_us: i64) -> LyricTime {
        if !self.is_advancing() {
            return self.media_position;
        }

        let elapsed_us = frame_time_us.saturating_sub(self.frame_time_us);
        let media_delta_us = scale_microseconds(elapsed_us, self.rate);
        let position_us = self
            .media_position
            .as_micros()
            .saturating_add(media_delta_us);

        LyricTime::from_micros(position_us)
    }

    pub fn is_advancing(&self) -> bool {
        self.state.is_playing() && self.rate.is_finite() && self.rate != 0.0
    }

    /// Re-anchors at `frame_time_us` without introducing a position jump.
    pub fn rebase(&self, frame_time_us: i64, rate: f64, state: PlaybackState, serial: u64) -> Self {
        Self::new(
            self.position_at(frame_time_us),
            frame_time_us,
            rate,
            state,
            serial,
        )
    }
}

fn scale_microseconds(value_us: i64, rate: f64) -> i64 {
    if !rate.is_finite() {
        return 0;
    }

    let scaled = value_us as f64 * rate;
    if scaled >= i64::MAX as f64 {
        i64::MAX
    } else if scaled <= i64::MIN as f64 {
        i64::MIN
    } else {
        scaled.round() as i64
    }
}

/// Independent reasons for which continuous frame updates may be required.
///
/// This is intentionally a tiny local bit mask rather than a dependency on a
/// general-purpose bitflags crate.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct AnimationReasons(u8);

impl AnimationReasons {
    pub const NONE: Self = Self(0);
    pub const WORD_PROGRESS: Self = Self(1 << 0);
    pub const LINE_TRANSITION: Self = Self(1 << 1);
    pub const SCROLL_SETTLE: Self = Self(1 << 2);
    pub const COVER_CROSSFADE: Self = Self(1 << 3);
    pub const BACKGROUND_MOTION: Self = Self(1 << 4);
    pub const USER_GESTURE: Self = Self(1 << 5);
    pub const GAP_PULSE: Self = Self(1 << 6);
    pub const INTERACTION: Self = Self(1 << 7);
    pub const ALL: Self = Self(u8::MAX);

    pub const fn from_bits_truncate(bits: u8) -> Self {
        Self(bits & Self::ALL.0)
    }

    pub const fn bits(self) -> u8 {
        self.0
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }

    pub fn remove(&mut self, other: Self) {
        self.0 &= !other.0;
    }
}

impl std::fmt::Debug for AnimationReasons {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_tuple("AnimationReasons")
            .field(&self.0)
            .finish()
    }
}

impl BitOr for AnimationReasons {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for AnimationReasons {
    fn bitor_assign(&mut self, rhs: Self) {
        self.insert(rhs);
    }
}

impl BitAnd for AnimationReasons {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for AnimationReasons {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for AnimationReasons {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0 & Self::ALL.0)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RenderState {
    #[default]
    Detached,
    Empty,
    Preparing {
        generation: u64,
    },
    Static {
        position: LyricTime,
    },
    Waiting {
        next_event: LyricTime,
    },
    Animating {
        reasons: AnimationReasons,
    },
    Seeking {
        generation: u64,
        target: LyricTime,
    },
    ManualScroll {
        return_deadline_us: i64,
        advancing: bool,
        animating: bool,
    },
}

impl RenderState {
    pub const fn needs_frame_tick(&self) -> bool {
        matches!(
            self,
            Self::Animating { .. }
                | Self::ManualScroll {
                    advancing: true,
                    ..
                }
                | Self::ManualScroll {
                    animating: true,
                    ..
                }
        )
    }

    pub const fn has_document(&self) -> bool {
        !matches!(self, Self::Detached | Self::Empty)
    }

    pub const fn has_ready_scene(&self) -> bool {
        matches!(
            self,
            Self::Static { .. }
                | Self::Waiting { .. }
                | Self::Animating { .. }
                | Self::ManualScroll { .. }
        )
    }

    pub const fn source_intents(&self) -> SourceIntents {
        match self {
            Self::Waiting { next_event } => SourceIntents {
                frame_tick: false,
                wakeup: Some(Wakeup::MediaTime(*next_event)),
            },
            Self::Animating { .. } => SourceIntents {
                frame_tick: true,
                wakeup: None,
            },
            Self::ManualScroll {
                return_deadline_us,
                advancing,
                animating,
            } => SourceIntents {
                frame_tick: *advancing || *animating,
                wakeup: Some(Wakeup::FrameTime(*return_deadline_us)),
            },
            Self::Detached
            | Self::Empty
            | Self::Preparing { .. }
            | Self::Static { .. }
            | Self::Seeking { .. } => SourceIntents::NONE,
        }
    }
}

/// Timeline information needed to choose a render state at one absolute time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameStatus {
    pub position: LyricTime,
    pub next_event: Option<LyricTime>,
    pub animations: AnimationReasons,
}

impl FrameStatus {
    pub const fn new(
        position: LyricTime, next_event: Option<LyricTime>, animations: AnimationReasons,
    ) -> Self {
        Self {
            position,
            next_event,
            animations,
        }
    }

    pub fn from_anchor(
        anchor: &PlaybackAnchor, frame_time_us: i64, next_event: Option<LyricTime>,
        animations: AnimationReasons,
    ) -> Self {
        Self::new(anchor.position_at(frame_time_us), next_event, animations)
    }

    pub fn render_state(self, playback: PlaybackState) -> RenderState {
        if !playback.is_playing() {
            let ui_animations = self.animations
                & (AnimationReasons::LINE_TRANSITION
                    | AnimationReasons::SCROLL_SETTLE
                    | AnimationReasons::USER_GESTURE
                    | AnimationReasons::INTERACTION);
            if !ui_animations.is_empty() {
                return RenderState::Animating {
                    reasons: ui_animations,
                };
            }
            return RenderState::Static {
                position: self.position,
            };
        }

        if !self.animations.is_empty() {
            return RenderState::Animating {
                reasons: self.animations,
            };
        }

        match self.next_event {
            Some(next_event) => RenderState::Waiting { next_event },
            None => RenderState::Static {
                position: self.position,
            },
        }
    }

    pub fn render_state_for_anchor(self, anchor: &PlaybackAnchor) -> RenderState {
        let playback = if anchor.is_advancing() {
            PlaybackState::Playing
        } else {
            PlaybackState::Paused
        };
        self.render_state(playback)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wakeup {
    MediaTime(LyricTime),
    FrameTime(i64),
}

/// Desired scheduler state after a reduction.
///
/// Reconciliation is idempotent: applying the same desired state twice does
/// not request a second GTK source.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SourceIntents {
    pub frame_tick: bool,
    pub wakeup: Option<Wakeup>,
}

impl SourceIntents {
    pub const NONE: Self = Self {
        frame_tick: false,
        wakeup: None,
    };

    pub fn changes_from(self, current: Self) -> SourceChanges {
        let frame_tick = match (current.frame_tick, self.frame_tick) {
            (false, true) => FrameTickChange::Start,
            (true, false) => FrameTickChange::Stop,
            _ => FrameTickChange::Keep,
        };

        let wakeup = if same_wakeup(current.wakeup, self.wakeup) {
            WakeupChange::Keep
        } else {
            match self.wakeup {
                Some(wakeup) => WakeupChange::Schedule(wakeup),
                None => WakeupChange::Cancel,
            }
        };

        SourceChanges { frame_tick, wakeup }
    }
}

fn same_wakeup(left: Option<Wakeup>, right: Option<Wakeup>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(Wakeup::FrameTime(left)), Some(Wakeup::FrameTime(right))) => left == right,
        (Some(Wakeup::MediaTime(left)), Some(Wakeup::MediaTime(right))) => left == right,
        _ => false,
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SourceChanges {
    pub frame_tick: FrameTickChange,
    pub wakeup: WakeupChange,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FrameTickChange {
    #[default]
    Keep,
    Start,
    Stop,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WakeupChange {
    #[default]
    Keep,
    Schedule(Wakeup),
    Cancel,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct DirtyFlags(u8);

impl DirtyFlags {
    pub const NONE: Self = Self(0);
    pub const POSITION: Self = Self(1 << 0);
    pub const TIMELINE: Self = Self(1 << 1);
    pub const LAYOUT: Self = Self(1 << 2);
    pub const TEXT_VISUALS: Self = Self(1 << 3);
    pub const STATIC_SCENE: Self = Self(1 << 4);
    pub const COVER: Self = Self(1 << 5);
    pub const BACKGROUND: Self = Self(1 << 6);
    pub const VIEWPORT: Self = Self(1 << 7);
    pub const ALL: Self = Self(u8::MAX);

    pub const fn bits(self) -> u8 {
        self.0
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }
}

impl std::fmt::Debug for DirtyFlags {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_tuple("DirtyFlags").field(&self.0).finish()
    }
}

impl BitOr for DirtyFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for DirtyFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.insert(rhs);
    }
}

impl BitAnd for DirtyFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for DirtyFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BatchPlan {
    #[default]
    None,
    Prepare {
        generation: u64,
    },
    Rebuild {
        generation: u64,
    },
    DeferRebuild {
        generation: u64,
    },
    Cancel,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RenderEffects {
    pub sources: SourceIntents,
    pub dirty: DirtyFlags,
    pub batch: BatchPlan,
    pub queue_draw: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reduction {
    pub state: RenderState,
    pub effects: RenderEffects,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RenderEvent {
    Mapped {
        document_generation: Option<u64>,
    },
    Unmapped,
    Disposed,
    DocumentChanged {
        generation: Option<u64>,
    },
    Prepared {
        generation: u64,
        playback: PlaybackState,
        frame: FrameStatus,
    },
    AnchorChanged {
        anchor: PlaybackAnchor,
        frame_time_us: i64,
        next_event: Option<LyricTime>,
        animations: AnimationReasons,
    },
    Play {
        frame: FrameStatus,
    },
    Pause {
        position: LyricTime,
    },
    Seek {
        generation: u64,
        target: LyricTime,
    },
    SeekCommitted {
        generation: u64,
    },
    AllocationChanged {
        generation: u64,
        valid: bool,
        relayout: bool,
    },
    ScaleChanged {
        generation: u64,
    },
    StyleChanged {
        generation: u64,
    },
    ThemeChanged {
        generation: u64,
    },
    CoverChanged {
        generation: u64,
    },
    Wakeup {
        anchor: PlaybackAnchor,
        frame_time_us: i64,
        next_event: Option<LyricTime>,
        animations: AnimationReasons,
    },
    Tick {
        anchor: PlaybackAnchor,
        frame_time_us: i64,
        next_event: Option<LyricTime>,
        animations: AnimationReasons,
    },
    AnimationsChanged {
        playback: PlaybackState,
        frame: FrameStatus,
    },
    UserScroll {
        return_deadline_us: i64,
        advancing: bool,
    },
    ManualScrollTimeout {
        frame_time_us: i64,
        playback: PlaybackState,
        frame: FrameStatus,
    },
}

pub fn reduce(state: RenderState, event: RenderEvent) -> Reduction {
    match event {
        RenderEvent::Mapped {
            document_generation,
        } => reduce_mapped(state, document_generation),
        RenderEvent::Unmapped | RenderEvent::Disposed => reduce_detached(state),
        RenderEvent::DocumentChanged { generation } => reduce_document_changed(state, generation),
        RenderEvent::Prepared {
            generation,
            playback,
            frame,
        } => reduce_prepared(state, generation, playback, frame),
        RenderEvent::AnchorChanged {
            anchor,
            frame_time_us,
            next_event,
            animations,
        } => {
            let frame = FrameStatus::from_anchor(&anchor, frame_time_us, next_event, animations);
            reduce_clock_sample(state, &anchor, frame, state.has_ready_scene(), true)
        }
        RenderEvent::Play { frame } => reduce_play(state, frame),
        RenderEvent::Pause { position } => reduce_pause(state, position),
        RenderEvent::Seek { generation, target } => reduce_seek(state, generation, target),
        RenderEvent::SeekCommitted { generation } => reduce_seek_committed(state, generation),
        RenderEvent::AllocationChanged {
            generation,
            valid,
            relayout,
        } => reduce_allocation(state, generation, valid, relayout),
        RenderEvent::ScaleChanged { generation } | RenderEvent::StyleChanged { generation } => {
            reduce_invalidation(
                state,
                generation,
                DirtyFlags::LAYOUT | DirtyFlags::TEXT_VISUALS | DirtyFlags::STATIC_SCENE,
                None,
            )
        }
        RenderEvent::ThemeChanged { generation } => reduce_invalidation(
            state,
            generation,
            DirtyFlags::TEXT_VISUALS | DirtyFlags::STATIC_SCENE | DirtyFlags::BACKGROUND,
            None,
        ),
        RenderEvent::CoverChanged { generation } => reduce_invalidation(
            state,
            generation,
            DirtyFlags::COVER | DirtyFlags::BACKGROUND,
            None,
        ),
        RenderEvent::Wakeup {
            anchor,
            frame_time_us,
            next_event,
            animations,
        } => {
            let frame = FrameStatus::from_anchor(&anchor, frame_time_us, next_event, animations);
            reduce_clock_sample(
                state,
                &anchor,
                frame,
                matches!(state, RenderState::Waiting { .. }),
                false,
            )
        }
        RenderEvent::Tick {
            anchor,
            frame_time_us,
            next_event,
            animations,
        } => {
            let frame = FrameStatus::from_anchor(&anchor, frame_time_us, next_event, animations);
            reduce_clock_sample(state, &anchor, frame, state.needs_frame_tick(), true)
        }
        RenderEvent::AnimationsChanged { playback, frame } => {
            reduce_animations_changed(state, playback, frame)
        }
        RenderEvent::UserScroll {
            return_deadline_us,
            advancing,
        } => reduce_user_scroll(state, return_deadline_us, advancing),
        RenderEvent::ManualScrollTimeout {
            frame_time_us,
            playback,
            frame,
        } => reduce_manual_scroll_timeout(state, frame_time_us, playback, frame),
    }
}

fn reduce_mapped(state: RenderState, generation: Option<u64>) -> Reduction {
    if !matches!(state, RenderState::Detached) {
        return unchanged(state);
    }

    match generation {
        Some(generation) => complete(
            RenderState::Preparing { generation },
            DirtyFlags::ALL,
            BatchPlan::Prepare { generation },
            true,
        ),
        None => complete(RenderState::Empty, DirtyFlags::NONE, BatchPlan::None, true),
    }
}

fn reduce_detached(state: RenderState) -> Reduction {
    if matches!(state, RenderState::Detached) {
        unchanged(state)
    } else {
        complete(
            RenderState::Detached,
            DirtyFlags::NONE,
            BatchPlan::Cancel,
            false,
        )
    }
}

fn reduce_document_changed(state: RenderState, generation: Option<u64>) -> Reduction {
    match (state, generation) {
        (RenderState::Detached, _) => complete(state, DirtyFlags::ALL, BatchPlan::None, false),
        (_, None) => complete(
            RenderState::Empty,
            DirtyFlags::ALL,
            BatchPlan::Cancel,
            !matches!(state, RenderState::Empty),
        ),
        (RenderState::Preparing { generation: old }, Some(generation)) if old == generation => {
            complete(state, DirtyFlags::ALL, BatchPlan::None, false)
        }
        (_, Some(generation)) => complete(
            RenderState::Preparing { generation },
            DirtyFlags::ALL,
            BatchPlan::Prepare { generation },
            true,
        ),
    }
}

fn reduce_prepared(
    state: RenderState, generation: u64, playback: PlaybackState, frame: FrameStatus,
) -> Reduction {
    let current_generation = match state {
        RenderState::Preparing { generation } => generation,
        _ => return unchanged(state),
    };
    if current_generation != generation {
        return unchanged(state);
    }

    complete(
        frame.render_state(playback),
        DirtyFlags::STATIC_SCENE | DirtyFlags::POSITION,
        BatchPlan::None,
        true,
    )
}

fn reduce_play(state: RenderState, frame: FrameStatus) -> Reduction {
    if let RenderState::ManualScroll {
        return_deadline_us,
        animating,
        ..
    } = state
    {
        return complete(
            RenderState::ManualScroll {
                return_deadline_us,
                advancing: true,
                animating,
            },
            DirtyFlags::POSITION,
            BatchPlan::None,
            true,
        );
    }
    reduce_frame_change(
        state,
        frame.render_state(PlaybackState::Playing),
        state.has_ready_scene(),
    )
}

fn reduce_pause(state: RenderState, position: LyricTime) -> Reduction {
    if let RenderState::ManualScroll {
        return_deadline_us,
        animating,
        ..
    } = state
    {
        return complete(
            RenderState::ManualScroll {
                return_deadline_us,
                advancing: false,
                animating,
            },
            DirtyFlags::POSITION,
            BatchPlan::None,
            true,
        );
    }
    reduce_frame_change(
        state,
        RenderState::Static { position },
        state.has_ready_scene(),
    )
}

fn reduce_animations_changed(
    state: RenderState, playback: PlaybackState, frame: FrameStatus,
) -> Reduction {
    if let RenderState::ManualScroll {
        return_deadline_us, ..
    } = state
    {
        let next = RenderState::ManualScroll {
            return_deadline_us,
            advancing: playback.is_playing(),
            animating: has_ui_animation(frame.animations),
        };
        return complete(next, DirtyFlags::NONE, BatchPlan::None, false);
    }
    reduce_frame_change(state, frame.render_state(playback), state.has_ready_scene())
}

fn reduce_frame_change(state: RenderState, next: RenderState, allowed: bool) -> Reduction {
    if !allowed || state == next {
        return unchanged(state);
    }

    complete(next, DirtyFlags::POSITION, BatchPlan::None, true)
}

fn reduce_seek(state: RenderState, generation: u64, target: LyricTime) -> Reduction {
    if !state.has_document() {
        return unchanged(state);
    }

    let next = RenderState::Seeking { generation, target };
    if state == next {
        unchanged(state)
    } else {
        complete(
            next,
            DirtyFlags::TIMELINE | DirtyFlags::POSITION,
            BatchPlan::None,
            true,
        )
    }
}

fn reduce_seek_committed(state: RenderState, generation: u64) -> Reduction {
    let current_generation = match state {
        RenderState::Seeking { generation, .. } => generation,
        _ => return unchanged(state),
    };
    if current_generation != generation {
        return unchanged(state);
    }

    complete(
        RenderState::Preparing { generation },
        DirtyFlags::TIMELINE | DirtyFlags::STATIC_SCENE,
        BatchPlan::Prepare { generation },
        false,
    )
}

fn reduce_allocation(
    state: RenderState, generation: u64, valid: bool, relayout: bool,
) -> Reduction {
    if relayout {
        return reduce_invalidation(
            state,
            generation,
            DirtyFlags::LAYOUT | DirtyFlags::TEXT_VISUALS | DirtyFlags::STATIC_SCENE,
            Some(valid),
        );
    }
    complete(
        state,
        DirtyFlags::VIEWPORT,
        BatchPlan::None,
        valid && !matches!(state, RenderState::Detached),
    )
}

fn reduce_invalidation(
    state: RenderState, generation: u64, dirty: DirtyFlags, valid_allocation: Option<bool>,
) -> Reduction {
    let batch = if state.has_document() {
        match valid_allocation {
            Some(false) => BatchPlan::DeferRebuild { generation },
            _ => BatchPlan::Rebuild { generation },
        }
    } else {
        BatchPlan::None
    };
    let queue_draw = valid_allocation.unwrap_or(true) && !matches!(state, RenderState::Detached);

    complete(state, dirty, batch, queue_draw)
}

fn reduce_clock_sample(
    state: RenderState, anchor: &PlaybackAnchor, frame: FrameStatus, allowed: bool,
    preserve_manual_scroll: bool,
) -> Reduction {
    if !allowed {
        return unchanged(state);
    }

    let next = match state {
        RenderState::ManualScroll {
            return_deadline_us, ..
        } if preserve_manual_scroll => RenderState::ManualScroll {
            return_deadline_us,
            advancing: anchor.is_advancing(),
            animating: has_ui_animation(frame.animations),
        },
        _ => frame.render_state_for_anchor(anchor),
    };

    complete(next, DirtyFlags::POSITION, BatchPlan::None, true)
}

fn has_ui_animation(reasons: AnimationReasons) -> bool {
    reasons.intersects(
        AnimationReasons::LINE_TRANSITION
            | AnimationReasons::SCROLL_SETTLE
            | AnimationReasons::USER_GESTURE
            | AnimationReasons::INTERACTION,
    )
}

fn reduce_user_scroll(state: RenderState, return_deadline_us: i64, advancing: bool) -> Reduction {
    if !state.has_ready_scene() {
        return unchanged(state);
    }

    let animating = match state {
        RenderState::Animating { reasons } => has_ui_animation(reasons),
        RenderState::ManualScroll { animating, .. } => animating,
        _ => false,
    };
    let next = RenderState::ManualScroll {
        return_deadline_us,
        advancing,
        animating,
    };
    complete(next, DirtyFlags::NONE, BatchPlan::None, state != next)
}

fn reduce_manual_scroll_timeout(
    state: RenderState, frame_time_us: i64, playback: PlaybackState, frame: FrameStatus,
) -> Reduction {
    let deadline = match state {
        RenderState::ManualScroll {
            return_deadline_us, ..
        } => return_deadline_us,
        _ => return unchanged(state),
    };
    if frame_time_us < deadline {
        return unchanged(state);
    }

    complete(
        frame.render_state(playback),
        DirtyFlags::POSITION,
        BatchPlan::None,
        true,
    )
}

fn unchanged(state: RenderState) -> Reduction {
    complete(state, DirtyFlags::NONE, BatchPlan::None, false)
}

fn complete(
    state: RenderState, dirty: DirtyFlags, batch: BatchPlan, queue_draw: bool,
) -> Reduction {
    Reduction {
        state,
        effects: RenderEffects {
            sources: state.source_intents(),
            dirty,
            batch,
            queue_draw,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn time(value_us: i64) -> LyricTime {
        LyricTime::from_micros(value_us)
    }

    fn playing_anchor(media_us: i64, frame_time_us: i64) -> PlaybackAnchor {
        PlaybackAnchor::new(
            time(media_us),
            frame_time_us,
            1.0,
            PlaybackState::Playing,
            1,
        )
    }

    #[test]
    fn paused_playback_still_runs_short_ui_interactions() {
        let interaction = FrameStatus::new(
            LyricTime::ZERO,
            None,
            AnimationReasons::INTERACTION | AnimationReasons::LINE_TRANSITION,
        );
        assert!(matches!(
            interaction.render_state(PlaybackState::Paused),
            RenderState::Animating { .. }
        ));

        let word_only = FrameStatus::new(LyricTime::ZERO, None, AnimationReasons::WORD_PROGRESS);
        assert!(matches!(
            word_only.render_state(PlaybackState::Paused),
            RenderState::Static { .. }
        ));
    }

    #[test]
    fn anchor_uses_absolute_interpolation() {
        let anchor =
            PlaybackAnchor::new(time(1_000_000), 10_000_000, 1.5, PlaybackState::Playing, 7);

        assert_eq!(anchor.position_at(12_000_000), time(4_000_000));
        assert_eq!(anchor.position_at(14_000_000), time(7_000_000));
        assert_eq!(anchor.serial, 7);
    }

    #[test]
    fn paused_and_invalid_anchors_are_fixed() {
        let paused = PlaybackAnchor::new(time(3_000_000), 10, 2.0, PlaybackState::Paused, 1);
        let invalid = PlaybackAnchor::new(time(4_000_000), 10, f64::NAN, PlaybackState::Playing, 2);

        assert_eq!(paused.position_at(5_000_000), time(3_000_000));
        assert_eq!(invalid.position_at(5_000_000), time(4_000_000));
    }

    #[test]
    fn rebasing_preserves_position_across_rate_changes() {
        let anchor = playing_anchor(1_000_000, 2_000_000);
        let rebased = anchor.rebase(3_000_000, 2.0, PlaybackState::Playing, anchor.serial + 1);

        assert_eq!(rebased.media_position, time(2_000_000));
        assert_eq!(rebased.position_at(3_500_000), time(3_000_000));
        assert_eq!(rebased.serial, 2);
    }

    #[test]
    fn animation_reasons_support_local_bit_operations() {
        let mut reasons = AnimationReasons::WORD_PROGRESS | AnimationReasons::LINE_TRANSITION;
        reasons.insert(AnimationReasons::GAP_PULSE);
        assert!(reasons.contains(AnimationReasons::WORD_PROGRESS));
        assert!(reasons.intersects(AnimationReasons::LINE_TRANSITION));
        assert!(reasons.contains(AnimationReasons::GAP_PULSE));

        reasons.remove(AnimationReasons::WORD_PROGRESS);
        reasons.insert(AnimationReasons::COVER_CROSSFADE);

        assert!(!reasons.contains(AnimationReasons::WORD_PROGRESS));
        assert!(reasons.contains(AnimationReasons::COVER_CROSSFADE));
        assert_eq!(
            AnimationReasons::from_bits_truncate(u8::MAX),
            AnimationReasons::ALL
        );
    }

    #[test]
    fn animating_and_advancing_manual_scroll_request_a_frame_tick() {
        let quiet_states = [
            RenderState::Detached,
            RenderState::Empty,
            RenderState::Preparing { generation: 1 },
            RenderState::Static { position: time(0) },
            RenderState::Waiting {
                next_event: time(10),
            },
            RenderState::Seeking {
                generation: 1,
                target: time(20),
            },
            RenderState::ManualScroll {
                return_deadline_us: 30,
                advancing: false,
                animating: false,
            },
        ];

        for state in quiet_states {
            assert!(!state.needs_frame_tick());
            assert!(!state.source_intents().frame_tick);
        }

        let animating = RenderState::Animating {
            reasons: AnimationReasons::WORD_PROGRESS,
        };
        assert!(animating.needs_frame_tick());
        assert!(animating.source_intents().frame_tick);

        let manual = RenderState::ManualScroll {
            return_deadline_us: 30,
            advancing: true,
            animating: false,
        };
        assert!(manual.needs_frame_tick());
        assert!(manual.source_intents().frame_tick);
        assert_eq!(manual.source_intents().wakeup, Some(Wakeup::FrameTime(30)));
        let paused_manual_animation = RenderState::ManualScroll {
            return_deadline_us: 30,
            advancing: false,
            animating: true,
        };
        assert!(paused_manual_animation.needs_frame_tick());
        assert!(paused_manual_animation.source_intents().frame_tick);
    }

    #[test]
    fn paused_manual_scroll_ticks_until_hover_animation_finishes() {
        let manual = RenderState::ManualScroll {
            return_deadline_us: 5_000_000,
            advancing: false,
            animating: false,
        };
        let started = reduce(
            manual,
            RenderEvent::AnimationsChanged {
                playback: PlaybackState::Paused,
                frame: FrameStatus::new(time(0), None, AnimationReasons::INTERACTION),
            },
        );
        assert_eq!(
            started.state,
            RenderState::ManualScroll {
                return_deadline_us: 5_000_000,
                advancing: false,
                animating: true,
            }
        );
        assert!(started.effects.sources.frame_tick);

        let paused_anchor = PlaybackAnchor::new(time(0), 100, 1.0, PlaybackState::Paused, 1);
        let finished = reduce(
            started.state,
            RenderEvent::Tick {
                anchor: paused_anchor,
                frame_time_us: 200,
                next_event: None,
                animations: AnimationReasons::NONE,
            },
        );
        assert_eq!(
            finished.state,
            RenderState::ManualScroll {
                return_deadline_us: 5_000_000,
                advancing: false,
                animating: false,
            }
        );
        assert!(!finished.effects.sources.frame_tick);
        assert_eq!(
            finished.effects.sources.wakeup,
            Some(Wakeup::FrameTime(5_000_000))
        );
    }

    #[test]
    fn source_reconciliation_is_idempotent() {
        let desired = RenderState::Animating {
            reasons: AnimationReasons::WORD_PROGRESS,
        }
        .source_intents();

        assert_eq!(
            desired.changes_from(SourceIntents::NONE).frame_tick,
            FrameTickChange::Start
        );
        assert_eq!(
            desired.changes_from(desired),
            SourceChanges::default(),
            "an already active source must not be started again"
        );

        let stopped = SourceIntents::NONE;
        assert_eq!(
            stopped.changes_from(desired).frame_tick,
            FrameTickChange::Stop
        );
        assert_eq!(stopped.changes_from(stopped), SourceChanges::default());
    }

    #[test]
    fn gap_pulse_requests_ticks_only_while_playing() {
        let frame = FrameStatus::new(
            time(1_000_000),
            Some(time(2_000_000)),
            AnimationReasons::GAP_PULSE,
        );
        let playing = frame.render_state(PlaybackState::Playing);
        assert_eq!(
            playing,
            RenderState::Animating {
                reasons: AnimationReasons::GAP_PULSE
            }
        );
        assert!(playing.source_intents().frame_tick);

        let paused = frame.render_state(PlaybackState::Paused);
        assert_eq!(
            paused,
            RenderState::Static {
                position: time(1_000_000)
            }
        );
        assert_eq!(paused.source_intents(), SourceIntents::NONE);
    }

    #[test]
    fn reducer_waits_animates_then_becomes_static_without_ticks() {
        let mapped = reduce(
            RenderState::Detached,
            RenderEvent::Mapped {
                document_generation: Some(4),
            },
        );
        assert_eq!(mapped.state, RenderState::Preparing { generation: 4 });
        assert_eq!(mapped.effects.batch, BatchPlan::Prepare { generation: 4 });
        assert!(!mapped.effects.sources.frame_tick);

        let prepared = reduce(
            mapped.state,
            RenderEvent::Prepared {
                generation: 4,
                playback: PlaybackState::Playing,
                frame: FrameStatus::new(time(10), Some(time(20)), AnimationReasons::NONE),
            },
        );
        assert_eq!(
            prepared.state,
            RenderState::Waiting {
                next_event: time(20)
            }
        );
        assert_eq!(
            prepared.effects.sources.wakeup,
            Some(Wakeup::MediaTime(time(20)))
        );
        assert!(!prepared.effects.sources.frame_tick);

        let anchor = playing_anchor(20, 100);
        let awake = reduce(
            prepared.state,
            RenderEvent::Wakeup {
                anchor,
                frame_time_us: 100,
                next_event: Some(time(30)),
                animations: AnimationReasons::WORD_PROGRESS,
            },
        );
        assert_eq!(
            awake.state,
            RenderState::Animating {
                reasons: AnimationReasons::WORD_PROGRESS
            }
        );
        assert!(awake.effects.sources.frame_tick);
        assert_eq!(awake.effects.sources.wakeup, None);

        let settled = reduce(
            awake.state,
            RenderEvent::Tick {
                anchor,
                frame_time_us: 110,
                next_event: None,
                animations: AnimationReasons::NONE,
            },
        );
        assert_eq!(settled.state, RenderState::Static { position: time(30) });
        assert_eq!(settled.effects.sources, SourceIntents::NONE);
    }

    #[test]
    fn pause_and_unmap_stop_all_sources() {
        let animating = RenderState::Animating {
            reasons: AnimationReasons::BACKGROUND_MOTION,
        };
        let paused = reduce(animating, RenderEvent::Pause { position: time(42) });

        assert_eq!(paused.state, RenderState::Static { position: time(42) });
        assert_eq!(paused.effects.sources, SourceIntents::NONE);
        assert_eq!(
            paused
                .effects
                .sources
                .changes_from(animating.source_intents())
                .frame_tick,
            FrameTickChange::Stop
        );

        let waiting = RenderState::Waiting {
            next_event: time(100),
        };
        let detached = reduce(waiting, RenderEvent::Unmapped);
        assert_eq!(detached.state, RenderState::Detached);
        assert_eq!(detached.effects.sources, SourceIntents::NONE);
        assert_eq!(detached.effects.batch, BatchPlan::Cancel);
    }

    #[test]
    fn stale_generation_results_are_ignored() {
        let state = RenderState::Preparing { generation: 9 };
        let reduction = reduce(
            state,
            RenderEvent::Prepared {
                generation: 8,
                playback: PlaybackState::Paused,
                frame: FrameStatus::new(time(5), None, AnimationReasons::NONE),
            },
        );

        assert_eq!(reduction.state, state);
        assert!(reduction.effects.dirty.is_empty());
        assert!(!reduction.effects.queue_draw);
    }

    #[test]
    fn manual_scroll_uses_one_wakeup_and_ignores_early_timeout() {
        let scrolling = reduce(
            RenderState::Static { position: time(0) },
            RenderEvent::UserScroll {
                return_deadline_us: 500,
                advancing: true,
            },
        );
        assert_eq!(
            scrolling.effects.sources.wakeup,
            Some(Wakeup::FrameTime(500))
        );
        assert!(scrolling.effects.sources.frame_tick);

        let anchor = playing_anchor(5, 100);
        let sampled = reduce(
            scrolling.state,
            RenderEvent::Tick {
                anchor,
                frame_time_us: 105,
                next_event: Some(time(10)),
                animations: AnimationReasons::WORD_PROGRESS,
            },
        );
        assert_eq!(sampled.state, scrolling.state);
        assert!(sampled.effects.dirty.contains(DirtyFlags::POSITION));
        assert!(sampled.effects.queue_draw);
        assert!(sampled.effects.sources.frame_tick);
        assert_eq!(sampled.effects.sources.wakeup, Some(Wakeup::FrameTime(500)));

        let paused_anchor =
            PlaybackAnchor::new(time(10), 105, 1.0, PlaybackState::Paused, anchor.serial);
        let paused = reduce(
            sampled.state,
            RenderEvent::AnchorChanged {
                anchor: paused_anchor,
                frame_time_us: 105,
                next_event: Some(time(10)),
                animations: AnimationReasons::NONE,
            },
        );
        assert_eq!(
            paused.state,
            RenderState::ManualScroll {
                return_deadline_us: 500,
                advancing: false,
                animating: false,
            }
        );
        assert!(!paused.effects.sources.frame_tick);
        assert_eq!(paused.effects.sources.wakeup, Some(Wakeup::FrameTime(500)));

        let frame = FrameStatus::new(time(5), Some(time(10)), AnimationReasons::NONE);
        let early = reduce(
            scrolling.state,
            RenderEvent::ManualScrollTimeout {
                frame_time_us: 499,
                playback: PlaybackState::Playing,
                frame,
            },
        );
        assert_eq!(early.state, scrolling.state);
        assert_eq!(early.effects.sources.wakeup, Some(Wakeup::FrameTime(500)));

        let expired = reduce(
            early.state,
            RenderEvent::ManualScrollTimeout {
                frame_time_us: 500,
                playback: PlaybackState::Playing,
                frame,
            },
        );
        assert_eq!(
            expired.state,
            RenderState::Waiting {
                next_event: time(10)
            }
        );
    }

    #[test]
    fn invalid_allocation_defers_batch_work() {
        let reduction = reduce(
            RenderState::Static { position: time(0) },
            RenderEvent::AllocationChanged {
                generation: 12,
                valid: false,
                relayout: true,
            },
        );

        assert_eq!(
            reduction.effects.batch,
            BatchPlan::DeferRebuild { generation: 12 }
        );
        assert!(reduction.effects.dirty.contains(DirtyFlags::LAYOUT));
        assert!(!reduction.effects.queue_draw);
        assert_eq!(reduction.effects.sources, SourceIntents::NONE);
    }

    #[test]
    fn height_only_allocation_updates_only_the_viewport() {
        let state = RenderState::Static { position: time(0) };
        let reduction = reduce(
            state,
            RenderEvent::AllocationChanged {
                generation: 12,
                valid: true,
                relayout: false,
            },
        );

        assert_eq!(reduction.state, state);
        assert_eq!(reduction.effects.batch, BatchPlan::None);
        assert_eq!(reduction.effects.dirty, DirtyFlags::VIEWPORT);
        assert!(!reduction.effects.dirty.contains(DirtyFlags::LAYOUT));
        assert!(reduction.effects.queue_draw);
    }

    #[test]
    fn invalid_to_valid_allocation_rebuilds_deferred_layout() {
        let state = RenderState::Preparing { generation: 12 };
        let reduction = reduce(
            state,
            RenderEvent::AllocationChanged {
                generation: 13,
                valid: true,
                relayout: true,
            },
        );

        assert_eq!(
            reduction.effects.batch,
            BatchPlan::Rebuild { generation: 13 }
        );
        assert!(reduction.effects.dirty.contains(DirtyFlags::LAYOUT));
        assert!(reduction.effects.queue_draw);
    }
}
