use std::time::Duration;

/// 抽象睡眠，便于在测试中零延迟。
pub trait Sleeper: Send + Sync {
    fn sleep(&self, dur: Duration);
}

pub struct RealSleeper;
impl Sleeper for RealSleeper {
    fn sleep(&self, dur: Duration) {
        std::thread::sleep(dur);
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    pub struct FakeSleeper {
        pub count: AtomicU32,
    }
    impl Sleeper for FakeSleeper {
        fn sleep(&self, _dur: Duration) {
            self.count.fetch_add(1, Ordering::SeqCst);
        }
    }
    #[test]
    fn fake_sleeper_counts() {
        let s = FakeSleeper {
            count: AtomicU32::new(0),
        };
        s.sleep(Duration::from_secs(99));
        assert_eq!(s.count.load(Ordering::SeqCst), 1);
    }
}
