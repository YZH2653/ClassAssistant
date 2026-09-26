// 真实麦克风采集（TODO：接入 cpal / WASAPI）
use super::{AudioCapture, AudioError, AudioFrame, CaptureHandle};
use std::sync::mpsc::SyncSender;

pub struct CpalCapture;

impl AudioCapture for CpalCapture {
    fn name(&self) -> &'static str {
        "cpal"
    }

    fn start(&self, _tx: SyncSender<AudioFrame>) -> Result<Box<dyn CaptureHandle>, AudioError> {
        Err(AudioError("cpal 采集尚未接入".to_string()))
    }
}
