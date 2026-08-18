use std::cell::Cell;

#[derive(Default)]
pub struct DanmakuSync {
    pending: Cell<bool>,
    observed_position: Cell<Option<f64>>,
}

impl DanmakuSync {
    pub fn reset(&self) {
        self.pending.set(false);
        self.observed_position.set(None);
    }

    pub fn begin_seek(&self) {
        self.pending.set(true);
        self.observed_position.set(None);
    }

    pub fn observe_position(&self, time_millis: f64, seeking: bool) -> Option<f64> {
        if !self.pending.get() {
            return None;
        }

        if seeking {
            self.observed_position.set(Some(time_millis));
            return None;
        }

        self.reset();
        Some(time_millis)
    }

    pub fn finish_seek(&self, event_position: f64) -> Option<f64> {
        let position = self.observed_position.take().or_else(|| {
            (event_position.is_finite() && event_position > 0.0).then_some(event_position)
        });

        if position.is_some() {
            self.reset();
        }
        position
    }
}
