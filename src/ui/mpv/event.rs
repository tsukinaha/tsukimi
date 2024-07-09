use gtk::prelude::*;
use libmpv2::{events::*, *};

use std::{
    thread,
    time::{Duration, Instant},
};

use crate::{
    client::{client::EMBY_CLIENT, network::*, structs::Back},
    APP_ID,
};
pub fn play(
    url: String,
    suburl: Option<String>,
    name: Option<String>,
    back: Option<Back>,
    percentage: Option<f64>,
) -> Result<()> {
    unsafe {
        use libc::setlocale;
        use libc::LC_NUMERIC;
        setlocale(LC_NUMERIC, "C\0".as_ptr() as *const _);
    }

    let url = EMBY_CLIENT.get_streaming_url(&url);
    let settings = gtk::gio::Settings::new(APP_ID);
    let interval = if settings.boolean("is-progress-enabled") {
        Duration::from_secs(10)
    } else {
        Duration::from_secs(300)
    };
    let mut duration: u64 = back.clone().map_or(0, |b| b.tick);
    // Create an `Mpv` and set some properties.
    let mpv = Mpv::with_initializer(|init| {
        init.set_property("osc", true)?;
        init.set_property("config", true)?;
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

        if settings.boolean("ytdl") {
            init.set_property("ytdl-format", "best")?;
        }

        if settings.boolean("is-resume") {
            if let Some(percentage) = percentage {
                init.set_property("start", format!("{}%", percentage as u32))?;
            }
        }

        Ok(())
    })
    .unwrap();
    mpv.set_property("volume", 75)?;

    let mut ev_ctx = EventContext::new(mpv.ctx);
    ev_ctx.disable_deprecated_events()?;
    ev_ctx.observe_property("volume", Format::Int64, 0)?;
    ev_ctx.observe_property("time-pos", Format::Double, 0)?;

    if let Some(back) = back.clone() {
        RUNTIME.spawn(async move { EMBY_CLIENT.position_start(&back).await });
    }
    crossbeam::scope(|scope| {
        scope.spawn(|_| {
            mpv.command("loadfile", &[&url, "append-play"]).unwrap();
            thread::sleep(Duration::from_secs(1));
            if let Some(suburl) = suburl {
                let url = EMBY_CLIENT.get_streaming_url(&suburl);
                mpv.command("sub-add", &[&url, "select"]).unwrap();
            }
        });
        let mut last_print = Instant::now();
        scope.spawn(move |_| loop {
            let ev = ev_ctx.wait_event(10000.).unwrap_or(Err(Error::Null));
            match ev {
                Ok(Event::EndFile(r)) => {
                    if r == 3 {
                        if let Some(mut back) = back.clone() {
                            back.tick = duration;
                            RUNTIME.spawn(async move { EMBY_CLIENT.position_stop(&back).await });
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
                    duration = mpv_node as u64 * 10000000;
                    if last_print.elapsed() >= interval {
                        last_print = Instant::now();
                        if let Some(mut back) = back.clone() {
                            back.tick = duration;
                            RUNTIME.spawn(async move { EMBY_CLIENT.position_back(&back).await });
                        }
                    }
                }
                Ok(e) => println!("Event triggered: {:?}", e),
                Err(e) => println!("Event errored: {:?}", e),
            }
        });
    })
    .unwrap();
    Ok(())
}
