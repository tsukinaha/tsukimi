use std::{
    fmt,
    ops::{
        Add,
        AddAssign,
        Sub,
        SubAssign,
    },
};

/// A signed media timestamp measured in microseconds.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LyricTime(i64);

impl LyricTime {
    pub const ZERO: Self = Self(0);
    pub const MIN: Self = Self(i64::MIN);
    pub const MAX: Self = Self(i64::MAX);

    pub const fn from_micros(micros: i64) -> Self {
        Self(micros)
    }

    pub const fn from_millis(millis: i64) -> Self {
        Self(millis.saturating_mul(1_000))
    }

    pub const fn from_secs(seconds: i64) -> Self {
        Self(seconds.saturating_mul(1_000_000))
    }

    pub const fn as_micros(self) -> i64 {
        self.0
    }

    pub const fn checked_add(self, rhs: Self) -> Option<Self> {
        match self.0.checked_add(rhs.0) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    pub const fn checked_sub(self, rhs: Self) -> Option<Self> {
        match self.0.checked_sub(rhs.0) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    pub const fn saturating_add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }

    pub const fn saturating_sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }

    pub fn abs_diff(self, other: Self) -> u64 {
        self.0.abs_diff(other.0)
    }
}

impl fmt::Debug for LyricTime {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "LyricTime({}µs)", self.0)
    }
}

impl fmt::Display for LyricTime {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.0 < 0 { "-" } else { "" };
        let micros = self.0.unsigned_abs();
        write!(
            formatter,
            "{sign}{}.{:06}s",
            micros / 1_000_000,
            micros % 1_000_000
        )
    }
}

impl From<i64> for LyricTime {
    fn from(micros: i64) -> Self {
        Self::from_micros(micros)
    }
}

impl From<LyricTime> for i64 {
    fn from(time: LyricTime) -> Self {
        time.as_micros()
    }
}

impl Add for LyricTime {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        self.saturating_add(rhs)
    }
}

impl AddAssign for LyricTime {
    fn add_assign(&mut self, rhs: Self) {
        *self = self.saturating_add(rhs);
    }
}

impl Sub for LyricTime {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        self.saturating_sub(rhs)
    }
}

impl SubAssign for LyricTime {
    fn sub_assign(&mut self, rhs: Self) {
        *self = self.saturating_sub(rhs);
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TimeRange {
    pub start: LyricTime,
    pub end: Option<LyricTime>,
}

impl TimeRange {
    pub const fn new(start: LyricTime, end: Option<LyricTime>) -> Self {
        Self { start, end }
    }

    pub fn contains(self, position: LyricTime) -> bool {
        self.start <= position && self.end.is_none_or(|end| position < end)
    }

    pub fn is_valid(self) -> bool {
        self.end.is_none_or(|end| self.start <= end)
    }

    pub fn duration(self) -> Option<LyricTime> {
        self.end.map(|end| end.saturating_sub(self.start))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signed_arithmetic_is_in_microseconds() {
        let time = LyricTime::from_millis(-500) + LyricTime::from_secs(2);
        assert_eq!(time.as_micros(), 1_500_000);
        assert_eq!(time.to_string(), "1.500000s");
    }

    #[test]
    fn ranges_are_end_exclusive_and_can_be_open() {
        let closed = TimeRange::new(LyricTime::from_secs(1), Some(LyricTime::from_secs(2)));
        assert!(closed.contains(LyricTime::from_secs(1)));
        assert!(!closed.contains(LyricTime::from_secs(2)));

        let open = TimeRange::new(LyricTime::from_secs(3), None);
        assert!(open.contains(LyricTime::from_secs(300)));
    }
}
