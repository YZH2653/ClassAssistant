import { useState } from "react";

import type { SessionStatus } from "../types/session";

interface SummaryPaneProps {
  status: SessionStatus;
  summaryMarkdown: string | null;
  summaryStage: string | null;
}

const STAGE_LABEL: Record<string, string> = {
  collecting: "整理转写",
  generating: "生成总结",
  saving: "保存中",
};

export function SummaryPane({ status, summaryMarkdown, summaryStage }: SummaryPaneProps) {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    if (!summaryMarkdown) return;
    try {
      await navigator.clipboard.writeText(summaryMarkdown);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      setCopied(false);
    }
  };

  return (
    <section className="flex min-h-0 flex-col rounded-xl border border-slate-200 bg-white">
      <div className="flex items-center justify-between border-b border-slate-100 px-4 py-3">
        <h2 className="text-sm font-semibold text-slate-700">知识点总结</h2>
        <button
          className="rounded-lg border border-slate-200 px-2.5 py-1 text-xs text-slate-600 hover:bg-slate-50 disabled:cursor-not-allowed disabled:text-slate-300"
          onClick={handleCopy}
          disabled={!summaryMarkdown}
        >
          {copied ? "已复制" : "复制"}
        </button>
      </div>
      <div className="flex-1 overflow-y-auto px-4 py-3 text-sm leading-7 text-slate-800">
        {status === "summarizing" && (
          <p className="text-amber-600">
            {summaryStage ? `${STAGE_LABEL[summaryStage] ?? summaryStage}中…` : "生成总结中…"}
          </p>
        )}
        {!summaryMarkdown && status !== "summarizing" && (
          <p className="text-slate-400">下课后自动在这里生成本节课的知识点总结。</p>
        )}
        {summaryMarkdown && <pre className="whitespace-pre-wrap font-sans">{summaryMarkdown}</pre>}
      </div>
    </section>
  );
}
