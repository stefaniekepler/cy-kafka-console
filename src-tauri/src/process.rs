use crate::error::SidecarError;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};

/// 已启动的被管子进程。
pub trait ManagedProcess: Send {
    fn is_running(&mut self) -> bool;
    /// 进程已退出时返回其退出码（若有）。
    fn exit_code(&mut self) -> Option<i32>;
    fn terminate(&mut self) -> Result<(), SidecarError>;
}

/// 负责拉起 JVM 子进程。
pub trait ProcessSpawner: Send + Sync {
    /// 拉起 JVM；子进程 stdout/stderr 重定向到 `stdio_log`，
    /// 以便捕获 Spring 启动前的早期失败（如缺 jlink 模块、Java 版本不符）。
    fn spawn(
        &self,
        java_bin: &Path,
        jar: &Path,
        env: &[(String, String)],
        stdio_log: &Path,
    ) -> Result<Box<dyn ManagedProcess>, SidecarError>;
}

pub struct OsProcessSpawner;

struct OsProcess {
    child: Child,
    exited: Option<ExitStatus>,
}

impl ManagedProcess for OsProcess {
    fn is_running(&mut self) -> bool {
        if self.exited.is_some() {
            return false;
        }
        match self.child.try_wait() {
            Ok(Some(status)) => {
                self.exited = Some(status);
                false
            }
            Ok(None) => true,
            // 保守处理：waitpid 出错（如 EINTR）不当作进程已死，避免误杀健康进程
            Err(_) => true,
        }
    }
    fn exit_code(&mut self) -> Option<i32> {
        let _ = self.is_running();
        self.exited.and_then(|s| s.code())
    }
    fn terminate(&mut self) -> Result<(), SidecarError> {
        if self.exited.is_some() {
            return Ok(());
        }
        let _ = self.child.kill();
        if let Ok(status) = self.child.wait() {
            self.exited = Some(status);
        }
        Ok(())
    }
}

impl ProcessSpawner for OsProcessSpawner {
    fn spawn(
        &self,
        java_bin: &Path,
        jar: &Path,
        env: &[(String, String)],
        stdio_log: &Path,
    ) -> Result<Box<dyn ManagedProcess>, SidecarError> {
        let log = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(stdio_log)
            .map_err(|e| SidecarError::Spawn(e.to_string()))?;
        let log_err = log
            .try_clone()
            .map_err(|e| SidecarError::Spawn(e.to_string()))?;
        let mut cmd = Command::new(java_bin);
        cmd.arg("-jar")
            .arg(jar)
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(log_err));
        for (k, v) in env {
            cmd.env(k, v);
        }
        let child = cmd
            .spawn()
            .map_err(|e| SidecarError::Spawn(e.to_string()))?;
        Ok(Box::new(OsProcess {
            child,
            exited: None,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    pub struct FakeProcess {
        pub running: Arc<AtomicBool>,
        pub terminated: Arc<AtomicBool>,
        pub code: Option<i32>,
    }
    impl ManagedProcess for FakeProcess {
        fn is_running(&mut self) -> bool {
            self.running.load(Ordering::SeqCst)
        }
        fn exit_code(&mut self) -> Option<i32> {
            self.code
        }
        fn terminate(&mut self) -> Result<(), SidecarError> {
            self.terminated.store(true, Ordering::SeqCst);
            self.running.store(false, Ordering::SeqCst);
            Ok(())
        }
    }

    #[test]
    fn fake_process_contract() {
        let running = Arc::new(AtomicBool::new(true));
        let terminated = Arc::new(AtomicBool::new(false));
        let mut p = FakeProcess {
            running: running.clone(),
            terminated: terminated.clone(),
            code: None,
        };
        assert!(p.is_running());
        p.terminate().unwrap();
        assert!(terminated.load(Ordering::SeqCst));
        assert!(!p.is_running());
    }

    #[test]
    fn spawn_creates_stdio_log_and_errors_on_bad_binary() {
        let tmp = tempfile::tempdir().unwrap();
        let logf = tmp.path().join("jvm.out.log");
        let spawner = OsProcessSpawner;
        let r = spawner.spawn(
            Path::new("/nonexistent/java-binary"),
            Path::new("/x.jar"),
            &[],
            &logf,
        );
        assert!(matches!(r, Err(SidecarError::Spawn(_))));
        // 日志文件在 spawn 失败前已创建（用于后续捕获输出）
        assert!(logf.exists());
    }
}
