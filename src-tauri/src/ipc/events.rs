// UiEvent → 前端事件（事件名与 src/types/ipc.ts 一致）
use tauri::{AppHandle, Emitter, Runtime};

use crate::domain::events::UiEvent;

pub const EVT_SESSION_STATE: &str = "session:state";
pub const EVT_ASR_PARTIAL: &str = "asr:partial";
pub const EVT_ASR_SEGMENT: &str = "asr:segment";
pub const EVT_SUMMARY_PROGRESS: &str = "summary:progress";
pub const EVT_SUMMARY_READY: &str = "summary:ready";
pub const EVT_APP_ERROR: &str = "app:error";

pub fn emit_ui_event<R: Runtime>(app: &AppHandle<R>, event: &UiEvent) {
    let _ = match event {
        UiEvent::State(payload) => app.emit(EVT_SESSION_STATE, payload),
        UiEvent::Partial(payload) => app.emit(EVT_ASR_PARTIAL, payload),
        UiEvent::Segment(payload) => app.emit(EVT_ASR_SEGMENT, payload),
        UiEvent::SummaryProgress(payload) => app.emit(EVT_SUMMARY_PROGRESS, payload),
        UiEvent::SummaryReady(payload) => app.emit(EVT_SUMMARY_READY, payload),
        UiEvent::Error(payload) => app.emit(EVT_APP_ERROR, payload),
    };
}
