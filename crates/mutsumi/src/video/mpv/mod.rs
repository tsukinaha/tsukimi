use std::ptr;

mod actor;
mod area;
mod contexted;
mod logging;
mod paintable;
mod proxy;

pub use actor::*;
pub use area::*;
pub use contexted::*;
pub use paintable::*;
pub use proxy::*;

use flume::{
    Receiver,
    Sender,
    unbounded,
};
use once_cell::sync::Lazy;

type TimeMillis = f64;

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

pub struct MPVEventChannel {
    pub tx: Sender<ListenEvent>,
    pub rx: Receiver<ListenEvent>,
}

pub static MPV_EVENT_CHANNEL: Lazy<MPVEventChannel> = Lazy::new(|| {
    let (tx, rx) = unbounded::<ListenEvent>();

    MPVEventChannel { tx, rx }
});

pub struct RenderUpdate {
    pub tx: Sender<bool>,
    pub rx: Receiver<bool>,
}

// Give render update a unique channel
pub static RENDER_UPDATE: Lazy<RenderUpdate> = Lazy::new(|| {
    let (tx, rx) = unbounded::<bool>();

    RenderUpdate { tx, rx }
});

fn epoxy_library() -> &'static libloading::Library {
    use std::sync::OnceLock;

    static EPOXY: OnceLock<libloading::Library> = OnceLock::new();

    EPOXY.get_or_init(|| {
        #[cfg(target_os = "macos")]
        let filename = "libepoxy.0.dylib";
        #[cfg(all(unix, not(target_os = "macos")))]
        let filename = "libepoxy.so.0";
        #[cfg(windows)]
        let filename = "libepoxy-0.dll";
        // SAFETY: the filename is a compile-time constant.
        let library = unsafe { libloading::Library::new(filename) };

        #[cfg(windows)]
        let library = library.or_else(|_| unsafe { libloading::Library::new("epoxy-0.dll") });

        library.unwrap()
    })
}

/// `epoxy_<name>` symbols are entries in libepoxy's dispatch table, not
/// functions, so they must be dereferenced once to obtain the actual
/// dispatcher for the currently bound GL context.
pub(crate) fn get_proc_address(name: &str) -> *const std::ffi::c_void {
    use std::ffi::c_void;

    let library = epoxy_library();
    let symbol = format!("epoxy_{name}");
    unsafe {
        library
            .get::<*const c_void>(&symbol)
            .map(|sym| {
                let entry = sym.try_as_raw_ptr().unwrap() as *const *const c_void;
                *entry
            })
            .unwrap_or(ptr::null())
    }
}
