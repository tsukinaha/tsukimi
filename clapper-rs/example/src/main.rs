use gtk4::{glib, prelude::*};
use clapper_gtk::{Video, TitleHeader, SimpleControls};
use clapper::MediaItem;

fn main() -> glib::ExitCode {
    glib::setenv("CLAPPER_USE_PLAYBIN3", "1", false).unwrap();
    clapper::init().unwrap();

    let app = libadwaita::Application::new(Some("org.gnome.clapper-rs.example"), Default::default());
    app.connect_startup(|_app| {
        let style_manager = libadwaita::StyleManager::default();
        style_manager.set_color_scheme(libadwaita::ColorScheme::ForceDark);
    });
    app.connect_activate(move |app| {
        let video = Video::new();
        video.set_vexpand(true);
        
        let header = TitleHeader::new();
        header.set_valign(gtk4::Align::Start);
        video.add_fading_overlay(&header);

        let controls = SimpleControls::new();
        controls.set_valign(gtk4::Align::End);
        video.add_fading_overlay(&controls);
        
        // replace this with a path to a local video file or an url that points to a video stream
        let item = MediaItem::new("http://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4");
        
        video.player().unwrap().queue().unwrap().add_item(&item);
        video.player().unwrap().queue().unwrap().select_item(Some(&item));

        video.player().unwrap().play();

        let content = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        content.append(&libadwaita::HeaderBar::new());
        content.append(&video);

        let window = libadwaita::ApplicationWindow::builder()
            .application(app)
            .default_width(600)
            .default_height(400)
            .content(&content)
            .build();

        let window_clone = window.clone();
        video.connect_toggle_fullscreen(move |_video| {
            window_clone.set_fullscreened(!window_clone.is_fullscreened());
        });
        window.present();
    });
    app.run()
}
