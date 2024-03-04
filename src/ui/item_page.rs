use crate::ui::network;
use crate::ui::network::SearchResult;
use gtk::gdk_pixbuf::Pixbuf;
use gtk::gio::{Cancellable, MemoryInputStream};
use gtk::glib::{self, clone, BoxedAnyObject};
use gtk::{gio, prelude::*, Stack};
use gtk::{Box, Orientation};
use std::cell::Ref;
use super::episodes_page;
use super::network::{get_image, runtime};

pub fn itempage(stack: Stack, result: Ref<SearchResult>) -> Box {
    let pagebox = Box::new(Orientation::Vertical, 5);

    let introbox = Box::new(Orientation::Horizontal, 10);
    introbox.set_margin_start(11);
    let intropic = gtk::Picture::new();
    intropic.set_size_request(221, 325);

    let series_id = result.Id.clone();
    
    let (sender, receiver) = async_channel::bounded::<Vec<u8>>(1);
    runtime().spawn(clone!(@strong sender =>async move {
        let bytes = get_image(series_id).await.expect("msg");
        sender.send(bytes).await.expect("The channel needs to be open.");
    }));

    glib::spawn_future_local(clone!(@strong intropic => async move {
        while let Ok(bytes) = receiver.recv().await {
            let bytes = glib::Bytes::from(&bytes.to_vec());
            let stream = MemoryInputStream::from_bytes(&bytes);
            let cancellable = Cancellable::new();
            let cancellable= Some(&cancellable);
            let pixbuf = Pixbuf::from_stream(&stream, cancellable).unwrap();
            intropic.set_pixbuf(Some(&pixbuf));
        }
    }));

    let label = gtk::Label::new(Some(&result.Name));
    let markup = format!("<b>{}</b>", result.Name);
    label.set_markup(markup.as_str());
    let introvbox = Box::new(Orientation::Vertical, 5);
    introbox.append(&intropic);
    introvbox.append(&label);
    introbox.append(&introvbox);

    let vbox = Box::new(Orientation::Vertical, 5);
    vbox.append(&introbox);

    let store = gio::ListStore::new::<BoxedAnyObject>();
    let store_clone = store.clone();
    let sel = gtk::SingleSelection::new(Some(store_clone));
    let listfactory = gtk::SignalListItemFactory::new();

    listfactory.connect_bind(move |_factory, item| {
        let listitem = item.downcast_ref::<gtk::ListItem>().unwrap();
        let entry = listitem.item().and_downcast::<BoxedAnyObject>().unwrap();
        let seriesinfo: Ref<network::SeriesInfo> = entry.borrow();
        let vbox = Box::new(Orientation::Vertical, 5);
        let label = gtk::Label::new(Some(&seriesinfo.Name));
        let overview = gtk::Inscription::new(Some(&seriesinfo.Overview));
        label.set_halign(gtk::Align::Start);
        let markup = format!("<b>S{}E{}: {}</b>", seriesinfo.ParentIndexNumber, seriesinfo.IndexNumber, seriesinfo.Name);
        label.set_markup(markup.as_str());
        overview.set_nat_lines(6);
        overview.set_hexpand(true);
        vbox.append(&label);
        vbox.append(&overview);
        let hbox = Box::new(Orientation::Horizontal, 10);
        let imgbox = crate::ui::image::set_image(seriesinfo.Id.clone());
        imgbox.set_size_request(250, 141);
        imgbox.set_homogeneous(true);
        hbox.append(&imgbox);
        hbox.append(&vbox);
        listitem.set_child(Some(&hbox));
    });

    let resultid = result.Id.clone();
    let listview = gtk::ListView::new(Some(sel), Some(listfactory));
    listview.connect_activate(move |listview, position| {
        let model = listview.model().unwrap();
        let item = model.item(position).and_downcast::<BoxedAnyObject>().unwrap();
        let series_info: Ref<network::SeriesInfo> = item.borrow();
        let stackclone = stack.clone();
        let id = series_info.Id.clone();
        let resultid = resultid.clone();
        let episodes_page = episodes_page::episodes_page(stackclone, series_info,resultid);
        let pagename = format!("episodes_page_{}", id);
        if stack.child_by_name(&pagename).is_none() {
            stack.add_named(&episodes_page, Some(&pagename));
        } 
        stack.set_visible_child_name(&pagename);
    });

    let scrolled_window = gtk::ScrolledWindow::new();
    vbox.append(&listview);
    scrolled_window.set_child(Some(&vbox));
    scrolled_window.set_vexpand(true);
    pagebox.append(&scrolled_window);

    let series_id = result.Id.clone();

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
