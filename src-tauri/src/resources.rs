use crate::error::SidecarError;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resources {
    pub java_bin: PathBuf,
    pub jar: PathBuf,
}

fn java_exe_name() -> &'static str {
    if cfg!(windows) {
        "java.exe"
    } else {
        "java"
    }
}

/// 在 `resource_dir` 下解析内置 JRE 的 java 可执行文件与 kafbat jar。
pub fn resolve(resource_dir: &Path) -> Result<Resources, SidecarError> {
    let java_bin = resource_dir.join("jre").join("bin").join(java_exe_name());
    if !java_bin.exists() {
        return Err(SidecarError::ResourceNotFound(
            java_bin.display().to_string(),
        ));
    }
    let kafbat_dir = resource_dir.join("kafbat");
    let jar = std::fs::read_dir(&kafbat_dir)
        .map_err(|_| SidecarError::ResourceNotFound(kafbat_dir.display().to_string()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .find(|p| p.extension().map(|x| x == "jar").unwrap_or(false))
        .ok_or_else(|| SidecarError::ResourceNotFound(format!("{}/*.jar", kafbat_dir.display())))?;
    Ok(Resources { java_bin, jar })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn layout(dir: &Path, with_java: bool, with_jar: bool) {
        if with_java {
            let bin = dir.join("jre").join("bin");
            fs::create_dir_all(&bin).unwrap();
            fs::write(bin.join(java_exe_name()), b"x").unwrap();
        }
        if with_jar {
            let k = dir.join("kafbat");
            fs::create_dir_all(&k).unwrap();
            fs::write(k.join("api-1.5.0.jar"), b"x").unwrap();
        }
    }

    #[test]
    fn resolves_full_layout() {
        let tmp = tempfile::tempdir().unwrap();
        layout(tmp.path(), true, true);
        let r = resolve(tmp.path()).unwrap();
        assert!(r.java_bin.ends_with(java_exe_name()));
        assert!(r.jar.to_string_lossy().ends_with(".jar"));
    }
    #[test]
    fn errors_when_java_missing() {
        let tmp = tempfile::tempdir().unwrap();
        layout(tmp.path(), false, true);
        assert!(matches!(
            resolve(tmp.path()),
            Err(SidecarError::ResourceNotFound(_))
        ));
    }
    #[test]
    fn errors_when_jar_missing() {
        let tmp = tempfile::tempdir().unwrap();
        layout(tmp.path(), true, false);
        assert!(matches!(
            resolve(tmp.path()),
            Err(SidecarError::ResourceNotFound(_))
        ));
    }
}
