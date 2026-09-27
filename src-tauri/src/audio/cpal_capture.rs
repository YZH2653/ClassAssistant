// 真实麦克风采集（cpal，Windows 走 WASAPI）
use std::sync::mpsc::{self as std_mpsc, SyncSender};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};

use super::{apply_auto_gain, AudioCapture, AudioError, AudioFrame, CaptureHandle, CaptureOptions};

// 目标采样率（下游 ASR 约定 16kHz 单声道 s16le）
const TARGET_RATE: u32 = 16000;

pub struct CpalCapture;

// 采集句柄：音频线程持有流，本句柄只发停止信号
struct CpalHandle {
    stop_tx: Option<std_mpsc::Sender<()>>,
}

impl CaptureHandle for CpalHandle {
    fn stop(&mut self) {
        if let Some(stop_tx) = self.stop_tx.take() {
            let _ = stop_tx.send(());
        }
    }
}

impl AudioCapture for CpalCapture {
    fn name(&self) -> &'static str {
        "cpal"
    }

    fn start(
        &self,
        tx: SyncSender<AudioFrame>,
        options: &CaptureOptions,
    ) -> Result<Box<dyn CaptureHandle>, AudioError> {
        let options = options.clone();
        let (ready_tx, ready_rx) = std_mpsc::channel();
        let (stop_tx, stop_rx) = std_mpsc::channel();
        std::thread::spawn(move || run_capture(tx, stop_rx, ready_tx, options));
        match ready_rx.recv() {
            Ok(Ok(())) => Ok(Box::new(CpalHandle {
                stop_tx: Some(stop_tx),
            })),
            Ok(Err(err)) => Err(err),
            Err(_) => Err(AudioError("音频线程异常退出".to_string())),
        }
    }
}

// 专用线程：建流（Stream 不跨线程）→ 通知就绪 → 等停止
fn run_capture(
    tx: SyncSender<AudioFrame>,
    stop_rx: std_mpsc::Receiver<()>,
    ready_tx: std_mpsc::Sender<Result<(), AudioError>>,
    options: CaptureOptions,
) {
    match build_stream(tx, &options) {
        Ok(stream) => {
            if let Err(err) = stream.play() {
                let _ = ready_tx.send(Err(AudioError(format!("启动麦克风失败：{err}"))));
                return;
            }
            let _ = ready_tx.send(Ok(()));
            let _ = stop_rx.recv();
            let _ = stream.pause();
        }
        Err(err) => {
            let _ = ready_tx.send(Err(err));
        }
    }
}

// 列出可用输入设备名
pub fn list_input_devices() -> Vec<String> {
    let host = cpal::default_host();
    host.input_devices()
        .map(|devices| {
            devices
                .filter_map(|device| device.name().ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

// 选输入设备：按名字找，找不到/未指定用系统默认
fn pick_device(name: Option<&str>) -> Result<cpal::Device, AudioError> {
    let host = cpal::default_host();
    if let Some(wanted) = name.map(str::trim).filter(|n| !n.is_empty()) {
        if let Ok(devices) = host.input_devices() {
            for device in devices {
                if device.name().map(|n| n == wanted).unwrap_or(false) {
                    return Ok(device);
                }
            }
        }
        return Err(AudioError(format!("找不到输入设备：{wanted}")));
    }
    host.default_input_device()
        .ok_or_else(|| AudioError("找不到麦克风设备，请检查系统声音设置".to_string()))
}

// 按设备默认配置建流，回调里转成 16kHz 单声道 s16le
fn build_stream(
    tx: SyncSender<AudioFrame>,
    options: &CaptureOptions,
) -> Result<cpal::Stream, AudioError> {
    let device = pick_device(options.device_name.as_deref())?;
    let supported = device
        .default_input_config()
        .map_err(|e| AudioError(format!("读取麦克风配置失败：{e}")))?;
    let input_rate = supported.sample_rate().0;
    let channels = supported.channels() as usize;
    let config: StreamConfig = supported.config();
    let auto_gain = options.auto_gain;
    let err_fn = |err: cpal::StreamError| eprintln!("audio stream error: {err}");

    let format = supported.sample_format();
    if !matches!(
        format,
        SampleFormat::F32
            | SampleFormat::I16
            | SampleFormat::U16
            | SampleFormat::I32
            | SampleFormat::U32
            | SampleFormat::F64
    ) {
        return Err(AudioError(format!("暂不支持的音频格式：{format:?}")));
    }

    let mut tick: u64 = 0;
    let built = match supported.sample_format() {
        SampleFormat::F32 => device
            .build_input_stream(
                &config,
                move |data: &[f32], _| {
                    let mut samples = to_mono_s16(data, channels);
                    push_frame(&tx, &mut samples, input_rate, &mut tick, auto_gain);
                },
                err_fn,
                None,
            ),
        SampleFormat::I16 => device
            .build_input_stream(
                &config,
                move |data: &[i16], _| {
                    let floats: Vec<f32> =
                        data.iter().map(|s| *s as f32 / i16::MAX as f32).collect();
                    let mut samples = to_mono_s16(&floats, channels);
                    push_frame(&tx, &mut samples, input_rate, &mut tick, auto_gain);
                },
                err_fn,
                None,
            ),
        SampleFormat::U16 => device
            .build_input_stream(
                &config,
                move |data: &[u16], _| {
                    let floats: Vec<f32> =
                        data.iter().map(|s| (*s as f32 - 32768.0) / 32768.0).collect();
                    let mut samples = to_mono_s16(&floats, channels);
                    push_frame(&tx, &mut samples, input_rate, &mut tick, auto_gain);
                },
                err_fn,
                None,
            ),
        SampleFormat::I32 => device
            .build_input_stream(
                &config,
                move |data: &[i32], _| {
                    let floats: Vec<f32> =
                        data.iter().map(|s| *s as f32 / i32::MAX as f32).collect();
                    let mut samples = to_mono_s16(&floats, channels);
                    push_frame(&tx, &mut samples, input_rate, &mut tick, auto_gain);
                },
                err_fn,
                None,
            ),
        SampleFormat::U32 => device
            .build_input_stream(
                &config,
                move |data: &[u32], _| {
                    let floats: Vec<f32> = data
                        .iter()
                        .map(|s| (*s as f64 / u32::MAX as f64 * 2.0 - 1.0) as f32)
                        .collect();
                    let mut samples = to_mono_s16(&floats, channels);
                    push_frame(&tx, &mut samples, input_rate, &mut tick, auto_gain);
                },
                err_fn,
                None,
            ),
        SampleFormat::F64 => device
            .build_input_stream(
                &config,
                move |data: &[f64], _| {
                    let floats: Vec<f32> = data.iter().map(|s| *s as f32).collect();
                    let mut samples = to_mono_s16(&floats, channels);
                    push_frame(&tx, &mut samples, input_rate, &mut tick, auto_gain);
                },
                err_fn,
                None,
            ),
        other => unreachable!("已在前面过滤不支持的格式：{other:?}"),
    };
    built.map_err(|e| AudioError(format!("打开麦克风失败：{e}")))
}

// 送一帧（队列满则丢帧，不阻塞音频线程）
fn push_frame(
    tx: &SyncSender<AudioFrame>,
    samples: &mut Vec<i16>,
    input_rate: u32,
    tick: &mut u64,
    auto_gain: bool,
) {
    if auto_gain {
        apply_auto_gain(samples);
    }
    let output = resample_linear(samples, input_rate, TARGET_RATE);
    if output.is_empty() {
        return;
    }
    let mut pcm = Vec::with_capacity(output.len() * 2);
    let duration_ms = output.len() as u64 * 1000 / TARGET_RATE as u64;
    for sample in &output {
        pcm.extend_from_slice(&sample.to_le_bytes());
    }
    let frame = AudioFrame {
        pcm_s16le: pcm,
        sample_rate: TARGET_RATE,
        channels: 1,
        captured_at_ms: *tick,
    };
    *tick += duration_ms;
    let _ = tx.try_send(frame);
}

// 多声道 f32 → 单声道 s16
pub fn to_mono_s16(samples: &[f32], channels: usize) -> Vec<i16> {
    if channels == 0 {
        return Vec::new();
    }
    samples
        .chunks(channels)
        .map(|frame| {
            let avg = frame.iter().sum::<f32>() / frame.len() as f32;
            (avg.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
        })
        .collect()
}

// 线性重采样
pub fn resample_linear(input: &[i16], from_rate: u32, to_rate: u32) -> Vec<i16> {
    if input.is_empty() || from_rate == 0 || to_rate == 0 {
        return Vec::new();
    }
    if from_rate == to_rate {
        return input.to_vec();
    }
    let out_len = (input.len() as u64 * to_rate as u64 / from_rate as u64).max(1) as usize;
    let step = from_rate as f64 / to_rate as f64;
    let mut out = Vec::with_capacity(out_len);
    for index in 0..out_len {
        let pos = index as f64 * step;
        let low = pos.floor() as usize;
        let frac = pos - low as f64;
        let first = input.get(low).copied().unwrap_or(0) as f64;
        let second = input.get(low + 1).copied().unwrap_or(first as i16) as f64;
        out.push((first + (second - first) * frac).round() as i16);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stereo_is_averaged_to_mono() {
        let samples = [0.5f32, -0.5, 1.0, 1.0];
        let mono = to_mono_s16(&samples, 2);
        assert_eq!(mono.len(), 2);
        assert_eq!(mono[0], 0);
        assert_eq!(mono[1], i16::MAX);
    }

    #[test]
    fn resample_keeps_length_ratio() {
        let input: Vec<i16> = (0..480).map(|i| (i % 100) as i16).collect();
        let out = resample_linear(&input, 48000, 16000);
        assert_eq!(out.len(), 160);
    }

    #[test]
    fn resample_is_identity_for_same_rate() {
        let input = vec![1i16, 2, 3];
        assert_eq!(resample_linear(&input, 16000, 16000), input);
    }

    #[test]
    fn mono_conversion_handles_zero_channels() {
        assert!(to_mono_s16(&[0.1], 0).is_empty());
    }
}
