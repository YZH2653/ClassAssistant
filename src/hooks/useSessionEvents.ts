// 会话事件订阅（步骤 12 接入 Tauri listen）
import { useEffect } from "react";
import { useSessionDispatch } from "../state/SessionContext";

export function useSessionEvents() {
  const dispatch = useSessionDispatch();
  useEffect(() => {
    // 步骤 12：listen 后端事件并 dispatch
    return () => {
      // 取消订阅
    };
  }, [dispatch]);
}
