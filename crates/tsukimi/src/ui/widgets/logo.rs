use gtk::{
    Revealer,
    gio::prelude::FileExt,
    glib::{
        self,
        clone,
    },
    prelude::*,
};
use tracing::debug;

use crate::{
    client::picture_source::PictureSource,
    utils::{
        resolve_picture_file,
        spawn,
    },
};

pub async fn set_logo(source: PictureSource) -> Revealer {
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

    spawn(clone!(
        #[weak]
        image,
        #[weak]
        revealer,
        async move {
            if let Ok(file) = resolve_picture_file(source).await {
                debug!("Setting image: {}", file.uri());
                image.set_file(Some(&file));
                revealer.set_reveal_child(true);
            }
        }
    ));

    revealer
}
