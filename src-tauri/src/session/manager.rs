// 会话编排 actor：采集 → ASR → 落盘 → 总结
use std::future::Future;
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::{mpsc, oneshot};

use crate::audio::{AudioCapture, AudioFrame, CaptureHandle};
use crate::asr::{AsrConfig, AsrEvent, AsrProvider};
use crate::domain::events::{
    AppErrorEvent, AsrPartialEvent, AsrSegmentEvent, SessionStateEvent, SummaryProgressEvent,
    SummaryReadyEvent, UiEvent,
};
use crate::domain::session::{SessionDetail, SessionMeta, SessionSnapshot, SessionStatus};
use crate::domain::summary::{SummaryRequest, SummaryResult};
use crate::domain::transcript::Segment;
use crate::error::{AppError, ErrorScope};
use crate::session::state::{SessionEvent, SessionStateMachine};
use crate::storage::{SessionPaths, SessionStore, StoreError};
use crate::summarizer::{SummaryError, SummarizerProvider};

// Provider 集合
pub struct Providers {
    pub capture: Arc<dyn AudioCapture>,
    pub asr: Arc<dyn AsrProvider>,
    pub summarizer: Arc<dyn SummarizerProvider>,
}

// 可热更新的 Provider 集合（设置页保存后替换）
pub type SharedProviders = Arc<std::sync::RwLock<Arc<Providers>>>;

// actor 命令
pub enum ManagerCmd {
    Start {
        title: Option<String>,
        resp: oneshot::Sender<Result<SessionDetail, AppError>>,
    },
    End {
        resp: oneshot::Sender<Result<SessionSnapshot, AppError>>,
    },
    Get {
        resp: oneshot::Sender<Option<SessionDetail>>,
    },
    Reset {
        resp: oneshot::Sender<Result<(), AppError>>,
    },
    Asr(AsrEvent),
    SummaryDone(Result<SummaryResult, SummaryError>),
}

// 会话编排句柄
pub struct SessionManager {
    tx: mpsc::Sender<ManagerCmd>,
}

impl SessionManager {
    // 创建句柄与 actor 循环（由调用方 spawn）
    pub fn create(
        providers: SharedProviders,
        store: Arc<dyn SessionStore>,
        sink: mpsc::UnboundedSender<UiEvent>,
    ) -> (Self, impl Future<Output = ()>) {
        let (tx, rx) = mpsc::channel(64);
        let actor = Actor {
            cmd_tx: tx.clone(),
            providers,
            store,
            sink,
            machine: SessionStateMachine::new(),
            ctx: None,
            segments: Vec::new(),
            summary: None,
        };
        (Self { tx }, actor.run(rx))
    }

    pub async fn start(&self, title: Option<String>) -> Result<SessionDetail, AppError> {
        let (resp, rx) = oneshot::channel();
        self.tx
            .send(ManagerCmd::Start { title, resp })
            .await
            .map_err(|_| stopped())?;
        rx.await.map_err(|_| stopped())?
    }

    pub async fn end(&self) -> Result<SessionSnapshot, AppError> {
        let (resp, rx) = oneshot::channel();
        self.tx
            .send(ManagerCmd::End { resp })
            .await
            .map_err(|_| stopped())?;
        rx.await.map_err(|_| stopped())?
    }

    pub async fn get(&self) -> Option<SessionDetail> {
        let (resp, rx) = oneshot::channel();
        self.tx.send(ManagerCmd::Get { resp }).await.ok()?;
        rx.await.ok()?
    }

    pub async fn reset(&self) -> Result<(), AppError> {
        let (resp, rx) = oneshot::channel();
        self.tx
            .send(ManagerCmd::Reset { resp })
            .await
            .map_err(|_| stopped())?;
        rx.await.map_err(|_| stopped())?
    }
}

fn stopped() -> AppError {
    AppError::new(ErrorScope::Session, "会话服务已停止")
}

fn store_err(e: StoreError) -> AppError {
    AppError::new(ErrorScope::Storage, e.0)
}

// 单个会话的运行时上下文
struct SessionCtx {
    meta: SessionMeta,
    paths: SessionPaths,
    capture: Option<Box<dyn CaptureHandle>>,
    started: Instant,
}

struct Actor {
    cmd_tx: mpsc::Sender<ManagerCmd>,
    providers: SharedProviders,
    store: Arc<dyn SessionStore>,
    sink: mpsc::UnboundedSender<UiEvent>,
    machine: SessionStateMachine,
    ctx: Option<SessionCtx>,
    segments: Vec<Segment>,
    summary: Option<String>,
}

impl Actor {
    async fn run(mut self, mut rx: mpsc::Receiver<ManagerCmd>) {
        while let Some(cmd) = rx.recv().await {
            match cmd {
                ManagerCmd::Start { title, resp } => {
                    let _ = resp.send(self.handle_start(title).await);
                }
                ManagerCmd::End { resp } => {
                    let _ = resp.send(self.handle_end().await);
                }
                ManagerCmd::Get { resp } => {
                    let _ = resp.send(self.detail());
                }
                ManagerCmd::Reset { resp } => {
                    let _ = resp.send(self.handle_reset());
                }
                ManagerCmd::Asr(event) => self.handle_asr(event),
                ManagerCmd::SummaryDone(result) => self.handle_summary(result),
            }
        }
    }

    fn emit(&self, event: UiEvent) {
        let _ = self.sink.send(event);
    }

    // 取当前 Provider 快照
    fn current_providers(&self) -> Result<Arc<Providers>, AppError> {
        self.providers
            .read()
            .map(|guard| guard.clone())
            .map_err(|e| AppError::new(ErrorScope::Session, e.to_string()))
    }

    fn emit_state(&self, status: SessionStatus, message: Option<String>) {
        let session_id = self.ctx.as_ref().map(|c| c.meta.id.clone());
        self.emit(UiEvent::State(SessionStateEvent {
            status,
            session_id,
            message,
        }));
    }

    async fn handle_start(&mut self, title: Option<String>) -> Result<SessionDetail, AppError> {
        self.machine.handle(SessionEvent::Start)?;
        let providers = self.current_providers()?;

        let now = chrono::Local::now();
        let id = format!(
            "{}_{:06x}",
            now.format("%Y-%m-%d_%H%M%S"),
            now.timestamp_subsec_nanos() & 0xff_ffff
        );
        let title = title
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| format!("{} 课堂", now.format("%Y-%m-%d %H:%M")));
        let meta = SessionMeta {
            schema_version: 1,
            id: id.clone(),
            title,
            started_at: Some(now.to_rfc3339()),
            ended_at: None,
            duration_ms: 0,
            status: SessionStatus::Recording,
            asr_provider: providers.asr.name().to_string(),
            summarizer_provider: providers.summarizer.name().to_string(),
            sample_rate: 16000,
            channels: 1,
            segment_count: 0,
            word_count: 0,
            summary_generated_at: None,
            error: None,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            device: whoami(),
        };

        let paths = self.store.create(&meta).map_err(store_err)?;
        self.store
            .write_transcript_header(&id, &transcript_header(&meta))
            .map_err(store_err)?;

        self.segments.clear();
        self.summary = None;

        let (frame_tx, frame_rx) = std::sync::mpsc::sync_channel(64);
        let capture = providers
            .capture
            .start(frame_tx)
            .map_err(|e| AppError::new(ErrorScope::Audio, e.0))?;
        let (tok_tx, tok_rx) = mpsc::channel(64);
        std::thread::spawn(move || {
            while let Ok(frame) = frame_rx.recv() {
                if tok_tx.blocking_send(frame).is_err() {
                    break;
                }
            }
        });

        let asr = providers.asr.clone();
        let cmd_tx = self.cmd_tx.clone();
        tokio::spawn(async move {
            run_pipeline(asr, tok_rx, cmd_tx).await;
        });

        self.ctx = Some(SessionCtx {
            meta,
            paths,
            capture: Some(capture),
            started: Instant::now(),
        });
        self.emit_state(SessionStatus::Recording, None);
        Ok(self.detail().expect("刚创建的会话必然存在"))
    }

    async fn handle_end(&mut self) -> Result<SessionSnapshot, AppError> {
        self.machine.handle(SessionEvent::End)?;

        let segment_count = self.segments.len() as u32;
        let word_count = self
            .segments
            .iter()
            .map(|seg| seg.text.chars().count() as u32)
            .sum();

        // 先取走需要的字段，结束对 self.ctx 的借用
        let (id, title, started_at, duration_ms) = {
            let Some(ctx) = self.ctx.as_mut() else {
                return Err(AppError::new(ErrorScope::Session, "没有进行中的会话"));
            };
            if let Some(mut handle) = ctx.capture.take() {
                handle.stop();
            }
            let now = chrono::Local::now();
            ctx.meta.status = SessionStatus::Summarizing;
            ctx.meta.ended_at = Some(now.to_rfc3339());
            ctx.meta.duration_ms = ctx.started.elapsed().as_millis() as u64;
            ctx.meta.segment_count = segment_count;
            ctx.meta.word_count = word_count;
            (
                ctx.meta.id.clone(),
                ctx.meta.title.clone(),
                ctx.meta.started_at.clone().unwrap_or_default(),
                ctx.meta.duration_ms,
            )
        };

        self.store.flush(&id).map_err(store_err)?;
        if let Some(ctx) = self.ctx.as_ref() {
            self.store.write_meta(&ctx.meta).map_err(store_err)?;
        }

        self.emit_state(SessionStatus::Summarizing, None);
        self.emit(UiEvent::SummaryProgress(SummaryProgressEvent {
            session_id: id.clone(),
            stage: "collecting".to_string(),
            percent: Some(10),
        }));

        let request = SummaryRequest {
            session_id: id,
            title,
            started_at,
            duration_ms,
            transcript_markdown: self.transcript_markdown(),
        };
        let summarizer = self.current_providers()?.summarizer.clone();
        let cmd_tx = self.cmd_tx.clone();
        tokio::spawn(async move {
            let result = summarizer.summarize(request).await;
            let _ = cmd_tx.send(ManagerCmd::SummaryDone(result)).await;
        });

        Ok(self.snapshot())
    }

    fn handle_reset(&mut self) -> Result<(), AppError> {
        self.machine.handle(SessionEvent::Reset)?;
        self.ctx = None;
        self.segments.clear();
        self.summary = None;
        self.emit_state(SessionStatus::Idle, None);
        Ok(())
    }

    fn handle_asr(&mut self, event: AsrEvent) {
        let Some(session_id) = self.ctx.as_ref().map(|c| c.meta.id.clone()) else {
            return;
        };
        match event {
            AsrEvent::Partial { text, start_ms } => {
                self.emit(UiEvent::Partial(AsrPartialEvent {
                    session_id,
                    text,
                    start_ms,
                }));
            }
            AsrEvent::Segment {
                text,
                start_ms,
                end_ms,
            } => {
                if self.machine.handle(SessionEvent::Segment).is_err() {
                    return;
                }
                let seg = Segment {
                    text: text.clone(),
                    start_ms,
                    end_ms,
                };
                self.segments.push(seg.clone());
                if let Err(e) = self.store.append_segments(&session_id, &[seg]) {
                    self.emit_error(ErrorScope::Storage, e.0);
                }
                self.emit(UiEvent::Segment(AsrSegmentEvent {
                    session_id,
                    text,
                    start_ms,
                    end_ms,
                }));
            }
            AsrEvent::Error { message } => {
                self.emit_error(ErrorScope::Asr, message);
            }
            AsrEvent::Ended => {}
        }
    }

    fn handle_summary(&mut self, result: Result<SummaryResult, SummaryError>) {
        // 先更新会话元数据，结束对 self.ctx 的借用
        let (id, meta) = {
            let Some(ctx) = self.ctx.as_mut() else {
                return;
            };
            match &result {
                Ok(res) => {
                    ctx.meta.status = SessionStatus::Completed;
                    ctx.meta.summary_generated_at = Some(res.generated_at.clone());
                }
                Err(err) => {
                    ctx.meta.status = SessionStatus::Error;
                    ctx.meta.error = Some(err.0.clone());
                }
            }
            (ctx.meta.id.clone(), ctx.meta.clone())
        };

        match result {
            Ok(res) => {
                if self.machine.handle(SessionEvent::SummarizeOk).is_err() {
                    return;
                }
                if let Err(e) = self.store.write_summary(&id, &res.markdown) {
                    self.emit_error(ErrorScope::Storage, e.0);
                }
                if let Err(e) = self.store.write_meta(&meta) {
                    self.emit_error(ErrorScope::Storage, e.0);
                }
                self.summary = Some(res.markdown.clone());
                self.emit(UiEvent::SummaryReady(SummaryReadyEvent {
                    session_id: id,
                    summary_markdown: res.markdown,
                    meta,
                }));
                self.emit_state(SessionStatus::Completed, None);
            }
            Err(err) => {
                let _ = self.machine.handle(SessionEvent::Fail);
                if let Err(e) = self.store.write_meta(&meta) {
                    self.emit_error(ErrorScope::Storage, e.0);
                }
                self.emit_error(ErrorScope::Summary, err.0);
                self.emit_state(SessionStatus::Error, None);
            }
        }
    }

    fn emit_error(&self, scope: ErrorScope, message: String) {
        self.emit(UiEvent::Error(AppErrorEvent {
            scope: format!("{scope:?}").to_lowercase(),
            message,
            recoverable: true,
        }));
    }

    fn transcript_markdown(&self) -> String {
        self.segments
            .iter()
            .map(|s| format!("[{}] {}", fmt_ts(s.start_ms), s.text))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn snapshot(&self) -> SessionSnapshot {
        let ctx = self.ctx.as_ref().expect("会话上下文");
        SessionSnapshot {
            meta: ctx.meta.clone(),
            data_dir: Some(ctx.paths.dir.display().to_string()),
            summary_markdown: self.summary.clone(),
        }
    }

    fn detail(&self) -> Option<SessionDetail> {
        self.ctx.as_ref().map(|_| SessionDetail {
            snapshot: self.snapshot(),
            segments: self.segments.clone(),
        })
    }
}

// 采集 → ASR → 事件回传（单任务：批量推帧 + 限时等事件）
async fn run_pipeline(
    asr: Arc<dyn AsrProvider>,
    mut frames: mpsc::Receiver<AudioFrame>,
    cmd_tx: mpsc::Sender<ManagerCmd>,
) {
    let config = AsrConfig {
        sample_rate: 16000,
        channels: 1,
        language: "zh-CN".to_string(),
    };
    let mut stream = match asr.start(config).await {
        Ok(stream) => stream,
        Err(e) => {
            let _ = cmd_tx
                .send(ManagerCmd::Asr(AsrEvent::Error { message: e.0 }))
                .await;
            return;
        }
    };

    let mut finished = false;
    loop {
        while let Ok(frame) = frames.try_recv() {
            if stream.push_audio(frame).await.is_err() {
                return;
            }
        }
        if !finished && frames.is_closed() {
            finished = true;
            let _ = stream.finish().await;
        }
        let polled = tokio::time::timeout(std::time::Duration::from_millis(5), stream.next_event())
            .await;
        match polled {
            Ok(Some(event)) => {
                let ended = matches!(event, AsrEvent::Ended);
                let _ = cmd_tx.send(ManagerCmd::Asr(event)).await;
                if ended {
                    break;
                }
            }
            Ok(None) => break,
            Err(_) => {}
        }
    }
}

// 毫秒转 HH:MM:SS
fn fmt_ts(ms: u64) -> String {
    let total = ms / 1000;
    format!(
        "{:02}:{:02}:{:02}",
        total / 3600,
        (total % 3600) / 60,
        total % 60
    )
}

// 转写文件头
fn transcript_header(meta: &SessionMeta) -> String {
    format!(
        "# 课堂转写：{}\n\n- 开始时间：{}\n- 识别引擎：{}\n\n## 转写正文",
        meta.title,
        meta.started_at.clone().unwrap_or_default(),
        meta.asr_provider
    )
}

fn whoami() -> String {
    std::env::var("COMPUTERNAME").unwrap_or_else(|_| "windows".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asr::MockAsrProvider;
    use crate::audio::MockAudioCapture;
    use crate::storage::fs_store::FsSessionStore;
    use crate::summarizer::MockSummarizer;
    use std::time::Duration;

    fn fast_providers() -> SharedProviders {
        Arc::new(std::sync::RwLock::new(Arc::new(Providers {
            capture: Arc::new(MockAudioCapture::default()),
            asr: Arc::new(MockAsrProvider::fast()),
            summarizer: Arc::new(MockSummarizer::fast()),
        })))
    }

    #[tokio::test]
    async fn mock_pipeline_writes_session_files() {
        let root = std::env::temp_dir().join(format!("ca_manager_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let (sink, mut events) = mpsc::unbounded_channel();
        let (manager, actor) =
            SessionManager::create(fast_providers(), Arc::new(FsSessionStore::new(&root)), sink);
        tokio::spawn(actor);

        let detail = manager.start(Some("测试课".into())).await.unwrap();
        assert_eq!(detail.snapshot.meta.status, SessionStatus::Recording);
        assert!(detail.snapshot.data_dir.is_some());

        // 等待定稿转写段落产生（Windows 计时器粒度较粗，按轮询等待）
        let mut got_segments = false;
        for _ in 0..100 {
            if let Some(detail) = manager.get().await {
                if detail.segments.len() >= 2 {
                    got_segments = true;
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(got_segments, "转写段落未产生");

        let snapshot = manager.end().await.unwrap();
        assert_eq!(snapshot.meta.status, SessionStatus::Summarizing);

        let mut completed = false;
        for _ in 0..200 {
            if let Some(detail) = manager.get().await {
                if detail.snapshot.meta.status == SessionStatus::Completed {
                    completed = true;
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        assert!(completed, "总结未完成");

        let dir = std::path::PathBuf::from(
            manager
                .get()
                .await
                .unwrap()
                .snapshot
                .data_dir
                .unwrap()
                .to_string(),
        );
        assert!(dir.join("meta.json").exists());
        assert!(dir.join("transcript.md").exists());
        assert!(dir.join("summary.md").exists());
        let summary = std::fs::read_to_string(dir.join("summary.md")).unwrap();
        assert!(summary.contains("## 知识点"));
        let transcript = std::fs::read_to_string(dir.join("transcript.md")).unwrap();
        assert!(transcript.contains("## 转写正文"));
        let meta: SessionMeta =
            serde_json::from_str(&std::fs::read_to_string(dir.join("meta.json")).unwrap()).unwrap();
        assert_eq!(meta.status, SessionStatus::Completed);
        assert!(meta.segment_count > 0, "segment_count 未统计");
        assert!(meta.word_count > 0, "word_count 未统计");

        let mut saw_ready = false;
        while let Ok(event) = events.try_recv() {
            if matches!(event, UiEvent::SummaryReady(_)) {
                saw_ready = true;
            }
        }
        assert!(saw_ready);

        manager.reset().await.unwrap();
        assert!(manager.get().await.is_none());

        let _ = std::fs::remove_dir_all(&root);
    }
}
