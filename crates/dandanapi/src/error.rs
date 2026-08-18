#[derive(thiserror::Error, Debug)]

pub enum Error {
    #[error("http: {0}")]
    HttpError(reqwest::Error),
}
