use std::{borrow::Cow, path::PathBuf};

pub struct PlayParams {
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub source: PlaySource,
    pub start_time: Option<u64>,
}

pub enum PlaySource {
    Url(String),
    File(PathBuf),
}

impl PlaySource {
    pub fn new_for_url(url: impl Into<String>) -> Self {
        Self::Url(url.into())
    }

    pub fn new_for_file(file: impl Into<PathBuf>) -> Self {
        Self::File(file.into())
    }
}

pub struct PlayParamsBuilder {
    title: Option<String>,
    subtitle: Option<String>,
    source: PlaySource,
    start_time: Option<u64>,
}

impl PlayParamsBuilder {
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    pub fn start_time(mut self, second: u64) -> Self {
        self.start_time = Some(second);
        self
    }

    pub fn build(self) -> PlayParams {
        PlayParams {
            title: self.title,
            subtitle: self.subtitle,
            source: self.source,
            start_time: self.start_time,
        }
    }
}

impl PlayParams {
    pub fn new(source: PlaySource) -> Self {
        Self::builder(source).build()
    }

    pub fn builder(source: PlaySource) -> PlayParamsBuilder {
        PlayParamsBuilder {
            title: None,
            subtitle: None,
            source,
            start_time: None,
        }
    }

    pub fn url(&self) -> Cow<'_, str> {
        match &self.source {
            PlaySource::Url(url) => Cow::Borrowed(url),
            PlaySource::File(f) => f.to_string_lossy(),
        }
    }
}
