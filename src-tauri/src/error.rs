// 统一错误类型
use serde::Serialize;

// 错误归属模块
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorScope {
    Audio,
    Asr,
    Summary,
    Storage,
    Session,
}

// 应用错误
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AppError {
    pub scope: ErrorScope,
    pub message: String,
    pub recoverable: bool,
}

impl AppError {
    pub fn new(scope: ErrorScope, message: impl Into<String>) -> Self {
        Self {
            scope,
            message: message.into(),
            recoverable: true,
        }
    }

    pub fn fatal(scope: ErrorScope, message: impl Into<String>) -> Self {
        Self {
            scope,
            message: message.into(),
            recoverable: false,
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.scope, self.message)
    }
}

impl std::error::Error for AppError {}
