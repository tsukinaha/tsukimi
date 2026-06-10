use std::{future::Future, pin::Pin};

pub type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackSelection {
    Track(i64),
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackKind {
    Video,
    Audio,
    Subtitle,
}

impl std::fmt::Display for TrackSelection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            TrackSelection::Track(id) => id.to_string(),
            TrackSelection::None => "no".to_string(),
        };
        write!(f, "{str}")
    }
}
