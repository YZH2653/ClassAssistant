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
            base_url: "https://api.xiaomimimo.com/v1".to_string(),
            model: String::new(),
        }
    }
}

// 按量付费接口地址
pub const PAY_AS_YOU_GO_BASE_URL: &str = "https://api.xiaomimimo.com/v1";
// Token Plan 接口地址
pub const TOKEN_PLAN_BASE_URL: &str = "https://token-plan-cn.xiaomimimo.com/v1";

impl ProviderConfig {
    // 生效的接口地址：非默认地址视为手动指定优先，否则按密钥前缀推断
    pub fn effective_base_url(&self) -> String {
        let trimmed = self.base_url.trim().trim_end_matches('/');
        let key = self.api_key.trim();
        let derived = if key.starts_with("tp-") || key.starts_with("ttp-") {
            TOKEN_PLAN_BASE_URL
        } else {
            PAY_AS_YOU_GO_BASE_URL
        };
        let is_known = trimmed.is_empty()
            || trimmed == PAY_AS_YOU_GO_BASE_URL
            || trimmed == TOKEN_PLAN_BASE_URL;
        if is_known {
            derived.to_string()
        } else {
            trimmed.to_string()
        }
    }
}

// 应用 API 配置
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub asr: ProviderConfig,
    pub summarizer: ProviderConfig,
    // 总结是否开启深度思考
    pub thinking: bool,
}

// 连接测试结果
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestResult {
    pub ok: bool,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_url_prefers_manual_value() {
        let config = ProviderConfig {
            api_key: "tp-123".to_string(),
            base_url: "https://example.test/v1/".to_string(),
            ..ProviderConfig::default()
        };
        assert_eq!(config.effective_base_url(), "https://example.test/v1");
    }

    #[test]
    fn base_url_resolved_from_key_prefix() {
        let token_plan = ProviderConfig {
            api_key: "tp-123".to_string(),
            ..ProviderConfig::default()
        };
        assert_eq!(token_plan.effective_base_url(), TOKEN_PLAN_BASE_URL);

        let team = ProviderConfig {
            api_key: "ttp-456".to_string(),
            ..ProviderConfig::default()
        };
        assert_eq!(team.effective_base_url(), TOKEN_PLAN_BASE_URL);

        let pay_as_you_go = ProviderConfig {
            api_key: "sk-789".to_string(),
            ..ProviderConfig::default()
        };
        assert_eq!(pay_as_you_go.effective_base_url(), PAY_AS_YOU_GO_BASE_URL);
    }

    #[test]
    fn thinking_defaults_to_enabled() {
        assert!(AppSettings::default().thinking);
    }
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
            thinking: true,
        }
    }
}
