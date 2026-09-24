// Tauri invoke 薄封装（步骤 12 起接入真实调用）
import { invoke } from "@tauri-apps/api/core";
import { CMD } from "../types/ipc";
import type { SessionDetail, SessionSnapshot } from "../types/session";
import type { AppSettings, TestResult } from "../types/settings";

export interface AppInfo {
  app_version: string;
  data_dir: string;
  asr_provider: string;
  summarizer_provider: string;
}

export function getAppInfo(): Promise<AppInfo> {
  return invoke(CMD.GetAppInfo);
}

export function startClass(title?: string): Promise<SessionDetail> {
  return invoke(CMD.StartClass, { title });
}

export function endClass(): Promise<SessionSnapshot> {
  return invoke(CMD.EndClass);
}

export function getCurrentSession(): Promise<SessionDetail | null> {
  return invoke(CMD.GetCurrentSession);
}

export function resetSession(): Promise<void> {
  return invoke(CMD.ResetSession);
}

export function revealDataDir(): Promise<void> {
  return invoke(CMD.RevealDataDir);
}

export function getSettings(): Promise<AppSettings> {
  return invoke(CMD.GetSettings);
}

export function saveSettings(settings: AppSettings): Promise<void> {
  return invoke(CMD.SaveSettings, { settings });
}

export function testProviderConnection(kind: "asr" | "summarizer"): Promise<TestResult> {
  return invoke(CMD.TestProviderConnection, { kind });
}
