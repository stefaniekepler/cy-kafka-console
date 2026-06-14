//! 集成测试：用真实 jlink Java25 运行时跑真实 kafbat-ui jar。
//! 前置：仓库根 resources/jre 与 resources/kafbat/*.jar 已就绪。
//! 运行：cargo test --test integration_sidecar -- --ignored

use kafka_console_lib::clock::RealSleeper;
use kafka_console_lib::config::LaunchConfig;
use kafka_console_lib::health::{HealthProbe, HttpHealthProbe};
use kafka_console_lib::port;
use kafka_console_lib::process::OsProcessSpawner;
use kafka_console_lib::resources;
use kafka_console_lib::sidecar::{SidecarManager, StartParams};
use std::path::Path;
use std::time::Duration;

fn repo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn boot_and_assert_healthy() {
    let res = resources::resolve(&repo_root().join("resources"))
        .expect("需先运行 scripts/fetch-kafbat-jar 与 scripts/build-jre");
    let tmp = tempfile::tempdir().unwrap();
    let cfg = LaunchConfig {
        port: port::allocate_free_port().unwrap(),
        config_file: tmp.path().join("dynamic_config.yaml"),
        log_dir: tmp.path().to_path_buf(),
        max_heap_mb: 512,
    };
    std::fs::create_dir_all(&cfg.log_dir).unwrap();
    let spawner = OsProcessSpawner;
    let probe = HttpHealthProbe;
    let sleeper = RealSleeper;
    let mgr = SidecarManager {
        spawner: &spawner,
        probe: &probe,
        sleeper: &sleeper,
    };
    let params = StartParams {
        resources: &res,
        config: &cfg,
        max_attempts: 120,
        poll_interval: Duration::from_millis(500),
    };
    let mut run = mgr
        .start(&params)
        .expect("后端应在超时内就绪（失败常因 jlink 模块缺失）");
    assert!(HttpHealthProbe.is_ready(run.port), "健康端点应为 UP");
    let _ = run.process.terminate();
}

#[test]
#[ignore]
fn backend_boots_on_bundled_runtime() {
    boot_and_assert_healthy();
}

#[test]
#[ignore]
fn backend_boots_with_real_kafka() {
    use testcontainers::runners::SyncRunner;
    use testcontainers_modules::kafka::Kafka;
    let _kafka = Kafka::default()
        .start()
        .expect("启动 Kafka 容器需要 Docker");
    boot_and_assert_healthy();
}
