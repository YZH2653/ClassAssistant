import { useState } from "react";
import { TopBar } from "./components/TopBar";
import { ControlBar } from "./components/ControlBar";
import { TranscriptPane } from "./components/TranscriptPane";
import { SummaryPane } from "./components/SummaryPane";
import { ErrorBanner } from "./components/ErrorBanner";
import { SettingsPage } from "./components/SettingsPage";
import { useSessionDispatch, useSessionState } from "./state/SessionContext";

// 骨架期静态假数据（步骤 12 起换成真实 IPC 事件）
const DEMO_SEGMENTS = [
  { text: "同学们好，我们开始上课。", start_ms: 3000, end_ms: 6000 },
  { text: "今天我们讲线性表的顺序存储结构。", start_ms: 11000, end_ms: 15000 },
  { text: "顺序表用一段连续的存储单元依次存储数据元素。", start_ms: 19000, end_ms: 24000 },
];

const DEMO_SUMMARY = `## 知识点
- 线性表的顺序存储结构（顺序表）
- 数据元素与存储单元的映射关系

## 重点与难点
- 顺序表的地址计算公式
- 插入、删除操作的时间复杂度

## 课堂例题
- 已知首地址与元素大小，求第 i 个元素的地址

## 课后待办
- 复习顺序表插入删除的实现
- 完成教材第 2 章习题`;

export default function App() {
  const [view, setView] = useState<"main" | "settings">("main");
  const state = useSessionState();
  const dispatch = useSessionDispatch();

  const handleStart = () => {
    dispatch({ type: "RESET" });
    dispatch({ type: "SET_STATUS", status: "recording" });
    for (const seg of DEMO_SEGMENTS) dispatch({ type: "APPEND_SEGMENT", seg });
  };

  const handleEnd = () => {
    dispatch({ type: "SET_STATUS", status: "summarizing" });
    dispatch({ type: "SET_SUMMARY", markdown: DEMO_SUMMARY });
    dispatch({ type: "SET_STATUS", status: "completed" });
  };

  return (
    <div className="flex h-full flex-col gap-3 p-4">
      <div className="rounded-xl border border-slate-200 bg-white">
        <TopBar
          view={view}
          onOpenSettings={() => setView("settings")}
          onBackToMain={() => setView("main")}
          onRevealDataDir={() => dispatch({ type: "SET_ERROR", error: "（骨架演示）等步骤 12 接入打开数据目录" })}
        />
        {view === "main" ? (
          <>
            <ControlBar
              status={state.status}
              onStart={handleStart}
              onEnd={handleEnd}
              onReset={() => dispatch({ type: "RESET" })}
            />
            <div className="grid h-[calc(100vh-190px)] grid-cols-2 gap-4 p-4">
              <TranscriptPane segments={state.segments} partial={state.partial} />
              <SummaryPane
                status={state.status}
                summaryMarkdown={state.summaryMarkdown}
                summaryStage={state.summaryStage}
              />
            </div>
          </>
        ) : (
          <SettingsPage />
        )}
      </div>
      <ErrorBanner error={state.error} onDismiss={() => dispatch({ type: "SET_ERROR", error: null })} />
    </div>
  );
}
