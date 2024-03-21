use crate::ui::network;
use crate::ui::network::SearchResult;
use gtk::glib::{self, clone,  BoxedAnyObject};
use gtk::{gio, prelude::*, Stack};
use gtk::{Box, Orientation};
use std::cell::Ref;
use super::episodes_page;
use super::network::{get_image, runtime};

pub fn itempage(result: Ref<SearchResult>) -> Box {
    let pagebox = Box::new(Orientation::Vertical, 5);

    let introbox = Box::new(Orientation::Horizontal, 10);
    introbox.set_margin_start(11);
    let intropic = gtk::Picture::new();
    intropic.set_size_request(221, 325);

    let series_id = result.Id.clone();
    
    let (sender, receiver) = async_channel::bounded::<String>(1);
    runtime().spawn(clone!(@strong sender =>async move {
        let id = get_image(series_id).await.expect("msg");
        sender.send(id).await.expect("The channel needs to be open.");
    }));

    glib::spawn_future_local(clone!(@strong intropic => async move {
        while let Ok(id) = receiver.recv().await {
            let path = format!("{}/.local/share/tsukimi/{}.png",dirs::home_dir().expect("msg").display(), id);
            let file = gtk::gio::File::for_path(&path);
            intropic.set_file(Some(&file));
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
        label.set_halign(gtk::Align::Start);
        let markup = format!("<b>S{}E{}: {}</b>", seriesinfo.ParentIndexNumber, seriesinfo.IndexNumber, seriesinfo.Name);
        label.set_markup(markup.as_str());
        vbox.append(&label);
        if seriesinfo.Overview.is_some() {
            let overview = gtk::Inscription::new(Some(&seriesinfo.Overview.as_ref().unwrap()));
            overview.set_nat_lines(6);
            overview.set_hexpand(true);
            vbox.append(&overview);
        }
        let hbox = Box::new(Orientation::Horizontal, 10);
        let imgbox = crate::ui::image::set_image(seriesinfo.Id.clone());
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

    let resultid = result.Id.clone();
    let listview = gtk::ListView::new(Some(sel), Some(listfactory));
    listview.connect_activate(move |listview, position| {
        let model = listview.model().unwrap();
        let item = model.item(position).and_downcast::<BoxedAnyObject>().unwrap();
        let series_info: Ref<network::SeriesInfo> = item.borrow();
        let resultid = resultid.clone();
    });

    let scrolled_window = gtk::ScrolledWindow::new();

    let revealer = gtk::Revealer::new();
    revealer.set_transition_type(gtk::RevealerTransitionType::Crossfade);
    revealer.set_transition_duration(600);
    revealer.set_child(Some(&listview));
    revealer.set_reveal_child(false);

    vbox.append(&revealer);
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
        revealer.set_reveal_child(true);
    });

    pagebox
}
