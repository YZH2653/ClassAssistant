// 会话载荷类型（与 Rust domain 镜像）
export type SessionStatus = "idle" | "recording" | "summarizing" | "completed" | "error";

export interface Segment {
  text: string;
  start_ms: number;
  end_ms: number;
}

export interface SessionMeta {
  schema_version: number;
  id: string;
  title: string;
  started_at: string | null;
  ended_at: string | null;
  duration_ms: number;
  status: SessionStatus;
  asr_provider: string;
  summarizer_provider: string;
  sample_rate: number;
  channels: number;
  segment_count: number;
  word_count: number;
  summary_generated_at: string | null;
  error: string | null;
  app_version: string;
  device: string;
}

export interface SessionSnapshot {
  meta: SessionMeta;
  data_dir: string | null;
  summary_markdown: string | null;
}

export interface SessionDetail {
  snapshot: SessionSnapshot;
  segments: Segment[];
}

export interface SessionStateEvent {
  status: SessionStatus;
  session_id?: string;
  message?: string;
}

export interface AsrPartialEvent {
  session_id: string;
  text: string;
  start_ms: number;
}

export interface AsrSegmentEvent {
  session_id: string;
  text: string;
  start_ms: number;
  end_ms: number;
}

export type SummaryStage = "collecting" | "generating" | "saving";

export interface SummaryProgressEvent {
  session_id: string;
  stage: SummaryStage;
  percent?: number;
}

export interface SummaryReadyEvent {
  session_id: string;
  summary_markdown: string;
  meta: SessionMeta;
}

export interface AppErrorEvent {
  scope: "audio" | "asr" | "summary" | "storage" | "session";
  message: string;
  recoverable: boolean;
}
