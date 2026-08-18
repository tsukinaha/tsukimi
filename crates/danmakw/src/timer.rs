pub struct DanmakuClock {
    start_time: i64,
    paused_time: Option<i64>,
    speed_factor: f64,
}

impl Default for DanmakuClock {
    fn default() -> Self {
        Self::new(1.0)
    }
}

impl DanmakuClock {
    pub fn new(speed_factor: f64) -> Self {
        Self {
            start_time: glib::monotonic_time(),
            paused_time: None,
            speed_factor,
        }
    }

    #[inline]
    pub fn time_milis(&self) -> f64 {
        self.time_milis_at(glib::monotonic_time())
    }

    #[inline]
    pub fn time_milis_at(&self, time_micros: i64) -> f64 {
        let Some(paused_time) = self.paused_time else {
            return (time_micros - self.start_time) as f64 / 1000.0 * self.speed_factor;
        };

        (paused_time - self.start_time) as f64 / 1000.0 * self.speed_factor
    }

    pub fn pause(&mut self) {
        if self.paused_time.is_none() {
            self.paused_time = Some(glib::monotonic_time());
        }
    }

    pub fn resume(&mut self) {
        let Some(paused_time) = self.paused_time else {
            return;
        };

        self.start_time = glib::monotonic_time() - (paused_time - self.start_time);
        self.paused_time = None;
    }

    pub fn set_speed_factor(&mut self, factor: f64) {
        let current_time = self.time_milis();
        self.speed_factor = factor;
        self.seek(current_time);
    }

    pub fn seek(&mut self, time_milis: f64) {
        let desired_micros = (time_milis * 1000.0 / self.speed_factor) as i64;

        match self.paused_time {
            Some(_) => {
                self.paused_time = Some(self.start_time + desired_micros);
            }
            None => {
                self.start_time = glib::monotonic_time() - desired_micros;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        thread::sleep,
        time::Duration,
    };

    const TOL_MS: f64 = 60.0;

    fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn test_time_milis_basic() {
        let clk = DanmakuClock::new(1.0);
        sleep(Duration::from_millis(220));
        let t = clk.time_milis();
        assert!(approx_eq(t, 220.0, TOL_MS), "expected ~220ms, got {}", t);
    }

    #[test]
    fn test_speed_factor_effect() {
        let clk = DanmakuClock::new(2.0);
        sleep(Duration::from_millis(200));
        let t = clk.time_milis();
        assert!(approx_eq(t, 400.0, TOL_MS), "expected ~400ms, got {}", t);
    }

    #[test]
    fn test_pause_resume_continuity() {
        let mut clk = DanmakuClock::new(1.0);
        sleep(Duration::from_millis(150));
        let t1 = clk.time_milis();
        clk.pause();
        sleep(Duration::from_millis(200));
        let t2 = clk.time_milis();
        assert!(
            approx_eq(t1, t2, TOL_MS),
            "time advanced while paused: {} -> {}",
            t1,
            t2
        );
        clk.resume();
        sleep(Duration::from_millis(120));
        let t3 = clk.time_milis();
        assert!(
            t3 > t2 + 80.0,
            "expected clock to advance after resume, {} -> {}",
            t2,
            t3
        );
    }

    #[test]
    fn test_seek_running() {
        let mut clk = DanmakuClock::new(1.0);
        clk.seek(1000.0);
        let t = clk.time_milis();
        assert!(
            approx_eq(t, 1000.0, 1.0),
            "expected exactly ~1000ms, got {}",
            t
        );
    }

    #[test]
    fn test_seek_while_paused() {
        let mut clk = DanmakuClock::new(1.0);
        clk.pause();
        clk.seek(500.0);
        let t = clk.time_milis();
        assert!(
            approx_eq(t, 500.0, 1.0),
            "expected ~500ms while paused, got {}",
            t
        );
        clk.resume();
        sleep(Duration::from_millis(120));
        let t2 = clk.time_milis();
        assert!(
            t2 > 600.0,
            "expected time to advance after resume, got {}",
            t2
        );
    }

    #[test]
    fn test_set_speed_factor_continuity() {
        let mut clk = DanmakuClock::new(1.0);
        sleep(Duration::from_millis(150));
        let before = clk.time_milis();
        clk.set_speed_factor(2.0);
        let after = clk.time_milis();
        assert!(
            approx_eq(before, after, TOL_MS),
            "speed change should not jump time: {} -> {}",
            before,
            after
        );
        sleep(Duration::from_millis(150));
        let later = clk.time_milis();
        assert!(
            later > after + 250.0,
            "with speed=2 expected larger advance, {} -> {}",
            after,
            later
        );
    }

    #[test]
    fn test_set_speed_factor_while_paused() {
        let mut clk = DanmakuClock::new(1.0);
        clk.pause();
        sleep(Duration::from_millis(100));
        clk.set_speed_factor(3.0);
        let t = clk.time_milis();
        assert!(
            approx_eq(t, 0.0, TOL_MS),
            "time should not advance while paused even with speed change, got {}",
            t
        );
        clk.resume();
        sleep(Duration::from_millis(100));
        let t2 = clk.time_milis();
        assert!(
            t2 > 300.0,
            "with speed=3 expected larger advance after resume, got {}",
            t2
        );
    }

    #[test]
    fn test_set_speed_factor_mutiple() {
        let mut clk = DanmakuClock::new(1.0);
        sleep(Duration::from_millis(100));
        clk.set_speed_factor(2.0);
        sleep(Duration::from_millis(100));
        clk.set_speed_factor(0.5);
        sleep(Duration::from_millis(200));
        let t = clk.time_milis();
        assert!(
            approx_eq(t, 400.0, TOL_MS),
            "expected ~400ms with multiple speed changes, got {}",
            t
        );
    }
}
