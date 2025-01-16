use gettextrs::gettext;
use tracing::warn;

pub trait UserFacingError {
    fn to_user_facing(&self) -> String;
}

impl UserFacingError for reqwest::Error {
    fn to_user_facing(&self) -> String {
        let status_code = self.status();
        if let Some(status_code) = status_code {
            warn!("Request Error: {}", status_code);
            format!("Error: {}", status_code)
        } else if self.is_decode() {
            warn!("Request Decoding Error: {}", self);
            format!("Decoding Error: {}", self)
        } else if self.is_timeout() {
            warn!("Request Timeout Error: {}", self);
            gettext("Timeout Error, Check your internet connection")
        } else if self.is_connect() {
            warn!("Request Connection Error: {}", self);
            gettext("Connection Error, Check your internet connection")
        } else {
            warn!("Request Error: {}", self);
            format!("Error: {}", self)
        }
    }
}

impl UserFacingError for std::boxed::Box<dyn std::error::Error> {
    fn to_user_facing(&self) -> String {
        warn!("Unknown Error: {}", self);
        self.to_string()
    }
}

impl UserFacingError for libmpv2::Error {
    fn to_user_facing(&self) -> String {
        match self {
            Self::Loadfile { error } => {
                warn!("MPV ErrorLoadfile: {}", error);
                format!("ErrorLoadfile: {}", error)
            }
            Self::Raw(error) => {
                let string = mpv_error_to_string(*error);
                warn!("MPV Error: {} ({})", string, error);
                format!("Error: {} ({})", string, error)
            }
            _ => {
                warn!("MPV Error: {}", self);
                format!("Unknown Error: {}", self)
            }
        }
    }
}

fn mpv_error_to_string(error: i32) -> &'static str {
    match error {
        0 => "Success",
        -1 => "Event queue full",
        -2 => "Out of memory",
        -3 => "Uninitialized",
        -4 => "Invalid parameter",
        -5 => "Option not found",
        -6 => "Option format",
        -7 => "Option error",
        -8 => "Property not found",
        -9 => "Property format",
        -10 => "Property unavailable",
        -11 => "Property error",
        -12 => "Command",
        -13 => "Loading failed",
        -14 => "Audio output init failed",
        -15 => "Video output init failed",
        -16 => "Nothing to play",
        -17 => "Unknown format",
        -18 => "Unsupported",
        -19 => "Not implemented",
        -20 => "Generic",
        _ => "Unknown",
    }
}

impl UserFacingError for anyhow::Error {
    fn to_user_facing(&self) -> String {
        warn!("Unknown Error: {}", self);
        self.to_string()
    }
}
