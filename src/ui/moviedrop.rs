use super::network;
use super::network::Back;
use super::network::SearchResult;
use super::new_dropsel::play_event;
use gtk::prelude::*;
use gtk::Orientation;

pub fn newmediadropsel(playbackinfo: network::Media, info: SearchResult) -> gtk::Box {
    let hbox = gtk::Box::new(Orientation::Horizontal, 5);
    let leftvbox = gtk::Box::new(Orientation::Vertical, 5);
    leftvbox.set_margin_start(80);
    leftvbox.set_margin_top(80);
    leftvbox.set_margin_bottom(20);
    leftvbox.set_halign(gtk::Align::Start);
    leftvbox.set_valign(gtk::Align::End);
    let label = gtk::Label::builder()
        .label(format!("<b>{}</b>", info.name))
        .use_markup(true)
        .build();
    leftvbox.append(&label);
    hbox.append(&leftvbox);
    let vbox = gtk::Box::new(Orientation::Vertical, 5);
    vbox.set_margin_start(20);
    vbox.set_margin_end(80);
    vbox.set_margin_bottom(20);
    vbox.set_halign(gtk::Align::End);
    vbox.set_hexpand(true);
    let namelist = gtk::StringList::new(&[]);

    let sublist = gtk::StringList::new(&[]);

    let mut _set = 1;
    for media in playbackinfo.media_sources.clone() {
        namelist.append(&media.name);
        if _set == 1 {
            for stream in media.media_streams {
                if stream.stream_type == "Subtitle" {
                    if let Some(d) = stream.display_title {
                        sublist.append(&d);
                    } else {
                        println!("No value");
                    }
                }
            }
        }
        _set = 0;
    }

    let namedropdown = gtk::DropDown::new(Some(namelist), Option::<gtk::Expression>::None);
    let subdropdown = gtk::DropDown::new(Some(sublist.clone()), Option::<gtk::Expression>::None);
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
    vbox.append(&namedropdown);
    vbox.append(&subdropdown);
    let info = info.clone();
    let playbutton = gtk::Button::with_label("Play");
    let settings = gtk::gio::Settings::new(crate::APP_ID);
    if settings.boolean("is-resume") {
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

    vbox.append(&playbutton);
    hbox.append(&vbox);
    hbox
}
