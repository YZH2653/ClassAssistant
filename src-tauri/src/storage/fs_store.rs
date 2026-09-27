// 文件系统会话存储
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::domain::session::SessionMeta;
use crate::domain::transcript::Segment;

use super::{SessionPaths, SessionStore, StoreError};

// 每攒够多少段落盘一次
const FLUSH_EVERY: usize = 5;

pub struct FsSessionStore {
    root: PathBuf,
    pending: Mutex<HashMap<String, Vec<Segment>>>,
}

impl FsSessionStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            pending: Mutex::new(HashMap::new()),
        }
    }

    fn session_dir(&self, id: &str) -> PathBuf {
        self.root.join("sessions").join(id)
    }

    fn transcript_path(&self, id: &str) -> PathBuf {
        self.session_dir(id).join("transcript.md")
    }

    fn append_lines(&self, id: &str, segs: &[Segment]) -> Result<(), StoreError> {
        if segs.is_empty() {
            return Ok(());
        }
        let path = self.transcript_path(id);
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| StoreError(format!("{}: {}", path.display(), e)))?;
        for seg in segs {
            writeln!(file, "[{}] {}", format_ts(seg.start_ms), seg.text)
                .map_err(|e| StoreError(e.to_string()))?;
        }
        Ok(())
    }
}

// 毫秒转 HH:MM:SS
fn format_ts(ms: u64) -> String {
    let total = ms / 1000;
    format!(
        "{:02}:{:02}:{:02}",
        total / 3600,
        (total % 3600) / 60,
        total % 60
    )
}

fn io_err(path: &Path, e: std::io::Error) -> StoreError {
    StoreError(format!("{}: {}", path.display(), e))
}

impl SessionStore for FsSessionStore {
    fn create(&self, meta: &SessionMeta) -> Result<SessionPaths, StoreError> {
        let dir = self.session_dir(&meta.id);
        fs::create_dir_all(&dir).map_err(|e| io_err(&dir, e))?;
        let paths = SessionPaths {
            dir,
            meta: self.session_dir(&meta.id).join("meta.json"),
            transcript: self.transcript_path(&meta.id),
            summary: self.session_dir(&meta.id).join("summary.md"),
        };
        self.write_meta(meta)?;
        Ok(paths)
    }

    fn write_transcript_header(&self, id: &str, header: &str) -> Result<(), StoreError> {
        let path = self.transcript_path(id);
        fs::write(&path, format!("{header}\n")).map_err(|e| io_err(&path, e))
    }

    fn append_segments(&self, id: &str, segs: &[Segment]) -> Result<(), StoreError> {
        let to_write = {
            let mut pending = self.pending.lock().map_err(|e| StoreError(e.to_string()))?;
            let list = pending.entry(id.to_string()).or_default();
            list.extend_from_slice(segs);
            if list.len() >= FLUSH_EVERY {
                std::mem::take(list)
            } else {
                Vec::new()
            }
        };
        self.append_lines(id, &to_write)
    }

    fn flush(&self, id: &str) -> Result<(), StoreError> {
        let to_write = {
            let mut pending = self.pending.lock().map_err(|e| StoreError(e.to_string()))?;
            pending.remove(id).unwrap_or_default()
        };
        self.append_lines(id, &to_write)
    }

    fn write_summary(&self, id: &str, markdown: &str) -> Result<(), StoreError> {
        let path = self.session_dir(id).join("summary.md");
        fs::write(&path, markdown).map_err(|e| io_err(&path, e))
    }

    fn write_meta(&self, meta: &SessionMeta) -> Result<(), StoreError> {
        let path = self.session_dir(&meta.id).join("meta.json");
        let json = serde_json::to_string_pretty(meta).map_err(|e| StoreError(e.to_string()))?;
        fs::create_dir_all(self.session_dir(&meta.id)).map_err(|e| io_err(&path, e))?;
        fs::write(&path, json).map_err(|e| io_err(&path, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::session::SessionStatus;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_root() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ca_store_test_{}_{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn sample_meta(id: &str) -> SessionMeta {
        SessionMeta {
            schema_version: 1,
            id: id.to_string(),
            title: "测试课".to_string(),
            started_at: Some("2026-09-27T10:00:00+08:00".to_string()),
            ended_at: None,
            duration_ms: 0,
            status: SessionStatus::Recording,
            asr_provider: "mock".to_string(),
            summarizer_provider: "mock".to_string(),
            sample_rate: 16000,
            channels: 1,
            segment_count: 0,
            word_count: 0,
            summary_generated_at: None,
            error: None,
            app_version: "0.1.0".to_string(),
            device: "test".to_string(),
        }
    }

    fn seg(text: &str, ms: u64) -> Segment {
        Segment {
            text: text.to_string(),
            start_ms: ms,
            end_ms: ms + 1000,
        }
    }

    #[test]
    fn create_writes_meta_and_paths() {
        let root = temp_root();
        let store = FsSessionStore::new(&root);
        let meta = sample_meta("2026-09-27_100000_abc123");
        let paths = store.create(&meta).unwrap();
        assert!(paths.dir.exists());
        assert!(paths.meta.exists());
        let back: SessionMeta =
            serde_json::from_str(&fs::read_to_string(&paths.meta).unwrap()).unwrap();
        assert_eq!(back.id, meta.id);
        assert_eq!(back.status, SessionStatus::Recording);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn transcript_flushes_every_five_and_on_flush() {
        let root = temp_root();
        let store = FsSessionStore::new(&root);
        let id = "2026-09-27_100000_flush01";
        store.create(&sample_meta(id)).unwrap();
        store.write_transcript_header(id, "# 课堂转写：测试课").unwrap();

        store
            .append_segments(
                id,
                &[seg("第一句", 3000), seg("第二句", 11000), seg("第三句", 19000)],
            )
            .unwrap();
        let content = fs::read_to_string(store.transcript_path(id)).unwrap();
        assert_eq!(content.matches("[00:00:").count(), 0);

        store
            .append_segments(
                id,
                &[seg("第四句", 27000), seg("第五句", 35000), seg("第六句", 43000)],
            )
            .unwrap();
        let content = fs::read_to_string(store.transcript_path(id)).unwrap();
        assert_eq!(content.matches("[00:00:").count(), 6);

        store.flush(id).unwrap();
        let content = fs::read_to_string(store.transcript_path(id)).unwrap();
        assert!(content.contains("[00:00:03] 第一句"));
        assert!(content.contains("[00:00:43] 第六句"));
        assert_eq!(content.matches("[00:00:").count(), 6);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn summary_written() {
        let root = temp_root();
        let store = FsSessionStore::new(&root);
        let id = "2026-09-27_100000_sum001";
        store.create(&sample_meta(id)).unwrap();
        store.write_summary(id, "## 知识点\n- 顺序表\n").unwrap();
        let content = fs::read_to_string(store.session_dir(id).join("summary.md")).unwrap();
        assert!(content.contains("顺序表"));
        let _ = fs::remove_dir_all(&root);
    }
}
