use crate::ui::network;
use crate::ui::network::SearchResult;
use async_channel::bounded;
use gtk::glib::{self, clone, BoxedAnyObject};
use gtk::{gio, pango, prelude::*, Stack};
use gtk::{Box, Button, Entry, Label, Orientation, ScrolledWindow};
use std::cell::Ref;

pub fn create_page1() -> Stack {
    let hbox = Box::new(Orientation::Horizontal, 10);
    let vbox = Box::new(Orientation::Vertical, 5);
    let store = gio::ListStore::new::<BoxedAnyObject>();
    let store_clone = store.clone();
    let sel = gtk::SingleSelection::new(Some(store_clone));
    let gridfactory = gtk::SignalListItemFactory::new();
    let backbutton = Button::with_label("Back");
    let searchstack = Stack::new();

    gridfactory.connect_bind(move |_factory, item| {
        let listitem = item.downcast_ref::<gtk::ListItem>().unwrap();
        let entry = listitem.item().and_downcast::<BoxedAnyObject>().unwrap();
        let result: Ref<SearchResult> = entry.borrow();
        let vbox = Box::new(Orientation::Vertical, 5);
        let overlay = gtk::Overlay::new();
        let imgbox = crate::ui::image::set_image(result.Id.clone());
        imgbox.set_size_request(187, 275);
        overlay.set_child(Some(&imgbox));
        overlay.set_size_request(187, 275);
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

    let entry = Entry::new();
    entry.set_placeholder_text(Some("输入搜索内容..."));
    entry.set_size_request(400, -1);
    hbox.append(&entry);

    let (sender, receiver) = bounded::<Vec<network::SearchResult>>(1);

    let search_button = Button::with_label("搜索");
    search_button.connect_clicked(
        clone!(@strong sender, @strong entry,@weak search_button => move |_| {
            search_button.set_label("搜索中...");
            let search_content = entry.text().to_string();
            network::runtime().spawn(clone!(@strong sender => async move {
                let search_results = network::search(search_content).await.unwrap_or_else(|e| {
                    eprintln!("Error: {}", e);
                    Vec::<SearchResult>::new()
                });
                sender.send(search_results).await.expect("search results not received.");
            }));
        }),
    );

    entry.connect_activate(
        clone!(@strong sender,@weak entry,@weak search_button => move |_| {
            search_button.set_label("搜索中...");
            let search_content = entry.text().to_string();
            network::runtime().spawn(clone!(@strong sender => async move {
                let search_results = network::search(search_content).await.unwrap_or_else(|e| {
                    eprintln!("Error: {}", e);
                    Vec::<SearchResult>::new()
                });
                sender.send(search_results).await.expect("search results not received.");
            }));
        }),
    );

    hbox.append(&search_button);
    hbox.set_halign(gtk::Align::Center);

    let spacer = Label::new(None);
    spacer.set_size_request(-1, 5);
    vbox.append(&spacer);
    vbox.append(&hbox);

    vbox.append(&scrolled_window);

    glib::spawn_future_local(clone!(@weak gridview,@weak store=> async move {
        while let Ok(search_results) = receiver.recv().await {
            search_button.set_label("搜索");
            store.remove_all();
            for result in search_results {
                if result.Type == "Series" || result.Type == "Movie" {
                    let object = BoxedAnyObject::new(result);
                    store.append(&object);
                }
            }
        }
    }));

    searchstack.set_transition_type(gtk::StackTransitionType::Crossfade);

    gridview.connect_activate(clone!(@weak searchstack=>move |gridview, position| {
        let model = gridview.model().unwrap();
        let item = model.item(position).and_downcast::<BoxedAnyObject>().unwrap();
        let result: Ref<SearchResult> = item.borrow();

        let item_page;

        if result.Type == "Movie" {
            item_page = crate::ui::movie_page::movie_page(result);
        } else {
            item_page = crate::ui::item_page::itempage(searchstack.clone(), result);
        }

        let pagename = format!("item_page");
        if searchstack.child_by_name(&pagename).is_none() {
            searchstack.add_named(&item_page, Some(&pagename));
        } else {
            searchstack.remove(&searchstack.child_by_name("item_page").unwrap());
        }
        backbutton.set_visible(true);
        searchstack.set_visible_child_name(&pagename);
    }));
    searchstack.add_titled(&vbox, Some("page1"), "Search");
    searchstack
}
