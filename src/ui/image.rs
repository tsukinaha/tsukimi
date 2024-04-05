use gtk::glib::{self, clone};
use gtk::{prelude::*, Revealer};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
pub fn setimage(id: String, mutex: Arc<Mutex<()>>) -> Revealer {
    let (sender, receiver) = async_channel::bounded::<String>(1);

    let image = gtk::Picture::new();
    image.set_halign(gtk::Align::Fill);
    image.set_content_fit(gtk::ContentFit::Cover);
    let revealer = gtk::Revealer::builder()
        .transition_type(gtk::RevealerTransitionType::Crossfade)
        .child(&image)
        .reveal_child(false)
        .vexpand(true)
        .transition_duration(700)
        .build();

    let path = format!(
        "{}/.local/share/tsukimi/{}.png",
        dirs::home_dir().expect("msg").display(),
        id
    );
    let pathbuf = PathBuf::from(&path);
    let idfuture = id.clone();
    if pathbuf.exists() {
        if image.file().is_none() {
            image.set_file(Some(&gtk::gio::File::for_path(&path)));
            revealer.set_reveal_child(true);
            crate::ui::network::runtime().spawn(async move {
                let _lock = mutex.lock().await;
                crate::ui::network::get_image(id.clone()).await.unwrap();
            });
        }
    } else {
        crate::ui::network::runtime().spawn(async move {
            let _lock = mutex.lock().await;
            let mut retries = 0;
            while retries < 3 {
                match crate::ui::network::get_image(id.clone()).await {
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
        while let Ok(_) = receiver.recv().await {
            let path = format!("{}/.local/share/tsukimi/{}.png",dirs::home_dir().expect("msg").display(), idfuture);
            let file = gtk::gio::File::for_path(&path);
            image.set_file(Some(&file));
            revealer.set_reveal_child(true);
        }
    }));

    revealer
}

pub fn setthumbimage(id: String, mutex: Arc<Mutex<()>>) -> Revealer {
    let (sender, receiver) = async_channel::bounded::<String>(1);

    let image = gtk::Picture::new();
    image.set_halign(gtk::Align::Fill);
    image.set_content_fit(gtk::ContentFit::Cover);
    let revealer = gtk::Revealer::builder()
        .transition_type(gtk::RevealerTransitionType::Crossfade)
        .child(&image)
        .reveal_child(false)
        .vexpand(true)
        .transition_duration(700)
        .build();

    let path = format!(
        "{}/.local/share/tsukimi/t{}.png",
        dirs::home_dir().expect("msg").display(),
        id
    );
    let pathbuf = PathBuf::from(&path);
    let idfuture = id.clone();
    if pathbuf.exists() {
        if image.file().is_none() {
            image.set_file(Some(&gtk::gio::File::for_path(&path)));
            revealer.set_reveal_child(true);
            crate::ui::network::runtime().spawn(async move {
                let _lock = mutex.lock().await;
                crate::ui::network::get_image(id.clone()).await.unwrap();
            });
        }
    } else {
        crate::ui::network::runtime().spawn(async move {
            let _lock = mutex.lock().await;
            let mut retries = 0;
            while retries < 3 {
                match crate::ui::network::get_thumbimage(id.clone()).await {
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
        while let Ok(_) = receiver.recv().await {
            let path = format!("{}/.local/share/tsukimi/t{}.png",dirs::home_dir().expect("msg").display(), idfuture);
            let file = gtk::gio::File::for_path(&path);
            image.set_file(Some(&file));
            revealer.set_reveal_child(true);
        }
    }));

    revealer
}

pub fn setbackdropimage(id: String, mutex: Arc<Mutex<()>>) -> Revealer {
    let (sender, receiver) = async_channel::bounded::<String>(1);

    let image = gtk::Picture::new();
    image.set_halign(gtk::Align::Fill);
    image.set_content_fit(gtk::ContentFit::Cover);
    let revealer = gtk::Revealer::builder()
        .transition_type(gtk::RevealerTransitionType::Crossfade)
        .child(&image)
        .reveal_child(false)
        .vexpand(true)
        .transition_duration(700)
        .build();

    let path = format!(
        "{}/.local/share/tsukimi/b{}.png",
        dirs::home_dir().expect("msg").display(),
        id
    );
    let pathbuf = PathBuf::from(&path);
    let idfuture = id.clone();
    if pathbuf.exists() {
        if image.file().is_none() {
            image.set_file(Some(&gtk::gio::File::for_path(&path)));
            revealer.set_reveal_child(true);
            crate::ui::network::runtime().spawn(async move {
                let _lock = mutex.lock().await;
                crate::ui::network::get_image(id.clone()).await.unwrap();
            });
        }
    } else {
        crate::ui::network::runtime().spawn(async move {
            let _lock = mutex.lock().await;
            let mut retries = 0;
            while retries < 3 {
                match crate::ui::network::get_backdropimage(id.clone()).await {
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
        while let Ok(_) = receiver.recv().await {
            let path = format!("{}/.local/share/tsukimi/b{}.png",dirs::home_dir().expect("msg").display(), idfuture);
            let file = gtk::gio::File::for_path(&path);
            image.set_file(Some(&file));
            revealer.set_reveal_child(true);
        }
    }));

    revealer
}


pub fn setlogoimage(id: String, mutex: Arc<Mutex<()>>) -> Revealer {
    let (sender, receiver) = async_channel::bounded::<String>(1);

    let image = gtk::Picture::new();
    image.set_halign(gtk::Align::Start);
    image.set_valign(gtk::Align::Start);
    let revealer = gtk::Revealer::builder()
        .transition_type(gtk::RevealerTransitionType::Crossfade)
        .child(&image)
        .reveal_child(false)
        .transition_duration(700)
        .build();

    let path = format!(
        "{}/.local/share/tsukimi/l{}.png",
        dirs::home_dir().expect("msg").display(),
        id
    );
    let pathbuf = PathBuf::from(&path);
    let idfuture = id.clone();
    if pathbuf.exists() {
        if image.file().is_none() {
            image.set_file(Some(&gtk::gio::File::for_path(&path)));
            revealer.set_reveal_child(true);
        }
    } else {
        crate::ui::network::runtime().spawn(async move {
            let _lock = mutex.lock().await;
            let mut retries = 0;
            while retries < 3 {
                match crate::ui::network::get_logoimage(id.clone()).await {
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
        while let Ok(_) = receiver.recv().await {
            let path = format!("{}/.local/share/tsukimi/l{}.png",dirs::home_dir().expect("msg").display(), idfuture);
            let file = gtk::gio::File::for_path(&path);
            image.set_file(Some(&file));
            revealer.set_reveal_child(true);
        }
    }));

    revealer
}
