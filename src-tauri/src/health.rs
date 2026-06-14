/// 探测 kafbat-ui 后端是否就绪。
pub trait HealthProbe: Send + Sync {
    fn is_ready(&self, port: u16) -> bool;
}

/// 真实实现：GET http://127.0.0.1:<port>/actuator/health，200 视为就绪。
pub struct HttpHealthProbe;

impl HealthProbe for HttpHealthProbe {
    fn is_ready(&self, port: u16) -> bool {
        let url = format!("http://127.0.0.1:{port}/actuator/health");
        match ureq::get(&url)
            .timeout(std::time::Duration::from_secs(2))
            .call()
        {
            Ok(resp) => resp.status() == 200,
            Err(_) => false,
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    pub struct FakeProbe {
        pub ready_after: u32,
        pub calls: AtomicU32,
    }
    impl HealthProbe for FakeProbe {
        fn is_ready(&self, _port: u16) -> bool {
            let n = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
            n >= self.ready_after
        }
    }
    #[test]
    fn fake_probe_becomes_ready() {
        let p = FakeProbe {
            ready_after: 3,
            calls: AtomicU32::new(0),
        };
        assert!(!p.is_ready(1));
        assert!(!p.is_ready(1));
        assert!(p.is_ready(1));
    }
}
