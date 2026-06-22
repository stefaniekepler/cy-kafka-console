use std::path::Path;

pub const DEFAULT_HEAP_MB: u32 = 512;
pub const MIN_HEAP_MB: u32 = 128;
pub const MAX_HEAP_MB: u32 = 8192;

/// 读取设置中的最大堆（MB）；文件缺失/损坏/越界 → 默认值。
pub fn read_max_heap(settings_file: &Path) -> u32 {
    std::fs::read_to_string(settings_file)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get("maxHeapMb").and_then(|x| x.as_u64()))
        .and_then(|n| u32::try_from(n).ok())
        .filter(|&n| (MIN_HEAP_MB..=MAX_HEAP_MB).contains(&n))
        .unwrap_or(DEFAULT_HEAP_MB)
}

/// 校验并写入最大堆设置（读-改-写，保留其他键）。
pub fn write_max_heap(settings_file: &Path, mb: u32) -> Result<(), String> {
    if !(MIN_HEAP_MB..=MAX_HEAP_MB).contains(&mb) {
        return Err(format!("堆内存需在 {MIN_HEAP_MB}–{MAX_HEAP_MB} MB 之间"));
    }
    let mut obj = std::fs::read_to_string(settings_file)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();
    obj.insert("maxHeapMb".to_string(), serde_json::json!(mb));
    std::fs::write(
        settings_file,
        serde_json::to_string_pretty(&serde_json::Value::Object(obj)).unwrap(),
    )
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(
            read_max_heap(&tmp.path().join("nope.json")),
            DEFAULT_HEAP_MB
        );
    }
    #[test]
    fn roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let f = tmp.path().join("settings.json");
        write_max_heap(&f, 1024).unwrap();
        assert_eq!(read_max_heap(&f), 1024);
    }
    #[test]
    fn rejects_out_of_range() {
        let tmp = tempfile::tempdir().unwrap();
        let f = tmp.path().join("settings.json");
        assert!(write_max_heap(&f, 64).is_err());
        assert!(write_max_heap(&f, 99999).is_err());
    }
    #[test]
    fn corrupt_file_falls_back_to_default() {
        let tmp = tempfile::tempdir().unwrap();
        let f = tmp.path().join("settings.json");
        std::fs::write(&f, "not json").unwrap();
        assert_eq!(read_max_heap(&f), DEFAULT_HEAP_MB);
    }
    #[test]
    fn write_preserves_other_keys() {
        let tmp = tempfile::tempdir().unwrap();
        let f = tmp.path().join("settings.json");
        std::fs::write(&f, r#"{"theme":"dark"}"#).unwrap();
        write_max_heap(&f, 1024).unwrap();
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&f).unwrap()).unwrap();
        assert_eq!(v.get("theme").and_then(|x| x.as_str()), Some("dark"));
        assert_eq!(v.get("maxHeapMb").and_then(|x| x.as_u64()), Some(1024));
    }
}
