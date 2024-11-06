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
            _ => {
                warn!("MPV Error: {}", self);
                format!("Unknown Error: {}", self)
            }
        }
    }
}

impl UserFacingError for anyhow::Error {
    fn to_user_facing(&self) -> String {
        warn!("Unknown Error: {}", self);
        self.to_string()
    }
}
