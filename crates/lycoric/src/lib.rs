//! GTK4/GSK lyrics rendering primitives and an immersive libadwaita player page.

pub mod assemble;
pub mod diagnostics;
pub mod model;
pub mod parser;
mod render;
pub mod time;
pub mod timeline;
mod widgets;

pub use assemble::{
    AssemblyReport,
    MatchStrategy,
    MergeStrategy,
    SidecarMatchStrategy,
    VariantAssembler,
    VariantMatchStrategy,
};
pub use diagnostics::{
    Diagnostic,
    DiagnosticCode,
    DiagnosticSeverity,
    SourcePosition,
    SourceSpan,
};
pub use model::{
    LaneKind,
    LineId,
    LyricLane,
    LyricLine,
    LyricTrack,
    LyricsDocument,
    LyricsMetadata,
    MetadataTag,
    TimedSegment,
};
pub use parser::{
    ParseMode,
    ParseOptions,
    ParseReport,
    parse_lrc,
};
pub use render::{
    AnimationReasons,
    BatchPlan,
    DirtyFlags,
    FrameStatus,
    FrameTickChange,
    LaneColors,
    LaneSlot,
    LaneStyle,
    LaneStyles,
    LaneVisibility,
    LayoutStyle,
    LyricsStyle,
    OutlineStyle,
    Overscan,
    PlaybackAnchor,
    PlaybackState,
    Reduction,
    RenderEffects,
    RenderEvent,
    RenderState,
    ShadowStyle,
    SourceChanges,
    SourceIntents,
    TextEffects,
    TransitionEasing,
    TransitionStyle,
    Wakeup,
    WakeupChange,
    reduce,
};
pub use time::{
    LyricTime,
    TimeRange,
};
pub use timeline::{
    ActiveLine,
    SegmentProgress,
    Timeline,
    TimelineFrame,
    TimelineLine,
};
pub use widgets::{
    AnimatedBackdrop,
    BackgroundQuality,
    CoverPalette,
    LyricPlayerPage,
    LyricsView,
};

/// Registers the public GObject types before they are referenced by UI templates.
pub fn init() {
    use std::sync::Once;

    use gtk::prelude::*;

    static REGISTER_RESOURCES: Once = Once::new();
    REGISTER_RESOURCES.call_once(|| {
        gtk::gio::resources_register_include!("lycoric.gresource")
            .expect("failed to register Lycoric resources");
    });

    LyricsView::ensure_type();
    AnimatedBackdrop::ensure_type();
    LyricPlayerPage::ensure_type();
}
