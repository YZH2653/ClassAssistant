// Tauri 命令（前端 invoke 入口）
use serde::Serialize;
use tauri::State;

use crate::app::{build_providers, AppState};
use crate::domain::session::{SessionDetail, SessionSnapshot};
use crate::domain::settings::{AppSettings, ProviderKind, TestResult};
use crate::error::{AppError, ErrorScope};

// 应用信息
#[derive(Serialize)]
pub struct AppInfo {
    pub app_version: String,
    pub data_dir: String,
    pub asr_provider: String,
    pub summarizer_provider: String,
}

#[tauri::command]
pub fn get_app_info(state: State<'_, AppState>) -> AppInfo {
    let providers = state.providers.read().map(|g| g.clone());
    let (asr_provider, summarizer_provider) = match providers {
        Ok(providers) => (
            providers.asr.name().to_string(),
            providers.summarizer.name().to_string(),
        ),
        Err(_) => ("unknown".to_string(), "unknown".to_string()),
    };
    AppInfo {
        app_version: state.app_version.clone(),
        data_dir: state.data_dir.display().to_string(),
        asr_provider,
        summarizer_provider,
    }
}

#[tauri::command]
pub async fn start_class(
    state: State<'_, AppState>,
    title: Option<String>,
) -> Result<SessionDetail, AppError> {
    state.manager.start(title).await
}

#[tauri::command]
pub async fn end_class(state: State<'_, AppState>) -> Result<SessionSnapshot, AppError> {
    state.manager.end().await
}

#[tauri::command]
pub async fn get_current_session(
    state: State<'_, AppState>,
) -> Result<Option<SessionDetail>, AppError> {
    Ok(state.manager.get().await)
}

#[tauri::command]
pub async fn reset_session(state: State<'_, AppState>) -> Result<(), AppError> {
    state.manager.reset().await
}

#[tauri::command]
pub fn reveal_data_dir(state: State<'_, AppState>) -> Result<(), AppError> {
    std::process::Command::new("explorer")
        .arg(&state.data_dir)
        .spawn()
        .map_err(|e| AppError::new(ErrorScope::Storage, e.to_string()))?;
    Ok(())
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> AppSettings {
    state.config_store.load()
}

#[tauri::command]
pub fn save_settings(
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<(), AppError> {
    state
        .config_store
        .save(&settings)
        .map_err(|e| AppError::new(ErrorScope::Storage, e.0))?;
    let providers = build_providers(&settings);
    let mut guard = state
        .providers
        .write()
        .map_err(|e| AppError::new(ErrorScope::Session, e.to_string()))?;
    *guard = std::sync::Arc::new(providers);
    Ok(())
}

#[tauri::command]
pub fn test_provider_connection(
    state: State<'_, AppState>,
    kind: String,
) -> Result<TestResult, AppError> {
    let settings = state.config_store.load();
    let config = match kind.as_str() {
        "asr" => settings.asr,
        "summarizer" => settings.summarizer,
        _ => {
            return Ok(TestResult {
                ok: false,
                message: "未知的配置项".to_string(),
            })
        }
    };
    Ok(match config.provider {
        ProviderKind::Mock => TestResult {
            ok: true,
            message: "Mock 连接正常".to_string(),
        },
        ProviderKind::Mimo => {
            if config.api_key.trim().is_empty() {
                TestResult {
                    ok: false,
                    message: "请先填写 API Key".to_string(),
                }
            } else {
                TestResult {
                    ok: false,
                    message: "MiMo 接口尚未接入（等 API 文档）".to_string(),
                }
            }
        }
    })
}
