use libmpv2::Mpv;
use libmpv2_sys::{self as mpv, mpv_log_level};
use tracing::level_filters::LevelFilter;

pub fn request_logs(mpv: &Mpv) {
    let level = match LevelFilter::current() {
        LevelFilter::OFF => c"no",
        LevelFilter::ERROR | LevelFilter::WARN | LevelFilter::INFO => c"error",
        LevelFilter::DEBUG | LevelFilter::TRACE => c"trace",
    };

    unsafe {
        let _ = mpv::mpv_request_log_messages(mpv.ctx.as_ptr(), level.as_ptr());
    }
}

pub fn emit_log(prefix: &str, log_level: mpv_log_level, text: &str) {
    let message = text.trim_end_matches(['\r', '\n']);
    if message.is_empty() {
        return;
    }

    macro_rules! emit {
        ($level:expr) => {
            tracing::event!(target: "mutsumi::mpv", $level, component = prefix, "{message}")
        };
    }

    if log_level <= mpv::mpv_log_level_MPV_LOG_LEVEL_ERROR {
        emit!(tracing::Level::ERROR)
    }

    emit!(tracing::Level::DEBUG)
}
