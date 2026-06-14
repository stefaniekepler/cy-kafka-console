use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum SidecarError {
    #[error("资源缺失或损坏: {0}")]
    ResourceNotFound(String),
    #[error("端口分配失败: {0}")]
    PortAllocation(String),
    #[error("用户数据目录不可用")]
    DataDirUnavailable,
    #[error("JVM 进程提前退出 (code={code:?})\n日志末尾:\n{log_tail}")]
    JvmExitedEarly { code: Option<i32>, log_tail: String },
    #[error("启动超时（{0:?}）")]
    StartupTimeout(Duration),
    #[error("启动 JVM 失败: {0}")]
    Spawn(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn display_includes_context() {
        let e = SidecarError::ResourceNotFound("jre/bin/java".into());
        assert!(format!("{e}").contains("jre/bin/java"));
        let e2 = SidecarError::JvmExitedEarly {
            code: Some(1),
            log_tail: "boom".into(),
        };
        assert!(format!("{e2}").contains("boom"));
    }
}
