use std::time::Duration;

use gtk::{
    gdk,
    glib,
    prelude::*,
};

use crate::{
    render::state::PlaybackAnchor,
    time::LyricTime,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WakeupKey {
    Media { target: LyricTime, serial: u64 },
    ManualScroll { deadline_us: i64 },
}

struct PendingWakeup {
    key: WakeupKey,
    source: glib::SourceId,
}

#[derive(Default)]
pub struct RenderScheduler {
    tick: Option<gtk::TickCallbackId>,
    wakeup: Option<PendingWakeup>,
}

impl RenderScheduler {
    pub fn ensure_tick<W, F>(&mut self, widget: &W, callback: F)
    where
        W: IsA<gtk::Widget> + 'static,
        F: Fn(&W, &gdk::FrameClock) -> glib::ControlFlow + 'static,
    {
        if self.tick.is_none() {
            self.tick = Some(widget.add_tick_callback(callback));
        }
    }

    pub fn stop_tick(&mut self) {
        if let Some(tick) = self.tick.take() {
            tick.remove();
        }
    }

    pub fn mark_tick_finished(&mut self) {
        self.tick.take();
    }

    pub fn schedule(&mut self, key: WakeupKey, delay: Duration, callback: impl FnOnce() + 'static) {
        if self
            .wakeup
            .as_ref()
            .is_some_and(|pending| pending.key == key)
        {
            return;
        }
        self.cancel_wakeup();
        let source = glib::timeout_add_local_once(delay, callback);
        self.wakeup = Some(PendingWakeup { key, source });
    }

    pub fn mark_wakeup_fired(&mut self, key: WakeupKey) {
        if self
            .wakeup
            .as_ref()
            .is_some_and(|pending| pending.key == key)
        {
            self.wakeup.take();
        }
    }

    pub fn cancel_wakeup(&mut self) {
        if let Some(pending) = self.wakeup.take() {
            pending.source.remove();
        }
    }

    pub fn stop_all(&mut self) {
        self.stop_tick();
        self.cancel_wakeup();
    }

    pub fn has_tick(&self) -> bool {
        self.tick.is_some()
    }
}

pub fn frame_delay(frame_time_us: i64, deadline_us: i64) -> Duration {
    Duration::from_micros(deadline_us.saturating_sub(frame_time_us).max(0) as u64)
}

pub fn media_delay(
    anchor: &PlaybackAnchor, frame_time_us: i64, target: LyricTime,
) -> Option<Duration> {
    if !anchor.is_advancing() {
        return None;
    }
    let position = anchor.position_at(frame_time_us).as_micros();
    let media_delta = target.as_micros().saturating_sub(position) as f64;
    let frame_delta = media_delta / anchor.rate;
    if !frame_delta.is_finite() || frame_delta < 0.0 {
        return None;
    }
    Some(Duration::from_micros(
        frame_delta.ceil().clamp(0.0, u64::MAX as f64) as u64,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::state::PlaybackState;

    fn anchor(position_us: i64, rate: f64, state: PlaybackState) -> PlaybackAnchor {
        PlaybackAnchor::new(LyricTime::from_micros(position_us), 1_000, rate, state, 7)
    }

    #[test]
    fn frame_deadline_does_not_depend_on_media_playback() {
        assert_eq!(frame_delay(1_000, 4_000), Duration::from_micros(3_000));
        assert_eq!(frame_delay(5_000, 4_000), Duration::ZERO);
    }

    #[test]
    fn media_wakeup_uses_the_absolute_anchor_in_both_directions() {
        let forward = anchor(1_000_000, 2.0, PlaybackState::Playing);
        let reverse = anchor(10_000_000, -2.0, PlaybackState::Playing);

        assert_eq!(
            media_delay(&forward, 1_000, LyricTime::from_micros(3_000_000)),
            Some(Duration::from_secs(1))
        );
        assert_eq!(
            media_delay(&reverse, 1_000, LyricTime::from_micros(8_000_000)),
            Some(Duration::from_secs(1))
        );
    }

    #[test]
    fn paused_or_wrong_direction_wakeups_are_not_scheduled() {
        let paused = anchor(1_000_000, 1.0, PlaybackState::Paused);
        let playing = anchor(1_000_000, 1.0, PlaybackState::Playing);

        assert_eq!(
            media_delay(&paused, 1_000, LyricTime::from_micros(2_000_000)),
            None
        );
        assert_eq!(media_delay(&playing, 1_000, LyricTime::ZERO), None);
    }
}
