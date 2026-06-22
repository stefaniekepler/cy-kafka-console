use crate::error::SidecarError;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    pub config_file: PathBuf,
    pub log_dir: PathBuf,
    pub settings_file: PathBuf,
}

/// 基于给定数据根目录推导各子路径（纯函数，便于测试）。
pub fn app_paths_from(data_root: &Path) -> AppPaths {
    AppPaths {
        config_file: data_root.join("dynamic_config.yaml"),
        log_dir: data_root.join("logs"),
        settings_file: data_root.join("settings.json"),
    }
}

/// 解析 OS 标准数据目录并确保其存在。
pub fn app_paths() -> Result<AppPaths, SidecarError> {
    let dirs = directories::ProjectDirs::from("com", "cy", "kafkaconsole")
        .ok_or(SidecarError::DataDirUnavailable)?;
    let root = dirs.data_dir().to_path_buf();
    let paths = app_paths_from(&root);
    std::fs::create_dir_all(&paths.log_dir).map_err(|_| SidecarError::DataDirUnavailable)?;
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    #[test]
    fn derives_subpaths() {
        let p = app_paths_from(Path::new("/data/app"));
        assert!(p.config_file.ends_with("dynamic_config.yaml"));
        assert!(p.log_dir.ends_with("logs"));
        assert!(p.settings_file.ends_with("settings.json"));
        assert!(p.config_file.starts_with("/data/app"));
        // open_config_dir 依赖：配置目录 = config_file 的父目录 = 数据根目录
        assert_eq!(p.config_file.parent().unwrap(), Path::new("/data/app"));
    }
}
