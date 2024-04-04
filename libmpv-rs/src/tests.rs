// Copyright (C) 2016  ParadoxSpiral
//
// This file is part of libmpv-rs.
//
// This library is free software; you can redistribute it and/or
// modify it under the terms of the GNU Lesser General Public
// License as published by the Free Software Foundation; either
// version 2.1 of the License, or (at your option) any later version.
//
// This library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
// Lesser General Public License for more details.
//
// You should have received a copy of the GNU Lesser General Public
// License along with this library; if not, write to the Free Software
// Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA  02110-1301  USA

use crate::events::{Event, PropertyData};
use crate::*;

use std::collections::HashMap;
use std::thread;
use std::time::Duration;

#[test]
fn initializer() {
    let mpv = Mpv::with_initializer(|init| {
        init.set_property("osc", true)?;
        init.set_property("input-default-bindings", true)?;
        init.set_property("volume", 30)?;

        Ok(())
    })
    .unwrap();

    assert_eq!(true, mpv.get_property("osc").unwrap());
    assert_eq!(true, mpv.get_property("input-default-bindings").unwrap());
    assert_eq!(30i64, mpv.get_property("volume").unwrap());
}

#[test]
fn properties() {
    let mpv = Mpv::new().unwrap();
    mpv.set_property("volume", 0).unwrap();
    mpv.set_property("vo", "null").unwrap();
    mpv.set_property("ytdl-format", "best[width<240]").unwrap();
    mpv.set_property("sub-gauss", 0.6).unwrap();

    assert_eq!(0i64, mpv.get_property("volume").unwrap());
    let vo: MpvStr = mpv.get_property("vo").unwrap();
    assert_eq!("null", &*vo);
    assert_eq!(true, mpv.get_property("ytdl").unwrap());
    let subg: f64 = mpv.get_property("sub-gauss").unwrap();
    assert_eq!(
        0.6,
        f64::round(subg * f64::powi(10.0, 4)) / f64::powi(10.0, 4)
    );

    mpv.playlist_load_files(&[(
        "test-data/speech_12kbps_mb.wav",
        FileState::AppendPlay,
        None,
    )])
    .unwrap();
    thread::sleep(Duration::from_millis(250));

    let title: MpvStr = mpv.get_property("media-title").unwrap();
    assert_eq!(&*title, "speech_12kbps_mb.wav");
}

macro_rules! assert_event_occurs {
    ($ctx:ident, $timeout:literal, $( $expected:pat),+) => {
        loop {
            match $ctx.wait_event($timeout) {
                $( Some($expected) )|+ => {
                    break;
                },
                None => {
                    continue
                },
                other => panic!("Event did not occur, got: {:?}", other),
            }
        }
    }
}

#[test]
fn events() {
    let mpv = Mpv::new().unwrap();
    let mut ev_ctx = mpv.create_event_context();
    ev_ctx.disable_deprecated_events().unwrap();

    ev_ctx.observe_property("volume", Format::Int64, 0).unwrap();
    ev_ctx
        .observe_property("media-title", Format::String, 1)
        .unwrap();

    mpv.set_property("vo", "null").unwrap();
    mpv.set_property("ytdl", false).unwrap();

    assert_event_occurs!(
        ev_ctx,
        3.,
        Ok(Event::PropertyChange {
            name: "volume",
            change: PropertyData::Int64(100),
            reply_userdata: 0,
        })
    );

    mpv.set_property("volume", 0).unwrap();
    assert_event_occurs!(
        ev_ctx,
        10.,
        Ok(Event::PropertyChange {
            name: "volume",
            change: PropertyData::Int64(0),
            reply_userdata: 0,
        })
    );
    assert!(ev_ctx.wait_event(3.).is_none());

    mpv.playlist_load_files(&[(
        "https://www.youtube.com/watch?v=DLzxrzFCyOs",
        FileState::AppendPlay,
        None,
    )])
    .unwrap();
    assert_event_occurs!(ev_ctx, 10., Ok(Event::StartFile));
    assert_event_occurs!(
        ev_ctx,
        10.,
        Ok(Event::PropertyChange {
            name: "media-title",
            change: PropertyData::Str("watch?v=DLzxrzFCyOs"),
            reply_userdata: 1,
        })
    );
    assert_event_occurs!(ev_ctx, 20., Err(Error::Raw(mpv_error::UnknownFormat)));
    assert!(ev_ctx.wait_event(3.).is_none());

    mpv.playlist_load_files(&[(
        "test-data/speech_12kbps_mb.wav",
        FileState::AppendPlay,
        None,
    )])
    .unwrap();
    assert_event_occurs!(ev_ctx, 10., Ok(Event::StartFile));
    assert_event_occurs!(
        ev_ctx,
        3.,
        Ok(Event::PropertyChange {
            name: "media-title",
            change: PropertyData::Str("speech_12kbps_mb.wav"),
            reply_userdata: 1,
        })
    );
    assert_event_occurs!(ev_ctx, 3., Ok(Event::AudioReconfig));
    assert_event_occurs!(ev_ctx, 3., Ok(Event::AudioReconfig));
    assert_event_occurs!(ev_ctx, 3., Ok(Event::FileLoaded));
    assert_event_occurs!(ev_ctx, 3., Ok(Event::AudioReconfig));
    assert_event_occurs!(ev_ctx, 3., Ok(Event::PlaybackRestart));
    assert!(ev_ctx.wait_event(3.).is_none());
}

#[test]
fn node_map() -> Result<()> {
    let mpv = Mpv::new()?;

    mpv.playlist_load_files(&[(
        "test-data/speech_12kbps_mb.wav",
        FileState::AppendPlay,
        None,
    )])?;

    thread::sleep(Duration::from_millis(250));
    let audio_params: MpvNode = mpv.get_property("audio-params")?;
    let params: HashMap<&str, MpvNode> =
        audio_params.to_map().ok_or_else(|| Error::Null)?.collect();

    assert_eq!(params.len(), 5);

    let format = params.get("format").unwrap().value()?;
    assert!(matches!(format, MpvNodeValue::String("s16")));

    let samplerate = params.get("samplerate").unwrap().value()?;
    assert!(matches!(samplerate, MpvNodeValue::Int64(48_000)));

    let channels = params.get("channels").unwrap().value()?;
    assert!(matches!(channels, MpvNodeValue::String("mono")));

    let hr_channels = params.get("hr-channels").unwrap().value()?;
    assert!(matches!(hr_channels, MpvNodeValue::String("mono")));

    let channel_count = params.get("channel-count").unwrap().value()?;
    assert!(matches!(channel_count, MpvNodeValue::Int64(1)));

    Ok(())
}

#[test]
fn node_array() -> Result<()> {
    let mpv = Mpv::new()?;

    mpv.playlist_load_files(&[(
        "test-data/speech_12kbps_mb.wav",
        FileState::AppendPlay,
        None,
    )])?;

    thread::sleep(Duration::from_millis(250));
    let playlist: MpvNode = mpv.get_property("playlist")?;
    let items: Vec<MpvNode> = playlist.to_array().ok_or_else(|| Error::Null)?.collect();

    assert_eq!(items.len(), 1);
    let track: HashMap<&str, MpvNode> = items[0].to_map().ok_or_else(|| Error::Null)?.collect();

    let filename = track.get("filename").unwrap().value()?;

    assert!(matches!(
        filename,
        MpvNodeValue::String("test-data/speech_12kbps_mb.wav")
    ));

    Ok(())
}
