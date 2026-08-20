use std::{
    ops::Deref,
    sync::{Arc, OnceLock},
};

use crate::MutsumiMpvError;

use super::{logging, *};
use flume::{Receiver, Sender, unbounded};
use libmpv2::{
    Format, Mpv,
    events::{Event, PropertyData},
};
use mutsumi_prelude::spawn_tokio_blocking;
use once_cell::sync::Lazy;
use serde_json::Value;

type MpvInitializer =
    dyn Fn(libmpv2::MpvInitializer) -> libmpv2::Result<()> + Send + Sync + 'static;

static MPV_INITIALIZER: OnceLock<Box<MpvInitializer>> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the mpv initializer has already been set")]
pub struct MpvInitializerAlreadySet;

pub fn set_mpv_initializer<F>(initializer: F) -> Result<(), MpvInitializerAlreadySet>
where
    F: Fn(libmpv2::MpvInitializer) -> libmpv2::Result<()> + Send + Sync + 'static,
{
    MPV_INITIALIZER
        .set(Box::new(initializer))
        .map_err(|_| MpvInitializerAlreadySet)
}

struct SendMpv {
    mpv: Arc<Mpv>,
    has_file: std::cell::Cell<bool>,
}
unsafe impl Send for SendMpv {}

#[derive(Debug, Clone)]
pub enum MpvValue {
    Bool(bool),
    I64(i64),
    F64(f64),
    String(String),
}

#[derive(Debug, Clone)]
pub enum MpvValueType {
    Bool,
    I64,
    F64,
    String,
}

impl From<i32> for MpvValue {
    fn from(val: i32) -> Self {
        MpvValue::I64(val as i64)
    }
}

impl From<u32> for MpvValue {
    fn from(val: u32) -> Self {
        MpvValue::I64(val as i64)
    }
}

impl From<bool> for MpvValue {
    fn from(val: bool) -> Self {
        MpvValue::Bool(val)
    }
}

impl From<i64> for MpvValue {
    fn from(val: i64) -> Self {
        MpvValue::I64(val)
    }
}

impl From<f64> for MpvValue {
    fn from(val: f64) -> Self {
        MpvValue::F64(val)
    }
}

impl From<String> for MpvValue {
    fn from(val: String) -> Self {
        MpvValue::String(val)
    }
}

impl From<&str> for MpvValue {
    fn from(val: &str) -> Self {
        MpvValue::String(val.to_string())
    }
}

impl MpvValue {
    pub fn set_on(&self, mpv: &Mpv, property: &str) -> libmpv2::Result<()> {
        match self {
            MpvValue::Bool(v) => mpv.set_property(property, *v),
            MpvValue::I64(v) => mpv.set_property(property, *v),
            MpvValue::F64(v) => mpv.set_property(property, *v),
            MpvValue::String(v) => mpv.set_property(property, v.as_str()),
        }
    }
}

pub enum MpvMessage {
    Command {
        cmd: String,
        args: Vec<String>,
    },
    SetProperty {
        property: String,
        value: MpvValue,
    },
    GetProperty {
        property: String,
        value_type: MpvValueType,
        tx: tokio::sync::oneshot::Sender<MpvValue>,
    },
    InitRenderContext(tokio::sync::oneshot::Sender<Arc<Mpv>>),
    Shutdown,
}

pub static MPV_CTRL: Lazy<MpvCtrl> = Lazy::new(|| {
    let (tx, rx) = unbounded::<MpvMessage>();

    MpvCtrl { tx, rx }
});

pub struct MpvCtrl {
    pub tx: Sender<MpvMessage>,
    pub rx: Receiver<MpvMessage>,
}

#[derive(Clone, Copy)]
pub struct MpvActor {
    _phantom: std::marker::PhantomData<()>,
}

impl Deref for SendMpv {
    type Target = Mpv;

    fn deref(&self) -> &Self::Target {
        &self.mpv
    }
}

impl Default for MpvActor {
    fn default() -> Self {
        Self::new()
    }
}

impl MpvActor {
    pub fn new() -> Self {
        Self::with_initializer(|mpv| {
            mpv.set_option("input-default-bindings", "yes")?;
            mpv.set_option("hwdec", "auto-safe")?;
            mpv.set_option("keep-open", "yes")?;
            mpv.set_option("vo", "gpu-next")?;

            if let Some(initializer) = MPV_INITIALIZER.get() {
                initializer(mpv)?;
            }

            Ok(())
        })
        .expect("Failed to create mpv instance")
    }

    pub fn with_initializer<F>(initializer: F) -> libmpv2::Result<Self>
    where
        F: FnOnce(libmpv2::MpvInitializer) -> libmpv2::Result<()>,
    {
        let mpv = Mpv::with_initializer(initializer)?;
        logging::request_logs(&mpv);
        tracing::debug!(target: "mutsumi::mpv", "mpv instance initialized");

        mpv.disable_deprecated_events()?;

        mpv.observe_property("duration", Format::Double, 0)?;
        mpv.observe_property("pause", Format::Flag, 1)?;
        mpv.observe_property("cache-speed", Format::Int64, 2)?;
        mpv.observe_property("track-list", Format::String, 3)?;
        mpv.observe_property("paused-for-cache", Format::Flag, 4)?;
        mpv.observe_property("demuxer-cache-time", Format::Int64, 5)?;
        mpv.observe_property("time-pos", Format::Int64, 6)?;
        mpv.observe_property("volume", Format::Int64, 7)?;
        mpv.observe_property("chapter-list", Format::String, 8)?;
        mpv.observe_property("speed", Format::Double, 9)?;
        mpv.observe_property("playlist", Format::String, 10)?;
        mpv.observe_property("eof-reached", Format::Flag, 11)?;

        let mpv = SendMpv {
            mpv: Arc::new(mpv),
            has_file: std::cell::Cell::new(false),
        };

        let event_mpv = SendMpv {
            mpv: Arc::clone(&mpv.mpv),
            has_file: std::cell::Cell::new(false),
        };
        std::thread::Builder::new()
            .name("mpv event loop".into())
            .spawn(move || while event_mpv.handle_event() {})
            .expect("Failed to spawn mpv event thread");

        spawn_tokio_blocking(move || {
            loop {
                let Ok(msg) = MPV_CTRL.rx.recv() else {
                    continue;
                };

                match msg {
                    MpvMessage::Command { cmd, args } => {
                        let args_ref: Vec<&str> = args.iter().map(String::as_str).collect();
                        let _ = mpv.command(&cmd, &args_ref);
                    }
                    MpvMessage::SetProperty { property, value } => {
                        let _ = value.set_on(&mpv, &property);
                    }
                    MpvMessage::GetProperty {
                        property,
                        value_type,
                        tx,
                    } => {
                        if let Ok(result) = mpv.get_property_value(&property, value_type) {
                            let _ = tx.send(result);
                        }
                    }
                    MpvMessage::InitRenderContext(tx) => {
                        let _ = tx.send(Arc::clone(&mpv.mpv));
                    }
                    MpvMessage::Shutdown => break,
                }
            }
        });

        Ok(Self {
            _phantom: std::marker::PhantomData,
        })
    }

    pub fn set_property<V>(&self, property: &str, value: V)
    where
        V: Into<MpvValue>,
    {
        if let Err(error) = MPV_CTRL.tx.send(MpvMessage::SetProperty {
            property: property.to_owned(),
            value: value.into(),
        }) {
            tracing::warn!(target: "mutsumi::mpv", %error, "mpv actor is unavailable");
        }
    }

    pub async fn get_property(
        &self,
        property: &str,
        value_type: MpvValueType,
    ) -> Result<MpvValue, tokio::sync::oneshot::error::RecvError> {
        let (tx, rx) = tokio::sync::oneshot::channel::<MpvValue>();
        if let Err(error) = MPV_CTRL.tx.send(MpvMessage::GetProperty {
            property: property.to_owned(),
            value_type,
            tx,
        }) {
            tracing::warn!(target: "mutsumi::mpv", %error, "mpv actor is unavailable");
        }

        rx.await
    }

    pub fn command(&self, cmd: &str, args: &[&str]) {
        let cmd_owned = cmd.to_string();
        let args_owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        if let Err(error) = MPV_CTRL.tx.send(MpvMessage::Command {
            cmd: cmd_owned,
            args: args_owned,
        }) {
            tracing::warn!(target: "mutsumi::mpv", %error, "mpv actor is unavailable");
        }
    }
}

impl SendMpv {
    // Blocks until the next event. Returns false once mpv shuts down.
    fn handle_event(&self) -> bool {
        let Some(event) = self.wait_event(1.0) else {
            return true;
        };

        match event {
            Ok(event) => match event {
                Event::LogMessage {
                    prefix,
                    text,
                    log_level,
                    ..
                } => logging::emit_log(prefix, log_level, text),
                Event::PropertyChange { name, change, .. } => match name {
                    "duration" => {
                        if let PropertyData::Double(dur) = change {
                            let _ = MPV_EVENT_CHANNEL.tx.send(ListenEvent::Duration(dur));
                        }
                    }
                    "pause" => {
                        if let PropertyData::Flag(pause) = change
                            && (self.has_file.get() || pause)
                        {
                            let _ = MPV_EVENT_CHANNEL.tx.send(ListenEvent::Pause(pause));
                        }
                    }
                    "cache-speed" => {
                        if let PropertyData::Int64(speed) = change {
                            let _ = MPV_EVENT_CHANNEL.tx.send(ListenEvent::CacheSpeed(speed));
                        }
                    }
                    "track-list" => {
                        if let PropertyData::Str(node) = change {
                            let _ = MPV_EVENT_CHANNEL
                                .tx
                                .send(ListenEvent::TrackList(node_to_tracks(node)));
                        }
                    }
                    "chapter-list" => {
                        if let PropertyData::Str(node) = change {
                            let _ = MPV_EVENT_CHANNEL
                                .tx
                                .send(ListenEvent::ChapterList(node_to_chapter_list(node)));
                        }
                    }
                    "playlist" => {
                        if let PropertyData::Str(node) = change {
                            let _ = MPV_EVENT_CHANNEL
                                .tx
                                .send(ListenEvent::Playlist(node_to_playlist(node)));
                        }
                    }
                    "volume" => {
                        if let PropertyData::Int64(volume) = change {
                            let _ = MPV_EVENT_CHANNEL.tx.send(ListenEvent::Volume(volume));
                        }
                    }
                    "speed" => {
                        if let PropertyData::Double(speed) = change {
                            let _ = MPV_EVENT_CHANNEL.tx.send(ListenEvent::Speed(speed));
                        }
                    }
                    "demuxer-cache-time" => {
                        if let PropertyData::Int64(time) = change {
                            let _ = MPV_EVENT_CHANNEL
                                .tx
                                .send(ListenEvent::DemuxerCacheTime(time));
                        }
                    }
                    "time-pos" => {
                        if let PropertyData::Int64(time) = change {
                            let _ = MPV_EVENT_CHANNEL.tx.send(ListenEvent::TimePos(time));
                        }
                    }
                    "eof-reached" => {
                        if let PropertyData::Flag(true) = change {
                            let _ = MPV_EVENT_CHANNEL.tx.send(ListenEvent::PlaybackEnded);
                        }
                    }
                    "paused-for-cache" => {
                        if let PropertyData::Flag(pause) = change {
                            let seeking = self.get_property::<bool>("seeking").unwrap_or(false);
                            let time_millis =
                                self.get_property::<f64>("audio-pts").unwrap_or(0.0) * 1000.0;
                            let _ = MPV_EVENT_CHANNEL
                                .tx
                                .send(ListenEvent::PausedForCache(pause || seeking, time_millis));
                        }
                    }
                    _ => {}
                },
                Event::Seek { .. } => {
                    let time_millis = self.get_property::<f64>("audio-pts").unwrap_or(0.0) * 1000.0;
                    let _ = MPV_EVENT_CHANNEL.tx.send(ListenEvent::Seek(time_millis));
                }
                Event::PlaybackRestart { .. } => {
                    let time_millis = self.get_property::<f64>("audio-pts").unwrap_or(0.0) * 1000.0;
                    let _ = MPV_EVENT_CHANNEL
                        .tx
                        .send(ListenEvent::PlaybackRestart(time_millis));
                }
                Event::FileLoaded => {
                    let _ = MPV_EVENT_CHANNEL.tx.send(ListenEvent::FileLoaded);
                }
                Event::EndFile(reason) => {
                    self.has_file.set(false);
                    let _ = MPV_EVENT_CHANNEL.tx.send(ListenEvent::Eof(reason));
                }
                Event::StartFile => {
                    self.has_file.set(true);
                    let _ = MPV_EVENT_CHANNEL.tx.send(ListenEvent::StartFile);
                    let pause = self.get_property::<bool>("pause").unwrap_or(false);
                    let _ = MPV_EVENT_CHANNEL.tx.send(ListenEvent::Pause(pause));
                }
                Event::Shutdown => {
                    let _ = MPV_EVENT_CHANNEL.tx.send(ListenEvent::Shutdown);
                    let _ = MPV_CTRL.tx.send(MpvMessage::Shutdown);
                    return false;
                }
                _ => {}
            },
            Err(error) => {
                let libmpv2::Error::Raw(code) = error else {
                    return true;
                };

                let message = MutsumiMpvError::from_code(code).to_string();
                let _ = MPV_EVENT_CHANNEL.tx.send(ListenEvent::Error(message));
            }
        }

        true
    }

    fn get_property_value(
        &self,
        property: &str,
        value_type: MpvValueType,
    ) -> libmpv2::Result<MpvValue> {
        match value_type {
            MpvValueType::Bool => self.get_property::<bool>(property).map(MpvValue::Bool),
            MpvValueType::I64 => self.get_property::<i64>(property).map(MpvValue::I64),
            MpvValueType::F64 => self.get_property::<f64>(property).map(MpvValue::F64),
            MpvValueType::String => self.get_property::<String>(property).map(MpvValue::String),
        }
    }
}

fn node_to_chapter_list(value: &str) -> ChapterList {
    let mut chapters = Vec::new();

    let Ok(json) = serde_json::from_str::<Value>(value) else {
        return ChapterList(chapters);
    };
    let Some(array) = json.as_array() else {
        return ChapterList(chapters);
    };

    for node in array {
        let Some(obj) = node.as_object() else {
            continue;
        };

        let title = obj
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let time = obj
            .get("time")
            .and_then(Value::as_f64)
            .or_else(|| obj.get("time").and_then(Value::as_i64).map(|v| v as f64))
            .unwrap_or(0.0);

        chapters.push(Chapter { title, time });
    }

    ChapterList(chapters)
}

pub struct ChapterList(pub Vec<Chapter>);

impl IntoIterator for ChapterList {
    type Item = Chapter;
    type IntoIter = std::vec::IntoIter<Chapter>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

pub struct Chapter {
    pub title: String,
    pub time: f64,
}

#[derive(Debug, Clone)]
pub struct PlaylistEntry {
    pub filename: String,
    pub title: String,
    pub current: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Playlist(pub Vec<PlaylistEntry>);

impl IntoIterator for Playlist {
    type Item = PlaylistEntry;
    type IntoIter = std::vec::IntoIter<PlaylistEntry>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

fn node_to_playlist(value: &str) -> Playlist {
    let mut entries = Vec::new();

    let Ok(json) = serde_json::from_str::<Value>(value) else {
        return Playlist(entries);
    };
    let Some(array) = json.as_array() else {
        return Playlist(entries);
    };

    for node in array {
        let Some(obj) = node.as_object() else {
            continue;
        };

        let filename = obj
            .get("filename")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let title = obj
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let current = obj.get("current").and_then(Value::as_bool).unwrap_or(false);

        entries.push(PlaylistEntry {
            filename,
            title,
            current,
        });
    }

    Playlist(entries)
}

#[derive(Debug)]
pub struct MpvTrack {
    pub id: i64,
    pub title: String,
    pub lang: String,
    pub type_: String,
}

#[derive(Debug)]
pub struct DanmakuTrack {
    pub external_url: String,
}

pub struct MpvTracks {
    pub audio_tracks: Vec<MpvTrack>,
    pub sub_tracks: Vec<MpvTrack>,
    pub danmaku_track: Option<DanmakuTrack>,
}

fn node_to_tracks(value: &str) -> MpvTracks {
    let mut audio_tracks = Vec::new();
    let mut sub_tracks = Vec::new();
    let mut danmaku_track = None;

    let Ok(json) = serde_json::from_str::<Value>(value) else {
        return MpvTracks {
            audio_tracks,
            sub_tracks,
            danmaku_track,
        };
    };
    let Some(array) = json.as_array() else {
        return MpvTracks {
            audio_tracks,
            sub_tracks,
            danmaku_track,
        };
    };

    for node in array {
        let Some(obj) = node.as_object() else {
            continue;
        };

        let id = obj.get("id").and_then(Value::as_i64).unwrap_or(0);
        let title = obj
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let lang = obj
            .get("lang")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let type_ = obj
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();

        if type_ == "sub" && lang == "danmaku" {
            let external = obj
                .get("external")
                .and_then(Value::as_bool)
                .unwrap_or(false);

            if !external {
                continue;
            }

            let external_filename = obj
                .get("external-filename")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();

            let Some(external) =
                external_filename.strip_prefix("edl://!no_clip;!delay_open,media_type=sub;%44%")
            else {
                continue;
            };

            danmaku_track = Some(DanmakuTrack {
                external_url: external.to_string(),
            });

            continue;
        }

        let track = MpvTrack {
            id,
            title,
            lang,
            type_,
        };

        if track.type_ == "audio" {
            audio_tracks.push(track);
        } else if track.type_ == "sub" {
            sub_tracks.push(track);
        }
    }

    MpvTracks {
        audio_tracks,
        sub_tracks,
        danmaku_track,
    }
}
