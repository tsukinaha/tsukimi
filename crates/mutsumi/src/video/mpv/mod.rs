mod actor;
mod area;
mod contexted;
mod logging;
mod paintable;
mod proxy;
mod session;

pub use actor::*;
pub use area::*;
pub use contexted::*;
pub use paintable::*;
pub use proxy::*;
pub use session::*;

type TimeMillis = f64;

#[derive(Debug, Clone)]
pub enum ListenEvent {
    Seek(TimeMillis),
    PlaybackRestart(TimeMillis),
    Eof(u32),
    StartFile,
    FileLoaded,
    Duration(f64),
    Pause(bool),
    CacheSpeed(i64),
    Error(String),
    TrackList(MpvTracks),
    Volume(i64),
    Speed(f64),
    Shutdown,
    PlaybackEnded,
    DemuxerCacheTime(i64),
    DemuxerCacheIdle(bool),
    TimePos(i64),
    PausedForCache(bool, TimeMillis),
    ChapterList(ChapterList),
    Playlist(Playlist),
}
