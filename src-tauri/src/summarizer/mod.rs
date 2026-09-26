// 知识点总结抽象
pub mod mimo_flash;

use std::time::Duration;

use async_trait::async_trait;

use crate::domain::summary::{SummaryRequest, SummaryResult};

// 总结错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryError(pub String);

impl std::fmt::Display for SummaryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// 总结 Provider
#[async_trait]
pub trait SummarizerProvider: Send + Sync {
    fn name(&self) -> &'static str;
    async fn summarize(&self, req: SummaryRequest) -> Result<SummaryResult, SummaryError>;
}

// Mock 总结器
pub struct MockSummarizer {
    delay: Duration,
}

impl Default for MockSummarizer {
    fn default() -> Self {
        Self {
            delay: Duration::from_millis(800),
        }
    }
}

impl MockSummarizer {
    // 测试用快节奏
    pub fn fast() -> Self {
        Self {
            delay: Duration::from_millis(1),
        }
    }
}

// 生成四段式总结
fn build_markdown(req: &SummaryRequest) -> String {
    let lines: Vec<&str> = req
        .transcript_markdown
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    let first = lines.first().copied().unwrap_or("（无转写内容）");
    let last = lines.last().copied().unwrap_or("（无转写内容）");
    format!(
        "## 知识点\n- 本节课共 {} 段转写\n- 开场：{}\n\n## 重点与难点\n- 结尾要点：{}\n\n## 课堂例题\n- 例题占位（Mock）\n\n## 课后待办\n- 复习《{}》的课堂内容\n",
        lines.len(),
        first,
        last,
        req.title
    )
}

#[async_trait]
impl SummarizerProvider for MockSummarizer {
    fn name(&self) -> &'static str {
        "mock"
    }

    async fn summarize(&self, req: SummaryRequest) -> Result<SummaryResult, SummaryError> {
        tokio::time::sleep(self.delay).await;
        Ok(SummaryResult {
            markdown: build_markdown(&req),
            model: "mock".to_string(),
            generated_at: chrono::Local::now().to_rfc3339(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> SummaryRequest {
        SummaryRequest {
            session_id: "2026-09-27_100000_abc123".to_string(),
            title: "数据结构课".to_string(),
            started_at: "2026-09-27T10:00:00+08:00".to_string(),
            duration_ms: 2700000,
            transcript_markdown: "[00:00:03] 同学们好。\n[00:00:11] 今天讲顺序表。".to_string(),
        }
    }

    #[tokio::test]
    async fn mock_summarizer_builds_four_sections() {
        let summarizer = MockSummarizer::fast();
        let result = summarizer.summarize(request()).await.unwrap();
        for heading in ["## 知识点", "## 重点与难点", "## 课堂例题", "## 课后待办"] {
            assert!(result.markdown.contains(heading));
        }
        assert!(result.markdown.contains("同学们好。"));
        assert_eq!(result.model, "mock");
        assert!(!result.generated_at.is_empty());
    }
}
