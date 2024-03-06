use crate::ui::network::SearchResult;
use crate::ui::network::{self, Resume};
use async_channel::bounded;
use gtk::glib::{self, clone, BoxedAnyObject};
use gtk::{gio, pango, prelude::*, Stack};
use gtk::{Box, Button, Label, Orientation, ScrolledWindow};
use std::cell::{Ref, RefCell};

use super::image;

pub fn create_page(homestack: Stack, backbutton: Button) -> Stack {
    let hbox = Box::new(Orientation::Horizontal, 10);
    let vbox = Box::new(Orientation::Vertical, 5);
    let store = gio::ListStore::new::<BoxedAnyObject>();
    let store_clone = store.clone();
    let sel = gtk::SingleSelection::new(Some(store_clone));
    let gridfactory = gtk::SignalListItemFactory::new();

    gridfactory.connect_bind(move |_factory, item| {
        let listitem = item.downcast_ref::<gtk::ListItem>().unwrap();
        let entry = listitem.item().and_downcast::<BoxedAnyObject>().unwrap();
        let result: Ref<Resume> = entry.borrow();
        let vbox = Box::new(Orientation::Vertical, 5);
        let overlay = gtk::Overlay::new();
        let imgbox ;
        if result.ParentThumbItemId.is_some() {
            imgbox = crate::ui::image::set_thumbimage(result.ParentThumbItemId.as_ref().expect("").clone());
        } else {
            if result.Type == "Movie" {
                imgbox = crate::ui::image::set_backdropimage(result.Id.clone());
            } else {
                imgbox = crate::ui::image::set_backdropimage(result.SeriesId.as_ref().expect("").to_string());
            }
        }
        imgbox.set_size_request(300, 169);
        overlay.set_child(Some(&imgbox));
        overlay.set_size_request(300, 169);
        vbox.append(&overlay);
        let label = Label::new(Some(&result.Name));
        let markup = format!("{}", result.Name);
        label.set_markup(markup.as_str());
        label.set_wrap(true);
        label.set_size_request(-1, 24);
        label.set_ellipsize(pango::EllipsizeMode::End);
        let labeltype = Label::new(Some(&result.Type));
        let markup = format!("<span color='lightgray' font='10'>{}</span>", result.Type);
        labeltype.set_markup(markup.as_str());
        labeltype.set_size_request(-1, 24);
        vbox.append(&label);
        vbox.append(&labeltype);
        listitem.set_child(Some(&vbox));
    });

    let gridview = gtk::GridView::new(Some(sel), Some(gridfactory));
    let scrolled_window = ScrolledWindow::new();
    scrolled_window.set_child(Some(&gridview));
    scrolled_window.set_vexpand(true);

    let (sender, receiver) = bounded::<Vec<network::Resume>>(1);

    network::runtime().spawn(clone!(@strong sender => async move {
        let search_results = network::resume().await.unwrap_or_else(|e| {
            eprintln!("Error: {}", e);
            Vec::<Resume>::new()
        });
        sender.send(search_results).await.expect("search results not received.");
    }));

    hbox.set_halign(gtk::Align::Center);

    let spacer = Label::new(None);
    spacer.set_size_request(-1, 5);
    vbox.append(&spacer);
    vbox.append(&hbox);

    vbox.append(&scrolled_window);

    glib::spawn_future_local(clone!(@weak gridview,@weak store=> async move {
        while let Ok(search_results) = receiver.recv().await {
            store.remove_all();
            for result in search_results {
                let object = BoxedAnyObject::new(result);
                store.append(&object);
            }
        }
    }));

    gridview.connect_activate(clone!(@weak homestack=>move |gridview, position| {
        let model = gridview.model().unwrap();
        let item = model.item(position).and_downcast::<BoxedAnyObject>().unwrap();
        let result: Ref<Resume> = item.borrow();
        let stack_clone = homestack.clone();
        let result_clone = result.clone();
        let item_page;
        let result1 = SearchResult {
                Id: result.Id.clone(),
                Name: result.Name.clone(),
                Type: result.Type.clone(),
            };
        let result_cell = RefCell::new(result1);
        if result.Type == "Movie" {
            item_page = crate::ui::movie_page::movie_page(result_cell.borrow());
            backbutton.set_visible(true);
        } else {
            let series = network::SeriesInfo {
                Id: result.Id.clone(),
                Name: result.Name.clone(),
                ParentIndexNumber: result.ParentIndexNumber.clone().expect("msg"),
                IndexNumber: result.IndexNumber.clone().expect("msg"), 
                Overview: "".to_string(),
            };
            let series_cell = RefCell::new(series);
            item_page = crate::ui::episodes_page::episodes_page(stack_clone,series_cell.borrow(),result.SeriesId.as_ref().expect("no series id").to_string());
            backbutton.set_visible(true);
        }

        let id = result_clone.Id;
        let pagename = format!("item_page_{}", id);
        if homestack.child_by_name(&pagename).is_none() {
            homestack.add_named(&item_page, Some(&pagename));
        }
        homestack.set_visible_child_name(&pagename);
    }));
    homestack.add_titled(&vbox, Some("page0"), "Search");
    homestack
}
