// 音频采集抽象
pub mod cpal_capture;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::SyncSender;
use std::sync::Arc;

// 音频错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioError(pub String);

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// 音频帧（16kHz / 单声道 / s16le）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioFrame {
    pub pcm_s16le: Vec<u8>,
    pub sample_rate: u32,
    pub channels: u16,
    pub captured_at_ms: u64,
}

// 采集句柄
pub trait CaptureHandle: Send {
    fn stop(&mut self);
}

// 音频采集器
pub trait AudioCapture: Send + Sync {
    fn name(&self) -> &'static str;
    fn start(&self, tx: SyncSender<AudioFrame>) -> Result<Box<dyn CaptureHandle>, AudioError>;
}

// Mock 采集器（正弦波）
pub struct MockAudioCapture {
    pub frame_interval_ms: u64,
}

impl Default for MockAudioCapture {
    fn default() -> Self {
        Self {
            frame_interval_ms: 20,
        }
    }
}

struct MockCaptureHandle {
    stop: Arc<AtomicBool>,
}

impl CaptureHandle for MockCaptureHandle {
    fn stop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

impl AudioCapture for MockAudioCapture {
    fn name(&self) -> &'static str {
        "mock"
    }

    fn start(&self, tx: SyncSender<AudioFrame>) -> Result<Box<dyn CaptureHandle>, AudioError> {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = stop.clone();
        let interval = self.frame_interval_ms.max(1);
        std::thread::spawn(move || {
            let samples_per_frame = 16000 * interval / 1000;
            let mut phase = 0.0f32;
            let mut tick = 0u64;
            while !stop_flag.load(Ordering::SeqCst) {
                let mut pcm = Vec::with_capacity(samples_per_frame as usize * 2);
                for _ in 0..samples_per_frame {
                    let value = (phase.sin() * 3000.0) as i16;
                    pcm.extend_from_slice(&value.to_le_bytes());
                    phase += 2.0 * std::f32::consts::PI * 440.0 / 16000.0;
                }
                let frame = AudioFrame {
                    pcm_s16le: pcm,
                    sample_rate: 16000,
                    channels: 1,
                    captured_at_ms: tick * interval,
                };
                if tx.send(frame).is_err() {
                    break;
                }
                tick += 1;
                std::thread::sleep(std::time::Duration::from_millis(interval));
            }
        });
        Ok(Box::new(MockCaptureHandle { stop }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::sync_channel;
    use std::time::Duration;

    #[test]
    fn mock_capture_produces_frames() {
        let capture = MockAudioCapture {
            frame_interval_ms: 5,
        };
        let (tx, rx) = sync_channel(8);
        let mut handle = capture.start(tx).unwrap();
        let frame = rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(frame.sample_rate, 16000);
        assert_eq!(frame.channels, 1);
        assert!(!frame.pcm_s16le.is_empty());
        handle.stop();
    }
}
