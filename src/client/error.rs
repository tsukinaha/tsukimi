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
            format!("Timeout Error: {}", self)
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
        warn!("MPV Error: {}", self);
        format!("MPV Back: {}, Connectivity error occurred", self)
    }
}

impl UserFacingError for anyhow::Error {
    fn to_user_facing(&self) -> String {
        warn!("Unknown Error: {}", self);
        self.to_string()
    }
}
