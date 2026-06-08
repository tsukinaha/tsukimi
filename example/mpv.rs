use gtk::glib;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Box as GtkBox, Button, Entry, Orientation};
use mutsumi::video::{MutsumiVideoPlayer, VideoBackend};

fn main() {
    gtk::init().expect("Failed to initialize GTK");

    let player = MutsumiVideoPlayer::new("mpvgl");
    let player_clone = player.clone();

    glib::spawn_future_local(async move {
        glib::timeout_future(std::time::Duration::from_secs(5)).await;
        player_clone.play("https://www.youtube.com/watch?v=IalBrXP3LVU", 0.0);
    });

    let app = Application::builder()
        .application_id("org.mutsumi.example.mpvglarea")
        .build();

    let player_for_activate = player.clone();

    app.connect_activate(move |app| {
        let window = ApplicationWindow::builder()
            .application(app)
            .title("mutsumi MPVGLArea example")
            .default_width(960)
            .default_height(540)
            .build();

        let vbox = GtkBox::new(Orientation::Vertical, 6);

        let video = player_for_activate.clone();
        video.set_hexpand(true);
        video.set_vexpand(true);
        vbox.append(&video);

        let controls = GtkBox::new(Orientation::Horizontal, 6);

        let entry = Entry::new();
        entry.set_placeholder_text(Some("file:///path/to/video.mp4 or https://..."));
        controls.append(&entry);

        let play_btn = Button::with_label("Play");
        let pause_btn = Button::with_label("Toggle Pause");
        let stop_btn = Button::with_label("Stop");

        controls.append(&play_btn);
        controls.append(&pause_btn);
        controls.append(&stop_btn);

        vbox.append(&controls);

        let video_play = video.clone();
        let entry_play = entry.clone();
        play_btn.connect_clicked(move |_| {
            let url = entry_play.text().to_string();
            if url.trim().is_empty() {
                eprintln!("Please enter a URL or file path to play.");
                return;
            }
            video_play.play(&url, 0.0);
            eprintln!("Play requested: {}", url);
        });

        let video_pause = video.clone();
        pause_btn.connect_clicked(move |_| {
            video_pause.command_pause();
            eprintln!("Toggle pause");
        });

        let video_stop = video.clone();
        stop_btn.connect_clicked(move |_| {
            video_stop.stop();
            eprintln!("Stop requested");
        });

        window.set_child(Some(&vbox));
        window.present();
    });

    app.run();
}
