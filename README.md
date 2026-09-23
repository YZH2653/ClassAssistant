# ClassAssistant 课堂助手

运行在 Windows 11 上的课堂助手软件：上课时实时语音转文字，下课后自动总结知识点。

- **技术栈**：Tauri 2（Rust 后端）+ React / TypeScript / Tailwind CSS（Web 前端）
- **语音识别**：小米 MiMo v2.5 ASR（云端 API，目前仅支持小米 MiMo API）
- **知识点总结**：小米 MiMo v2.6 flash（云端 API）
- **数据存储**：纯本地单机，文件存储

## 功能（v1.0.0.0）

1. **实时转写**：麦克风实时采集，流式语音转文字，界面实时显示
2. **下课自动总结**：下课后自动调用总结模型，生成知识点总结
3. **设置页面**：每位使用者自行输入自己的 MiMo API 密钥（不内置任何密钥）

## 文档

| 文档 | 说明 |
| --- | --- |
| [docs/dev-requirements.md](docs/dev-requirements.md) | 开发需求文档 |
| [docs/tech-design.md](docs/tech-design.md) | 技术设计规范 |
| [docs/execution-steps.md](docs/execution-steps.md) | 执行步骤与里程碑 |
| [docs/progress.md](docs/progress.md) | 进度记录（已完成 + 待办） |
| [docs/dev-logs/](docs/dev-logs/) | 开发日志 |
| [CLAUDE.md](CLAUDE.md) | AI 协作工作规范 |

## 开发

```bash
npm install
npm run tauri dev     # 开发运行
npm run tauri build   # 发布打包
cargo test            # Rust 单元测试
```

## 运行环境

- Windows 11（目标机型：REDMI Book Pro 14 2024）
- WebView2 运行时（Win11 一般自带）
