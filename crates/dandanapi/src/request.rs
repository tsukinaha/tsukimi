use std::borrow::Cow;

use reqwest::Method;

pub trait Request: Sized + Send + 'static {
    type Response: serde::de::DeserializeOwned + Send + 'static;

    type Body: serde::ser::Serialize + Send + 'static;

    type Params: serde::ser::Serialize + Send + 'static;

    const METHOD: Method;

    const PATH: &'static str;

    fn body(&self) -> Option<&Self::Body> {
        None
    }

    fn params(&self) -> Option<&Self::Params> {
        None
    }

    fn path(&self) -> Cow<'static, str> {
        Cow::Borrowed(Self::PATH)
    }
}
