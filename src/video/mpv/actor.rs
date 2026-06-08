use std::{ops::Deref, sync::Arc};

use super::*;
use flume::{Receiver, Sender, unbounded};
use libmpv2::{
    Mpv,
    events::{Event, PropertyData},
};
use once_cell::sync::Lazy;
use serde_json::Value;
use tsutils::spawn_tokio_blocking;

struct SendMpv(Arc<Mpv>);
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
        property: &'static str,
        value: MpvValue,
    },
    GetProperty {
        property: &'static str,
        value_type: MpvValueType,
        rx: Sender<MpvValue>,
    },
    InitRenderContext(Sender<Arc<Mpv>>),
    WakeUp,
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
pub struct MpvActor;

impl Deref for SendMpv {
    type Target = Mpv;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for MpvActor {
    fn default() -> Self {
        Self::with_initializer(|mpv| {
            _ = mpv.set_property("vo", "libmpv");
            _ = mpv.set_property("hwdec", "auto-safe");
            _ = mpv.set_property("video-timing-offset", 0);
            Ok(())
        })
        .expect("Failed to create mpv instance")
    }
}

impl MpvActor {
    pub fn with_initializer<F>(initializer: F) -> libmpv2::Result<Self>
    where
        F: FnOnce(libmpv2::MpvInitializer) -> libmpv2::Result<()>,
    {
        let mut mpv = Mpv::with_initializer(initializer)?;

        mpv.set_wakeup_callback(move || {
            let _ = MPV_CTRL.tx.send(MpvMessage::WakeUp);
        });

        let mut mpv = SendMpv(Arc::new(mpv));

        spawn_tokio_blocking(move || {
            loop {
                let Ok(msg) = MPV_CTRL.rx.recv() else {
                    continue;
                };

                match msg {
                    MpvMessage::Command { cmd, args } => {
                        let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                        let _ = mpv.command(&cmd, &args_ref);
                    }
                    MpvMessage::SetProperty { property, value } => {
                        _ = value.set_on(&mpv, property);
                    }
                    MpvMessage::GetProperty {
                        property,
                        value_type,
                        rx,
                    } => {
                        let Some(result): Option<MpvValue> =
                            mpv.get_property_value(property, value_type)
                        else {
                            continue;
                        };

                        let _ = rx.send(result);
                    }
                    MpvMessage::InitRenderContext(tx) => {
                        let _ = tx.send(Arc::clone(&mpv.0));
                    }
                    MpvMessage::WakeUp => 'l: loop {
                        if !mpv.handle_event() {
                            break 'l;
                        }
                    },
                    MpvMessage::Shutdown => break,
                }
            }
        });

        Ok(Self)
    }

    pub fn set_property<V>(&self, property: &str, value: V)
    where
        V: Into<MpvValue>,
    {
        _ = MPV_CTRL.tx.send(MpvMessage::SetProperty {
            property: Box::leak(property.to_string().into_boxed_str()),
            value: value.into(),
        });
    }

    pub async fn get_property(
        &self,
        property: &str,
        value_type: MpvValueType,
    ) -> Result<MpvValue, flume::RecvError> {
        let property = Box::leak(property.to_string().into_boxed_str());
        let (rx, tx) = flume::unbounded();
        _ = MPV_CTRL.tx.send(MpvMessage::GetProperty {
            property,
            value_type,
            rx,
        });

        tx.recv_async().await
    }

    pub fn command(&self, cmd: &str, args: &[&str]) {
        let cmd_owned = cmd.to_string();
        let args_owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let _ = MPV_CTRL.tx.send(MpvMessage::Command {
            cmd: cmd_owned,
            args: args_owned,
        });
    }
}

impl SendMpv {
    //If returns false, events are drained and caller should wait for next wakeup
    fn handle_event(&mut self) -> bool {
        let Some(event) = self.wait_event(0.0) else {
            return false;
        };

        match event {
            Ok(event) => match event {
                Event::PropertyChange { name, change, .. } => match name {
                    "duration" => {
                        if let PropertyData::Double(dur) = change {
                            let _ = MPV_EVENT_CHANNEL.tx.send(ListenEvent::Duration(dur));
                        }
                    }
                    "pause" => {
                        if let PropertyData::Flag(pause) = change {
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
                    "paused-for-cache" => {
                        if let PropertyData::Flag(pause) = change {
                            let seeking = self.get_property::<bool>("seeking").unwrap_or(false);
                            let _ = MPV_EVENT_CHANNEL
                                .tx
                                .send(ListenEvent::PausedForCache(pause || seeking));
                        }
                    }
                    _ => {}
                },
                Event::Seek { .. } => {
                    let _ = MPV_EVENT_CHANNEL.tx.send(ListenEvent::Seek);
                }
                Event::PlaybackRestart { .. } => {
                    let _ = MPV_EVENT_CHANNEL.tx.send(ListenEvent::PlaybackRestart);
                }
                Event::EndFile(r) => {
                    let _ = MPV_EVENT_CHANNEL.tx.send(ListenEvent::Eof(r));
                }
                Event::StartFile => {
                    let _ = MPV_EVENT_CHANNEL.tx.send(ListenEvent::StartFile);
                }
                Event::Shutdown => {
                    let _ = MPV_EVENT_CHANNEL.tx.send(ListenEvent::Shutdown);
                }
                _ => {}
            },
            Err(e) => {
                let _ = MPV_EVENT_CHANNEL.tx.send(ListenEvent::Error(e.to_string()));
            }
        }

        true
    }

    fn get_property_value(&self, property: &str, value_type: MpvValueType) -> Option<MpvValue> {
        match value_type {
            MpvValueType::Bool => self.get_property::<bool>(property).ok().map(MpvValue::Bool),
            MpvValueType::I64 => self.get_property::<i64>(property).ok().map(MpvValue::I64),
            MpvValueType::F64 => self.get_property::<f64>(property).ok().map(MpvValue::F64),
            MpvValueType::String => self
                .get_property::<String>(property)
                .ok()
                .map(MpvValue::String),
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

#[derive(Debug)]
pub struct MpvTrack {
    pub id: i64,
    pub title: String,
    pub lang: String,
    pub type_: String,
}

pub struct MpvTracks {
    pub audio_tracks: Vec<MpvTrack>,
    pub sub_tracks: Vec<MpvTrack>,
}

fn node_to_tracks(value: &str) -> MpvTracks {
    let mut audio_tracks = Vec::new();
    let mut sub_tracks = Vec::new();

    let Ok(json) = serde_json::from_str::<Value>(value) else {
        return MpvTracks {
            audio_tracks,
            sub_tracks,
        };
    };
    let Some(array) = json.as_array() else {
        return MpvTracks {
            audio_tracks,
            sub_tracks,
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
    }
}
