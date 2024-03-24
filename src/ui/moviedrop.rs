use super::network;
use super::network::SearchResult;
use gtk::prelude::*;
use gtk::Orientation;

pub fn newmediadropsel(playbackinfo: network::Media,info:SearchResult) -> gtk::Box {
    let hbox = gtk::Box::new(Orientation::Horizontal, 5);
    let leftvbox = gtk::Box::new(Orientation::Vertical, 5);
    leftvbox.set_margin_start(80);
    leftvbox.set_margin_top(80);
    leftvbox.set_margin_bottom(20);
    leftvbox.set_halign(gtk::Align::Start);
    leftvbox.set_valign(gtk::Align::End);
    let label = gtk::Label::new(Some(&info.Name));
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
        for _i in 1..sublist.n_items() {
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
    let playbutton = gtk::Button::with_label("播放");
    playbutton.connect_clicked(move |_| {
        let nameselected = namedropdown.selected_item();
        let nameselected = nameselected.and_downcast_ref::<gtk::StringObject>().unwrap();
        let nameselected = nameselected.string();
        let subselected = subdropdown.selected_item();
        if subselected.is_none() {
            for media in playback_info.MediaSources.clone() {
                if media.Name == nameselected {
                    let directurl = media.DirectStreamUrl.clone();
                    let name = info.Id.clone();
                    let sourceid = media.Id.clone();
                    network::runtime().spawn(async move {
                        let _ = network::markwatched(name,sourceid).await;
                    });
                    network::mpv_play(directurl.expect("no url"),media.Name.clone());
                }
            }
            return;
        }
        let subselected = subselected.and_downcast_ref::<gtk::StringObject>().unwrap();
        let subselected = subselected.string();
        for media in playback_info.MediaSources.clone() {
            if media.Name == nameselected {
                for mediastream in media.MediaStreams {
                    if mediastream.Type == "Subtitle" {
                        let displaytitle = mediastream.DisplayTitle.unwrap_or("".to_string());
                        if displaytitle == subselected {
                            let directurl = media.DirectStreamUrl.clone();
                            if mediastream.IsExternal == true {
                                let suburl = mediastream.DeliveryUrl.clone();
                                let name = info.Id.clone();
                                let sourceid = media.Id.clone();
                                network::runtime().spawn(async move {
                                    let _  = network::markwatched(name,sourceid).await;
                                });
                                let _ = network::mpv_play_withsub(directurl.expect("no url"),suburl.expect("no url"),media.Name.clone());
                            } else {
                                let name = info.Id.clone();
                                let sourceid = media.Id.clone();
                                network::runtime().spawn(async move {
                                    let _  = network::markwatched(name,sourceid).await;
                                });
                                let _ = network::mpv_play(directurl.expect("no url"),media.Name.clone());
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
