use gtk::{
    Revealer,
    glib::{
        self,
        clone,
    },
    prelude::*,
};
use tracing::debug;

use crate::{
    client::jellyfin_client::JELLYFIN_CLIENT,
    ui::models::jellyfin_cache_path,
    utils::{
        spawn,
        spawn_tokio,
    },
};

pub async fn set_logo(id: String, image_type: &str, tag: Option<u8>) -> Revealer {
    let image = gtk::Picture::new();
    image.set_halign(gtk::Align::Fill);
    image.set_content_fit(gtk::ContentFit::Contain);
    let revealer = gtk::Revealer::builder()
        .transition_type(gtk::RevealerTransitionType::Crossfade)
        .child(&image)
        .reveal_child(false)
        .vexpand(true)
        .transition_duration(400)
        .build();

    let cache_path = jellyfin_cache_path().await;
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

    spawn(clone!(
        #[weak]
        image,
        #[weak]
        revealer,
        async move {
            let _ =
                spawn_tokio(async move { JELLYFIN_CLIENT.get_image(&id, &image_type, tag).await })
                    .await;
            debug!("Setting image: {}", &pathbuf.display());
            let file = gtk::gio::File::for_path(pathbuf);
            image.set_file(Some(&file));
            revealer.set_reveal_child(true);
        }
    ));

    revealer
}
