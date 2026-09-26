// 语音识别抽象
pub mod mimo;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::Serialize;
use tokio::sync::mpsc;

use crate::audio::AudioFrame;

// ASR 事件
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AsrEvent {
    Partial { text: String, start_ms: u64 },
    Segment {
        text: String,
        start_ms: u64,
        end_ms: u64,
    },
    Error { message: String },
    Ended,
}

// ASR 配置
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsrConfig {
    pub sample_rate: u32,
    pub channels: u16,
    pub language: String,
}

// ASR 错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsrError(pub String);

impl std::fmt::Display for AsrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ASR 事件流
#[async_trait]
pub trait AsrStream: Send {
    async fn push_audio(&mut self, frame: AudioFrame) -> Result<(), AsrError>;
    async fn finish(&mut self) -> Result<(), AsrError>;
    async fn next_event(&mut self) -> Option<AsrEvent>;
}

// ASR Provider
#[async_trait]
pub trait AsrProvider: Send + Sync {
    fn name(&self) -> &'static str;
    async fn start(&self, cfg: AsrConfig) -> Result<Box<dyn AsrStream>, AsrError>;
}

// Mock 课文脚本
const SCRIPT: [&str; 12] = [
    "同学们好，我们开始上课。",
    "今天我们讲线性表的顺序存储结构。",
    "顺序表用一段连续的存储单元依次存储数据元素。",
    "假设首地址是 LOC，每个元素占 k 个单元。",
    "那么第 i 个元素的地址就是 LOC 加上 i 减一乘以 k。",
    "这个公式说明顺序表支持随机存取。",
    "插入操作需要把插入位置之后的元素整体后移。",
    "所以插入的平均时间复杂度是 O(n)。",
    "删除操作同理，需要把后面的元素前移。",
    "顺序表的优点是存储密度高、可以随机访问。",
    "缺点是插入删除慢、需要预先分配连续空间。",
    "好，这节课就讲到这里，下课。",
];

// Mock ASR Provider
pub struct MockAsrProvider {
    tick: Duration,
}

impl Default for MockAsrProvider {
    fn default() -> Self {
        Self {
            tick: Duration::from_millis(50),
        }
    }
}

impl MockAsrProvider {
    // 测试用快节奏
    pub fn fast() -> Self {
        Self {
            tick: Duration::from_millis(1),
        }
    }
}

struct MockAsrStream {
    rx: mpsc::Receiver<AsrEvent>,
    finished: Arc<AtomicBool>,
}

#[async_trait]
impl AsrStream for MockAsrStream {
    async fn push_audio(&mut self, _frame: AudioFrame) -> Result<(), AsrError> {
        Ok(())
    }

    async fn finish(&mut self) -> Result<(), AsrError> {
        self.finished.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn next_event(&mut self) -> Option<AsrEvent> {
        self.rx.recv().await
    }
}

#[async_trait]
impl AsrProvider for MockAsrProvider {
    fn name(&self) -> &'static str {
        "mock"
    }

    async fn start(&self, _cfg: AsrConfig) -> Result<Box<dyn AsrStream>, AsrError> {
        let (tx, rx) = mpsc::channel(64);
        let finished = Arc::new(AtomicBool::new(false));
        let stop = finished.clone();
        let tick = self.tick;
        tokio::spawn(async move {
            let mut start_ms = 0u64;
            'outer: for sentence in SCRIPT {
                let chars: Vec<char> = sentence.chars().collect();
                for i in 1..=chars.len() {
                    if stop.load(Ordering::SeqCst) {
                        break 'outer;
                    }
                    let text: String = chars[..i].iter().collect();
                    let event = AsrEvent::Partial { text, start_ms };
                    if tx.send(event).await.is_err() {
                        break 'outer;
                    }
                    tokio::time::sleep(tick).await;
                }
                let end_ms = start_ms + chars.len() as u64 * tick.as_millis() as u64;
                let event = AsrEvent::Segment {
                    text: sentence.to_string(),
                    start_ms,
                    end_ms,
                };
                if tx.send(event).await.is_err() {
                    break;
                }
                start_ms = end_ms;
                tokio::time::sleep(tick * 3).await;
            }
            let _ = tx.send(AsrEvent::Ended).await;
        });
        Ok(Box::new(MockAsrStream { rx, finished }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> AsrConfig {
        AsrConfig {
            sample_rate: 16000,
            channels: 1,
            language: "zh-CN".to_string(),
        }
    }

    fn frame() -> AudioFrame {
        AudioFrame {
            pcm_s16le: vec![0, 0],
            sample_rate: 16000,
            channels: 1,
            captured_at_ms: 0,
        }
    }

    #[tokio::test]
    async fn mock_stream_produces_partial_segment_ended() {
        let provider = MockAsrProvider::fast();
        let mut stream = provider.start(config()).await.unwrap();
        stream.push_audio(frame()).await.unwrap();

        let mut partial_count = 0;
        let mut segment_count = 0;
        let mut ended = false;
        while let Some(event) = stream.next_event().await {
            match event {
                AsrEvent::Partial { .. } => partial_count += 1,
                AsrEvent::Segment { .. } => segment_count += 1,
                AsrEvent::Ended => {
                    ended = true;
                    break;
                }
                AsrEvent::Error { message } => panic!("意外错误：{message}"),
            }
        }

        assert!(ended);
        assert_eq!(segment_count, SCRIPT.len());
        assert!(partial_count > SCRIPT.len());
    }

    #[tokio::test]
    async fn finish_stops_stream_early() {
        let provider = MockAsrProvider::fast();
        let mut stream = provider.start(config()).await.unwrap();
        stream.next_event().await.unwrap();
        stream.finish().await.unwrap();

        let mut got_ended = false;
        while let Some(event) = stream.next_event().await {
            if matches!(event, AsrEvent::Ended) {
                got_ended = true;
                break;
            }
        }
        assert!(got_ended);
    }

    #[tokio::test]
    async fn push_audio_ignores_content() {
        let provider = MockAsrProvider::fast();
        let mut stream = provider.start(config()).await.unwrap();
        for i in 0..3u8 {
            let mut f = frame();
            f.pcm_s16le = vec![i; 4];
            stream.push_audio(f).await.unwrap();
        }
        stream.finish().await.unwrap();
    }
}
