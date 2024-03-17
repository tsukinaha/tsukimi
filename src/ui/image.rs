use gtk::glib::{self, clone};
use gtk::prelude::*;
use gtk::{Box, Orientation};
use std::env;
// use std::path::PathBuf;

pub fn set_image(id: String) -> Box {
    let imgbox = Box::new(Orientation::Vertical, 5);

    let (sender, receiver) = async_channel::bounded::<String>(1);

    let image = gtk::Picture::new();
    image.set_halign(gtk::Align::Center);

    #[cfg(unix)]
    let pathbuf = dirs::home_dir()
        .unwrap()
        .join(format!("{}/.local/share/tsukimi/{}.png", id));

    #[cfg(windows)]
    let pathbuf = env::current_dir()
        .unwrap()
        .join("thumbnails")
        .join(format!("{}.png", id));

    // let pathbuf = PathBuf::from(&path);
    // let idfuture = id.clone();
    if pathbuf.exists() {
        image.set_file(Some(&gtk::gio::File::for_path(&pathbuf)));
    } else {
        crate::ui::network::runtime().spawn(async move {
            let id = crate::ui::network::get_image(id.clone())
                .await
                .expect("msg");
            sender
                .send(id.clone())
                .await
                .expect("The channel needs to be open.");
        });
    }

    glib::spawn_future_local(clone!(@weak image => async move {
        while let Ok(_) = receiver.recv().await {
            // let path = format!("{}/.local/share/tsukimi/{}.png",dirs::home_dir().expect("msg").display(), idfuture);
            let file = gtk::gio::File::for_path(&pathbuf);
            image.set_file(Some(&file));
        }
    }));

    imgbox.append(&image);
    imgbox
}

pub fn set_thumbimage(id: String) -> Box {
    let imgbox = Box::new(Orientation::Vertical, 5);

    let (sender, receiver) = async_channel::bounded::<String>(1);

    let image = gtk::Picture::new();
    image.set_halign(gtk::Align::Center);

    #[cfg(unix)]
    let pathbuf = dirs::home_dir()
        .unwrap()
        .join(format!("{}/.local/share/tsukimi/t{}.png", id));

    #[cfg(windows)]
    let pathbuf = env::current_dir()
        .unwrap()
        .join("thumbnails")
        .join(format!("t{}.png", id));

    // let path = format!(
    //     "{}/.local/share/tsukimi/t{}.png",
    //     dirs::home_dir().expect("msg").display(),
    //     id
    // );
    // let pathbuf = PathBuf::from(&path);
    // let idfuture = id.clone();
    if pathbuf.exists() {
        image.set_file(Some(&gtk::gio::File::for_path(&pathbuf)));
    } else {
        crate::ui::network::runtime().spawn(async move {
            let id = crate::ui::network::get_thumbimage(id.clone())
                .await
                .expect("msg");
            sender
                .send(id.clone())
                .await
                .expect("The channel needs to be open.");
        });
    }

    glib::spawn_future_local(clone!(@weak image => async move {
        while let Ok(_) = receiver.recv().await {
            // let path = format!("{}/.local/share/tsukimi/t{}.png",dirs::home_dir().expect("msg").display(), idfuture);
            let file = gtk::gio::File::for_path(&pathbuf);
            image.set_file(Some(&file));
        }
    }));

    imgbox.append(&image);
    imgbox
}

pub fn set_backdropimage(id: String) -> Box {
    let imgbox = Box::new(Orientation::Vertical, 5);

    let (sender, receiver) = async_channel::bounded::<String>(1);

    let image = gtk::Picture::new();
    image.set_halign(gtk::Align::Center);

    #[cfg(unix)]
    let pathbuf = dirs::home_dir()
        .unwrap()
        .join(format!("{}/.local/share/tsukimi/b{}.png", id));

    #[cfg(windows)]
    let pathbuf = env::current_dir()
        .unwrap()
        .join("thumbnails")
        .join(format!("b{}.png", id));

    // let path = format!(
    //     "{}/.local/share/tsukimi/b{}.png",
    //     dirs::home_dir().expect("msg").display(),
    //     id
    // );
    // let pathbuf = PathBuf::from(&path);
    // let idfuture = id.clone();
    if pathbuf.exists() {
        image.set_file(Some(&gtk::gio::File::for_path(&pathbuf)));
    } else {
        crate::ui::network::runtime().spawn(async move {
            let id = crate::ui::network::get_backdropimage(id.clone())
                .await
                .expect("msg");
            sender
                .send(id.clone())
                .await
                .expect("The channel needs to be open.");
        });
    }

    glib::spawn_future_local(clone!(@weak image=> async move {
        while let Ok(_) = receiver.recv().await {
            // let path = format!("{}/.local/share/tsukimi/b{}.png",dirs::home_dir().expect("msg").display(), idfuture);
            let file = gtk::gio::File::for_path(&pathbuf);
            image.set_file(Some(&file));
        }
    }));

    imgbox.append(&image);
    imgbox
}
