use std::{
    env,
    fs::File,
    io,
    sync::Mutex,
};

use clap::Parser;
use tracing::{
    error,
    info,
    level_filters::LevelFilter,
};
use tracing_subscriber::fmt::time::ChronoLocal;

use crate::dyn_event;

/// gl renderer will glitch on fractional scaling
/// vulkan renderer has poor performance
const DEFAULT_RENDERER: &str = "ngl";

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// File to write the log to. If not specified, logs will be written to
    /// stderr.
    #[clap(long, short = 'f')]
    log_file: Option<String>,

    /// Log level. Possible values are: error, warn, info, debug, trace.    
    #[clap(long, short)]
    log_level: Option<String>,

    /// GSK renderer to use. Possible values are: gl, ngl, vulkan and cairo (CPU rendering).
    #[clap(long, short)]
    gsk_renderer: Option<String>,

    /// XDG_CACHE_HOME. If not set, it will be set to %LOCALAPPDATA% on *Windows*. Never set on
    /// *Linux*.
    #[clap(long)]
    xdg_cache_home: Option<String>,

    /// GDK_SCALE. If not set, it will be set to 1. Never set on *Linux*.
    #[clap(long)]
    gdk_scale: Option<i8>,
}

impl Args {
    /// Build the tracing subscriber using parameters from the command line
    /// arguments
    ///
    /// ## Panics
    ///
    /// Panics if the log file cannot be opened.
    fn init_tracing_subscriber(&self) {
        let builder = tracing_subscriber::fmt().with_timer(ChronoLocal::rfc_3339());

        let builder = match self.log_level.as_deref() {
            Some("error") => builder.with_max_level(LevelFilter::ERROR),
            Some("warn") => builder.with_max_level(LevelFilter::WARN),
            Some("info") => builder.with_max_level(LevelFilter::INFO),
            Some("debug") => builder.with_max_level(LevelFilter::DEBUG),
            Some("trace") => builder.with_max_level(LevelFilter::TRACE),
            _ => builder.with_max_level(LevelFilter::INFO),
        };

        match &self.log_file {
            None => builder.with_writer(io::stderr).init(),
            Some(f) => {
                let tracing_writer = match File::create(f) {
                    Ok(f) => f,
                    Err(e) => {
                        error!("Failed to create tracing file {}", e);
                        return;
                    }
                };

                info!("Logging to file {}", f);
                builder
                    .with_ansi(false)
                    .with_writer(Mutex::new(tracing_writer))
                    .init()
            }
        }
    }

    /// Set the GSK renderer environment variable
    fn init_gsk_renderer(&self) {
        if let Some(renderer) = self.gsk_renderer.as_deref() {
            info!("Setting GSK_RENDERER to {}", renderer);
            std::env::set_var("GSK_RENDERER", renderer);
            return;
        }

        if std::env::var("GSK_RENDERER").is_err() {
            info!("Falling back to default GSK_RENDERER: {}", DEFAULT_RENDERER);
            std::env::set_var("GSK_RENDERER", DEFAULT_RENDERER);
        }
    }

    fn init_gilb_to_tracing(&self) {
        gtk::glib::log_set_writer_func(|level, x| {
            let domain = x
                .iter()
                .find(|&it| it.key() == "GLIB_DOMAIN")
                .and_then(|it| it.value_str());
            let Some(message) = x
                .iter()
                .find(|&it| it.key() == "MESSAGE")
                .and_then(|it| it.value_str())
            else {
                return gtk::glib::LogWriterOutput::Unhandled;
            };

            match domain {
                Some(domain) => {
                    dyn_event!(level, domain = %domain, message);
                }
                None => {
                    dyn_event!(level, message);
                }
            }
            gtk::glib::LogWriterOutput::Handled
        });

        info!("Glib logging redirected to tracing");
    }

    #[cfg(target_os = "windows")]
    fn init_config_dirs(&self) {
        if let Some(xdg_cache_home) = self.xdg_cache_home.as_deref() {
            info!("Windows: Setting XDG_CACHE_HOME to {}", xdg_cache_home);
            std::env::set_var("XDG_CACHE_HOME", xdg_cache_home);
        }

        if std::env::var("XDG_CACHE_HOME").is_err() {
            info!("Windows: Falling back to default XDG_CACHE_HOME: %LOCALAPPDATA%");
            let config_local_dir = dirs::config_local_dir().expect("Failed to get %LOCALAPPDATA%");
            std::env::set_var("XDG_CACHE_HOME", config_local_dir);
        }
    }

    #[cfg(target_os = "windows")]
    fn init_gdk_scale(&self) {
        if let Some(scale) = self.gdk_scale {
            info!("Windows: Setting GDK_SCALE to {}", scale);
            std::env::set_var("GDK_SCALE", scale.to_string());
        }

        if std::env::var("GDK_SCALE").is_err() {
            info!("Windows: Falling back to default GDK_SCALE: 1");
            std::env::set_var("GDK_SCALE", "1");
        }
    }

    pub fn init(&self) {
        self.init_tracing_subscriber();
        self.init_gsk_renderer();
        self.init_gilb_to_tracing();

        #[cfg(target_os = "windows")]
        self.init_config_dirs();
        #[cfg(target_os = "windows")]
        self.init_gdk_scale();

        std::panic::set_hook(Box::new(|p| {
            tracing::error!("{p}");
        }));

        info!("Args: {:?}", self);

        info!(
            "Application Version: {}, Platform: {} {}, CPU Architecture: {}",
            crate::config::VERSION,
            env::consts::OS,
            env::consts::FAMILY,
            env::consts::ARCH
        );
    }
}
