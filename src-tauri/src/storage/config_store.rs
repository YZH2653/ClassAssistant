// config.json 读写（含 API 密钥，仅本机保存，永不入 git）
use std::fs;
use std::path::PathBuf;

use crate::domain::settings::AppSettings;

use super::StoreError;

pub struct ConfigStore {
    path: PathBuf,
}

impl ConfigStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    // 读配置，文件缺失或损坏时回退默认值
    pub fn load(&self) -> AppSettings {
        match fs::read_to_string(&self.path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => AppSettings::default(),
        }
    }

    pub fn save(&self, settings: &AppSettings) -> Result<(), StoreError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| StoreError(e.to_string()))?;
        }
        let json = serde_json::to_string_pretty(settings).map_err(|e| StoreError(e.to_string()))?;
        fs::write(&self.path, json).map_err(|e| StoreError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::settings::{ProviderConfig, ProviderKind};
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_path() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ca_config_test_{}_{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&dir).unwrap();
        dir.join("config.json")
    }

    #[test]
    fn save_then_load_round_trip() {
        let path = temp_path();
        let store = ConfigStore::new(&path);
        let mut settings = AppSettings::default();
        settings.asr = ProviderConfig {
            provider: ProviderKind::Mimo,
            api_key: "sk-test-123".to_string(),
            base_url: "https://example.test/v1".to_string(),
            model: "mimo-v2.5-asr".to_string(),
        };
        store.save(&settings).unwrap();
        let back = store.load();
        assert_eq!(back.asr.api_key, "sk-test-123");
        assert_eq!(back.asr.provider, ProviderKind::Mimo);
        assert_eq!(back.summarizer.model, "mimo-v2.6-flash");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn missing_file_falls_back_to_default() {
        let path = std::env::temp_dir().join("ca_config_missing_nonexistent.json");
        let _ = fs::remove_file(&path);
        let store = ConfigStore::new(&path);
        let settings = store.load();
        assert!(settings.asr.api_key.is_empty());
        assert_eq!(settings.asr.model, "mimo-v2.5-asr");
    }

    #[test]
    fn broken_file_falls_back_to_default() {
        let path = temp_path();
        fs::write(&path, "{ 不是合法 JSON").unwrap();
        let store = ConfigStore::new(&path);
        let settings = store.load();
        assert_eq!(settings.summarizer.model, "mimo-v2.6-flash");
        let _ = fs::remove_file(&path);
    }
}
