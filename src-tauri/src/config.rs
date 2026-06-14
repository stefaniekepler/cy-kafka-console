use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct LaunchConfig {
    pub port: u16,
    pub config_file: PathBuf,
    pub log_dir: PathBuf,
    pub max_heap_mb: u32,
}

impl LaunchConfig {
    pub fn log_file(&self) -> PathBuf {
        self.log_dir.join("kafka-console.log")
    }

    /// JVM 子进程 stdout/stderr 的重定向目标（捕获早期失败）。
    pub fn console_log(&self) -> PathBuf {
        self.log_dir.join("jvm.out.log")
    }
}

/// 组装传给 JVM 的环境变量。强制 loopback 绑定与动态配置。
pub fn build_env(cfg: &LaunchConfig) -> Vec<(String, String)> {
    vec![
        ("SERVER_PORT".into(), cfg.port.to_string()),
        ("SERVER_ADDRESS".into(), "127.0.0.1".into()),
        ("DYNAMIC_CONFIG_ENABLED".into(), "true".into()),
        (
            "DYNAMIC_CONFIG_PATH".into(),
            cfg.config_file.to_string_lossy().into_owned(),
        ),
        (
            "LOGGING_FILE_NAME".into(),
            cfg.log_file().to_string_lossy().into_owned(),
        ),
        ("MANAGEMENT_ENDPOINT_HEALTH_ENABLED".into(), "true".into()),
        (
            "JAVA_TOOL_OPTIONS".into(),
            format!(
                "-Xmx{}m --add-opens=java.rmi/javax.rmi.ssl=ALL-UNNAMED",
                cfg.max_heap_mb
            ),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> LaunchConfig {
        LaunchConfig {
            port: 39123,
            config_file: PathBuf::from("/data/dynamic_config.yaml"),
            log_dir: PathBuf::from("/data/logs"),
            max_heap_mb: 512,
        }
    }
    #[test]
    fn env_forces_loopback_and_dynamic_config() {
        let env = build_env(&sample());
        let get = |k: &str| env.iter().find(|(a, _)| a == k).map(|(_, v)| v.clone());
        assert_eq!(get("SERVER_ADDRESS").as_deref(), Some("127.0.0.1"));
        assert_eq!(get("SERVER_PORT").as_deref(), Some("39123"));
        assert_eq!(get("DYNAMIC_CONFIG_ENABLED").as_deref(), Some("true"));
        let jto = get("JAVA_TOOL_OPTIONS").unwrap();
        assert!(jto.contains("-Xmx512m"), "JAVA_TOOL_OPTIONS={jto}");
        // 经真实环境验证：缺此参数会导致 JMX-over-SSL 指标拉取报错
        assert!(
            jto.contains("--add-opens=java.rmi/javax.rmi.ssl=ALL-UNNAMED"),
            "JAVA_TOOL_OPTIONS={jto}"
        );
        assert_eq!(
            get("MANAGEMENT_ENDPOINT_HEALTH_ENABLED").as_deref(),
            Some("true")
        );
    }
}
