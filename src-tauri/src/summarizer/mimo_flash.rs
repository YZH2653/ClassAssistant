// 小米 MiMo v2.6 flash 知识点总结（OpenAI 兼容 chat/completions）
use std::time::Duration;

use async_trait::async_trait;

use super::*;
use crate::domain::settings::ProviderConfig;

// 思考 + 回答的总输出上限
const MAX_COMPLETION_TOKENS: u32 = 4096;
// 总结提示词
const SYSTEM_PROMPT: &str = "你是课堂总结助手。请把课堂转写整理成知识点总结，\
只输出 Markdown，必须包含且仅包含这四个二级标题：\
## 知识点、## 重点与难点、## 课堂例题、## 课后待办。用简体中文，条目用无序列表。";

pub struct MimoFlashSummarizer {
    pub config: ProviderConfig,
    pub thinking: bool,
}

impl MimoFlashSummarizer {
    pub fn new(config: ProviderConfig, thinking: bool) -> Self {
        Self { config, thinking }
    }
}

#[async_trait]
impl SummarizerProvider for MimoFlashSummarizer {
    fn name(&self) -> &'static str {
        "mimo"
    }

    async fn summarize(&self, req: SummaryRequest) -> Result<SummaryResult, SummaryError> {
        let api_key = self.config.api_key.trim();
        if api_key.is_empty() {
            return Err(SummaryError("请先在设置页填写 MiMo API Key".to_string()));
        }
        let base_url = self.config.effective_base_url();
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(180))
            .build()
            .map_err(|e| SummaryError(format!("初始化失败：{e}")))?;

        let body = build_summary_body(&self.config.model, &req, self.thinking);
        let response = client
            .post(format!("{base_url}/chat/completions"))
            .bearer_auth(api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| SummaryError(format!("请求失败：{e}")))?;
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(SummaryError(format!(
                "接口返回 {status}：{}",
                truncate(&text, 200)
            )));
        }
        let value: serde_json::Value = response
            .json()
            .await
            .map_err(|e| SummaryError(format!("解析响应失败：{e}")))?;
        let markdown = value["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .trim()
            .to_string();
        if markdown.is_empty() {
            return Err(SummaryError("总结结果为空".to_string()));
        }
        Ok(SummaryResult {
            markdown,
            model: self.config.model.clone(),
            generated_at: chrono::Local::now().to_rfc3339(),
        })
    }
}

// 构造总结请求体（thinking 两档：enabled / disabled）
pub fn build_summary_body(model: &str, req: &SummaryRequest, thinking: bool) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "messages": [
            { "role": "system", "content": SYSTEM_PROMPT },
            {
                "role": "user",
                "content": format!(
                    "课程标题：{}\n课堂时长：{} 秒\n\n课堂转写：\n{}",
                    req.title,
                    req.duration_ms / 1000,
                    req.transcript_markdown
                )
            }
        ],
        "max_completion_tokens": MAX_COMPLETION_TOKENS,
        "stream": false,
        "thinking": { "type": if thinking { "enabled" } else { "disabled" } }
    })
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_string()
    } else {
        let cut: String = text.chars().take(max).collect();
        format!("{cut}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> SummaryRequest {
        SummaryRequest {
            session_id: "2026-09-27_120000_abc123".to_string(),
            title: "数据结构课".to_string(),
            started_at: "2026-09-27T12:00:00+08:00".to_string(),
            duration_ms: 2700,
            transcript_markdown: "[00:00:03] 同学们好。\n[00:00:11] 今天讲顺序表。".to_string(),
        }
    }

    #[test]
    fn summary_body_carries_transcript_and_model() {
        let body = build_summary_body("mimo-v2.6-flash", &request(), true);
        assert_eq!(body["model"], "mimo-v2.6-flash");
        assert_eq!(body["stream"], false);
        assert_eq!(body["max_completion_tokens"], 4096);
        let user = body["messages"][1]["content"].as_str().unwrap();
        assert!(user.contains("数据结构课"));
        assert!(user.contains("今天讲顺序表。"));
        assert!(body["messages"][0]["content"]
            .as_str()
            .unwrap()
            .contains("## 知识点"));
    }

    #[test]
    fn thinking_switches_between_two_modes() {
        let enabled = build_summary_body("mimo-v2.6-flash", &request(), true);
        assert_eq!(enabled["thinking"]["type"], "enabled");
        let disabled = build_summary_body("mimo-v2.6-flash", &request(), false);
        assert_eq!(disabled["thinking"]["type"], "disabled");
    }
}
