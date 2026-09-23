# ClassAssistant 技术设计规范

- 文档版本：1.0
- 编写日期：2026-09-23
- 对应版本：v1.0.0.0

---

## 一、技术选型与理由

| 层 | 选型 | 理由 |
| --- | --- | --- |
| 桌面框架 | Tauri 2 | 涵涵选定；体积小、Rust 后端性能好、自带 WebView2 前端容器 |
| 后端语言 | Rust（tokio 异步） | Tauri 原生语言；音频/网络/并发处理合适 |
| 前端 | Vite + React + TypeScript + Tailwind CSS v4 | Web 前端自由美化 UI；TS 类型与 Rust 载荷镜像；Tailwind 快速布局 |
| 音频采集 | cpal | 跨平台麦克风采集，WASAPI 后端 |
| ASR | 小米 MiMo v2.5 ASR（云端 API） | 涵涵指定；目前仅支持小米 MiMo API |
| 总结 | 小米 MiMo v2.6 flash（云端 API） | 涵涵指定 |
| 本地存储 | 文件（JSON + Markdown） | v1 轻量、可直接打开查看，不引入数据库 |
| 状态管理 | React useReducer + Context | 规模小，不上 Redux/Zustand |

### 工具链风险声明（重要）

- 本机 Rust 工具链为 **stable-x86_64-pc-windows-gnu（MinGW）**。Tauri 2 官方主支持 MSVC，windows-gnu 下 `windows` / `webview2-com` / `tao` / `wry` 可能链接失败
- **Fallback A**：`rustup toolchain install stable-x86_64-pc-windows-msvc` + VS Build Tools 2022（C++ 桌面开发 + Windows 11 SDK），在 `src-tauri/rust-toolchain.toml` 钉 msvc
- **Fallback B**：纯 Rust 模块（domain/state/store）在 GNU 下仍可 `cargo test`，UI 联调待 MSVC 就绪

## 二、总体架构与数据流

```
┌─────────────────────────── Tauri 窗口（WebView2）───────────────────────────┐
│  React UI：主视图（转写区/总结区/控制条） + 设置视图（API 配置表单）                │
└───────────────▲──────────────────────────────────────┬─────────────────────┘
        Events（listen）│                              │ Commands（invoke）
┌───────────────┴──────────────────────────────────────▼─────────────────────┐
│  ipc/commands + ipc/events（桥接层，不依赖业务）                                 │
│  session/manager（actor 编排）→ session/state（状态机）                          │
│  audio/ → asr/ → summarizer/ → storage/（Provider trait + Mock/MiMo 实现）     │
└──────────────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
        %APPDATA%\com.yzh2653.classassistant\（config.json + sessions/）
```

**数据流**：麦克风 PCM 帧（16kHz/mono/s16le）→ `AsrStream::push_audio` → `AsrEvent(Partial/Segment)` → 落盘追加 + emit 前端；`end_class` → flush 转写 → `SummarizerProvider::summarize` → `summary.md` + emit。

**依赖方向**：`ipc → session → {audio, asr, summarizer, storage} → domain`。trait 不依赖 Tauri；`UiEvent` 由 actor 产出、ipc 层桥接 emit，便于单测。

## 三、目录结构约定

```
ClassAssistant/
├─ docs/                  # 标准文档（需求/技术设计/执行步骤/进度/开发日志）
├─ output/                # 发布 zip 产物（gitignore）
├─ src/                   # 前端：types/ state/ hooks/ lib/ components/
└─ src-tauri/src/         # 后端：domain/ audio/ asr/ summarizer/ storage/ session/ ipc/
```

- 前端文件名：组件 PascalCase.tsx，其余 camelCase.ts
- Rust 模块文件名：snake_case.rs（遵循 Rust 社区惯例；本项目非 C++，style 规范的 PascalCase 命名条款不适用，但 Git 工作流/日志条款全部适用）

## 四、Rust 模块划分与核心 Trait

```rust
// audio：音频采集
pub struct AudioFrame { pub pcm_s16le: Vec<u8>, pub sample_rate: u32, pub channels: u16, pub captured_at_ms: u64 }
pub trait AudioCapture: Send + Sync {
    fn name(&self) -> &'static str;
    fn start(&self, tx: SyncSender<AudioFrame>) -> Result<Box<dyn CaptureHandle>, AudioError>; // 有界队列，满则丢帧
}
pub trait CaptureHandle: Send { fn stop(&mut self); }

// asr：语音识别
pub enum AsrEvent {
    Partial { text: String, start_ms: u64 },
    Segment { text: String, start_ms: u64, end_ms: u64 },
    Error { message: String },
    Ended,
}
pub trait AsrProvider: Send + Sync {
    fn name(&self) -> &'static str;
    async fn start(&self, cfg: AsrConfig) -> Result<Box<dyn AsrStream>, AsrError>;
}
pub trait AsrStream: Send {
    async fn push_audio(&mut self, frame: AudioFrame) -> Result<(), AsrError>;
    async fn finish(&mut self) -> Result<(), AsrError>;
    async fn next_event(&mut self) -> Option<AsrEvent>;
}

// summarizer：知识点总结
pub trait SummarizerProvider: Send + Sync {
    fn name(&self) -> &'static str;
    async fn summarize(&self, req: SummaryRequest) -> Result<SummaryResult, SummaryError>;
}

// storage：会话存储
pub trait SessionStore: Send + Sync {
    fn create(&self, meta: &SessionMeta) -> Result<SessionPaths, StoreError>;
    fn write_transcript_header(&self, id: &str, header: &str) -> Result<(), StoreError>;
    fn append_segments(&self, id: &str, segs: &[Segment]) -> Result<(), StoreError>; // 追加写防崩溃丢稿
    fn write_summary(&self, id: &str, markdown: &str) -> Result<(), StoreError>;
    fn write_meta(&self, meta: &SessionMeta) -> Result<(), StoreError>;
}
```

**文件职责**：`audio/cpal_capture.rs`（真实采集，v1 stub+TODO）、`asr/mimo.rs` 与 `summarizer/mimo_flash.rs`（MiMo 真实实现**只允许**出现在这两个文件）、`storage/config_store.rs`（config.json 读写）、`app.rs::build_providers()`（按配置装配 Provider）。

## 五、会话状态机

状态：`Idle → Recording → Summarizing → Completed`，活跃态可进 `Error`（转写已落盘），`reset` 回 Idle。

| 当前＼事件 | start | end | asr_partial/segment | summarize_ok | fail | reset |
|---|---|---|---|---|---|---|
| Idle | →Recording（新建会话） | 非法 | 丢弃 | — | →Error | — |
| Recording | 非法 | →Summarizing（flush 转写，启动总结） | 追加落盘 + emit | — | →Error | 非法 |
| Summarizing | 非法 | 非法 | 忽略 | →Completed（写 summary + meta） | →Error | 非法 |
| Completed | →Recording（自动新会话） | 非法 | — | — | — | →Idle |
| Error | →Recording（新会话） | 非法 | — | — | — | →Idle |

- 状态机为**纯逻辑**（`session/state.rs`），不依赖 IPC/IO，全部迁移规则可单测
- `session/manager.rs`：单 actor + `mpsc<ManagerCmd>` + `oneshot` 应答，跨 await 不持锁

## 六、IPC 契约

### Commands（前端 invoke → Rust）

| 名称 | 入参 | 返回 | 说明 |
| --- | --- | --- | --- |
| `get_app_info` | — | `{ app_version, data_dir, asr_provider, summarizer_provider }` | 顶栏展示 |
| `start_class` | `{ title? }` | `SessionDetail` | 开始上课 |
| `end_class` | — | `SessionSnapshot` | 下课，立即返回；总结后台完成 |
| `get_current_session` | — | `Option<SessionDetail>` | 前端刷新回填 |
| `reset_session` | — | `()` | 复位 |
| `reveal_data_dir` | — | `()` | 资源管理器打开数据目录 |
| `get_settings` | — | `AppSettings` | 读取 API 配置 |
| `save_settings` | `AppSettings` | `()` | 保存 API 配置（整体读写） |
| `test_provider_connection` | `{ kind: "asr"\|"summarizer" }` | `{ ok, message }` | 测试连接 |

### Events（Rust → 前端 listen）

| 名称 | 载荷 | 时机 |
| --- | --- | --- |
| `session:state` | `{ status, session_id?, message? }` | 状态迁移 |
| `asr:partial` | `{ session_id, text, start_ms }` | 未定稿文本（替换 partial 行） |
| `asr:segment` | `{ session_id, text, start_ms, end_ms }` | 定稿句（追加） |
| `summary:progress` | `{ session_id, stage: "collecting"\|"generating"\|"saving", percent? }` | 总结过程 |
| `summary:ready` | `{ session_id, summary_markdown, meta }` | 总结完成并落盘 |
| `app:error` | `{ scope: audio\|asr\|summary\|storage\|session, message, recoverable }` | 错误 |

### 核心载荷（TS 镜像见 `src/types/session.ts`）

- `SessionStatus`: `idle | recording | summarizing | completed | error`
- `Segment`: `{ text, start_ms, end_ms }`
- `SessionMeta`: `schema_version, id, title, started_at, ended_at, duration_ms, status, asr_provider, summarizer_provider, sample_rate, channels, segment_count, word_count, summary_generated_at, error, app_version, device`
- `SessionDetail`: `{ snapshot: { meta, data_dir, summary_markdown }, segments }`

## 七、本地存储格式

根目录：`%APPDATA%\com.yzh2653.classassistant\`

```
├─ config.json                       # 用户 API 配置（gitignore，绝不入库）
└─ sessions\<YYYY-MM-DD_hhmmss_hex6>\
   ├─ meta.json                      # SessionMeta（创建/结束/总结完成/出错时覆写）
   ├─ transcript.md                  # 转写文稿（每 5 段追加写）
   └─ summary.md                     # 知识点总结（完成后一次性写入）
```

- `config.json`：`{ "asr": { provider, api_key, base_url, model }, "summarizer": { ...同构 } }`，provider ∈ `mock | mimo`
- `transcript.md`：标题 + 元信息头 + `## 转写正文`，正文每段一行 `[HH:MM:SS] 文本`
- `summary.md`：`# 课堂总结` + 知识点/重点与难点/课堂例题/课后待办 四段
- 全部 UTF-8 无 BOM；会话目录名 ASCII；v1 不落盘音频

## 八、Provider 抽象与 Mock 约定

- **目前仅支持小米 MiMo API**；真实现只允许写在 `asr/mimo.rs`、`summarizer/mimo_flash.rs`
- 用户在设置页填写的 api_key / base_url / model 是**唯一密钥来源**（`config.json`）；开发者可用环境变量 `CLASSASSISTANT_ASR_PROVIDER` / `MIMO_API_KEY` 调试覆盖
- MiMo 对接时 DTO 一律 `#[serde(default)]` 宽容解析，传输层（WebSocket/HTTP 分块）在 trait 内部消化
- Mock 行为约定：
  - `MockAudioCapture`：20ms 间隔推 16kHz/mono/s16le 正弦波帧
  - `MockAsrProvider`：约每 800ms 播内置 12 句课本文本（partial 逐字增长 → segment），约 15s 播完，`finish()` 发 `Ended`
  - `MockSummarizer`：模拟 800ms 延迟，按转写内容生成确定性 Markdown 四段模板
  - `test_provider_connection`：Mock 恒成功

## 九、错误处理与日志约定

- Rust 侧统一 `AppError`（serde 可序列化），command 返回 `Result<T, AppError>`
- 错误经 `app:error` 事件上抛 UI；错误信息**不得回显完整 API 密钥**（脱敏）
- 中文错误信息只走 UI/文件，不写 Windows 控制台（GBK 乱码风险）
- 总结失败：转写已保留，状态进 `Error`，可复位后重新开课
- 运行日志不落盘（v1）；开发日志走 `docs/dev-logs/`

## 十、构建、运行与发布规范

```bash
npm install               # 前端依赖
npm run tauri dev         # 开发运行（含热更新）
npm run tauri build       # 发布打包（NSIS setup.exe）
cargo test                # Rust 单元测试（在 src-tauri/ 下）
npm run build             # 仅前端类型检查 + 构建
```

- 开发工具链：VS Code + rust-analyzer；Node 24 / npm 11；Rust 1.98（GNU，见第一节 fallback）
- 发布流程（须涵涵测试确认后执行）：合并版本分支 → main → 打同名 tag → push → `npm run tauri build` → 产物 zip 命名 `-v<版本号>`（如 `-v1.0.0.0`）输出到 `D:\Code\ClassAssistant\output`
- 版本映射：分支/tag `v1.0.0.0` ↔ 产品版本 `1.0.0.0`
