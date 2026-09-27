import type { SessionStatus } from "../types/session";
import { useSettings } from "../state/SettingsContext";
import { StatusPill } from "./StatusPill";

interface ControlBarProps {
  status: SessionStatus;
  level: number;
  onStart: () => void;
  onEnd: () => void;
  onReset: () => void;
}

const primary =
  "rounded-lg bg-indigo-600 px-5 py-2 text-sm font-medium text-white hover:bg-indigo-500 disabled:cursor-not-allowed disabled:bg-slate-300";
const secondary =
  "rounded-lg border border-slate-200 bg-white px-4 py-2 text-sm text-slate-700 hover:bg-slate-50 disabled:cursor-not-allowed disabled:text-slate-300";

export function ControlBar({ status, level, onStart, onEnd, onReset }: ControlBarProps) {
  const { settings, update } = useSettings();
  const isRecording = status === "recording";
  const isBusy = status === "summarizing";

  const thinkingChoice = (active: boolean) =>
    `px-2.5 py-1 rounded-md text-xs ${
      active ? "bg-indigo-600 text-white" : "text-slate-600 hover:bg-slate-100"
    }`;

  return (
    <div className="flex items-center gap-3 border-b border-slate-200 bg-white px-6 py-3">
      <StatusPill status={status} />
      <div className="flex items-center gap-2">
        <span className="text-xs text-slate-500">麦克风</span>
        <div className="h-2 w-24 overflow-hidden rounded-full bg-slate-100">
          <div
            className={`h-full transition-all duration-100 ${
              level > 0.6 ? "bg-emerald-500" : level > 0.1 ? "bg-amber-400" : "bg-slate-300"
            }`}
            style={{ width: `${Math.min(100, Math.round(level * 100))}%` }}
          />
        </div>
      </div>
      <div className="ml-auto flex items-center gap-3">
        <div className="flex items-center gap-2">
          <span className="text-xs text-slate-500">总结思考</span>
          <div className="flex items-center rounded-lg border border-slate-200 p-0.5">
            <button
              className={thinkingChoice(!settings.thinking)}
              onClick={() => update({ ...settings, thinking: false })}
            >
              最快输出
            </button>
            <button
              className={thinkingChoice(settings.thinking)}
              onClick={() => update({ ...settings, thinking: true })}
            >
              深度思考
            </button>
          </div>
        </div>
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
