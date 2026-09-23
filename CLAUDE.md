# ClassAssistant 工作说明（CLAUDE.md）

> 本文件是 AI 协作的工作规范。所有开发工作必须遵守本文件与下方标准文档。

## 一、项目简介与称呼规则

- **ClassAssistant 课堂助手**：Windows 11 课堂助手，上课实时语音转文字（小米 MiMo v2.5 ASR），下课自动总结知识点（小米 MiMo v2.6 flash），运行于 REDMI Book Pro 14 2024
- **必须始终称呼用户为「涵涵」**（每次回复开头或结尾必须包含「涵涵」称呼；若忘记说明上下文已丢失，需重置并重新加载规范）
- 涵涵的个人开发规范见 [.claude/skills/style/SKILL.md](.claude/skills/style/SKILL.md)：本项目为 Rust/TypeScript，其 C++ 格式条款不适用，但 **Git 工作流、开发日志、版本发布、称呼规则条款全部生效**

## 二、标准文件路径

| 文件 | 路径 | 用途 |
| --- | --- | --- |
| 开发需求文档 | [docs/dev-requirements.md](docs/dev-requirements.md) | 功能范围、验收标准 |
| 技术设计规范 | [docs/tech-design.md](docs/tech-design.md) | 架构、Trait、IPC、存储格式 |
| 执行步骤 | [docs/execution-steps.md](docs/execution-steps.md) | 里程碑与步骤清单（含勾选状态） |
| 进度记录 | [docs/progress.md](docs/progress.md) | 已完成事项 + 待办 TODO（每步更新） |
| 开发日志 | `docs/dev-logs/YYYY-MM-DD.md` | 按日记录（ISO 日期文件名） |
| 发布产物 | `D:\Code\ClassAssistant\output\` | zip 发布包输出目录（gitignore） |
| 后端源码入口 | `src-tauri/src/lib.rs`、`src-tauri/src/app.rs` | 装配与 Provider 工厂 |
| 前端源码入口 | `src/App.tsx`、`src/state/SessionContext.tsx` | UI 与状态 |

## 三、Git 工作流（强制）

1. **每个版本独立分支**，命名 `vX.Y.Z.W`（如 `v1.0.0.0`）；**禁止直接在 main/master 上开发**
2. **每执行完一步就 git commit 并 push**（远程 `https://github.com/YZH2653/ClassAssistant`，user：YZH2653 / yzh2653@163.com）
3. commit 格式：`type: 中文描述`（type ∈ chore/docs/feat/fix）
4. push 失败（凭据/网络）记入 progress.md 的 TODO，不阻塞开发
5. 合并/发布须**涵涵测试通过确认后**才执行（见第五节）
6. **代码生成环节必须先给简要改动说明，经涵涵手动批准后再执行**（2026-09-23 起）；每步计划只需简要说明改了什么，不写长篇审查计划

## 四、开发日志与进度更新规则

- **每执行完一步**：更新 `docs/progress.md`（已完成事项表 + TODO 勾选）；更新当日 `docs/dev-logs/YYYY-MM-DD.md`
- 开发日志内容：今日完成功能、遇到的问题、明日计划、进度统计
- 所有文档日期用**当前真实日期**，格式 YYYY-MM-DD

## 五、版本与发布规则

- 版本四段式 `vX.Y.Z.W`：分支名 = tag 名 = 发布版本（tauri.conf.json 产品版本同步）
- 发布流程（**须涵涵确认**）：合并版本分支 → main → 打同名 tag → push（含 tag）→ `npm run tauri build` → zip 命名 `-v<版本号>`（如 `-v1.0.0.0`）输出到 **`D:\Code\ClassAssistant\output`**
- zip 用于上传 GitHub Releases 分发

## 六、技术栈要点

- **Tauri 2（Rust 后端）+ Vite / React / TypeScript / Tailwind CSS v4（Web 前端）**
- 音频：cpal（16kHz/mono/s16le）；异步：tokio；存储：文件（JSON + Markdown），不引入数据库
- ASR / 总结走 Provider trait 抽象：`src-tauri/src/asr/`、`src-tauri/src/summarizer/`；**MiMo 真实实现只允许写在 `asr/mimo.rs` 与 `summarizer/mimo_flash.rs`**
- **目前只支持小米 MiMo 的 API**，不接入其他厂商模型
- 注意：本机 Rust 工具链为 **windows-gnu（MinGW）**，Tauri 2 若链接失败走 fallback（`stable-x86_64-pc-windows-msvc` + VS Build Tools 2022，`rust-toolchain.toml` 钉 msvc）
- Rust 命名遵循社区惯例 snake_case/PascalCase（style 的 C++ PascalCase 变量命名条款不适用于本项目）

## 七、常用命令

```bash
npm install                # 前端依赖
npm run tauri dev          # 开发运行
npm run tauri build        # 发布打包
npm run build              # 前端类型检查 + 构建
cd src-tauri && cargo test # Rust 单元测试
```

## 八、安全红线（强制）

1. **任何人的 API 密钥不入库**：不写进代码、文档、commit、日志；`config.json` 必须被 .gitignore 排除
2. 每个使用者在设置页自填自己的 MiMo API 密钥，**程序不内置任何密钥**
3. 错误信息/日志中密钥脱敏显示
4. 用户数据（音频/文稿/总结）仅存本机，不上传任何服务器（MiMo API 调用除外）

## 九、v1 范围红线（防止蔓延）

只做：**实时转写 + 下课自动总结 + 设置页面**。以下一律不做（已由涵涵确认）：
导出、课程历史管理+搜索、暂停/继续/手动标记重点、多端同步、数据库、音频落盘、**其他厂商 API/模型**。

新增需求一律先登记 `docs/progress.md` 变更记录并经涵涵确认，不直接开工。
