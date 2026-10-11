# ClassAssistant × FunASR 对接文档

> **读者**：ClassAssistant 开发机上的 AI 助手（mimo v2.6 pro）及涵涵。
> **目的**：在 ClassAssistant 中新增「FunASR 本地」语音识别 Provider，对接部署机上已跑通的 FunASR 2pass 实时转写服务。
> **本文自包含**——无需再查阅任何 FunASR 外部资料。协议细节以官方测试客户端 `funasr_wss_client.py` 源码为准（关键代码摘录于 B2）。架构细节已对照 ClassAssistant 源码核实，可直接照此改码。

---

## Part A 部署情况（部署机已完成）

### A1. 服务形态

| 项 | 值 |
|---|---|
| 镜像 | `registry.cn-hangzhou.aliyuncs.com/funasr_repo/funasr:funasr-runtime-sdk-online-cpu-0.1.13` |
| 模式 | **2pass**（流式草稿 + 句尾离线精修 + 标点 + ITN） |
| 运行方式 | Docker Desktop（WSL2），`D:\FunASR\docker-compose.yml` |
| 端口 | 容器 10095 → 宿主机 **10096**，**无 SSL**（`--certfile 0`） |
| 并发线程 | `--decoder-thread-num 8` |
| 部署机 | Windows 11，18 逻辑核，无 GPU（CPU 推理） |

### A2. 对接地址

| 场景 | 地址 |
|---|---|
| 应用与服务同机 | `ws://localhost:10096` |
| 开发机跨机联调 | `ws://100.76.245.236:10096`（部署机 WLAN IPv4，以部署机 `ipconfig` 实时输出为准） |

跨机联调需在部署机放行防火墙（管理员 PowerShell）：

```powershell
netsh advfirewall firewall add rule name="FunASR 10096" dir=in action=allow protocol=TCP localport=10096
```

### A3. 模型清单（ModelScope ID）

| 用途 | Model ID |
|---|---|
| VAD 语音端点 | `damo/speech_fsmn_vad_zh-cn-16k-common-onnx` |
| 离线精修 ASR（句尾定稿） | `damo/speech_paraformer-large-vad-punc_asr_nat-zh-cn-16k-common-vocab8404-onnx` |
| 流式 ASR（实时草稿） | `damo/speech_paraformer-large_asr_nat-zh-cn-16k-common-vocab8404-online-onnx` |
| 标点预测 | `damo/punc_ct-transformer_zh-cn-common-vad_realtime-vocab272727-onnx` |
| Ngram 语言模型 | `damo/speech_ngram_lm_zh-cn-ai-wesp-fst` |
| ITN 数字规范化 | `thuduj12/fst_itn_zh` |

**热词**：部署机 `D:\FunASR\models\hotwords.txt`，每行 `热词 权重`（权重 1–100，热词≤10 字，≤1000 个）。服务端全局生效；客户端首帧 `hotwords` 会与之合并。编辑后重启服务生效。

### A4. 启停与运维

| 操作 | 命令 |
|---|---|
| 启动 | `D:\FunASR\start.bat`（或 `docker compose up -d`） |
| 停止 | `D:\FunASR\stop.bat` |
| 日志 | `docker compose logs -f`（工作目录 D:\FunASR） |

### A5. 实测验证输出（2026-10-11 部署机实测）

测试命令：
```bash
cd D:\FunASR\samples
python python\funasr_wss_client.py --host 127.0.0.1 --port 10096 --mode 2pass --ssl 0 --audio_in audio\asr_example.wav --output_dir out
```

实测结果（`out/text.0_0`，UTF-8）——注意最后一行整句定稿带标点和字级时间戳：

```
demo	欢迎大
demo	家来
demo	体验达
demo	摩院推
demo	出的语
demo	音识
demo	别模
demo	欢迎大家来体验达摩院推出的语音识别模型。	[[880,1120],[1120,1380],[1380,1540],[1540,1780],[1780,2020],[2020,2180],[2180,2480],[2480,2600],[2600,2780],[2780,3040],[3040,3240],[3240,3479],[3479,3699],[3699,3899],[3899,4180],[4180,4420],[4420,4620],[4620,4780],[4780,5195]]
```

形态解读（与 B2/B3 语义吻合）：
- 前 7 行是 `2pass-online` 的**增量草稿片段**（客户端累加显示为「欢迎大家来体验达摩院推出的语音识别模型」的进行中草稿）
- 最后一行是 `2pass-offline` 的**整句定稿**：`欢迎大家来体验达摩院推出的语音识别模型。`——含标点，且带 `timestamp` 字符串（字级 ms 时间戳对，可忽略或用于精确计时）

另注：官方客户端 `funasr_wss_client.py` 在 Python 3.12+ 上需小补丁（`asyncio.get_event_loop()` → `asyncio.new_event_loop()`），部署机 samples 里已改好。

---

## Part B 对接指南（ClassAssistant 改码任务书）

### B1. 现有架构（已对照源码核实）

ASR 层已有 Provider 抽象，**新增 FunASR 只需加一个实现，不改 Mock/Mimo**。`src-tauri/src/asr/mod.rs` 真实签名（注意 `#[async_trait]` 与 supertrait）：

```rust
// ASR 事件（serde tag="type"，snake_case）
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AsrEvent {
    Partial { text: String, start_ms: u64 },                    // 实时草稿
    Segment { text: String, start_ms: u64, end_ms: u64 },       // 定稿
    Error { message: String },
    Ended,
}

pub struct AsrConfig { pub sample_rate: u32, pub channels: u16, pub language: String }
// 实际在 session/manager.rs 中固定构造为 16000 / 1 / "zh-CN"

// 错误是元组结构体
pub struct AsrError(pub String);

// 流式会话
#[async_trait]
pub trait AsrStream: Send {
    async fn push_audio(&mut self, frame: AudioFrame) -> Result<(), AsrError>;
    async fn finish(&mut self) -> Result<(), AsrError>;
    async fn next_event(&mut self) -> Option<AsrEvent>;
}

// Provider（注意 Send + Sync）
#[async_trait]
pub trait AsrProvider: Send + Sync {
    fn name(&self) -> &'static str;
    async fn start(&self, cfg: AsrConfig) -> Result<Box<dyn AsrStream>, AsrError>;
}
```

其他关键点（均已在源码核实）：

| 文件 | 现状 | 对接相关 |
|---|---|---|
| `src-tauri/src/asr/mimo.rs` | MiMo 云端 ASR（8 秒分片） | **参考其 worker + mpsc 通道结构**；不改 |
| `src-tauri/src/audio/mod.rs` | `AudioFrame { pcm_s16le: Vec<u8>, sample_rate: u32, channels: u16, captured_at_ms: u64 }` | `pcm_s16le` 已是 **16kHz 单声道 s16le**（`captured_at_ms` 为采集累计时钟），**零转换** |
| `src-tauri/src/audio/cpal_capture.rs` | `TARGET_RATE=16000`；默认开 `auto_gain`（只升不降）；队列 `try_send` 满即丢帧 | `push_audio` 须非阻塞、容忍丢帧 |
| `src-tauri/src/session/manager.rs` | `run_pipeline` 以 5ms 超时轮询 `next_event`；麦克风静音时发合成 `AsrEvent::Error「未检测到麦克风声音…」` | FunASR 流收到 Error 不要自杀，继续跑（mimo 语义：Error 非致命） |
| `src-tauri/src/domain/settings.rs` | `ProviderKind { Mock, Mimo }`（serde snake_case）+ `ProviderConfig { provider, api_key, base_url, model }` | 加 `Funasr` 变体 |
| `src-tauri/src/app.rs` | `build_providers`：**3 个 match**（capture / asr / summarizer） | 三处都要加 `Funasr` 分支（见 B4） |
| `src-tauri/src/ipc/commands.rs` | `test_provider_connection` 按 provider 分派探测；`start_class` 的 api_key 门禁只针对 Mimo | 加 `probe_funasr` 分支 |
| `src/types/settings.ts` | `type ProviderKind = "mock" \| "mimo"` | union 加 `"funasr"` |
| `src/components/SettingsPage.tsx` | 两个硬编码按钮；**`setProvider` 会同时设 asr 与 summarizer 的 provider** | 加第三个按钮并**解耦**（见 B4-7） |
| `src-tauri/Cargo.toml` | tokio features = `rt-multi-thread, macros, time, sync`（**无 net/io-util**）；无任何 WebSocket 依赖 | 见 B4-8 |

**事件消费语义**（决定映射方式，已核实）：前端是**单行草稿**（每个 Partial 整行替换，Segment 到达时清草稿并追加定稿）；Segment 列表只追加。2pass 的「当前句草稿累积 + 整句定稿」与此**恰好兼容，无需改前端渲染**。

### B2. WebSocket 协议（实测，摘自官方 funasr_wss_client.py）

**连接**：
- URI：`ws://<host>:10096`（部署机已关 SSL，用 `ws://` 不是 `wss://`）
- **必须请求子协议 `"binary"`** —— 官方客户端：`websockets.connect(uri, subprotocols=["binary"], ping_interval=None)`
- Rust（tokio-tungstenite）对应写法：
  ```rust
  let mut request = url.into_client_request()?;  // tokio_tungstenite::tungstenite::client::IntoClientRequest
  request.headers_mut().insert("Sec-WebSocket-Protocol", "binary".parse()?);
  let (ws, _) = connect_async(request).await?;
  ```

**客户端 → 服务端**，三步：

1. **首帧**（Text，JSON）：
   ```json
   {
     "mode": "2pass",
     "chunk_size": [5, 10, 5],
     "chunk_interval": 10,
     "audio_fs": 16000,
     "wav_name": "classassistant-<会话标识>",
     "wav_format": "pcm",
     "is_speaking": true,
     "hotwords": "",
     "itn": true
   }
   ```

   | 字段 | 说明 |
   |---|---|
   | `mode` | 固定 `"2pass"` |
   | `chunk_size` | `[5,10,5]` 流式延迟配置（当前 600ms / 回看 300ms / 右看 300ms） |
   | `chunk_interval` | 固定 `10`（官方默认；决定 60ms 帧节奏） |
   | `audio_fs` | `16000`（PCM 时必填） |
   | `wav_name` | 会话标识（会回显在响应里） |
   | `wav_format` | `"pcm"` |
   | `is_speaking` | 首帧 `true` |
   | `hotwords` | 热词 JSON **字符串**，如 `"{\"函数极限\":20}"`；无热词给空字符串 `""` |
   | `itn` | `true`（数字规范化，保持开启） |

2. **音频帧**（Binary）：裸 PCM 字节（s16le，无文件头）。
   官方节奏：每帧 **60ms** 音频 = 16000 × 0.06 × 2 = **1920 字节**，发送间隔 60ms（实时节拍）。
   实现：把 `AudioFrame.pcm_s16le` 累积到 ≥1920 字节即切片发送。服务端按流处理，整节课当一条连续语音流推，句级切分由服务端 VAD 完成。

3. **结束帧**（Text，JSON）：会话结束（停止录音）时发一帧：
   ```json
   {"is_speaking": false}
   ```
   然后**等待约 2 秒**（收尾部定稿），再关闭 WebSocket（官方行为）。

**服务端 → 客户端**（Text，JSON，每消息一个对象）：

| 字段 | 类型 | 说明 |
|---|---|---|
| `text` | string | 识别文本 |
| `mode` | string | `"2pass-online"` 或 `"2pass-offline"` |
| `wav_name` | string | 回显首帧的 wav_name |
| `is_final` | bool | 可选，是否最后一条 |
| `timestamp` | string | 可选，字级时间戳（带时间戳模型才有，**可忽略**） |

**语义**（官方客户端 message() 处理逻辑摘录）：

```python
meg = json.loads(meg)                     # 每条消息都是 JSON 文本
text = meg["text"]
if 'mode' not in meg: continue
if meg["mode"] == "2pass-online":
    text_print_2pass_online += text       # ← text 是「当前句」的增量草稿片段
    display = text_print_2pass_offline + text_print_2pass_online
else:  # "2pass-offline"
    text_print_2pass_online = ""          # ← 清空草稿
    display = text_print_2pass_offline + text
    text_print_2pass_offline += text      # ← text 是「整句定稿」（含标点），非增量！
```

消息流示例：

```
← {"mode":"2pass-online", "text":"今天",   "is_final":false, "wav_name":"..."}
← {"mode":"2pass-online", "text":"我们",   "is_final":false, "wav_name":"..."}
← {"mode":"2pass-online", "text":"学习",   "is_final":false, "wav_name":"..."}
← {"mode":"2pass-offline","text":"今天我们学习了函数。","is_final":true,"wav_name":"..."}
← {"mode":"2pass-online", "text":"首先",   "is_final":false, "wav_name":"..."}
...
```

### B3. AsrEvent 映射（与现有 UI 语义精确对齐）

| 服务端 | AsrEvent | 说明 |
|---|---|---|
| 2pass-online 增量片段 | `Partial` | **累积**成当前句草稿再发（text 为整句草稿，逐次增长；UI 整行替换草稿行） |
| 2pass-offline 整句定稿 | `Segment` | text 为整句定稿（含标点）；清空草稿缓冲。UI 自动清草稿并追加 |
| 连接失败 / 服务未启动 | `Error` | message 建议：`无法连接 FunASR 服务，请确认部署机已启动（D:\FunASR\start.bat）` |
| 结束帧后收完尾、连接关闭 | `Ended` | 全流程只发一次（参考 mimo.rs 语义） |

**与 mimo 语义的对齐说明**（mimo 的 Partial 也是"当前分片内累积文本"，UI 已适配这种"增长式草稿"）：
- 2pass **不会**修订已定稿的句子（2pass-offline 每句只来一次），因此**不需要**给 Segment 加"替换上一条"语义——现有 APPEND_SEGMENT 直接用
- `start_ms/end_ms` 估算（仿 mimo.rs 的 sent_ms 做法）：用推流累计时长记账——某句第一条 2pass-online 到达时的累计推流时长作 `start_ms`，2pass-offline 到达时作 `end_ms`（协议 timestamp 可忽略）
- 收到 `AsrEvent::Error`（如麦克风静音告警）**不终止**流，继续推（mimo 同语义）

### B4. 文件级改动清单（按源码核实的精确落点）

1. **新增** `src-tauri/src/asr/funasr.rs`（参考 mimo.rs 的 worker + mpsc 结构）：
   - `pub struct FunasrAsrProvider { pub config: ProviderConfig }` 实现 `AsrProvider`：
     - `name()` → `"funasr"`（会写入 SessionMeta.asr_provider 与转写文件头，取稳定小写名）
     - `start(cfg)`：校验 `config.base_url` 非空且以 `ws://`/`wss://` 开头，非法则 `Err(AsrError("FunASR 服务地址无效…"))`（中文）；连 `ws://<host>:10096`（URL 取 `config.base_url`，**不要用 `effective_base_url()`**——那是 MiMo 的密钥推导逻辑）；连接失败 `Err(AsrError("无法连接 FunASR 服务，请确认部署机已启动（D:\\FunASR\\start.bat）"))`
   - `pub struct FunasrStream` 实现 `AsrStream`：
     - `push_audio`：PCM 累积切 1920 字节帧 → Binary 发送；**非阻塞**（内部经 mpsc 交 worker 发送）
     - `finish`：发 `{"is_speaking": false}` → 等 ~2s 收尾 → 关连接
     - `next_event`：从内部 `mpsc::Receiver<AsrEvent>` 收
   - 内部 tokio task（仿 `mimo.rs::run_worker`）：WS 收发 `split()`；发首帧（B2 格式）；收 Text → `serde_json` 解析 → 按 B3 映射 → mpsc；断连/异常 → `Error` 后 `Ended`（Ended 只发一次）
   - 文件头部 + 关键项写**中文单行 `//` 注释**（说明做了什么，不写为什么/怎么做）；底部 `#[cfg(test)] mod tests`（风格同 mimo.rs）
2. **修改** `src-tauri/src/asr/mod.rs`：加 `pub mod funasr;`（同 `pub mod mimo;` 位置）
3. **修改** `src-tauri/src/app.rs::build_providers`（**三个 match 都要加**）：
   ```rust
   let capture = match settings.asr.provider {
       ProviderKind::Mock => Arc::new(MockAudioCapture::default()),
       ProviderKind::Mimo => Arc::new(CpalCapture),
       ProviderKind::Funasr => Arc::new(CpalCapture),        // 新增
   };
   let asr = match settings.asr.provider {
       ProviderKind::Mock => Arc::new(MockAsrProvider::default()),
       ProviderKind::Mimo => Arc::new(MimoAsrProvider::new(settings.asr.clone())),
       ProviderKind::Funasr => Arc::new(FunasrAsrProvider::new(settings.asr.clone())), // 新增
   };
   let summarizer = match settings.summarizer.provider {
       ProviderKind::Mock => Arc::new(MockSummarizer::default()),
       ProviderKind::Mimo => Arc::new(MimoFlashSummarizer::new(...)),
       ProviderKind::Funasr => Arc::new(MimoFlashSummarizer::new(...)), // 新增：防御回退，总结仍走 MiMo Flash
   };
   ```
   并把该函数注释「目前仅支持小米 MiMo 与 mock」同步更新。
4. **修改** `src-tauri/src/domain/settings.rs`：
   - `ProviderKind` 加 `Funasr`（serde 自动得 `"funasr"`）
   - 顶部注释「目前仅支持小米 MiMo API」同步更新
   - 注意：`ProviderConfig::effective_base_url()` 是 MiMo 专用的密钥→地址推导，**FunASR 不要调用**
5. **修改** `src-tauri/src/ipc/commands.rs`：
   - `test_provider_connection` 加 `ProviderKind::Funasr => probe_funasr(&config).await`：对 base_url 做一次 WS 连接探测（连上即关），失败给中文提示
   - `start_class` 的 api_key 门禁只针对 Mimo，FunASR 不需要 key，保持不动即可（可加 base_url 非空预检）
6. **修改** `src/types/settings.ts`：`export type ProviderKind = "mock" | "mimo" | "funasr";`
7. **修改** `src/components/SettingsPage.tsx`：
   - 第三个按钮「FunASR 本地」（同现有按钮样式）
   - **`setProvider` 解耦**（关键！现有实现选引擎会把 summarizer 也带偏）：
     ```ts
     const setProvider = (provider: ProviderKind) => {
       update({
         ...settings,
         asr: { ...settings.asr, provider },
         // FunASR 只换 ASR 引擎；总结保持 MiMo（无 FunASR 总结器）
         summarizer: { ...settings.summarizer, provider: provider === "funasr" ? "mimo" : provider },
       }, { persist: false });
     };
     ```
   - `provider === "funasr"` 时 API Key 区对 asr 隐藏（FunASR 无需密钥），base_url 占位/提示改为 `ws://localhost:10096`
   - 「目前仅支持小米 MiMo API」提示横幅文案同步调整
8. **修改** `src-tauri/Cargo.toml`：
   ```toml
   tokio = { version = "1", features = ["rt-multi-thread", "macros", "time", "sync", "net", "io-util"] }  # 补 net/io-util
   tokio-tungstenite = { version = "0.26", features = ["rustls-tls-native-roots"] }
   ```
   （版本可按最新兼容微调；本对接只用 `ws://`，TLS feature 有备无患。Cargo.lock 会随之更新）

### B5. 测试步骤

1. **网络/服务连通**（开发机终端）：
   ```bash
   pip install websockets
   python funasr_wss_client.py --host 100.76.245.236 --port 10096 --mode 2pass --ssl 0 --audio_in audio\asr_example.wav
   ```
   （客户端在部署机 `D:\FunASR\samples\python\`，音频在 `samples\audio\asr_example.wav`，可拷贝到开发机）
   预期：流式草稿 + 带标点定稿交替输出（与本文 A5 实测输出同形态）。
2. **应用内**：设置页选「FunASR 本地」→ 开始上课 → 逐字草稿、句末定稿替换草稿、文本带标点。
3. **异常路径**：停部署机服务 → 开始上课 → 收到 `Error` 事件并提示启动服务。
4. **回归**：Mock / MiMo 两个原有 Provider 行为不变；`cargo test --release`（见 B7）。

### B6. 边界与注意

- **Mock/Mimo 实现不要改动**；MiMo 保留为云端选项
- 音频格式无需任何转换（已是 16k mono s16le）
- 热词走服务端文件（部署机编辑 hotwords.txt 重启）；客户端首帧 `hotwords` 留空字符串
- `itn: true` 保持开启；首帧 `chunk_interval: 10` **不要漏**
- 一次会话 = 一条 WebSocket 连接 = 一节课；结束帧后等 2 秒再断开，否则末句定稿可能丢失
- `Partial.start_ms` 现有 UI 不使用，可填估算值
- **配置兼容风险**：旧版构建读到 `"provider":"funasr"` 会因 `unwrap_or_default()` **丢弃整个 config.json（含 API Key）**。发版说明里提示用户先升级应用再选 FunASR；开发期在本机切换无碍（新构建互相兼容）
- `name()="funasr"` 会写入 `SessionMeta.asr_provider` 与转写文件头，定名后不要改

### B7. 项目工程约定（CLAUDE.md 流程要求，务必遵守）

- **Scope 变更登记**：本需求（FunASR 本地 Provider）超出 v1 范围（v1 只做 MiMo），开工前在 `docs/progress.md` 登记该需求变更。**需求来自涵涵，即为已确认**（本文档随附）
- **版本分支**：在 `vX.Y.Z.W` 分支开发（建议 `v1.1.0.0`），不要直接改 main
- **提交规范**：每步一个提交，格式 `type: 中文描述`（type 为 feat/fix/docs 等）
- **过程记录**：每步更新 `docs/progress.md` 与 `docs/dev-logs/YYYY-MM-DD.md`
- **测试**：`cd src-tauri && cargo test --release`（MinGW 下 debug 测试二进制有问题，必须 --release）
- **代码风格**：中文单行 `//` 注释（只说做了什么）；错误信息用中文；不把密钥写进任何代码/文档/提交（CLAUDE.md 安全红线）
- **模块依赖方向**：`ipc → session → {audio, asr, summarizer, storage} → domain`；trait 不依赖 Tauri

---

## Part C 联调网络速查

| 项 | 值 |
|---|---|
| 部署机 WLAN IP | `100.76.245.236`（`ipconfig` 为准；两机同一路由器即可互通） |
| 防火墙放行 | `netsh advfirewall firewall add rule name="FunASR 10096" dir=in action=allow protocol=TCP localport=10096`（管理员） |
| 同机部署 | 应用 base_url 填 `ws://localhost:10096` |
| 连不上排查 | ① 部署机 `docker compose ps` 容器 Up？② `netstat -an \| findstr 10096` 监听？③ 防火墙规则？④ `telnet <IP> 10096` |
