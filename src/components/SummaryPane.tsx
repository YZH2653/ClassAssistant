import type { SessionStatus } from "../types/session";

interface SummaryPaneProps {
  status: SessionStatus;
  summaryMarkdown: string | null;
  summaryStage: string | null;
}

export function SummaryPane({ status, summaryMarkdown, summaryStage }: SummaryPaneProps) {
  return (
    <section className="flex min-h-0 flex-col rounded-xl border border-slate-200 bg-white">
      <h2 className="border-b border-slate-100 px-4 py-3 text-sm font-semibold text-slate-700">知识点总结</h2>
      <div className="flex-1 overflow-y-auto px-4 py-3 text-sm leading-7 text-slate-800">
        {status === "summarizing" && <p className="text-amber-600">生成总结中…{summaryStage ?? ""}</p>}
        {!summaryMarkdown && status !== "summarizing" && (
          <p className="text-slate-400">下课后自动在这里生成本节课的知识点总结。</p>
        )}
        {summaryMarkdown && <pre className="whitespace-pre-wrap font-sans">{summaryMarkdown}</pre>}
      </div>
    </section>
  );
}
