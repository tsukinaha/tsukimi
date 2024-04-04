use super::mpv;
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
        .label(format!("<b>{}</b>", info.Name))
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
    for media in playbackinfo.MediaSources.clone() {
        namelist.append(&media.Name);
        if _set == 1 {
            for stream in media.MediaStreams {
                if stream.Type == "Subtitle" {
                    if let Some(d) = stream.DisplayTitle {
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
        for media in playbackinfo.MediaSources.clone() {
            if media.Name == selected {
                for stream in media.MediaStreams {
                    if stream.Type == "Subtitle" {
                        if let Some(d) = stream.DisplayTitle {
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
    playbutton.connect_clicked(move |button| {
        button.set_label("Playing...");
        let nameselected = namedropdown.selected_item();
        let nameselected = nameselected
            .and_downcast_ref::<gtk::StringObject>()
            .unwrap();
        let nameselected = nameselected.string();
        let subselected = subdropdown.selected_item();
        if subselected.is_none() {
            for media in playback_info.MediaSources.clone() {
                if media.Name == nameselected {
                    let directurl = media.DirectStreamUrl.clone();
                    let back = Back {
                        id: info.Id.clone(),
                        mediasourceid: media.Id.clone(),
                        playsessionid: playback_info.PlaySessionId.clone(),
                        tick: 0.,
                    };
                    play_event(button.clone(),directurl,media.Name,back);
                    return;
                }
            }
            return;
        }
        let subselected = subselected.and_downcast_ref::<gtk::StringObject>().unwrap();
        let subselected = subselected.string();
        for media in playback_info.MediaSources.clone() {
            if media.Name == nameselected.to_string() {
                for mediastream in media.MediaStreams {
                    if mediastream.Type == "Subtitle" {
                        let displaytitle = mediastream.DisplayTitle.unwrap_or("".to_string());
                        if displaytitle == subselected {
                            if let Some(directurl) = media.DirectStreamUrl.clone() {
                                if mediastream.IsExternal == true {
                                    if let Some(_suburl) = mediastream.DeliveryUrl.clone() {
                                        let back = Back {
                                            id: info.Id.clone(),
                                            mediasourceid: media.Id.clone(),
                                            playsessionid: playback_info.PlaySessionId.clone(),
                                            tick: 0.,
                                        };
                                        play_event(button.clone(),Some(directurl),media.Name,back);
                                        return;
                                    } 
                                } else {
                                    let back = Back {
                                        id: info.Id.clone(),
                                        mediasourceid: media.Id.clone(),
                                        playsessionid: playback_info.PlaySessionId.clone(),
                                        tick: 0.,
                                    };
                                    play_event(button.clone(),Some(directurl),media.Name,back);
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
