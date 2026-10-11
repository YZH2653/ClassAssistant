// FunASR 本地语音识别（WebSocket 2pass 实时转写）
use std::time::Duration;

use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

use super::*;
use crate::audio::AudioFrame;
use crate::domain::settings::{ProviderConfig, FUNASR_BASE_URL};

// 单帧音频字节数（60 毫秒 @ 16kHz 单声道 s16le）
const FRAME_BYTES: usize = 1920;
// 结束帧后等待尾部定稿的时长
const DRAIN_WAIT: Duration = Duration::from_secs(2);
// 单次发送超时（服务无响应时按连接故障处理）
const SEND_TIMEOUT: Duration = Duration::from_secs(5);
// 结束帧
const END_FRAME: &str = "{\"is_speaking\": false}";

// WebSocket 连接类型
type WsStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

// FunASR 语音识别 Provider
pub struct FunasrAsrProvider {
    pub config: ProviderConfig,
}

impl FunasrAsrProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self { config }
    }
}

// 服务地址：未填写时用本地默认地址
fn resolve_url(config: &ProviderConfig) -> String {
    let trimmed = config.base_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        FUNASR_BASE_URL.to_string()
    } else {
        trimmed.to_string()
    }
}

// 构造连接请求（with_subprotocol 控制是否声明 binary 子协议）
fn build_request(
    url: &str,
    with_subprotocol: bool,
) -> Result<tokio_tungstenite::tungstenite::http::Request<()>, AsrError> {
    if !(url.starts_with("ws://") || url.starts_with("wss://")) {
        return Err(AsrError(format!(
            "FunASR 服务地址无效：{url}（应以 ws:// 或 wss:// 开头）"
        )));
    }
    let mut request = url
        .into_client_request()
        .map_err(|e| AsrError(format!("FunASR 服务地址无效：{e}")))?;
    if with_subprotocol {
        request.headers_mut().insert(
            "Sec-WebSocket-Protocol",
            "binary"
                .parse()
                .map_err(|e: tokio_tungstenite::tungstenite::http::header::InvalidHeaderValue| {
                    AsrError(format!("FunASR 协议头无效：{e}"))
                })?,
        );
    }
    Ok(request)
}

// 是否子协议协商不匹配（tungstenite 握手内部校验回显，遇此换策略重连）
fn is_subprotocol_error(e: &tokio_tungstenite::tungstenite::Error) -> bool {
    matches!(
        e,
        tokio_tungstenite::tungstenite::Error::Protocol(
            tokio_tungstenite::tungstenite::error::ProtocolError::SecWebSocketSubProtocolError(_)
        )
    )
}

// 连接服务：先不声明子协议（RFC 6455 允许服务端不回显，FunASR 即如此）；
// 若对端回显了子协议（部署侧回显代理场景）则改带 binary 重连一次
async fn connect_ws(url: &str) -> Result<WsStream, AsrError> {
    let fail = |_| {
        AsrError("无法连接 FunASR 服务，请确认部署机已启动（D:\\FunASR\\start.bat）".to_string())
    };
    match connect_async(build_request(url, false)?).await {
        Ok((ws, _)) => Ok(ws),
        Err(e) if is_subprotocol_error(&e) => {
            let (ws, _) = connect_async(build_request(url, true)?).await.map_err(fail)?;
            Ok(ws)
        }
        Err(e) => Err(fail(e)),
    }
}

// 探测服务连通性（连上即关）
pub async fn probe(url: &str) -> Result<(), AsrError> {
    drop(connect_ws(url).await?);
    Ok(())
}

// 构造首帧（会话参数）
pub fn build_first_frame(wav_name: &str) -> String {
    serde_json::json!({
        "mode": "2pass",
        "chunk_size": [5, 10, 5],
        "chunk_interval": 10,
        "audio_fs": 16000,
        "wav_name": wav_name,
        "wav_format": "pcm",
        "is_speaking": true,
        "hotwords": "",
        "itn": true
    })
    .to_string()
}

// 草稿累积状态
pub struct DraftState {
    pub draft: String,
    pub sentence_start_ms: u64,
    pub pushed_ms: u64,
}

impl DraftState {
    pub fn new() -> Self {
        Self {
            draft: String::new(),
            sentence_start_ms: 0,
            pushed_ms: 0,
        }
    }
}

// 解析服务端消息并映射为事件
pub fn map_message(payload: &str, state: &mut DraftState) -> Option<AsrEvent> {
    let value: serde_json::Value = serde_json::from_str(payload).ok()?;
    let mode = value["mode"].as_str()?;
    let text = value["text"].as_str().unwrap_or_default();
    match mode {
        // 流式草稿：累积成当前句文本
        "2pass-online" => {
            // 新句第一条草稿到达时记句子起点
            if state.draft.is_empty() {
                state.sentence_start_ms = state.pushed_ms;
            }
            state.draft.push_str(text);
            Some(AsrEvent::Partial {
                text: state.draft.clone(),
                start_ms: state.sentence_start_ms,
            })
        }
        // 整句定稿：含标点，清空草稿（空定稿也要清）
        "2pass-offline" => {
            state.draft.clear();
            let start_ms = state.sentence_start_ms;
            let end_ms = state.pushed_ms.max(start_ms);
            if text.is_empty() {
                return None;
            }
            Some(AsrEvent::Segment {
                text: text.to_string(),
                start_ms,
                end_ms,
            })
        }
        _ => None,
    }
}

// 按帧长切出音频分片
pub fn split_frames(buffer: &mut Vec<u8>) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    while buffer.len() >= FRAME_BYTES {
        out.push(buffer.drain(..FRAME_BYTES).collect());
    }
    out
}

#[async_trait]
impl AsrProvider for FunasrAsrProvider {
    fn name(&self) -> &'static str {
        "funasr"
    }

    async fn start(&self, cfg: AsrConfig) -> Result<Box<dyn AsrStream>, AsrError> {
        let url = resolve_url(&self.config);
        let ws = connect_ws(&url).await?;

        let (chunk_tx, chunk_rx) = mpsc::channel(32);
        let (event_tx, event_rx) = mpsc::channel(64);
        tokio::spawn(async move {
            run_worker(ws, cfg.sample_rate, chunk_rx, event_tx).await;
        });
        Ok(Box::new(FunasrStream {
            chunk_tx,
            event_rx,
            buffer: Vec::new(),
        }))
    }
}

// FunASR 事件流
pub struct FunasrStream {
    chunk_tx: mpsc::Sender<ChunkMsg>,
    event_rx: mpsc::Receiver<AsrEvent>,
    buffer: Vec<u8>,
}

// 送服务端消息
enum ChunkMsg {
    Audio(Vec<u8>),
    Finish,
}

#[async_trait]
impl AsrStream for FunasrStream {
    async fn push_audio(&mut self, frame: AudioFrame) -> Result<(), AsrError> {
        self.buffer.extend_from_slice(&frame.pcm_s16le);
        for chunk in split_frames(&mut self.buffer) {
            // 非阻塞：队列满则丢帧，断开则报错
            match self.chunk_tx.try_send(ChunkMsg::Audio(chunk)) {
                Ok(()) | Err(mpsc::error::TrySendError::Full(_)) => {}
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    return Err(AsrError("FunASR 发送队列已关闭".to_string()))
                }
            }
        }
        Ok(())
    }

    async fn finish(&mut self) -> Result<(), AsrError> {
        if !self.buffer.is_empty() {
            let rest = std::mem::take(&mut self.buffer);
            let _ = self.chunk_tx.try_send(ChunkMsg::Audio(rest));
        }
        tokio::time::timeout(SEND_TIMEOUT, self.chunk_tx.send(ChunkMsg::Finish))
            .await
            .map_err(|_| AsrError("FunASR 服务无响应，请检查服务是否正常".to_string()))?
            .map_err(|_| AsrError("FunASR 发送队列已关闭".to_string()))
    }

    async fn next_event(&mut self) -> Option<AsrEvent> {
        self.event_rx.recv().await
    }
}

// 发送消息（超时视为连接故障）
async fn send_msg<S>(sink: &mut S, message: Message) -> bool
where
    S: futures_util::Sink<Message> + Unpin,
{
    matches!(
        tokio::time::timeout(SEND_TIMEOUT, sink.send(message)).await,
        Ok(Ok(()))
    )
}

// 收发与事件回传
async fn run_worker(
    ws: WsStream,
    sample_rate: u32,
    mut chunk_rx: mpsc::Receiver<ChunkMsg>,
    event_tx: mpsc::Sender<AsrEvent>,
) {
    let (mut sink, mut stream) = ws.split();
    let wav_name = format!(
        "classassistant-{}",
        chrono::Local::now().format("%Y%m%d-%H%M%S")
    );
    if !send_msg(&mut sink, Message::Text(build_first_frame(&wav_name).into())).await {
        let _ = event_tx
            .send(AsrEvent::Error {
                message: "FunASR 会话建立失败".to_string(),
            })
            .await;
        let _ = event_tx.send(AsrEvent::Ended).await;
        return;
    }

    let mut state = DraftState::new();
    let bytes_per_ms = (sample_rate as u64 * 2 / 1000).max(1);
    // 是否异常结束（非正常收尾）
    let mut abnormal = false;

    loop {
        tokio::select! {
            msg = chunk_rx.recv() => {
                match msg {
                    Some(ChunkMsg::Audio(bytes)) => {
                        state.pushed_ms += bytes.len() as u64 / bytes_per_ms;
                        if !send_msg(&mut sink, Message::Binary(bytes.into())).await {
                            abnormal = true;
                            break;
                        }
                    }
                    _ => {
                        // 发结束帧并等尾部定稿
                        let _ = send_msg(&mut sink, Message::Text(END_FRAME.into())).await;
                        let deadline = tokio::time::Instant::now() + DRAIN_WAIT;
                        loop {
                            let now = tokio::time::Instant::now();
                            if now >= deadline {
                                break;
                            }
                            match tokio::time::timeout_at(deadline, stream.next()).await {
                                Ok(Some(Ok(message))) => {
                                    forward(message, &mut state, &event_tx).await;
                                }
                                _ => break,
                            }
                        }
                        break;
                    }
                }
            }
            incoming = stream.next() => {
                match incoming {
                    Some(Ok(message)) => forward(message, &mut state, &event_tx).await,
                    // 读失败或服务端提前关闭都算异常中断
                    _ => {
                        abnormal = true;
                        break;
                    }
                }
            }
        }
    }

    if abnormal {
        let _ = event_tx
            .send(AsrEvent::Error {
                message: "FunASR 连接异常中断，转写内容已保留".to_string(),
            })
            .await;
    }
    let _ = event_tx.send(AsrEvent::Ended).await;
}

// 转发服务端文本消息
async fn forward(message: Message, state: &mut DraftState, event_tx: &mpsc::Sender<AsrEvent>) {
    if let Message::Text(text) = message {
        if let Some(event) = map_message(text.as_ref(), state) {
            let _ = event_tx.send(event).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_frame_carries_required_fields() {
        let body = build_first_frame("classassistant-test");
        let value: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(value["mode"], "2pass");
        assert_eq!(value["chunk_size"], serde_json::json!([5, 10, 5]));
        assert_eq!(value["chunk_interval"], 10);
        assert_eq!(value["audio_fs"], 16000);
        assert_eq!(value["wav_format"], "pcm");
        assert_eq!(value["is_speaking"], true);
        assert_eq!(value["hotwords"], "");
        assert_eq!(value["itn"], true);
        assert_eq!(value["wav_name"], "classassistant-test");
    }

    #[test]
    fn online_messages_accumulate_draft() {
        let mut state = DraftState::new();
        state.pushed_ms = 1000;
        let first = map_message(r#"{"mode":"2pass-online","text":"今天"}"#, &mut state).unwrap();
        match first {
            AsrEvent::Partial {
                text, start_ms, ..
            } => {
                assert_eq!(text, "今天");
                // 句子起点取首条草稿到达时的推流时长
                assert_eq!(start_ms, 1000);
            }
            _ => panic!("应为 Partial"),
        }
        state.pushed_ms = 2000;
        let second = map_message(r#"{"mode":"2pass-online","text":"我们"}"#, &mut state).unwrap();
        match second {
            AsrEvent::Partial {
                text, start_ms, ..
            } => {
                assert_eq!(text, "今天我们");
                // 起点不随后续草稿变化
                assert_eq!(start_ms, 1000);
            }
            _ => panic!("应为 Partial"),
        }
    }

    #[test]
    fn empty_offline_still_clears_draft() {
        let mut state = DraftState::new();
        map_message(r#"{"mode":"2pass-online","text":"今天"}"#, &mut state);
        assert!(map_message(r#"{"mode":"2pass-offline","text":""}"#, &mut state).is_none());
        assert!(state.draft.is_empty());
    }

    #[test]
    fn offline_message_finalizes_sentence() {
        let mut state = DraftState::new();
        state.pushed_ms = 5000;
        map_message(r#"{"mode":"2pass-online","text":"今天"}"#, &mut state);
        state.pushed_ms = 8000;
        let event = map_message(
            r#"{"mode":"2pass-offline","text":"今天我们学习了函数。"}"#,
            &mut state,
        )
        .unwrap();
        match event {
            AsrEvent::Segment {
                text,
                start_ms,
                end_ms,
            } => {
                assert_eq!(text, "今天我们学习了函数。");
                assert_eq!(start_ms, 5000);
                assert_eq!(end_ms, 8000);
            }
            _ => panic!("应为 Segment"),
        }
        assert!(state.draft.is_empty());
    }

    #[test]
    fn frames_are_split_by_fixed_size() {
        let mut buffer = vec![0u8; FRAME_BYTES * 2 + 10];
        let frames = split_frames(&mut buffer);
        assert_eq!(frames.len(), 2);
        assert!(frames.iter().all(|f| f.len() == FRAME_BYTES));
        assert_eq!(buffer.len(), 10);
    }

    #[test]
    fn url_defaults_to_local_service() {
        let config = ProviderConfig {
            base_url: String::new(),
            ..ProviderConfig::default()
        };
        assert_eq!(resolve_url(&config), FUNASR_BASE_URL);
        let custom = ProviderConfig {
            base_url: "ws://192.168.1.8:10096/".to_string(),
            ..ProviderConfig::default()
        };
        assert_eq!(resolve_url(&custom), "ws://192.168.1.8:10096");
    }

    #[test]
    fn subprotocol_header_is_optional() {
        let plain = build_request("ws://localhost:10096", false).unwrap();
        assert!(plain.headers().get("Sec-WebSocket-Protocol").is_none());
        let with = build_request("ws://localhost:10096", true).unwrap();
        assert_eq!(
            with.headers().get("Sec-WebSocket-Protocol").unwrap(),
            "binary"
        );
    }

    // 起一个最小假 WebSocket 服务端：完成101握手即挂起（echo_subprotocol 控制是否回显 binary），返回 ws URL
    async fn spawn_fake_ws_server(echo_subprotocol: bool) -> String {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("ws://127.0.0.1:{}", listener.local_addr().unwrap().port());
        tokio::spawn(async move {
            loop {
                let Ok((mut sock, _)) = listener.accept().await else {
                    break;
                };
                tokio::spawn(async move {
                    let mut buf = Vec::new();
                    let mut tmp = [0u8; 1024];
                    // 读到请求头结束
                    loop {
                        let Ok(n) = sock.read(&mut tmp).await else {
                            return;
                        };
                        if n == 0 {
                            return;
                        }
                        buf.extend_from_slice(&tmp[..n]);
                        if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                            break;
                        }
                    }
                    let text = String::from_utf8_lossy(&buf);
                    let key = text
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.trim()
                                .eq_ignore_ascii_case("sec-websocket-key")
                                .then(|| value.trim().to_string())
                        })
                        .expect("请求应带 Sec-WebSocket-Key");
                    let accept = tokio_tungstenite::tungstenite::handshake::derive_accept_key(
                        key.as_bytes(),
                    );
                    let mut resp = format!(
                        "HTTP/1.1 101 Switching Protocols\r\n\
                         Upgrade: websocket\r\n\
                         Connection: Upgrade\r\n\
                         Sec-WebSocket-Accept: {accept}\r\n"
                    );
                    if echo_subprotocol {
                        resp.push_str("Sec-WebSocket-Protocol: binary\r\n");
                    }
                    resp.push_str("\r\n");
                    let _ = sock.write_all(resp.as_bytes()).await;
                    // 保持连接一段时间供客户端完成握手
                    tokio::time::sleep(Duration::from_millis(300)).await;
                });
            }
        });
        url
    }

    #[tokio::test]
    async fn connects_when_server_omits_subprotocol_echo() {
        let url = spawn_fake_ws_server(false).await;
        probe(&url).await.expect("无子协议回显也应连通（RFC 允许）");
    }

    #[tokio::test]
    async fn connects_when_server_echoes_subprotocol() {
        let url = spawn_fake_ws_server(true).await;
        probe(&url).await.expect("回显代理场景应自动重连连通");
    }
}
