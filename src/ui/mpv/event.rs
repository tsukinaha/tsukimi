use gtk::prelude::*;
use libmpv::{events::*, *};
use std::{
    collections::HashMap,
    thread,
    time::{Duration, Instant},
};

use crate::{
    client::{network::*, structs::Back},
    config::set_config,
    utils::spawn_tokio_blocking,
    APP_ID,
};
pub fn play(
    url: String,
    suburl: Option<String>,
    name: Option<String>,
    back: &Back,
    percentage: Option<f64>,
) -> Result<()> {
    unsafe {
        use libc::setlocale;
        use libc::LC_NUMERIC;
        setlocale(LC_NUMERIC, "C\0".as_ptr() as *const _);
    }

    let server_info = set_config();
    let url = format!("{}:{}/emby{}", server_info.domain, server_info.port, url);
    let settings = gtk::gio::Settings::new(APP_ID);
    let interval = if settings.boolean("is-progress-enabled") {
        Duration::from_secs(10)
    } else {
        Duration::from_secs(300)
    };
    let mut duration: u64 = back.tick;
    // Create an `Mpv` and set some properties.
    let mpv = Mpv::with_initializer(|init| {
        init.set_property("osc", true)?;
        init.set_property("config", true)?;
        #[cfg(unix)]
        init.set_property("input-vo-keyboard", true)?;
        init.set_property("input-default-bindings", true)?;
        init.set_property("user-agent", "Tsukimi")?;
        if let Some(name) = name {
            init.set_property("force-media-title", name)?;
        }

        if settings.boolean("is-fullscreen") {
            init.set_property("fullscreen", true)?;
        }

        if settings.boolean("is-force-window") {
            init.set_property("force-window", "immediate")?;
        }

        if settings.boolean("is-resume") {
            if let Some(percentage) = percentage {
                init.set_property("start", format!("{}%", percentage))?;
            }
        }

        if !settings.string("proxy").is_empty() {
            init.set_property("http-proxy", settings.string("proxy").as_str())?;
        }

        #[cfg(windows)]
        {
            let mpv_config_dir = std::env::current_exe()
                .unwrap()
                .ancestors()
                .nth(2)
                .unwrap()
                .join("mpv");
            if mpv_config_dir.join("mpv.conf").exists() {
                init.set_property("config-dir", mpv_config_dir.display().to_string())?;
            }
        }

        Ok(())
    })
    .unwrap();
    mpv.set_property("volume", 85)?;

    let mut ev_ctx = mpv.create_event_context();
    ev_ctx.disable_deprecated_events()?;
    ev_ctx.observe_property("volume", Format::Int64, 0)?;
    ev_ctx.observe_property("time-pos", Format::Double, 0)?;

    let backc = back.clone();
    RUNTIME.spawn(async move {
        playstart(backc).await;
    });

    crossbeam::scope(|scope| {
        scope.spawn(|_| {
            mpv.playlist_load_files(&[(&url, FileState::AppendPlay, None)])
                .unwrap();
            thread::sleep(Duration::from_secs(1));
            if let Some(suburl) = suburl {
                let suburl = format!("{}:{}/emby{}", server_info.domain, server_info.port, suburl);
                println!("Loading subtitle");
                mpv.subtitle_add_select(&suburl, None, None).unwrap();
            }
        });
        let mut last_print = Instant::now();
        scope.spawn(move |_| loop {
            let ev = ev_ctx.wait_event(2000.).unwrap_or(Err(Error::Null));
            match ev {
                Ok(Event::EndFile(r)) => {
                    if r == 3 {
                        let mut back = back.clone();
                        back.tick = duration;
                        spawn_tokio_blocking(async {
                            positionstop(back).await;
                        });
                    }
                    println!("Exiting! Reason: {:?}", r);
                    break;
                }

                Ok(Event::PropertyChange {
                    name: "time-pos",
                    change: PropertyData::Double(mpv_node),
                    ..
                }) => {
                    duration = mpv_node as u64 * 10000000;
                    if last_print.elapsed() >= interval {
                        last_print = Instant::now();
                        let mut back = back.clone();
                        back.tick = duration;
                        RUNTIME.spawn(async move {
                            positionback(back).await;
                        });
                    }
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
