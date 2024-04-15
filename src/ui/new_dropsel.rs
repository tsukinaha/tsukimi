use gtk::glib;
use gtk::prelude::*;

use super::models::SETTINGS;
use super::mpv;
use super::network;
use super::network::get_sub;
use super::network::runtime;
use super::network::Back;
use super::network::Media;
use super::network::SeriesInfo;
use super::provider::dropdown_factory::factory;

pub fn newmediadropsel(
    playbackinfo: network::Media,
    info: &SeriesInfo,
    namedropdown: gtk::DropDown,
    subdropdown: gtk::DropDown,
    playbutton: gtk::Button,
) {
    let namelist = gtk::StringList::new(&[]);
    let sublist = gtk::StringList::new(&[]);

    if let Some(media) = &playbackinfo.media_sources.first() {
        for stream in &media.media_streams {
            if stream.stream_type == "Subtitle" {
                if let Some(d) = &stream.display_title {
                    sublist.append(d);
                } else {
                    println!("No value");
                }
            }
        }
    }
    for media in &playbackinfo.media_sources {
        namelist.append(&media.name);
    }
    namedropdown.set_model(Some(&namelist));
    subdropdown.set_model(Some(&sublist));
    namedropdown.set_factory(Some(&factory()));
    subdropdown.set_factory(Some(&factory()));

    namedropdown.connect_selected_item_notify(move |dropdown| {
        let selected = dropdown.selected_item();
        let selected = selected.and_downcast_ref::<gtk::StringObject>().unwrap();
        let selected = selected.string();
        for _i in 0..sublist.n_items() {
            sublist.remove(0);
        }
        for media in playbackinfo.media_sources.clone() {
            if media.name == selected {
                for stream in media.media_streams {
                    if stream.stream_type == "Subtitle" {
                        if let Some(d) = stream.display_title {
                            sublist.append(&d);
                        } else {
                            println!("No value");
                        }
                    }
                }
                break;
            }
        }
    });
    let info = info.clone();

    if SETTINGS.resume() {
        if let Some(userdata) = &info.user_data {
            if let Some(percentage) = userdata.played_percentage {
                if percentage > 0. {
                    playbutton.set_label("Resume");
                }
            }
        }
    }
}

pub fn bind_button(
    playbackinfo: network::Media,
    info: SeriesInfo,
    namedropdown: gtk::DropDown,
    subdropdown: gtk::DropDown,
    playbutton: gtk::Button,
) -> glib::SignalHandlerId {
    let handlerid = playbutton.connect_clicked(move |button| {
        button.set_label("Playing...");
        button.set_sensitive(false);
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
    userdata: Option<crate::ui::network::UserData>,
) {
    let (sender, receiver) = async_channel::bounded::<Media>(1);
    let idc = id.clone();
    runtime().spawn(async move {
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
