use std::time::{Duration, Instant};

#[derive(Clone, Copy)]
pub(crate) struct Timeout {
    millis: u32,
}

impl Timeout {
    pub const fn new() -> Self {
        Self { millis: u32::MAX }
    }

    pub const fn into_poll(self) -> i32 {
        if self.millis > i32::MAX as u32 {
            -1
        } else {
            self.millis as i32
        }
    }

    pub fn into_expire(self) -> Instant {
        Instant::now() + Duration::from_millis(self.millis as u64)
    }
}

impl From<Duration> for Timeout {
    fn from(timeout: Duration) -> Self {
        let millis = timeout.as_millis();
        let millis = if millis > u32::MAX as u128 {
            u32::MAX
        } else {
            millis as u32
        };
        Self { millis: millis }
    }
}
