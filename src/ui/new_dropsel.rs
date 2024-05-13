use super::models::SETTINGS;
use super::mpv;
use crate::client::{network::*, structs::*};
use gtk::glib;
use gtk::prelude::*;


pub fn bind_button(
    playbackinfo: Media,
    info: SeriesInfo,
    namedropdown: gtk::DropDown,
    subdropdown: gtk::DropDown,
    playbutton: gtk::Button,
) -> glib::SignalHandlerId {
    let handlerid = playbutton.connect_clicked(move |button| {
        let nameselected = namedropdown.selected_item();
        let nameselected = nameselected
            .and_downcast_ref::<gtk::StringObject>()
            .unwrap();
        let nameselected = nameselected.string();
        let subselected = subdropdown.selected_item();
        if subselected.is_none() {
            for media in playbackinfo.media_sources.clone() {
                if media.name == nameselected {
                    let directurl = media.direct_stream_url.clone();
                    if let Some(userdata) = &info.user_data {
                        let back = Back {
                            id: info.id.clone(),
                            mediasourceid: media.id.clone(),
                            playsessionid: playbackinfo.play_session_id.clone(),
                            tick: userdata.playback_position_ticks.unwrap_or(0),
                        };
                        play_event(
                            button.clone(),
                            directurl,
                            None,
                            media.name,
                            back,
                            userdata.played_percentage,
                        );
                        return;
                    }
                }
            }
            return;
        }
        let subselected = subselected.and_downcast_ref::<gtk::StringObject>().unwrap();
        let subselected = subselected.string();
        for media in playbackinfo.media_sources.clone() {
            if media.name == nameselected {
                for mediastream in media.media_streams {
                    if mediastream.stream_type == "Subtitle" {
                        let displaytitle = mediastream.display_title.unwrap_or("".to_string());
                        if displaytitle == subselected {
                            if let Some(directurl) = media.direct_stream_url.clone() {
                                if mediastream.is_external {
                                    if let Some(suburl) = mediastream.delivery_url.clone() {
                                        if let Some(userdata) = &info.user_data {
                                            let back = Back {
                                                id: info.id.clone(),
                                                mediasourceid: media.id.clone(),
                                                playsessionid: playbackinfo.play_session_id.clone(),
                                                tick: userdata.playback_position_ticks.unwrap_or(0),
                                            };
                                            play_event(
                                                button.clone(),
                                                Some(directurl),
                                                Some(suburl),
                                                media.name,
                                                back,
                                                userdata.played_percentage,
                                            );
                                            return;
                                        }
                                    } else {
                                        // Ask Luke
                                        let userdata = info.user_data.clone();
                                        set_sub(
                                            info.id.clone(),
                                            media.id.clone(),
                                            nameselected.to_string(),
                                            subselected.to_string(),
                                            button.clone(),
                                            userdata,
                                        );
                                        return;
                                    }
                                } else if let Some(userdata) = &info.user_data {
                                    let back = Back {
                                        id: info.id.clone(),
                                        mediasourceid: media.id.clone(),
                                        playsessionid: playbackinfo.play_session_id.clone(),
                                        tick: userdata.playback_position_ticks.unwrap_or(0),
                                    };
                                    play_event(
                                        button.clone(),
                                        Some(directurl),
                                        None,
                                        media.name,
                                        back,
                                        userdata.played_percentage,
                                    );
                                    return;
                                }
                            }
                        }
                    }
                }
            }
        }
    });
    handlerid
}

pub fn play_event(
    button: gtk::Button,
    directurl: Option<String>,
    suburl: Option<String>,
    name: String,
    back: Back,
    percentage: Option<f64>,
) {
    let (sender, receiver) = async_channel::bounded(1);
    gtk::gio::spawn_blocking(move || {
        sender
            .send_blocking(false)
            .expect("The channel needs to be open.");
        match mpv::event::play(
            directurl.expect("no url"),
            suburl,
            Some(name),
            &back,
            percentage,
        ) {
            Ok(_) => {
                sender
                    .send_blocking(true)
                    .expect("The channel needs to be open.");
            }
            Err(e) => {
                eprintln!("Failed to play: {}", e);
            }
        };
    });
    glib::spawn_future_local(glib::clone!(@weak button =>async move {
        while let Ok(enable_button) = receiver.recv().await {
            if enable_button {
                if SETTINGS.resume() {
                    button.set_label("Resume");
                } else {
                    button.set_label("Play");
                }
            }
            button.set_sensitive(enable_button);
        }
    }));
}

pub fn set_sub(
    id: String,
    sourceid: String,
    nameselected: String,
    subselected: String,
    button: gtk::Button,
    userdata: Option<UserData>,
) {
    let (sender, receiver) = async_channel::bounded::<Media>(1);
    let idc = id.clone();
    RUNTIME.spawn(async move {
        match get_sub(idc, sourceid).await {
            Ok(media) => {
                sender.send(media).await.expect("series_info not received.");
            }
            Err(e) => eprintln!("Error: {}", e),
        }
    });
    glib::spawn_future_local(async move {
        while let Ok(media) = receiver.recv().await {
            for mediasource in media.media_sources.clone() {
                if mediasource.name == nameselected {
                    for mediastream in mediasource.media_streams {
                        if mediastream.stream_type == "Subtitle" {
                            let displaytitle = mediastream.display_title.unwrap_or("".to_string());
                            if displaytitle == subselected {
                                if let Some(directurl) = mediasource.direct_stream_url.clone() {
                                    if mediastream.is_external {
                                        if let Some(suburl) = mediastream.delivery_url.clone() {
                                            if let Some(userdata) = userdata {
                                                let back = Back {
                                                    id: id.clone(),
                                                    mediasourceid: mediasource.id.clone(),
                                                    playsessionid: media.play_session_id.clone(),
                                                    tick: userdata
                                                        .playback_position_ticks
                                                        .unwrap_or(0),
                                                };
                                                play_event(
                                                    button.clone(),
                                                    Some(directurl),
                                                    Some(suburl),
                                                    nameselected,
                                                    back,
                                                    userdata.played_percentage,
                                                );
                                                return;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    });
}
