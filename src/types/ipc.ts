// IPC 命令与事件名常量（与 src-tauri/src/ipc/ 对应）
export const CMD = {
  GetAppInfo: "get_app_info",
  StartClass: "start_class",
  EndClass: "end_class",
  GetCurrentSession: "get_current_session",
  ResetSession: "reset_session",
  RevealDataDir: "reveal_data_dir",
  GetSettings: "get_settings",
  SaveSettings: "save_settings",
  TestProviderConnection: "test_provider_connection",
} as const;

export const EVT = {
  SessionState: "session:state",
  AsrPartial: "asr:partial",
  AsrSegment: "asr:segment",
  SummaryProgress: "summary:progress",
  SummaryReady: "summary:ready",
  AppError: "app:error",
} as const;
