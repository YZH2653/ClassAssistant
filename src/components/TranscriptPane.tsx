import { useEffect, useRef } from "react";
import type { Segment } from "../types/session";
import { formatDuration } from "../lib/format";

interface TranscriptPaneProps {
  segments: Segment[];
  partial: string | null;
}

export function TranscriptPane({ segments, partial }: TranscriptPaneProps) {
  const bottomRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [segments, partial]);

  return (
    <section className="flex min-h-0 flex-col rounded-xl border border-slate-200 bg-white">
      <h2 className="border-b border-slate-100 px-4 py-3 text-sm font-semibold text-slate-700">实时转写</h2>
      <div className="flex-1 overflow-y-auto px-4 py-3 text-sm leading-7 text-slate-800">
        {segments.length === 0 && !partial && (
          <p className="text-slate-400">点击「开始上课」后，老师的讲话会实时转写在这里。</p>
        )}
        {segments.map((seg, i) => (
          <p key={`${seg.start_ms}-${i}`}>
            <span className="mr-2 text-slate-400">[{formatDuration(seg.start_ms)}]</span>
            {seg.text}
          </p>
        ))}
        {partial && <p className="text-slate-400">{partial}</p>}
        <div ref={bottomRef} />
      </div>
    </section>
  );
}
