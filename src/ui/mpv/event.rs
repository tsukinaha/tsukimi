use gtk::prelude::*;
use libmpv::{events::*, *};

use std::{collections::HashMap, env, thread, time::{Duration, Instant}};

use crate::{config::set_config, ui::network::{runtime, Back}, APP_ID};
pub fn play(url:String,suburl:Option<String>,name:Option<String>,back:&Back,percentage:Option<f64>) -> Result<()> {

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
        init.set_property("input-vo-keyboard", true)?;
        init.set_property("input-default-bindings", true)?;
        
        if let Some(name) = name {
            init.set_property("force-media-title", name)?;
        }
        
        let settings = gtk::gio::Settings::new(APP_ID);

        if settings.boolean("is-fullscreen") {
            init.set_property("fullscreen", true)?;
        }

        if settings.boolean("is-force-window") {
            init.set_property("force-window", "immediate")?;
        }

        if settings.boolean("is-resume") {
            if let Some(percentage) = percentage {
                init.set_property("start", format!("{}%",percentage as u32))?;
            }
        }

        Ok(())
    }).unwrap();
    mpv.set_property("volume", 75)?;

    let mut ev_ctx = mpv.create_event_context();
    ev_ctx.disable_deprecated_events()?;
    ev_ctx.observe_property("volume", Format::Int64, 0)?;
    ev_ctx.observe_property("time-pos", Format::Double, 0)?;

    let backc = back.clone();
    std::env::set_var("DURATION", (&backc.tick / 10000000).to_string());
    runtime().spawn(async move {
        crate::ui::network::playstart(backc).await;
    });    

    
    crossbeam::scope(|scope| {
        scope.spawn(|_| {
            mpv.playlist_load_files(&[(&url, FileState::AppendPlay, None)])
                .unwrap();
            thread::sleep(Duration::from_secs(1));
            if let Some(suburl) = suburl {
                let suburl = format!("{}:{}/emby{}", server_info.domain, server_info.port, suburl);
                println!("Loading subtitle");
                mpv.subtitle_add_select(&suburl, None, None)
                 .unwrap();
            }
            
        });
        let mut last_print = Instant::now();
        scope.spawn(move |_| loop {
            let ev = ev_ctx.wait_event(10000.).unwrap_or(Err(Error::Null));
            match ev {
                Ok(Event::EndFile(r)) => {
                    if r == 3 {
                        if let Ok(duration) = env::var("DURATION") {
                            println!("Duration: {}", duration);
                            let tick = duration.parse::<f64>().unwrap() as u64 * 10000000;
                            let mut back = back.clone();
                            back.tick = tick;
                            runtime().spawn(async move {
                                crate::ui::network::positionstop(back).await;
                            });
                        }
                    }
                    println!("Exiting! Reason: {:?}", r);
                    break;
                }

                Ok(Event::PropertyChange {
                    name: "time-pos",
                    change: PropertyData::Double(mpv_node),
                    ..
                }) => {
                    if last_print.elapsed() >= Duration::from_secs(10) {
                        std::env::set_var("DURATION", mpv_node.to_string());
                            last_print = Instant::now();
                            let settings = gtk::gio::Settings::new(APP_ID);
                            if last_print.elapsed() >= Duration::from_secs(300) || settings.boolean("is-progress-enabled") {
                            if let Ok(duration) = env::var("DURATION") {
                                let tick = duration.parse::<f64>().unwrap() as u64 * 10000000;
                                let mut back = back.clone();
                                println!("Position: {}", tick);
                                back.tick = tick;
                                runtime().spawn(async move {
                                    crate::ui::network::positionback(back).await;
                                });
                            }
                        }
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