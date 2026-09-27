// UI 事件载荷（与前端 src/types/session.ts 镜像）
use serde::Serialize;

use crate::domain::session::{SessionMeta, SessionStatus};

#[derive(Debug, Clone, Serialize)]
pub struct SessionStateEvent {
    pub status: SessionStatus,
    pub session_id: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AsrPartialEvent {
    pub session_id: String,
    pub text: String,
    pub start_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AsrSegmentEvent {
    pub session_id: String,
    pub text: String,
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SummaryProgressEvent {
    pub session_id: String,
    pub stage: String,
    pub percent: Option<u8>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SummaryReadyEvent {
    pub session_id: String,
    pub summary_markdown: String,
    pub meta: SessionMeta,
}

#[derive(Debug, Clone, Serialize)]
pub struct AppErrorEvent {
    pub scope: String,
    pub message: String,
    pub recoverable: bool,
}

// 麦克风电平
#[derive(Debug, Clone, Serialize)]
pub struct AudioLevelEvent {
    pub session_id: String,
    // 0.0 ~ 1.0
    pub level: f32,
}

// 会话编排产出的 UI 事件
#[derive(Debug, Clone)]
pub enum UiEvent {
    State(SessionStateEvent),
    Partial(AsrPartialEvent),
    Segment(AsrSegmentEvent),
    SummaryProgress(SummaryProgressEvent),
    SummaryReady(SummaryReadyEvent),
    Level(AudioLevelEvent),
    Error(AppErrorEvent),
}
