use super::mpv;
use super::network;
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
        info.Name, info.ParentIndexNumber, info.IndexNumber
    );
    let label = gtk::Label::new(Some(&info.Name));
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
                    let name = info.Id.clone();
                    let sourceid = media.Id.clone();
                    network::runtime().spawn(async move {
                        let _ = network::markwatched(name, sourceid).await;
                    });
                    gtk::gio::spawn_blocking(move || {
                        let _ = super::mpv::event::play(directurl.expect("no url"),None);
                    });
                    return;
                }
            }
            return;
        }
        let subselected = subselected.and_downcast_ref::<gtk::StringObject>().unwrap();
        let subselected = subselected.string();
        for media in playback_info.MediaSources.clone() {
            if media.Name == nameselected {
                for mediastream in media.MediaStreams {
                    let sub = subselected.clone();
                    if mediastream.Type == "Subtitle" {
                        let displaytitle = mediastream.DisplayTitle.unwrap_or("".to_string());
                        if displaytitle == subselected {
                            if let Some(directurl) = media.DirectStreamUrl.clone() {
                                if mediastream.IsExternal == true {
                                    if let Some(suburl) = mediastream.DeliveryUrl.clone() {
                                        let name = info.Id.clone();
                                        let sourceid = media.Id.clone();
                                        network::runtime().spawn(async move {
                                            let _ = network::markwatched(name, sourceid).await;
                                        });
                                        mpv::event::play(
                                            directurl.clone(),
                                            mediastream.DeliveryUrl,
                                        )
                                        .unwrap();
                                        return;
                                    } else {
                                        let name = info.Id.clone();
                                        let sourceid = media.Id.clone();
                                        button.set_label("Loading Subtitle");
                                        network::runtime().spawn(async move {
                                            let media = network::playbackinfo_withmediaid(name, sourceid).await;
                                            let media = media.unwrap();
                                            for mediasource in media.MediaSources {
                                                for mediastream in mediasource.MediaStreams {
                                                    if mediastream.Type == "Subtitle" {
                                                        if displaytitle == sub {
                                                            mpv::event::play(
                                                                directurl.clone(),
                                                                mediastream.DeliveryUrl,
                                                            )
                                                            .unwrap();
                                                        }
                                                    }
                                                }
                                            }
                                        });
                                    }
                                } else {
                                    let name = info.Id.clone();
                                    let sourceid = media.Id.clone();
                                    network::runtime().spawn(async move {
                                        let _ = network::markwatched(name, sourceid).await;
                                    });
                                    gtk::gio::spawn_blocking(move || {
                                        let _ = super::mpv::event::play(media.DirectStreamUrl.clone().unwrap(),None);
                                    });
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
