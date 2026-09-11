use reqwest::header::{
    HeaderMap,
    HeaderValue,
};

use crate::Result;

#[derive(Debug, Clone)]
pub struct RequestHeaderGenerator {
    x_appid: Box<str>,
    x_appid_header: HeaderValue,
    secret: Box<str>,
}

impl Default for RequestHeaderGenerator {
    fn default() -> Self {
        Self {
            x_appid: "".into(),
            x_appid_header: HeaderValue::from_static(""),
            secret: "".into(),
        }
    }
}

impl RequestHeaderGenerator {
    pub fn new(x_appid: String, secret: String) -> Result<Self> {
        let x_appid_header = HeaderValue::from_str(&x_appid)?;

        Ok(Self {
            x_appid: x_appid.into(),
            x_appid_header,
            secret: secret.into(),
        })
    }

    pub fn header(&self, path: &str) -> HeaderMap {
        self.header_at(path, chrono::Utc::now().timestamp())
    }

    pub fn calculate_signature(&self, path: &str) -> String {
        self.calculate_signature_at(path, chrono::Utc::now().timestamp())
    }

    fn header_at(&self, path: &str, timestamp: i64) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("x-appid", self.x_appid_header.clone());
        headers.insert("x-timestamp", HeaderValue::from(timestamp));
        headers.insert(
            "x-signature",
            HeaderValue::from_str(&self.calculate_signature_at(path, timestamp))
                .expect("base64 signatures are valid header values"),
        );
        headers
    }

    fn calculate_signature_at(&self, path: &str, timestamp: i64) -> String {
        use base64::prelude::*;
        use sha2::{
            Digest,
            Sha256,
        };

        let sha256 = Sha256::digest(
            format!("{}{}{}{}", self.x_appid, timestamp, path, self.secret).as_bytes(),
        );
        BASE64_STANDARD.encode(sha256)
    }
}

#[cfg(test)]
mod tests {
    use base64::prelude::*;
    use sha2::{
        Digest,
        Sha256,
    };

    use super::*;

    fn generator() -> RequestHeaderGenerator {
        RequestHeaderGenerator {
            x_appid: "test-app".into(),
            x_appid_header: HeaderValue::from_static("test-app"),
            secret: "test-secret".into(),
        }
    }

    #[test]
    fn header_uses_one_timestamp_for_value_and_signature() {
        let generator = generator();
        let path = "/api/v2/test";
        let timestamp = 1_700_000_000;
        let headers = generator.header_at(path, timestamp);
        let expected = BASE64_STANDARD.encode(Sha256::digest(
            format!("test-app{timestamp}{path}test-secret").as_bytes(),
        ));

        assert_eq!(headers["x-appid"], "test-app");
        assert_eq!(headers["x-timestamp"], timestamp.to_string());
        assert_eq!(headers["x-signature"], expected);
    }
}
