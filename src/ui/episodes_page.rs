use crate::ui::network;




use gtk::glib::{self, clone, BoxedAnyObject};
use gtk::{gio, prelude::*, Stack};
use gtk::{Box, Orientation};
use std::cell::Ref;

use super::image::set_image;
use super::network::{runtime};

pub fn episodes_page(stack: Stack, series_info: Ref<network::SeriesInfo>, seriesid: String) -> Box {
    let pagebox = Box::new(Orientation::Vertical, 5);

    let introbox = Box::new(Orientation::Horizontal, 10);
    introbox.set_margin_start(9);
    let overlay = gtk::Overlay::new();
    let intropic = set_image(series_info.Id.clone());
    let label = gtk::Label::new(Some(&series_info.Name));
    let markup = format!(
        "<b>S{}E{}: {}</b>",
        series_info.ParentIndexNumber, series_info.IndexNumber, series_info.Name
    );
    label.set_markup(markup.as_str());

    let playbackinfobox = Box::new(Orientation::Vertical, 5);
    playbackinfobox.set_hexpand(true);
    let overview = gtk::Inscription::new(Some(&series_info.Overview));
    overview.set_nat_lines(6);
    overview.set_hexpand(true);
    overview.set_valign(gtk::Align::Start);
    playbackinfobox.append(&label);

    let (sender, receiver) = async_channel::bounded::<network::Media>(1);
    let series_id = series_info.Id.clone();
    
    runtime().spawn(clone!(@strong sender =>async move {
        let playbackinfo = network::playbackinfo(series_id).await.expect("msg");
        sender.send(playbackinfo).await.expect("The channel needs to be open.");
    }));

    glib::spawn_future_local(clone!(@strong playbackinfobox => async move {
        while let Ok(playbackinfo) = receiver.recv().await {
            let mediadropsel = super::new_dropsel::newmediadropsel(playbackinfo);
            playbackinfobox.append(&mediadropsel);
            playbackinfobox.append(&overview);
        }
    }));

    overlay.set_child(Some(&intropic));
    overlay.set_size_request(300, 169);
    let vbox = Box::new(Orientation::Vertical, 5);
    vbox.append(&overlay);
    introbox.append(&vbox);
    introbox.append(&playbackinfobox);
    introbox.set_hexpand(true);
    introbox.set_size_request(-1, 320);

    let vbox = Box::new(Orientation::Vertical, 5);
    vbox.append(&introbox);

    let store = gio::ListStore::new::<BoxedAnyObject>();
    let store_clone = store.clone();
    let sel = gtk::SingleSelection::new(Some(store_clone));
    let listfactory = gtk::SignalListItemFactory::new();

    listfactory.connect_bind(move |_factory, item| {
        let hbox = Box::new(Orientation::Horizontal, 10);
        let listitem = item.downcast_ref::<gtk::ListItem>().unwrap();
        let entry = listitem.item().and_downcast::<BoxedAnyObject>().unwrap();
        let seriesinfo: Ref<network::SeriesInfo> = entry.borrow();
        let vbox = Box::new(Orientation::Vertical, 5);
        let label = gtk::Label::new(Some(&seriesinfo.Name));
        let overview = gtk::Inscription::new(Some(&seriesinfo.Overview));
        label.set_halign(gtk::Align::Start);
        let markup = format!(
            "<b>S{}E{}: {}</b>",
            seriesinfo.ParentIndexNumber, seriesinfo.IndexNumber, seriesinfo.Name
        );
        label.set_markup(markup.as_str());
        overview.set_nat_lines(6);
        overview.set_hexpand(true);
        vbox.append(&label);
        vbox.append(&overview);
        let id = seriesinfo.Id.clone();
        let imgbox = set_image(id);
        imgbox.set_size_request(250, 141);
        imgbox.set_homogeneous(true);
        hbox.append(&imgbox);
        hbox.append(&vbox);
        listitem.set_child(Some(&hbox));
    });

    let seriesidclone = seriesid.clone();
    let listview = gtk::ListView::new(Some(sel), Some(listfactory));
    listview.connect_activate(move |listview, position| {
        let model = listview.model().unwrap();
        let item = model
            .item(position)
            .and_downcast::<BoxedAnyObject>()
            .unwrap();
        let series_info: Ref<network::SeriesInfo> = item.borrow();
        let stackclone = stack.clone();
        let id = series_info.Id.clone();
        let seriesid = seriesidclone.clone();
        let episodes_page = episodes_page(stackclone, series_info, seriesid);
        let pagename = format!("episodes_page_{}", id);
        if stack.child_by_name(&pagename).is_none() {
            stack.add_named(&episodes_page, Some(&pagename));
        }
        stack.set_visible_child_name(&pagename);
    });

    let scrolled_window = gtk::ScrolledWindow::new();
    let label = gtk::Label::new(Some("其他剧集"));
    let markup = format!("<b>其他剧集</b>");
    label.set_markup(markup.as_str());
    label.set_margin_start(11);
    label.set_halign(gtk::Align::Start);
    vbox.append(&label);
    vbox.append(&listview);
    scrolled_window.set_child(Some(&vbox));
    scrolled_window.set_vexpand(true);
    pagebox.append(&scrolled_window);

    let series_id = seriesid.clone();

    let (sender, receiver) = async_channel::bounded::<Vec<network::SeriesInfo>>(1);
    network::runtime().spawn(async move {
        match network::get_series_info(series_id).await {
            Ok(series_info) => {
                sender
                    .send(series_info)
                    .await
                    .expect("series_info not received.");
            }
            Err(e) => eprintln!("Error: {}", e),
        }
    });

    glib::spawn_future_local(async move {
        let series_info = receiver.recv().await.expect("series_info not received.");
        for info in series_info {
            let object = BoxedAnyObject::new(info);
            store.append(&object);
        }
    });

    pagebox
}
