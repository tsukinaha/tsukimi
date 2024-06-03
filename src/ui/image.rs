use crate::client::client::EMBY_CLIENT;
use crate::utils::{spawn, spawn_tokio};
use gtk::glib::{self, clone};
use gtk::{prelude::*, Revealer};
use tracing::{debug, warn};

use super::models::emby_cache_path;

pub fn set_image(id: String, image_type: &str, tag: Option<u8>) -> Revealer {
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
    let path = format!("{}-{}-{}", id, image_type, tag.unwrap_or(0));

    let id = id.to_string();

    let pathbuf = cache_path.join(path);
    if pathbuf.exists() {
        if image.file().is_none() {
            image.set_file(Some(&gtk::gio::File::for_path(pathbuf)));
            revealer.set_reveal_child(true);
        }
        return revealer;
    }

    let image_type = image_type.to_string();

    spawn(clone!(@weak image,@weak revealer => async move {
        spawn_tokio(async move {
            let mut retries = 0;
            while retries < 3 {
                match EMBY_CLIENT.get_image(&id, &image_type, tag).await {
                    Ok(_) => {
                        break;
                    }
                    Err(e) => {
                        warn!("Failed to get image: {}, retrying...", e);
                        retries += 1;
                    }
                }
        }}).await;
        debug!("Setting image: {}", &pathbuf.display());
        let file = gtk::gio::File::for_path(pathbuf);
        image.set_file(Some(&file));
        revealer.set_reveal_child(true);
    }));

    revealer
}
