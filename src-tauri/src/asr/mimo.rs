// 小米 MiMo v2.5 ASR（OpenAI 兼容：chat/completions + input_audio，流式返回文本）
use std::time::Duration;

use async_trait::async_trait;
use base64::Engine;
use futures_util::StreamExt;
use tokio::sync::mpsc;

use super::*;
use crate::asr::wav::pcm_s16le_to_wav;
use crate::audio::AudioFrame;
use crate::domain::settings::ProviderConfig;

// 单片时长（毫秒）：该接口按片提交音频，片长即文字延迟
const CHUNK_MS: u64 = 8000;
// 单片 PCM 上限（Base64 后远低于 10MB 上限）
const MAX_CHUNK_BYTES: usize = 5 * 1024 * 1024;

// MiMo ASR Provider
pub struct MimoAsrProvider {
    pub config: ProviderConfig,
}

impl MimoAsrProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self { config }
    }
}

// 分片消息
enum ChunkMsg {
    Audio { start_ms: u64, pcm: Vec<u8> },
    Finish,
}

pub struct MimoAsrStream {
    chunk_tx: mpsc::Sender<ChunkMsg>,
    event_rx: mpsc::Receiver<AsrEvent>,
    buffer: Vec<u8>,
    sent_ms: u64,
    sample_rate: u32,
    channels: u16,
}

impl MimoAsrStream {
    fn chunk_bytes(&self) -> usize {
        let by_time = self.sample_rate as usize * self.channels as usize * 2 * CHUNK_MS as usize
            / 1000;
        by_time.min(MAX_CHUNK_BYTES).max(1)
    }

    fn pcm_duration_ms(&self, pcm: &[u8]) -> u64 {
        let denom = self.sample_rate as u64 * self.channels as u64 * 2;
        if denom == 0 {
            0
        } else {
            pcm.len() as u64 * 1000 / denom
        }
    }

    async fn flush_chunk(&mut self) -> Result<(), AsrError> {
        let pcm = std::mem::take(&mut self.buffer);
        let start_ms = self.sent_ms;
        self.sent_ms = start_ms + self.pcm_duration_ms(&pcm);
        self.chunk_tx
            .send(ChunkMsg::Audio { start_ms, pcm })
            .await
            .map_err(|_| AsrError("ASR 通道已关闭".to_string()))
    }
}

#[async_trait]
impl AsrStream for MimoAsrStream {
    async fn push_audio(&mut self, frame: AudioFrame) -> Result<(), AsrError> {
        self.buffer.extend_from_slice(&frame.pcm_s16le);
        if self.buffer.len() >= self.chunk_bytes() {
            self.flush_chunk().await?;
        }
        Ok(())
    }

    async fn finish(&mut self) -> Result<(), AsrError> {
        if !self.buffer.is_empty() {
            self.flush_chunk().await?;
        }
        self.chunk_tx
            .send(ChunkMsg::Finish)
            .await
            .map_err(|_| AsrError("ASR 通道已关闭".to_string()))
    }

    async fn next_event(&mut self) -> Option<AsrEvent> {
        self.event_rx.recv().await
    }
}

#[async_trait]
impl AsrProvider for MimoAsrProvider {
    fn name(&self) -> &'static str {
        "mimo"
    }

    async fn start(&self, cfg: AsrConfig) -> Result<Box<dyn AsrStream>, AsrError> {
        if self.config.api_key.trim().is_empty() {
            return Err(AsrError("请先在设置页填写 MiMo API Key".to_string()));
        }
        let (chunk_tx, chunk_rx) = mpsc::channel(8);
        let (event_tx, event_rx) = mpsc::channel(64);
        let config = self.config.clone();
        tokio::spawn(async move {
            run_worker(config, cfg.sample_rate, cfg.channels, chunk_rx, event_tx).await;
        });
        Ok(Box::new(MimoAsrStream {
            chunk_tx,
            event_rx,
            buffer: Vec::new(),
            sent_ms: 0,
            sample_rate: cfg.sample_rate,
            channels: cfg.channels,
        }))
    }
}

// 分片处理：按提交顺序转写，保证文稿顺序
async fn run_worker(
    config: ProviderConfig,
    sample_rate: u32,
    channels: u16,
    mut chunk_rx: mpsc::Receiver<ChunkMsg>,
    event_tx: mpsc::Sender<AsrEvent>,
) {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .unwrap_or_default();
    let base_url = config.effective_base_url();

    while let Some(msg) = chunk_rx.recv().await {
        match msg {
            ChunkMsg::Audio { start_ms, pcm } => {
                let duration_ms = pcm_duration_ms(&pcm, sample_rate, channels);
                let end_ms = start_ms + duration_ms;
                let result = transcribe(
                    &client,
                    &base_url,
                    &config,
                    &pcm,
                    sample_rate,
                    channels,
                    start_ms,
                    &event_tx,
                )
                .await;
                match result {
                    Ok(text) => {
                        emit_segments(&text, start_ms, end_ms, &event_tx).await;
                    }
                    Err(err) => {
                        let _ = event_tx.send(AsrEvent::Error { message: err.0 }).await;
                    }
                }
            }
            ChunkMsg::Finish => break,
        }
    }
    let _ = event_tx.send(AsrEvent::Ended).await;
}

// 提交单片音频并流式接收文本
#[allow(clippy::too_many_arguments)]
async fn transcribe(
    client: &reqwest::Client,
    base_url: &str,
    config: &ProviderConfig,
    pcm: &[u8],
    sample_rate: u32,
    channels: u16,
    start_ms: u64,
    event_tx: &mpsc::Sender<AsrEvent>,
) -> Result<String, AsrError> {
    let wav = pcm_s16le_to_wav(pcm, sample_rate, channels);
    let data_uri = format!(
        "data:audio/wav;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(wav)
    );
    let body = build_asr_body(&config.model, &data_uri, "auto");
    let response = client
        .post(format!("{base_url}/chat/completions"))
        .bearer_auth(config.api_key.trim())
        .json(&body)
        .send()
        .await
        .map_err(|e| AsrError(format!("请求失败：{e}")))?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(AsrError(format!("接口返回 {status}：{}", truncate(&text, 200))));
    }

    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut full = String::new();
    while let Some(item) = stream.next().await {
        let chunk = item.map_err(|e| AsrError(format!("读取响应失败：{e}")))?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(pos) = buffer.find('\n') {
            let line: String = buffer[..pos].trim().to_string();
            buffer = buffer[pos + 1..].to_string();
            if let Some(text) = parse_sse_delta(&line) {
                full.push_str(&text);
                let _ = event_tx
                    .send(AsrEvent::Partial {
                        text: full.clone(),
                        start_ms,
                    })
                    .await;
            }
        }
    }
    Ok(full.trim().to_string())
}

// 按句拆分后产出定稿段落
async fn emit_segments(text: &str, start_ms: u64, end_ms: u64, event_tx: &mpsc::Sender<AsrEvent>) {
    let sentences = split_sentences(text);
    if sentences.is_empty() {
        return;
    }
    let total_chars: u64 = sentences
        .iter()
        .map(|s| s.chars().count() as u64)
        .sum::<u64>()
        .max(1);
    let span = end_ms.saturating_sub(start_ms).max(1);
    let mut cursor = start_ms;
    for sentence in sentences {
        let chars = sentence.chars().count() as u64;
        let seg_end = (cursor + span * chars / total_chars).min(end_ms).max(cursor + 1);
        let _ = event_tx
            .send(AsrEvent::Segment {
                text: sentence,
                start_ms: cursor,
                end_ms: seg_end,
            })
            .await;
        cursor = seg_end;
    }
}

// 构造 ASR 请求体
pub fn build_asr_body(model: &str, data_uri: &str, language: &str) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "messages": [{
            "role": "user",
            "content": [{
                "type": "input_audio",
                "input_audio": { "data": data_uri }
            }]
        }],
        "asr_options": { "language": language },
        "stream": true
    })
}

// 从 SSE 行解析增量文本
pub fn parse_sse_delta(line: &str) -> Option<String> {
    let data = line.strip_prefix("data:")?.trim();
    if data == "[DONE]" {
        return None;
    }
    let value: serde_json::Value = serde_json::from_str(data).ok()?;
    let text = value["choices"][0]["delta"]["content"].as_str()?;
    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

// 按中英文标点拆句
pub fn split_sentences(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        current.push(ch);
        if matches!(ch, '。' | '！' | '？' | '；' | '!' | '?' | ';' | '\n') {
            let trimmed = current.trim().to_string();
            if !trimmed.is_empty() {
                out.push(trimmed);
            }
            current = String::new();
        }
    }
    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        out.push(trimmed);
    }
    out
}

// 补全并规整接口地址（保留给探测命令使用）
pub fn normalize_base_url(base_url: &str) -> String {
    let trimmed = base_url.trim().trim_end_matches('/');
    trimmed.to_string()
}

fn pcm_duration_ms(pcm: &[u8], sample_rate: u32, channels: u16) -> u64 {
    let denom = sample_rate as u64 * channels as u64 * 2;
    if denom == 0 {
        0
    } else {
        pcm.len() as u64 * 1000 / denom
    }
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_string()
    } else {
        let cut: String = text.chars().take(max).collect();
        format!("{cut}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asr_body_contains_audio_and_options() {
        let body = build_asr_body("mimo-v2.5-asr", "data:audio/wav;base64,AAAA", "auto");
        assert_eq!(body["model"], "mimo-v2.5-asr");
        assert_eq!(body["stream"], true);
        assert_eq!(body["asr_options"]["language"], "auto");
        assert_eq!(
            body["messages"][0]["content"][0]["input_audio"]["data"],
            "data:audio/wav;base64,AAAA"
        );
    }

    #[test]
    fn sse_delta_parsed() {
        let line = r#"data: {"choices":[{"delta":{"content":"同学们好"}}]}"#;
        assert_eq!(parse_sse_delta(line).unwrap(), "同学们好");
        assert!(parse_sse_delta("data: [DONE]").is_none());
        assert!(parse_sse_delta(": keep-alive").is_none());
    }

    #[test]
    fn sentences_split_by_punctuation() {
        let parts = split_sentences("今天我们讲顺序表。它支持随机访问！好");
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], "今天我们讲顺序表。");
        assert_eq!(parts[2], "好");
    }

    #[test]
    fn base_url_trailing_slash_trimmed() {
        assert_eq!(
            normalize_base_url("https://api.xiaomimimo.com/v1/"),
            "https://api.xiaomimimo.com/v1"
        );
        assert_eq!(normalize_base_url(""), "");
    }
}
