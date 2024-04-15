use gtk::prelude::*;

use super::models::SETTINGS;
use super::network;
use super::network::Back;
use super::network::SearchResult;
use super::new_dropsel::play_event;
use super::provider::dropdown_factory::factory;

pub fn newmediadropsel(
    playbackinfo: network::Media,
    info: SearchResult,
    namedropdown: gtk::DropDown,
    subdropdown: gtk::DropDown,
    playbutton: gtk::Button,
) {
    let namelist = gtk::StringList::new(&[]);

    let sublist = gtk::StringList::new(&[]);

    if let Some(media) = playbackinfo.media_sources.first() {
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
    let playback_info = playbackinfo.clone();

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

    playbutton.connect_clicked(move |button| {
        button.set_label("Playing...");
        let nameselected = namedropdown.selected_item();
        let nameselected = nameselected
            .and_downcast_ref::<gtk::StringObject>()
            .unwrap();
        let nameselected = nameselected.string();
        let subselected = subdropdown.selected_item();
        if subselected.is_none() {
            for media in playback_info.media_sources.clone() {
                if media.name == nameselected {
                    let directurl = media.direct_stream_url.clone();
                    if let Some(userdata) = &info.user_data {
                        let back = Back {
                            id: info.id.clone(),
                            mediasourceid: media.id.clone(),
                            playsessionid: playback_info.play_session_id.clone(),
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
        for media in playback_info.media_sources.clone() {
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
                                                playsessionid: playback_info
                                                    .play_session_id
                                                    .clone(),
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
                                        let userdata = info.user_data.clone();
                                        super::new_dropsel::set_sub(
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
                                        playsessionid: playback_info.play_session_id.clone(),
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
}
