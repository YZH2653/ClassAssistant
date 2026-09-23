# ClassAssistant 执行步骤（里程碑/任务）

- 文档版本：1.0
- 编写日期：2026-09-23
- 对应版本：v1.0.0.0
- 规则：**每步一次 git commit 并 push**；完成一步后同步更新 `docs/progress.md` 与当日开发日志

---

## 一、里程碑总览

| 里程碑 | 内容 | 对应步骤 |
| --- | --- | --- |
| M0 | 环境与仓库初始化 | 步骤 1 |
| M1 | 标准文档与 CLAUDE.md | 步骤 2–5 |
| M2 | Tauri 2 项目骨架 | 步骤 6–7 |
| M3 | 领域核心与 Mock Provider | 步骤 8–9 |
| M4 | 存储层与 IPC 编排闭环 | 步骤 10–11 |
| M5 | 前端 UI 与 Mock 链路联调 | 步骤 12–13 |
| M6 | 验证与 v1.0.0.0 发布 | 步骤 14–15 |

## 二、M0 环境与仓库初始化

- [x] 步骤 1：`git init -b main` + user 配置（YZH2653 / yzh2653@163.com）+ `remote add origin` + `.gitignore` + `README.md` → `chore: 初始化 Git 仓库与远程配置`（含合并远程初始提交 `chore: 合并远程仓库初始提交`）

## 三、M1 标准文档与 CLAUDE.md

- [x] 步骤 2：`git checkout -b v1.0.0.0` + `docs/dev-requirements.md` → `docs: 编写开发需求文档`
- [x] 步骤 3：`docs/tech-design.md` → `docs: 编写技术设计规范`
- [x] 步骤 4：`docs/execution-steps.md` + `docs/progress.md` + `docs/dev-logs/2026-09-23.md` → `docs: 建立执行步骤与进度记录体系`
- [ ] 步骤 5：`CLAUDE.md` → `docs: 编写 CLAUDE.md 工作规范`

## 四、M2 Tauri 2 项目骨架

- [ ] 步骤 6：Tauri 2 脚手架（React-TS）+ `npm install` + 冒烟出窗口；GNU 工具链失败则走 fallback（MSVC）→ `feat: Tauri 2 项目骨架可编译出窗口`
- [ ] 步骤 7：Tailwind v4 + 前端目录/UI 骨架（主视图组件 + SettingsPage 表单骨架，静态假数据） → `feat: 前端 UI 骨架与 Tailwind 样式`

## 五、M3 领域核心与 Mock Provider

- [ ] 步骤 8：domain 模型 + 会话状态机纯逻辑 + 单元测试 → `feat: 领域模型与会话状态机（含单测）`
- [ ] 步骤 9：Audio/ASR/Summarizer 三个 Provider trait + Mock 实现 + 工厂与 config 读取 → `feat: 采集/ASR/总结 Provider 抽象与 Mock 实现`

## 六、M4 存储层与 IPC 编排闭环

- [ ] 步骤 10：SessionStore + FsSessionStore + ConfigStore + 临时目录单测 → `feat: 本地文件会话存储与配置存储层`
- [ ] 步骤 11：SessionManager actor + IPC commands/events（含 get/save_settings、test_provider_connection）+ app.rs 装配 → `feat: 会话编排与 Tauri IPC 命令事件`

## 七、M5 前端 UI 与 Mock 链路联调

- [ ] 步骤 12：前端接 IPC（SessionContext + 事件订阅 + 主视图联调 Mock 链路） → `feat: 前端接入 IPC 打通 Mock 全链路`
- [ ] 步骤 13：设置页联调（读写 config.json、未配置拦截、测试连接、重启持久化） → `feat: 设置页面 API 配置管理`

## 八、M6 验证清单与 v1.0.0.0 发布流程

- [ ] 步骤 14：自测修正（状态互斥/错误横幅/自动滚动/reveal）+ `cargo test` + `npm run build` + 更新文档 → `fix: Mock 链路自测修正与文档进度更新`
- [ ] 步骤 15：**（须涵涵测试确认后才执行）** 合并 `v1.0.0.0`→`main` + tag `v1.0.0.0` + push + `npm run tauri build` + zip 到 `output/ClassAssistant-v1.0.0.0-win-x64.zip` → `chore: 发布 v1.0.0.0`

发布流程细则（步骤 15）：

1. 涵涵测试通过并确认
2. `git checkout main && git merge v1.0.0.0`
3. `git tag v1.0.0.0 && git push origin main --tags`
4. `npm run tauri build`
5. 产物 zip 命名 `-v1.0.0.0` 格式，输出到 `D:\Code\ClassAssistant\output`
6. 更新 progress / 开发日志写「已发布」

## 九、后续 TODO（v1 之后，不在本轮）

- MiMo v2.5 ASR 真实 API 对接（`asr/mimo.rs`，**等涵涵提供 API 文档**）
- MiMo v2.6 flash 真实 API 对接（`summarizer/mimo_flash.rs`，同上）
- 真实 cpal 麦克风采集（`audio/cpal_capture.rs`，本轮 stub）
- 导出 / 历史搜索 / 暂停标记 / 多端同步 / 数据库 / 音频落盘 / 其他厂商 API——**已确认不做**（v1 范围外，需要时另立版本）
