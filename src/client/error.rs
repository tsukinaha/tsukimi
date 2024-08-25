use tracing::warn;

pub trait UserFacingError {
    fn to_user_facing(&self) -> String;
}

impl UserFacingError for reqwest::Error {
    fn to_user_facing(&self) -> String {
        let status_code = self.status();
        if let Some(status_code) = status_code {
            warn!("Error: {}", status_code);
            format!("Error: {}", status_code).replace('&', "&amp;")
        } else if self.is_decode() {
            warn!("Decoding Error: {}", self);
            format!("Decoding Error: {}", self).replace('&', "&amp;")
        } else if self.is_timeout() {
            warn!("Timeout Error: {}", self);
            format!("Timeout Error: {}", self).replace('&', "&amp;")
        } else {
            warn!("Error: {}", self);
            format!("Error: {}", self.to_string().replace('&', "&amp;"))
        }
    }
}

impl UserFacingError for std::boxed::Box<dyn std::error::Error> {
    fn to_user_facing(&self) -> String {
        warn!("Error: {}", self);
        self.to_string()
    }
}
