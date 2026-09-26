// 知识点总结领域模型
use serde::{Deserialize, Serialize};

// 总结请求
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SummaryRequest {
    pub session_id: String,
    pub title: String,
    pub started_at: String,
    pub duration_ms: u64,
    pub transcript_markdown: String,
}

// 总结结果
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SummaryResult {
    pub markdown: String,
    pub model: String,
    pub generated_at: String,
}
