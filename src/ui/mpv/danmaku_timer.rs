use std::sync::Arc;

use danmakw::*;
use libmpv2::Mpv;

#[derive(Clone)]
pub struct MpvTimer {
    pub mpv: Arc<Mpv>,
}

impl Timer for MpvTimer {
    fn time_milis(&self) -> f64 {
        self.mpv
            .get_property::<f64>("audio-pts/full")
            .unwrap_or_default() * 1000.0
    }
}

impl MpvTimer {
    pub fn new(mpv: Arc<Mpv>) -> Self {
        Self { mpv }
    }
}