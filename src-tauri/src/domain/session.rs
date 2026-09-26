// 会话领域模型（与前端 src/types/session.ts 镜像）
use serde::{Deserialize, Serialize};

use crate::domain::transcript::Segment;

// 会话状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Idle,
    Recording,
    Summarizing,
    Completed,
    Error,
}

// 会话元数据
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionMeta {
    pub schema_version: u32,
    pub id: String,
    pub title: String,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub duration_ms: u64,
    pub status: SessionStatus,
    pub asr_provider: String,
    pub summarizer_provider: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub segment_count: u32,
    pub word_count: u32,
    pub summary_generated_at: Option<String>,
    pub error: Option<String>,
    pub app_version: String,
    pub device: String,
}

// 会话快照
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSnapshot {
    pub meta: SessionMeta,
    pub data_dir: Option<String>,
    pub summary_markdown: Option<String>,
}

// 会话详情
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionDetail {
    pub snapshot: SessionSnapshot,
    pub segments: Vec<Segment>,
}
