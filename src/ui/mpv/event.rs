use libmpv::{events::*, *};

use std::{collections::HashMap, env, thread, time::Duration};

use crate::config::set_config;

pub fn play(url:String,suburl:Option<String>) -> Result<()> {

    unsafe {
        use libc::setlocale;
        use libc::LC_NUMERIC;
        setlocale(LC_NUMERIC, "C\0".as_ptr() as *const _);
    }

    let server_info = set_config();
    let url = format!("{}:{}/emby{}", server_info.domain, server_info.port, url);

    // Create an `Mpv` and set some properties.
    let mpv = Mpv::with_initializer(|init| {
        init.set_property("osc", true)?;
        init.set_property("config", true)?;
        init.set_property("input-default-bindings", true)?;
        init.set_property("force-window", "immediate")?;
        
    
        Ok(())
    }).unwrap();
    mpv.set_property("volume", 75)?;

    let mut ev_ctx = mpv.create_event_context();
    ev_ctx.disable_deprecated_events()?;
    ev_ctx.observe_property("volume", Format::Int64, 0)?;
    ev_ctx.observe_property("demuxer-cache-state", Format::Node, 0)?;

    crossbeam::scope(|scope| {
        scope.spawn(|_| {
            mpv.playlist_load_files(&[(&url, FileState::AppendPlay, None)])
                .unwrap();
            thread::sleep(Duration::from_secs(1));
            if let Some(suburl) = suburl {
                let suburl = format!("{}:{}/emby{}", server_info.domain, server_info.port, suburl);
                println!("Loading subtitle: {}", suburl);
                mpv.subtitle_add_select(&suburl, None, None)
                 .unwrap();
            }
        });
        scope.spawn(move |_| loop {
            let ev = ev_ctx.wait_event(1000.).unwrap_or(Err(Error::Null));

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