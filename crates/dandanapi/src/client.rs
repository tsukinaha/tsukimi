use super::{
    Request,
    Route,
};

pub trait ClientPrelude: Clone + Send + 'static {
    const BASE_URI: &'static str;

    fn headers(&self, path: &str) -> Option<reqwest::header::HeaderMap>;

    fn client(&self) -> reqwest::Client;

    fn route<T>(&self, kind: T) -> Route<Self, T>
    where
        T: Request,
    {
        Route::new(self, kind)
    }
}
