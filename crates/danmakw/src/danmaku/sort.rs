use super::Danmaku;

pub trait SortByTime {
    // The latest one will be put on the right
    fn sort_by_time(&mut self);
}

impl SortByTime for Vec<Danmaku> {
    fn sort_by_time(&mut self) {
        self.sort_by(|a, b| a.start.total_cmp(&b.start));
    }
}
