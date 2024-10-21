use super::danmaku::{Filter, COMMENTS, ENABLED};
use anyhow::{anyhow, Result};
use hex::encode;
use md5::{Digest, Md5};
use reqwest::Client;
use serde::Deserialize;
use tracing::{info, error};
use std::{
    collections::HashMap,
    fs::File,
    hint,
    io::{copy, Read},
    path::Path,
    sync::{atomic::Ordering, Arc, LazyLock},
};
use unicode_segmentation::UnicodeSegmentation;

pub struct StatusInner {
    pub x: f64,
    pub row: usize,
    pub step: f64,
}

pub enum Status {
    Status(StatusInner),
    Overlapping,
    Uninitialized,
}

impl Status {
    pub fn insert(&mut self, status: StatusInner) -> &mut StatusInner {
        *self = Status::Status(status);
        match self {
            Status::Status(status) => status,
            _ => unsafe { hint::unreachable_unchecked() },
        }
    }
}

pub struct Danmaku {
    pub message: String,
    pub count: usize,
    pub time: f64,
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub source: Source,
    pub blocked: bool,
    pub status: Status,
}

#[derive(Deserialize)]
struct MatchResponse {
    #[serde(rename = "isMatched")]
    is_matched: bool,
    matches: Vec<Match>,
}

#[derive(Deserialize)]
struct Match {
    #[serde(rename = "episodeId")]
    episode_id: usize,
}

#[derive(Deserialize)]
struct CommentResponse {
    comments: Vec<Comment>,
}

#[derive(Deserialize)]
struct Comment {
    p: String,
    m: String,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Source {
    Bilibili,
    Gamer,
    AcFun,
    QQ,
    IQIYI,
    D,
    Dandan,
    Unknown,
}

impl From<&str> for Source {
    fn from(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "bilibili" => Source::Bilibili,
            "gamer" => Source::Gamer,
            "acfun" => Source::AcFun,
            "qq" => Source::QQ,
            "iqiyi" => Source::IQIYI,
            "d" => Source::D,
            "dandan" => Source::Dandan,
            _ => Source::Unknown,
        }
    }
}

static CLIENT: LazyLock<Client> = LazyLock::new(Client::new);

pub async fn get_danmaku(filter: Arc<Filter>) -> Result<Vec<Danmaku>> {

    let danmaku = CLIENT
        .get(format!(
            "https://api.dandanplay.net/api/v2/comment/62570002?withRelated=true",
        ))
        .send()
        .await?
        .json::<CommentResponse>()
        .await?
        .comments;
    let sources_rt = filter.sources_rt.lock().unwrap();
    let mut danmaku = danmaku
        .into_iter()
        .filter(|comment| filter.keywords.iter().all(|pat| !comment.m.contains(pat)))
        .map(|comment| {
            let mut p = comment.p.splitn(4, ',');
            let time = p.next().unwrap().parse().unwrap();
            _ = p.next().unwrap();
            let color = p.next().unwrap().parse::<u32>().unwrap();
            let user = p.next().unwrap();
            let source = if user.chars().all(char::is_numeric) {
                Source::Dandan
            } else {
                user.strip_prefix('[')
                    .and_then(|user| user.split_once(']').map(|(source, _)| source.into()))
                    .unwrap_or(Source::Unknown)
            };
            Danmaku {
                message: comment.m.replace('\n', "\\N"),
                count: comment.m.graphemes(true).count(),
                time,
                r: (color / (256 * 256)).try_into().unwrap(),
                g: (color % (256 * 256) / 256).try_into().unwrap(),
                b: (color % 256).try_into().unwrap(),
                source,
                blocked: sources_rt
                    .as_ref()
                    .map(|s| s.contains(&source))
                    .unwrap_or_else(|| filter.sources.contains(&source)),
                status: Status::Uninitialized,
            }
        })
        .collect::<Vec<_>>();

    danmaku.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    Ok(danmaku)
}

pub async fn get(filter: Arc<Filter>) {
    match get_danmaku(filter).await {
        Ok(danmaku) => {
            let n = danmaku.iter().filter(|c| !c.blocked).count();
            let mut comments = COMMENTS.lock().unwrap();
            *comments = Some(danmaku);
            if ENABLED.load(Ordering::SeqCst) {
                info!("{} comments loaded", n);
            }
        }
        Err(error) => {
            if ENABLED.load(Ordering::SeqCst) {
                error!("Failed to load comments: {}", error);
            }
        }
    }
}

fn reset_status(comments: &mut [Danmaku]) {
    for comment in comments {
        comment.status = Status::Uninitialized;
    }
}