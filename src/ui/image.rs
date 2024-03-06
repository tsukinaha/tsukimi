use gtk::gdk_pixbuf::Pixbuf;
use gtk::gio::{Cancellable, MemoryInputStream};
use gtk::glib::{self, clone};
use gtk::{prelude::*};
use gtk::{Box, Orientation};
use std::collections::{HashMap};
use std::sync::Mutex;
extern crate lazy_static;
use lazy_static::lazy_static;

lazy_static! {
    static ref IMAGE_MAP: Mutex<HashMap<String, Vec<u8>>> = Mutex::new(HashMap::new());
}

pub fn set_image(id:String) -> Box {
    let imgbox = Box::new(Orientation::Vertical, 5);

    let (sender, receiver) = async_channel::bounded::<Vec<u8>>(1);

    let image = gtk::Picture::new();
    image.set_halign(gtk::Align::Center);

    let bytes = {
        let image_map = IMAGE_MAP.lock().unwrap();
        if let Some(bytes) = image_map.get(&id) {
            Some(bytes.clone())
        } else {
            None
        }
    };

    if let Some(bytes) = bytes {
        let sender_clone = sender.clone();
        crate::ui::network::runtime().spawn(async move {
            sender_clone.send(bytes).await.expect("The channel needs to be open.");
        });
    } else {
        crate::ui::network::runtime().spawn(clone!(@strong sender =>async move {
            let bytes = crate::ui::network::get_image(id.clone()).await.expect("msg");
            {
                let mut image_map = IMAGE_MAP.lock().unwrap();
                image_map.insert(id.clone(), bytes.clone());
            }
            sender.send(bytes).await.expect("The channel needs to be open.");
        }));
    }

    glib::spawn_future_local(clone!(@strong image => async move {
        while let Ok(bytes) = receiver.recv().await {
            let bytes = glib::Bytes::from(&bytes.to_vec());
            let stream = MemoryInputStream::from_bytes(&bytes);
            let cancellable = Cancellable::new();
            let cancellable= Some(&cancellable);
            let pixbuf = Pixbuf::from_stream(&stream, cancellable).unwrap();
            image.set_pixbuf(Some(&pixbuf));
            image.set_can_shrink(true);
        }
    }));

    imgbox.append(&image);
    imgbox
}


pub fn set_thumbimage(id:String) -> Box {
    let imgbox = Box::new(Orientation::Vertical, 5);

    let (sender, receiver) = async_channel::bounded::<Vec<u8>>(1);

    let image = gtk::Picture::new();
    image.set_halign(gtk::Align::Center);

    let bytes = {
        let image_map = IMAGE_MAP.lock().unwrap();
        if let Some(bytes) = image_map.get(&id) {
            Some(bytes.clone())
        } else {
            None
        }
    };

    if let Some(bytes) = bytes {
        let sender_clone = sender.clone();
        crate::ui::network::runtime().spawn(async move {
            sender_clone.send(bytes).await.expect("The channel needs to be open.");
        });
    } else {
        crate::ui::network::runtime().spawn(clone!(@strong sender =>async move {
            let bytes = crate::ui::network::get_thumbimage(id.clone()).await.expect("msg");
            {
                let mut image_map = IMAGE_MAP.lock().unwrap();
                image_map.insert(id.clone(), bytes.clone());
            }
            sender.send(bytes).await.expect("The channel needs to be open.");
        }));
    }

    glib::spawn_future_local(clone!(@strong image => async move {
        while let Ok(bytes) = receiver.recv().await {
            let bytes = glib::Bytes::from(&bytes.to_vec());
            let stream = MemoryInputStream::from_bytes(&bytes);
            let cancellable = Cancellable::new();
            let cancellable= Some(&cancellable);
            let pixbuf = Pixbuf::from_stream(&stream, cancellable).unwrap();
            image.set_pixbuf(Some(&pixbuf));
            image.set_can_shrink(true);
        }
    }));

    imgbox.append(&image);
    imgbox
}

pub fn set_backdropimage(id:String) -> Box {
    let imgbox = Box::new(Orientation::Vertical, 5);

    let (sender, receiver) = async_channel::bounded::<Vec<u8>>(1);

    let image = gtk::Picture::new();
    image.set_halign(gtk::Align::Center);

    let bytes = {
        let image_map = IMAGE_MAP.lock().unwrap();
        if let Some(bytes) = image_map.get(&id) {
            Some(bytes.clone())
        } else {
            None
        }
    };

    if let Some(bytes) = bytes {
        let sender_clone = sender.clone();
        crate::ui::network::runtime().spawn(async move {
            sender_clone.send(bytes).await.expect("The channel needs to be open.");
        });
    } else {
        crate::ui::network::runtime().spawn(clone!(@strong sender =>async move {
            let bytes = crate::ui::network::get_backdropimage(id.clone()).await.expect("msg");
            {
                let mut image_map = IMAGE_MAP.lock().unwrap();
                image_map.insert(id.clone(), bytes.clone());
            }
            sender.send(bytes).await.expect("The channel needs to be open.");
        }));
    }

    glib::spawn_future_local(clone!(@strong image => async move {
        while let Ok(bytes) = receiver.recv().await {
            let bytes = glib::Bytes::from(&bytes.to_vec());
            let stream = MemoryInputStream::from_bytes(&bytes);
            let cancellable = Cancellable::new();
            let cancellable= Some(&cancellable);
            let pixbuf = Pixbuf::from_stream(&stream, cancellable).unwrap();
            image.set_pixbuf(Some(&pixbuf));
            image.set_can_shrink(true);
        }
    }));

    imgbox.append(&image);
    imgbox
}

