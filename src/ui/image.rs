use gtk::glib::{self, clone};
use gtk::{prelude::*, Revealer};
use std::env;

use crate::client::network::*;
use crate::config::get_cache_dir;

pub fn setimage(id: String) -> Revealer {
    let (sender, receiver) = async_channel::bounded::<String>(1);

    let image = gtk::Picture::new();
    image.set_halign(gtk::Align::Center);
    image.set_content_fit(gtk::ContentFit::Cover);
    let revealer = gtk::Revealer::builder()
        .transition_type(gtk::RevealerTransitionType::Crossfade)
        .child(&image)
        .reveal_child(false)
        .vexpand(true)
        .transition_duration(400)
        .build();

    let pathbuf = get_cache_dir(env::var("EMBY_NAME").unwrap())
        .expect("Failed to get cache dir!")
        .join(format!("{}.png", id));
    let idfuture = id.clone();
    if pathbuf.exists() {
        if image.file().is_none() {
            image.set_file(Some(&gtk::gio::File::for_path(&pathbuf)));
            revealer.set_reveal_child(true);
        }
    } else {
        RUNTIME.spawn(async move {
            let mut retries = 0;
            while retries < 3 {
                match get_image(id.clone(), "Primary", None).await {
                    Ok(id) => {
                        sender
                            .send(id.clone())
                            .await
                            .expect("The channel needs to be open.");
                        break;
                    }
                    Err(e) => {
                        eprintln!("Failed to get image: {}, retrying...", e);
                        retries += 1;
                    }
                }
            }
        });
    }

    glib::spawn_future_local(clone!(@weak image,@weak revealer => async move {
        while receiver.recv().await.is_ok() {
            let path = get_cache_dir(env::var("EMBY_NAME").unwrap()).expect("Failed to get cache dir").join(format!("{}.png",idfuture));
            let file = gtk::gio::File::for_path(&path);
            image.set_file(Some(&file));
            revealer.set_reveal_child(true);
        }
    }));

    revealer
}

pub fn setthumbimage(id: String) -> Revealer {
    let (sender, receiver) = async_channel::bounded::<String>(1);

    let image = gtk::Picture::new();
    image.set_halign(gtk::Align::Fill);
    image.set_content_fit(gtk::ContentFit::Cover);
    let revealer = gtk::Revealer::builder()
        .transition_type(gtk::RevealerTransitionType::Crossfade)
        .child(&image)
        .reveal_child(false)
        .vexpand(true)
        .transition_duration(400)
        .build();

    let pathbuf = get_cache_dir(env::var("EMBY_NAME").unwrap())
        .expect("Failed to get cache dir")
        .join(format!("t{}.png", id));
    let idfuture = id.clone();
    if pathbuf.exists() {
        if image.file().is_none() {
            image.set_file(Some(&gtk::gio::File::for_path(&pathbuf)));
            revealer.set_reveal_child(true);
        }
    } else {
        RUNTIME.spawn(async move {
            let mut retries = 0;
            while retries < 3 {
                match get_image(id.clone(), "Thumb", None).await {
                    Ok(id) => {
                        sender
                            .send(id.clone())
                            .await
                            .expect("The channel needs to be open.");
                        break;
                    }
                    Err(e) => {
                        eprintln!("Failed to get image: {}, retrying...", e);
                        retries += 1;
                    }
                }
            }
        });
    }

    glib::spawn_future_local(clone!(@weak image,@weak revealer => async move {
        while receiver.recv().await.is_ok() {
            let path = get_cache_dir(env::var("EMBY_NAME").unwrap()).expect("Faild to get cache dir").join(format!("t{}.png",idfuture));
            let file = gtk::gio::File::for_path(&path);
            image.set_file(Some(&file));
            revealer.set_reveal_child(true);
        }
    }));

    revealer
}

pub fn setbackdropimage(id: String, tag: u8) -> Revealer {
    let (sender, receiver) = async_channel::bounded::<String>(1);

    let image = gtk::Picture::new();
    image.set_halign(gtk::Align::Fill);
    image.set_content_fit(gtk::ContentFit::Cover);
    let revealer = gtk::Revealer::builder()
        .transition_type(gtk::RevealerTransitionType::Crossfade)
        .child(&image)
        .reveal_child(false)
        .vexpand(true)
        .transition_duration(400)
        .build();

    let pathbuf = get_cache_dir(env::var("EMBY_NAME").unwrap())
        .expect("Failed to get cache dir")
        .join(format!("b{}_{}.png", id, tag));
    let idfuture = id.clone();
    if pathbuf.exists() {
        if image.file().is_none() {
            image.set_file(Some(&gtk::gio::File::for_path(&pathbuf)));
            revealer.set_reveal_child(true);
        }
    } else {
        RUNTIME.spawn(async move {
            let mut retries = 0;
            while retries < 3 {
                match get_image(id.clone(), "Backdrop", Some(tag)).await {
                    Ok(id) => {
                        sender
                            .send(id.clone())
                            .await
                            .expect("The channel needs to be open.");
                        break;
                    }
                    Err(e) => {
                        eprintln!("Failed to get image: {}, retrying...", e);
                        retries += 1;
                    }
                }
            }
        });
    }

    glib::spawn_future_local(clone!(@weak image,@weak revealer => async move {
        while receiver.recv().await.is_ok() {
            let path = get_cache_dir(env::var("EMBY_NAME").unwrap()).expect("Failed to get cache dir").join(format!("b{}_{}.png",idfuture,tag));
            let file = gtk::gio::File::for_path(&path);
            image.set_file(Some(&file));
            revealer.set_reveal_child(true);
        }
    }));

    revealer
}

pub fn setbannerimage(id: String) -> Revealer {
    let (sender, receiver) = async_channel::bounded::<String>(1);

    let image = gtk::Picture::new();
    image.set_halign(gtk::Align::Fill);
    image.set_content_fit(gtk::ContentFit::Cover);
    let revealer = gtk::Revealer::builder()
        .transition_type(gtk::RevealerTransitionType::Crossfade)
        .child(&image)
        .reveal_child(false)
        .vexpand(true)
        .transition_duration(400)
        .build();

    let pathbuf = get_cache_dir(env::var("EMBY_NAME").unwrap())
        .expect("Failed to get cache dir")
        .join(format!("banner{}.png", id));
    let idfuture = id.clone();
    if pathbuf.exists() {
        if image.file().is_none() {
            image.set_file(Some(&gtk::gio::File::for_path(&pathbuf)));
            revealer.set_reveal_child(true);
        }
    } else {
        RUNTIME.spawn(async move {
            let mut retries = 0;
            while retries < 3 {
                match get_image(id.clone(), "Banner", None).await {
                    Ok(id) => {
                        sender
                            .send(id.clone())
                            .await
                            .expect("The channel needs to be open.");
                        break;
                    }
                    Err(e) => {
                        eprintln!("Failed to get image: {}, retrying...", e);
                        retries += 1;
                    }
                }
            }
        });
    }

    glib::spawn_future_local(clone!(@weak image,@weak revealer => async move {
        while receiver.recv().await.is_ok() {
            let path = get_cache_dir(env::var("EMBY_NAME").unwrap()).expect("Failed to get cache dir").join(format!("banner{}.png",idfuture));
            let file = gtk::gio::File::for_path(&path);
            image.set_file(Some(&file));
            revealer.set_reveal_child(true);
        }
    }));

    revealer
}

pub fn setlogoimage(id: String) -> Revealer {
    let (sender, receiver) = async_channel::bounded::<String>(1);

    let image = gtk::Picture::new();
    image.set_halign(gtk::Align::Start);
    image.set_valign(gtk::Align::Start);
    let revealer = gtk::Revealer::builder()
        .transition_type(gtk::RevealerTransitionType::Crossfade)
        .child(&image)
        .reveal_child(false)
        .transition_duration(400)
        .build();

    let pathbuf = get_cache_dir(env::var("EMBY_NAME").unwrap())
        .expect("Failed to get cache dir")
        .join(format!("l{}.png", id));
    let idfuture = id.clone();
    if pathbuf.exists() {
        if image.file().is_none() {
            image.set_file(Some(&gtk::gio::File::for_path(&pathbuf)));
            revealer.set_reveal_child(true);
        }
    } else {
        RUNTIME.spawn(async move {
            let mut retries = 0;
            while retries < 3 {
                match get_image(id.clone(), "Logo", None).await {
                    Ok(id) => {
                        sender
                            .send(id.clone())
                            .await
                            .expect("The channel needs to be open.");
                        break;
                    }
                    Err(e) => {
                        eprintln!("Failed to get image: {}, retrying...", e);
                        retries += 1;
                    }
                }
            }
        });
    }

    glib::spawn_future_local(clone!(@weak image,@weak revealer => async move {
        while receiver.recv().await.is_ok() {
            let path = get_cache_dir(env::var("EMBY_NAME").unwrap()).expect("Failed to get cache dir").join(format!("l{}.png",idfuture));
            let file = gtk::gio::File::for_path(&path);
            image.set_file(Some(&file));
            revealer.set_reveal_child(true);
        }
    }));

    revealer
}
