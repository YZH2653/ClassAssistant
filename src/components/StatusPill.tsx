import type { SessionStatus } from "../types/session";

const STATUS_LABEL: Record<SessionStatus, string> = {
  idle: "空闲",
  recording: "录音中",
  summarizing: "生成总结中",
  completed: "已完成",
  error: "错误",
};

const STATUS_STYLE: Record<SessionStatus, string> = {
  idle: "bg-slate-100 text-slate-600",
  recording: "bg-red-50 text-red-600",
  summarizing: "bg-amber-50 text-amber-600",
  completed: "bg-emerald-50 text-emerald-600",
  error: "bg-rose-50 text-rose-600",
};

export function StatusPill({ status }: { status: SessionStatus }) {
  return (
    <span className={`rounded-full px-3 py-1 text-sm font-medium ${STATUS_STYLE[status]}`}>
      {STATUS_LABEL[status]}
    </span>
  );
}
