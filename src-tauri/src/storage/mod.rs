// 会话与配置存储抽象
pub mod config_store;
pub mod fs_store;

use std::path::PathBuf;

use crate::domain::session::SessionMeta;
use crate::domain::transcript::Segment;

// 存储错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreError(pub String);

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// 会话文件路径
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionPaths {
    pub dir: PathBuf,
    pub meta: PathBuf,
    pub transcript: PathBuf,
    pub summary: PathBuf,
}

// 会话存储
pub trait SessionStore: Send + Sync {
    fn create(&self, meta: &SessionMeta) -> Result<SessionPaths, StoreError>;
    fn write_transcript_header(&self, id: &str, header: &str) -> Result<(), StoreError>;
    fn append_segments(&self, id: &str, segs: &[Segment]) -> Result<(), StoreError>;
    fn flush(&self, id: &str) -> Result<(), StoreError>;
    fn write_summary(&self, id: &str, markdown: &str) -> Result<(), StoreError>;
    fn write_meta(&self, meta: &SessionMeta) -> Result<(), StoreError>;
}
