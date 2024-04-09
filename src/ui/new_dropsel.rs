use super::mpv;
use super::network;
use super::network::get_sub;
use super::network::runtime;
use super::network::Back;
use super::network::Media;
use super::network::SeriesInfo;
use gtk::glib;
use gtk::prelude::*;
use gtk::Orientation;

pub fn newmediadropsel(playbackinfo: network::Media, info: SeriesInfo) -> gtk::Box {
    let hbox = gtk::Box::new(Orientation::Horizontal, 5);
    hbox.set_valign(gtk::Align::End);
    hbox.set_vexpand(true);
    let leftvbox = gtk::Box::new(Orientation::Vertical, 5);
    leftvbox.set_margin_start(80);
    leftvbox.set_margin_top(80);
    leftvbox.set_margin_bottom(20);
    leftvbox.set_halign(gtk::Align::Start);
    leftvbox.set_valign(gtk::Align::End);
    let markup = format!(
        "<b>{}\n\nSeason {} : Episode {}</b>",
        info.name, info.parent_index_number, info.index_number
    );
    let label = gtk::Label::new(Some(&info.name));
    label.set_markup(markup.as_str());
    label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    leftvbox.append(&label);
    hbox.append(&leftvbox);
    let vbox = gtk::Box::new(Orientation::Vertical, 5);
    vbox.set_margin_start(20);
    vbox.set_margin_end(80);
    vbox.set_margin_bottom(20);
    vbox.set_halign(gtk::Align::End);
    vbox.set_valign(gtk::Align::End);
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
        button.set_sensitive(false);
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
                let settings = gtk::gio::Settings::new(crate::APP_ID);
                if settings.boolean("is-resume") {
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
