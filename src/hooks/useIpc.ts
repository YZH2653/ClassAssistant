// 前端 IPC 调用入口（步骤 12 接入真实会话流程）
import * as ipc from "../lib/tauri";

export function useIpc() {
  return ipc;
}
