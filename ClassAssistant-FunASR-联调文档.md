# ClassAssistant × FunASR 联调文档

> **读者**：Redmi Book Pro 14 2024 上的 AI 助手（mimo v2.6 pro）及涵涵。
> **用途**：ClassAssistant 课堂助手已完成 FunASR 本地语音识别对接，本文说明软件侧的**实际行为**、配置方法、联调验证步骤与排查手册，供你核对 FunASR 服务侧并协助联调。
> **本文自包含**——协议部分与《ClassAssistant-FunASR-对接文档.md》一致，但以下方"软件实际实现"为准。
> **日期**：2026-10-11　|　**开发分支**：`v1.1.0.0`

---

## 1. 一句话背景

MiMo v2.5 ASR 云端计费较贵，已改为**识别走本机/局域网的 FunASR 服务**；**知识点总结仍走小米 MiMo v2.6 flash**（云端）。软件保留 MiMo 与 Mock 两种识别引擎可切换。

---

## 2. 软件侧实际实现（协议核对用）

### 2.1 连接

| 项 | 实现值 |
|---|---|
| URI | 由设置页「接口地址」决定，空则用 `ws://localhost:10096` |
| 协议 | `ws://`（服务端关闭 SSL 时）/ `wss://` 均支持 |
| 子协议 | **必须为 `binary`**，软件固定请求 `Sec-WebSocket-Protocol: binary` |
| 连接时机 | 「开始上课」时建立，**一节课 = 一条 WebSocket 连接** |
| 连接失败 | 产出错误事件并提示：`无法连接 FunASR 服务，请确认部署机已启动（D:\FunASR\start.bat）` |

### 2.2 客户端 → 服务端

**首帧**（Text / JSON，连接建立后立即发送）：

```json
{
  "mode": "2pass",
  "chunk_size": [5, 10, 5],
  "chunk_interval": 10,
  "audio_fs": 16000,
  "wav_name": "classassistant-<YYYYMMDD-HHMMSS>",
  "wav_format": "pcm",
  "is_speaking": true,
  "hotwords": "",
  "itn": true
}
```

- `wav_name` 形如 `classassistant-20261011-143022`，用于区分会话
- `hotwords` 恒为空字符串（热词请在服务端 `hotwords.txt` 配置）
- `itn: true`、`chunk_interval: 10` 固定

**音频帧**（Binary）：

| 项 | 实现值 |
|---|---|
| 格式 | 裸 PCM，**s16le，16000 Hz，单声道**，无文件头 |
| 帧长 | **1920 字节 = 60 ms**（不足 1920 字节的余量在结束时补发） |
| 节奏 | 采集到即发（非阻塞，队列满会丢帧），整节课当一条连续语音流 |
| 发送超时 | 单次发送 5 秒无响应视为连接故障，报错并结束本次会话 |
| 增益 | 软件默认开启「自动增益」：输入峰值 < 0.1 时提升（最多 10 倍，只升不降） |

**结束帧**（Text / JSON，点「下课」时发送）：

```json
{"is_speaking": false}
```

发送后**等待 2 秒**收集尾部定稿，再关闭 WebSocket。请确保 2 秒内把最后的 `2pass-offline` 发完，否则末句定稿会丢。

### 2.3 服务端 → 客户端

软件读取字段：`text`、`mode`（`2pass-online` / `2pass-offline`）；`wav_name`、`is_final`、`timestamp` **读到但忽略**。

| 服务端消息 | 软件行为 |
|---|---|
| `mode = "2pass-online"` | 把 `text` **累加**到当前句草稿，界面显示整句草稿（逐次变长） |
| `mode = "2pass-offline"` | 把 `text` 作为**整句定稿**追加到转写正文（含标点），并清空草稿 |
| 其他 `mode` | 忽略 |
| 连接中断 | 提示 `FunASR 连接异常中断，转写内容已保留`，转写内容不丢 |

> **注意**：`2pass-offline` 的 `text` 软件按"整句"处理，**不会**与草稿合并去重；若服务端把整句拆成多条 offline 消息，界面上会追加成多条。请保持"一句一条 offline"的输出形态。

### 2.4 时间戳记账

软件用推流时长估算每句起止（用于转写文件里的 `[HH:MM:SS]`）：某句第一条 `2pass-online` 到达时的累计推流时长为 `start_ms`，对应 `2pass-offline` 到达时为 `end_ms`。`timestamp` 字段暂不使用。

---

## 3. 软件配置方法

1. 打开软件 → 右上角**「设置」**
2. **识别引擎**选 **「FunASR 本地」**（默认已是它）
3. **接口地址**填服务地址：
   - 应用与服务同机：`ws://localhost:10096`
   - 跨机联调：`ws://<部署机 IPv4>:10096`（例：`ws://100.76.245.236:10096`）
4. 点**「测试连接」**——显示"FunASR 服务连接正常"即通
5. **知识点总结**区填自己的小米 MiMo API 密钥（sk- / tp- 开头会自动选对应地址）
6. 点**「保存」**

> FunASR 识别**不需要 API 密钥**，所以识别区只有接口地址一栏。总结仍需 MiMo 密钥。

---

## 4. 联调验证步骤

### 4.1 服务侧自检（你那边做）

```bash
# 容器在跑？
cd D:\FunASR && docker compose ps

# 端口在听？
netstat -an | findstr 10096

# 官方客户端跑一遍（音频 samples\audio\asr_example.wav）
cd D:\FunASR\samples
python python\funasr_wss_client.py --host 127.0.0.1 --port 10096 --mode 2pass --ssl 0 --audio_in audio\asr_example.wav --output_dir out
```

预期 `out/text.0_0` 里出现草稿碎片 + 最后一条带标点的整句定稿。

### 4.2 软件侧验证

| 步骤 | 预期 |
|---|---|
| 设置页选「FunASR 本地」→「测试连接」 | 提示连接正常 |
| 「开始上课」对着麦克风说完整句子 | 约 1 秒内出现**草稿**（逐字变长），句末变成**带标点的定稿** |
| 说话时看主界面「麦克风」电平条 | 随说话跳动（不跳=设备没选对） |
| 「下课」 | 约 2~3 秒后出现四段知识点总结 |
| 打开数据目录 `sessions\<日期_时间>\` | `transcript.md` 每行 `[HH:MM:SS] 文本`，`summary.md` 为总结 |

### 4.3 异常路径

| 操作 | 预期 |
|---|---|
| 停掉 FunASR 服务 → 开始上课 | 提示「无法连接 FunASR 服务，请确认部署机已启动（D:\FunASR\start.bat）」 |
| 填了 `http://...` 这类非 ws 地址 | 提示「FunASR 服务地址无效：…（应以 ws:// 或 wss:// 开头）」 |
| 选了不存在的麦克风设备 | 提示「找不到输入设备：…」 |
| 整节课没声音 | 提示「未检测到麦克风声音，请检查录音设备与系统音量」 |

---

## 5. 异常排查

| 现象 | 处理 |
|---|---|
| 测试连接失败 | ① `docker compose ps` 容器 Up？② `netstat -an \| findstr 10096` ③ 防火墙放行 10096 ④ 地址有没有写成 `ws://` |
| 连上了但没文字 | 看麦克风电平条是否跳动；不跳 → 设置页换输入设备 |
| 只出「嗯。」「yeah.」 | **输入设备选错了**，在设置页「麦克风」里换设备（这个坑已在 v1.1.0.0 修好，可选设备并有电平表） |
| 只有草稿没有定稿 | 检查服务端是否输出 `2pass-offline`（2pass 模式下应有） |
| 末句定稿丢失 | 结束帧后 2 秒收尾窗口内服务端没发完，请查服务端处理时延 |
| 文字重复 | 服务端把一句拆成多条 offline 消息；请保持一句一条 |

---

## 6. 软件版本与产物

| 项 | 值 |
|---|---|
| 开发分支 | `v1.1.0.0`（本对接在该分支） |
| 历史发布 | `v1.0.0.0`（仅 MiMo 识别） |
| 构建命令 | `npm run tauri build` |
| 产物位置 | `D:\Code\ClassAssistant\output\` |
| 绿色版 | `ClassAssistant.exe` + `WebView2Loader.dll`（必须同目录） |
| 安装版 | `ClassAssistant_1.1.0_x64-setup.exe` |
| 数据目录 | `C:\Users\<用户名>\AppData\Roaming\com.yzh2653.classassistant\` |
| 配置文件 | 上述目录下 `config.json`（含 API 密钥，**不入 git**） |

---

## 7. 需要你那边配合确认的点

1. **一句一条 `2pass-offline`**：请确认服务端不会把一句定稿拆成多条消息
2. **2 秒收尾**：结束帧后请在 2 秒内把最后的定稿发出
3. **子协议 `binary`**：确认服务端接受该子协议（官方客户端就是这么连的）
4. **热词**：继续用服务端 `hotwords.txt`（软件侧首帧恒为空字符串）
5. **帧节奏**：软件发 60ms/1920 字节帧；如服务端对 `chunk_interval: 10` 有更优取值，可反馈我们调整

---

## 8. 术语对照

| 文中用词 | 含义 |
|---|---|
| 草稿 | `2pass-online` 增量片段，软件累积显示为"进行中"的一句 |
| 定稿 | `2pass-offline` 整句，含标点，写入转写正文 |
| 部署机 | 跑 FunASR 的 Redmi Book Pro 14 2024 |
| 开发机 | 跑 ClassAssistant 构建的电脑 |
