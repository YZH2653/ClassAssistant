# ClassAssistant 课堂助手

运行在 Windows 11 上的课堂助手软件：上课时实时语音转文字，下课后自动总结知识点。

- **技术栈**：Tauri 2（Rust 后端）+ React / TypeScript / Tailwind CSS（Web 前端）
- **语音识别**：FunASR 本地服务（WebSocket 2pass 实时转写，默认，免费）/ 小米 MiMo v2.5 ASR（云端 API 可选项）
- **知识点总结**：小米 MiMo v2.6 flash（云端 API）
- **数据存储**：纯本地单机，文件存储

## 功能（v1.1.0.0）

1. **实时转写**：麦克风实时采集，约 1 秒出灰色草稿、句末变带标点定稿
2. **下课自动总结**：下课后收尾定稿再自动生成知识点总结（末句不丢）
3. **设置页面**：每位使用者自行输入自己的 MiMo API 密钥（不内置任何密钥）；识别引擎可选 FunASR 本地 / 小米 MiMo / 演示 Mock
4. **留存最近一次总结**：复位误触后仍可查看和复制上一次的总结
5. **麦克风输入设备选择**：识别不出字时可换输入设备，主界面带电平表辅助排查

## 文档

| 文档 | 说明 |
| --- | --- |
| [docs/dev-requirements.md](docs/dev-requirements.md) | 开发需求文档 |
| [docs/tech-design.md](docs/tech-design.md) | 技术设计规范 |
| [docs/execution-steps.md](docs/execution-steps.md) | 执行步骤与里程碑 |
| [docs/progress.md](docs/progress.md) | 进度记录（已完成 + 待办） |
| [docs/dev-logs/](docs/dev-logs/) | 开发日志 |
| [CLAUDE.md](CLAUDE.md) | AI 协作工作规范 |
| [ClassAssistant-FunASR-对接文档.md](ClassAssistant-FunASR-对接文档.md) | FunASR 部署与对接规范 |
| [ClassAssistant-FunASR-联调文档.md](ClassAssistant-FunASR-联调文档.md) | FunASR 联调契约与排障 |

## 开发

```bash
npm install
npm run tauri dev              # 开发运行
npm run tauri build            # 发布打包
cd src-tauri && cargo test --release   # Rust 单元测试（MinGW 下 debug 测试无法启动，须 --release）
```

## 运行环境

- Windows 11（目标机型：REDMI Book Pro 14 2024）
- WebView2 运行时（Win11 一般自带）
- FunASR 识别需自行部署 FunASR 服务（本地或局域网，见对接文档）；仅用 MiMo 云端识别则不需要
