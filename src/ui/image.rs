use crate::client::network::*;
use gtk::glib::{self, clone};
use gtk::{prelude::*, Revealer};
use std::path::PathBuf;

use super::models::emby_cache_path;

pub fn set_image(id: String, image_type: &str, tag: Option<u8>) -> Revealer {
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

    let cache_path = emby_cache_path();
    let path = match image_type {
        "Logo" => format!("{}/l{}", cache_path.display(), id),
        "Banner" => format!("{}/banner{}", cache_path.display(), id),
        "Backdrop" => format!("{}/b{}_{}", cache_path.display(), id, tag.unwrap()),
        "Thumb" => format!("{}/t{}", cache_path.display(), id),
        _ => format!("{}/{}", cache_path.display(), id),
    };

    let pathbuf = PathBuf::from(&path);
    if pathbuf.exists() {
        if image.file().is_none() {
            image.set_file(Some(&gtk::gio::File::for_path(&path)));
            revealer.set_reveal_child(true);
        }
    } else {
        let image_type = image_type.to_string();
        RUNTIME.spawn(async move {
            let mut retries = 0;
            while retries < 3 {
                match get_image(id.clone(), &image_type, tag).await {
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
            let file = gtk::gio::File::for_path(&path);
            image.set_file(Some(&file));
            revealer.set_reveal_child(true);
        }
    }));

    revealer
}
