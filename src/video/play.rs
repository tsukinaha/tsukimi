use std::path::Path;

pub struct PlayParams<P>
where
    P: AsRef<Path>,
{
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub source: PlaySource<P>,
    pub start_time: Option<f64>,
}

pub enum PlaySource<P>
where
    P: AsRef<Path>,
{
    Url(String),
    File(P),
}
