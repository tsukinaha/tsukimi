use adw::prelude::*;
use danmakw::Intensity;
use mutsumi::Danmakw;

mod parse;

const XML: &str = include_str!("test.xml");

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let app = adw::Application::builder()
        .application_id("io.github.mutsumi.example.player")
        .build();

    app.connect_activate(move |app| {
        mutsumi::init();

        let danmakw = Danmakw::new();
        danmakw.set_intensity(Intensity::Overlay);

        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("Mutsumi Player")
            .default_width(1280)
            .default_height(720)
            .content(&danmakw)
            .build();

        window.present();

        match parse::parse_bilibili_xml(XML) {
            Ok(danmaku) => {
                danmakw.load_danmaku(danmaku);
                danmakw.start_rendering();
            }
            Err(e) => eprintln!("parse error: {e}"),
        }
    });

    app.run_with_args::<&str>(&[]);
}
