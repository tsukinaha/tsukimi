

pub trait UserFacingError {
    fn to_user_facing(&self) -> String;
}

impl UserFacingError for reqwest::Error {
    fn to_user_facing(&self) -> String {
        let status_code = self.status();
        if let Some(status_code) = status_code {
            format!("Error: {}", status_code)
        } else {
            if self.is_decode() {
                format!("Decoding Error: {}", self)
            } else if self.is_timeout() {
                format!("Timeout Error: {}", self)
            } else {
                format!("Error: {}", self)
            }
        }
    }
}
