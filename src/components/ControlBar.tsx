import type { SessionStatus } from "../types/session";
import { StatusPill } from "./StatusPill";

interface ControlBarProps {
  status: SessionStatus;
  onStart: () => void;
  onEnd: () => void;
  onReset: () => void;
}

const primary =
  "rounded-lg bg-indigo-600 px-5 py-2 text-sm font-medium text-white hover:bg-indigo-500 disabled:cursor-not-allowed disabled:bg-slate-300";
const secondary =
  "rounded-lg border border-slate-200 bg-white px-4 py-2 text-sm text-slate-700 hover:bg-slate-50 disabled:cursor-not-allowed disabled:text-slate-300";

export function ControlBar({ status, onStart, onEnd, onReset }: ControlBarProps) {
  const isRecording = status === "recording";
  const isBusy = status === "summarizing";
  return (
    <div className="flex items-center gap-3 border-b border-slate-200 bg-white px-6 py-3">
      <StatusPill status={status} />
      <div className="ml-auto flex items-center gap-2">
        {isRecording ? (
          <button className={primary} onClick={onEnd} disabled={isBusy}>
            下课
          </button>
        ) : (
          <button className={primary} onClick={onStart} disabled={isBusy}>
            {isBusy ? "生成总结中…" : "开始上课"}
          </button>
        )}
        <button
          className={secondary}
          onClick={onReset}
          disabled={status !== "completed" && status !== "error"}
        >
          复位
        </button>
      </div>
    </div>
  );
}
