use adw::prelude::*;
use mutsumi::{
    MutsumiPlayer,
    PlayParams,
    PlaySource,
};

const DEFAULT_URL: &str = "https://www.bilibili.com/video/BV19X9eBpEfS";

fn main() {
    #[cfg(feature = "profiling")]
    let _profiling = mutsumi::start_proxy_profiling();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    mutsumi::force_gl_renderer();
    mutsumi::set_mpv_initializer(|init| {
        init.set_property(
            "ytdl-raw-options",
            "cookies-from-browser=firefox".to_string(),
        )?;
        Ok(())
    })
    .expect("Failed to set mpv initializer");

    let url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_URL.to_string());

    let app = adw::Application::builder()
        .application_id("io.github.mutsumi.example.player")
        .build();

    app.connect_activate(move |app| {
        mutsumi::init();

        let player = MutsumiPlayer::new();

        player
            .mpv()
            .command("script-binding", &["stats/display-stats-toggle"]);

        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("Mutsumi Player")
            .default_width(1280)
            .default_height(720)
            .content(&player)
            .build();

        window.present();

        let param = PlayParams::builder(PlaySource::Url(url.to_owned()))
            .title("wl-proxy embed mpv demo")
            .subtitle(url.to_owned())
            .build();

        player.play(&param);
    });

    app.run_with_args::<&str>(&[]);
}
