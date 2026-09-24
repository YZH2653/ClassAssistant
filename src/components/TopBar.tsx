interface TopBarProps {
  view: "main" | "settings";
  onOpenSettings: () => void;
  onBackToMain: () => void;
  onRevealDataDir: () => void;
}

const btn = "rounded-lg border border-slate-200 bg-white px-3 py-1.5 text-sm text-slate-700 hover:bg-slate-50";

export function TopBar({ view, onOpenSettings, onBackToMain, onRevealDataDir }: TopBarProps) {
  return (
    <header className="flex items-center justify-between border-b border-slate-200 bg-white px-6 py-3">
      <div className="flex items-baseline gap-3">
        <span className="text-lg font-semibold text-slate-800">ClassAssistant</span>
        <span className="text-sm text-slate-500">课堂助手</span>
      </div>
      <div className="flex items-center gap-2">
        {view === "main" ? (
          <>
            <button className={btn} onClick={onRevealDataDir}>
              打开数据目录
            </button>
            <button className={btn} onClick={onOpenSettings}>
              设置
            </button>
          </>
        ) : (
          <button className={btn} onClick={onBackToMain}>
            返回
          </button>
        )}
      </div>
    </header>
  );
}
