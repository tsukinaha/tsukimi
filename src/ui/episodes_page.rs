use crate::ui::network;
use gtk::glib::{self, clone, BoxedAnyObject};
use gtk::{gio, prelude::*, Stack};
use gtk::{Box, Orientation};
use std::cell::Ref;

use super::image;
use super::network::runtime;

pub fn episodes_page(stack: Stack, series_info: Ref<network::SeriesInfo>, seriesid: String) -> Box {
    let pagebox = Box::new(Orientation::Vertical, 5);

    let introbox = Box::new(Orientation::Horizontal, 10);
    introbox.set_margin_start(15);
    introbox.set_margin_end(15);
    introbox.set_margin_top(15);
    let overlay = gtk::Overlay::new();
    let intropic = image::set_image(series_info.Id.clone());
    let label = gtk::Label::new(Some(&series_info.Name));
    let markup = format!(
        "<b>S{}E{}: {}</b>",
        series_info.ParentIndexNumber, series_info.IndexNumber, series_info.Name
    );
    label.set_markup(markup.as_str());

    let playbackinfovbox = Box::new(Orientation::Vertical, 5);
    let playbackinfobox = Box::new(Orientation::Vertical, 5);
    playbackinfovbox.set_hexpand(true);
    playbackinfobox.append(&label);

    let (sender, receiver) = async_channel::bounded::<network::Media>(1);
    let series_id = series_info.Id.clone();

    runtime().spawn(clone!(@strong sender =>async move {
        let playbackinfo = network::playbackinfo(series_id).await.expect("msg");
        sender.send(playbackinfo).await.expect("The channel needs to be open.");
    }));

    let series_id = series_info.Id.clone();
    let seriesoverview = series_info.Overview.clone();
    glib::spawn_future_local(
        clone!(@strong playbackinfobox,@strong playbackinfovbox => async move {
            while let Ok(playbackinfo) = receiver.recv().await {
                let mediadropsel = super::new_dropsel::newmediadropsel(playbackinfo, series_id.clone());
                playbackinfobox.append(&mediadropsel);
                playbackinfovbox.append(&playbackinfobox);
                if seriesoverview.is_some() {
                    let overview = gtk::Inscription::new(Some(&seriesoverview.as_ref().unwrap()));
                        overview.set_nat_lines(6);
                        overview.set_hexpand(true);
                        overview.set_valign(gtk::Align::Start);
                        playbackinfovbox.append(&overview);
                }
            }
        }),
    );

    overlay.set_child(Some(&intropic));
    overlay.set_size_request(300, 169);
    let vbox = Box::new(Orientation::Vertical, 5);
    vbox.append(&overlay);
    introbox.append(&vbox);
    introbox.append(&playbackinfovbox);
    introbox.set_hexpand(true);
    introbox.set_size_request(-1, 330);

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
        label.set_halign(gtk::Align::Start);
        let markup = format!(
            "<b>S{}E{}: {}</b>",
            seriesinfo.ParentIndexNumber, seriesinfo.IndexNumber, seriesinfo.Name
        );
        label.set_markup(markup.as_str());
        vbox.append(&label);

        if seriesinfo.Overview.is_some() {
            let overview = gtk::Inscription::new(Some(&seriesinfo.Overview.as_ref().unwrap()));
            overview.set_nat_lines(6);
            overview.set_hexpand(true);
            vbox.append(&overview);
        }
        let id = seriesinfo.Id.clone();
        let imgbox = image::set_image(id);
        imgbox.set_size_request(250, 141);
        imgbox.set_homogeneous(true);
        hbox.append(&imgbox);
        hbox.append(&vbox);
        listitem.set_child(Some(&hbox));
    });

    listfactory.connect_unbind(move |_factory, item| {
        let listitem = item.downcast_ref::<gtk::ListItem>().unwrap();
        listitem.set_child(None::<&gtk::Widget>);
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
        let seriesid = seriesidclone.clone();
        let episodes_page = episodes_page(stackclone, series_info, seriesid);
        let pagename = format!("episodes_page");
        if stack.child_by_name(&pagename).is_some() {
            stack.remove(&stack.child_by_name(&pagename).unwrap());
        }
        stack.add_named(&episodes_page, Some(&pagename));
        stack.set_visible_child_name(&pagename);
    });

    let scrolled_window = gtk::ScrolledWindow::new();
    let label = gtk::Label::new(Some("其他剧集"));
    let markup = format!("<b>其他剧集</b>");
    label.set_markup(markup.as_str());
    label.set_margin_start(11);
    label.set_halign(gtk::Align::Start);
    vbox.append(&label);

    let revealer = gtk::Revealer::new();
    revealer.set_transition_type(gtk::RevealerTransitionType::Crossfade);
    revealer.set_transition_duration(1000);
    revealer.set_child(Some(&listview));
    revealer.set_reveal_child(false);

    vbox.append(&revealer);
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
        revealer.set_reveal_child(true);
    });

    pagebox
}
