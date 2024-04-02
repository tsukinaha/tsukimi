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

use libmpv::{events::*, *};

use std::{collections::HashMap, env, thread, time::Duration};

const VIDEO_URL: &str = "https://www.youtube.com/watch?v=DLzxrzFCyOs";

fn main() -> Result<()> {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| String::from(VIDEO_URL));

    // Create an `Mpv` and set some properties.
    let mpv = Mpv::new()?;
    mpv.set_property("volume", 15)?;
    mpv.set_property("vo", "null")?;

    let mut ev_ctx = mpv.create_event_context();
    ev_ctx.disable_deprecated_events()?;
    ev_ctx.observe_property("volume", Format::Int64, 0)?;
    ev_ctx.observe_property("demuxer-cache-state", Format::Node, 0)?;

    crossbeam::scope(|scope| {
        scope.spawn(|_| {
            mpv.playlist_load_files(&[(&path, FileState::AppendPlay, None)])
                .unwrap();

            thread::sleep(Duration::from_secs(3));

            mpv.set_property("volume", 25).unwrap();

            thread::sleep(Duration::from_secs(5));

            // Trigger `Event::EndFile`.
            mpv.playlist_next_force().unwrap();
        });
        scope.spawn(move |_| loop {
            let ev = ev_ctx.wait_event(600.).unwrap_or(Err(Error::Null));

            match ev {
                Ok(Event::EndFile(r)) => {
                    println!("Exiting! Reason: {:?}", r);
                    break;
                }

                Ok(Event::PropertyChange {
                    name: "demuxer-cache-state",
                    change: PropertyData::Node(mpv_node),
                    ..
                }) => {
                    let ranges = seekable_ranges(mpv_node).unwrap();
                    println!("Seekable ranges updated: {:?}", ranges);
                }
                Ok(e) => println!("Event triggered: {:?}", e),
                Err(e) => println!("Event errored: {:?}", e),
            }
        });
    })
    .unwrap();
    Ok(())
}

fn seekable_ranges(demuxer_cache_state: &MpvNode) -> Option<Vec<(f64, f64)>> {
    let mut res = Vec::new();
    let props: HashMap<&str, MpvNode> = demuxer_cache_state.to_map()?.collect();
    let ranges = props.get("seekable-ranges")?.to_array()?;

    for node in ranges {
        let range: HashMap<&str, MpvNode> = node.to_map()?.collect();
        let start = range.get("start")?.to_f64()?;
        let end = range.get("end")?.to_f64()?;
        res.push((start, end));
    }

    Some(res)
}
