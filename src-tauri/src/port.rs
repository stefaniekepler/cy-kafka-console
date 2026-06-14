use crate::error::SidecarError;
use std::net::TcpListener;

/// 在 loopback 上申请一个空闲端口（绑定到 0 让 OS 分配后立即释放）。
pub fn allocate_free_port() -> Result<u16, SidecarError> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|e| SidecarError::PortAllocation(e.to_string()))?;
    let port = listener
        .local_addr()
        .map_err(|e| SidecarError::PortAllocation(e.to_string()))?
        .port();
    Ok(port)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn returns_high_ephemeral_port() {
        let p = allocate_free_port().unwrap();
        // OS 分配的临时端口应在高位区间，避免 TOCTOU 重绑导致的偶发失败
        assert!(p >= 1024, "期望高位端口，得到 {p}");
    }
    #[test]
    fn successive_calls_succeed() {
        assert!(allocate_free_port().is_ok());
        assert!(allocate_free_port().is_ok());
    }
}
