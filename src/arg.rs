use std::{
    fs::File,
    io,
    sync::Mutex,
};

use clap::Parser;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::fmt::time::ChronoLocal;

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
                builder.with_ansi(false).with_writer(Mutex::new(File::create(f).unwrap())).init()
            }
        }
    }

    /// Set the GSK renderer environment variable
    fn init_gsk_renderer(&self) {
        if let Some(renderer) = self.gsk_renderer.as_deref() {
            std::env::set_var("GSK_RENDERER", renderer);
            return;
        }

        if std::env::var("GSK_RENDERER").is_err() {
            std::env::set_var("GSK_RENDERER", DEFAULT_RENDERER);
        }
    }

    pub fn init(&self) {
        self.init_tracing_subscriber();
        self.init_gsk_renderer();

        std::panic::set_hook(Box::new(|p| {
            tracing::error!("{p}");
        }));
    }
}
