use super::{
    Danmaku,
    sort::SortByTime,
};

pub struct DanmakuQueue {
    all_queue: Vec<Danmaku>,
    next_index: usize,
}

impl Default for DanmakuQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl DanmakuQueue {
    pub fn new() -> Self {
        Self {
            all_queue: Vec::new(),
            next_index: 0,
        }
    }

    pub fn init(&mut self, danmaku: Vec<Danmaku>, time: f64) {
        self.all_queue = danmaku;
        self.all_queue.sort_by_time();
        self.next_index = self.partition_index(time);
    }

    // When the time is changed, this should be called to update the queue
    pub fn pop_to_time(&mut self, time: f64) -> Vec<Danmaku> {
        self.pop_to_time_iter(time).cloned().collect()
    }

    pub fn pop_to_time_iter(&mut self, time: f64) -> impl Iterator<Item = &Danmaku> + '_ {
        let start_index = self.next_index;
        while self
            .all_queue
            .get(self.next_index)
            .is_some_and(|danmaku| danmaku.start <= time)
        {
            self.next_index += 1;
        }
        self.all_queue[start_index..self.next_index].iter()
    }

    pub fn reset_time(&mut self, time: f64) {
        self.next_index = self.partition_index(time);
    }

    fn partition_index(&self, time: f64) -> usize {
        self.all_queue
            .partition_point(|danmaku| danmaku.start <= time)
    }
}
