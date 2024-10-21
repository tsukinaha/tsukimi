use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::{
    collections::HashSet,
    fs::File,
    io::{BufRead, BufReader, ErrorKind},
    sync::{atomic::AtomicBool, Arc, LazyLock, Mutex},
};

use super::dandanplay::{Danmaku, Source};

pub const MAX_DURATION: f64 = 12.;
pub const INTERVAL: f64 = 0.005;
pub const MIN_STEP: f64 = INTERVAL / MAX_DURATION;
pub const MAX_STEP: f64 = MIN_STEP * 1.3;

pub static ENABLED: AtomicBool = AtomicBool::new(false);
pub static COMMENTS: LazyLock<Mutex<Option<Vec<Danmaku>>>> = LazyLock::new(|| Mutex::new(None));

#[derive(Deserialize)]
struct BilibiliFilterRule {
    r#type: usize,
    filter: String,
    opened: bool,
}

#[derive(Clone, Copy)]
pub struct Options {
    pub font_size: f64,
    pub transparency: u8,
    pub reserved_space: f64,
    pub speed: f64,
    pub no_overlap: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            font_size: 40.,
            transparency: 0x30,
            reserved_space: 0.,
            speed: 1.,
            no_overlap: true,
        }
    }
}

#[derive(Default)]
pub struct Filter {
    pub keywords: Vec<String>,
    pub sources: HashSet<Source>,
    pub sources_rt: Mutex<Option<HashSet<Source>>>,
}

#[derive(Default, Clone, Copy)]
pub struct Params {
    pub delay: f64,
    pub speed: f64,
    pub osd_width: f64,
    pub osd_height: f64,
}

#[derive(Clone, Copy)]
pub struct Row {
    pub end: f64,
    pub step: f64,
}
