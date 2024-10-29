use std::{
    fs::File,
    io,
    sync::Mutex,
};

use clap::Parser;
use tracing::{info, error, level_filters::LevelFilter};
use tracing_subscriber::fmt::time::ChronoLocal;

use crate::dyn_event;

const DEFAULT_RENDERER: &str = "gl";

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

    /// GSK renderer to use. Possible values are: gl, ngl, vulkan.
    #[clap(long, short)]
    gsk_renderer: Option<String>,
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
                    },
                };

                info!("Logging to file {}", f);
                builder.with_ansi(false).with_writer(Mutex::new(tracing_writer)).init()
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
            let domain =
                x.iter().find(|&it| it.key() == "GLIB_DOMAIN").and_then(|it| it.value_str());
            let Some(message) =
                x.iter().find(|&it| it.key() == "MESSAGE").and_then(|it| it.value_str())
            else {
                return gtk::glib::LogWriterOutput::Unhandled;
            };

            match domain {
                Some(domain) => {
                    dyn_event!(level, domain = domain, "{}", message);
                }
                None => {
                    dyn_event!(level, "{}", message);
                }
            }
            gtk::glib::LogWriterOutput::Handled
        });

        info!("Glib logging redirected to tracing");
    }

    pub fn init(&self) {
        self.init_tracing_subscriber();
        self.init_gsk_renderer();
        self.init_gilb_to_tracing();

        std::panic::set_hook(Box::new(|p| {
            tracing::error!("{p}");
        }));

        info!("Args: {:?}", self);
    }
}
