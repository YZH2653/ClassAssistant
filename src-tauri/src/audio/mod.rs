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

// 采集选项
#[derive(Debug, Clone, Default)]
pub struct CaptureOptions {
    // 输入设备名（None = 系统默认）
    pub device_name: Option<String>,
    // 音量过低时自动增益
    pub auto_gain: bool,
}

// 音频采集器
pub trait AudioCapture: Send + Sync {
    fn name(&self) -> &'static str;
    fn start(
        &self,
        tx: SyncSender<AudioFrame>,
        options: &CaptureOptions,
    ) -> Result<Box<dyn CaptureHandle>, AudioError>;
}

// 计算 s16le 音频的峰值（0.0 ~ 1.0）
pub fn peak_of_pcm(pcm: &[u8]) -> f32 {
    let mut peak: i32 = 0;
    for chunk in pcm.chunks_exact(2) {
        let sample = i16::from_le_bytes([chunk[0], chunk[1]]).unsigned_abs() as i32;
        if sample > peak {
            peak = sample;
        }
    }
    peak as f32 / i16::MAX as f32
}

// 峰值过低时提升增益（只升不降，最多 10 倍）
pub fn apply_auto_gain(samples: &mut [i16]) {
    let peak = samples
        .iter()
        .map(|s| s.unsigned_abs() as u32)
        .max()
        .unwrap_or(0) as f32
        / i16::MAX as f32;
    if peak < 0.1 {
        let gain = (0.3 / peak.max(1e-4)).clamp(1.0, 10.0);
        for sample in samples.iter_mut() {
            *sample = ((*sample as f32 * gain).clamp(-32767.0, 32767.0)) as i16;
        }
    }
}

// （原始音频落盘功能已按需求移除）

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

    fn start(
        &self,
        tx: SyncSender<AudioFrame>,
        _options: &CaptureOptions,
    ) -> Result<Box<dyn CaptureHandle>, AudioError> {
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
        let mut handle = capture.start(tx, &CaptureOptions::default()).unwrap();
        let frame = rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(frame.sample_rate, 16000);
        assert_eq!(frame.channels, 1);
        assert!(!frame.pcm_s16le.is_empty());
        handle.stop();
    }

    #[test]
    fn peak_is_computed_from_pcm() {
        let mut pcm = Vec::new();
        pcm.extend_from_slice(&1000i16.to_le_bytes());
        pcm.extend_from_slice(&i16::MAX.to_le_bytes());
        assert!((peak_of_pcm(&pcm) - 1.0).abs() < 0.001);
    }

    #[test]
    fn auto_gain_boosts_quiet_audio_only() {
        let mut quiet = vec![100i16, -100, 200, -200];
        apply_auto_gain(&mut quiet);
        assert!(quiet.iter().any(|s| s.unsigned_abs() > 1000));

        let mut loud = vec![20000i16, -20000];
        apply_auto_gain(&mut loud);
        assert_eq!(loud[0], 20000);
    }
}
