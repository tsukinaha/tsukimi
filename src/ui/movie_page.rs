use gtk::{
    glib::{self, clone},
    prelude::*,
    Box, Orientation,
};

use super::{
    image::set_image,
    network::{self, runtime, SearchResult},
};

pub fn movie_page(result: std::cell::Ref<'_, SearchResult>) -> Box {
    let pagebox = Box::new(Orientation::Vertical, 5);
    let introbox = Box::new(Orientation::Horizontal, 10);
    introbox.set_margin_start(9);
    let overlay = gtk::Overlay::new();
    let intropic = set_image(result.Id.clone());
    let label = gtk::Label::new(Some(&result.Name));
    let markup = format!("<b>{}</b>", result.Name);
    label.set_markup(markup.as_str());

    let playbackinfobox = Box::new(Orientation::Vertical, 5);
    playbackinfobox.set_hexpand(true);
    playbackinfobox.append(&label);

    let (sender, receiver) = async_channel::bounded::<network::Media>(1);
    let series_id = result.Id.clone();

    runtime().spawn(clone!(@strong sender =>async move {
        let playbackinfo = network::playbackinfo(series_id.clone()).await.expect("msg");
        sender.send(playbackinfo).await.expect("The channel needs to be open.");
    }));

    let seriesid = result.Id.clone();
    glib::spawn_future_local(clone!(@strong playbackinfobox => async move {
        while let Ok(playbackinfo) = receiver.recv().await {
            let mediadropsel = super::new_dropsel::newmediadropsel(playbackinfo, seriesid.clone());
            playbackinfobox.append(&mediadropsel);
        }
    }));

    overlay.set_child(Some(&intropic));
    overlay.set_size_request(400, 225);
    let vbox = Box::new(Orientation::Vertical, 5);
    vbox.append(&overlay);
    introbox.append(&vbox);
    introbox.append(&playbackinfobox);
    introbox.set_hexpand(true);
    introbox.set_size_request(-1, 320);

    let vbox = Box::new(Orientation::Vertical, 5);
    vbox.append(&introbox);

    pagebox.append(&vbox);
    pagebox
}
