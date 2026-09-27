// API 配置模型（与前端 src/types/settings.ts 镜像）
// 目前仅支持小米 MiMo API
use serde::{Deserialize, Serialize};

// Provider 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    Mock,
    #[default]
    Mimo,
}

// 单个 Provider 配置
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProviderConfig {
    pub provider: ProviderKind,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            provider: ProviderKind::Mimo,
            api_key: String::new(),
            base_url: String::new(),
            model: String::new(),
        }
    }
}

// 应用 API 配置
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub asr: ProviderConfig,
    pub summarizer: ProviderConfig,
}

// 连接测试结果
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestResult {
    pub ok: bool,
    pub message: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            asr: ProviderConfig {
                model: "mimo-v2.5-asr".to_string(),
                ..ProviderConfig::default()
            },
            summarizer: ProviderConfig {
                model: "mimo-v2.6-flash".to_string(),
                ..ProviderConfig::default()
            },
        }
    }
}
