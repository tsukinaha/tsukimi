mod client;
mod error;
mod secret;

pub use client::*;
pub use dandanapi::*;
pub use secret::*;

pub use error::Error as DandanapiError;

pub type Result<T> = std::result::Result<T, DandanapiError>;
