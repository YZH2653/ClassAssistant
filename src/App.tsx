import { useCallback, useEffect, useState } from "react";

import { ControlBar } from "./components/ControlBar";
import { ErrorBanner } from "./components/ErrorBanner";
import { SettingsPage } from "./components/SettingsPage";
import { SummaryPane } from "./components/SummaryPane";
import { TopBar } from "./components/TopBar";
import { TranscriptPane } from "./components/TranscriptPane";
import { useIpc } from "./hooks/useIpc";
import { useSessionEvents } from "./hooks/useSessionEvents";
import { formatError } from "./lib/format";
import { useSessionDispatch, useSessionState } from "./state/SessionContext";
import type { AppSettings } from "./types/settings";

const NEED_CONFIG = "请先在设置页配置 MiMo API 密钥（或切换到演示模式）";

// 是否还没配好 API（mimo 模式下密钥为空）
function needsApiConfig(settings: AppSettings): boolean {
  return settings.asr.provider === "mimo" && settings.asr.api_key.trim().length === 0;
}

export default function App() {
  const [view, setView] = useState<"main" | "settings">("main");
  const state = useSessionState();
  const dispatch = useSessionDispatch();
  const ipc = useIpc();

  useSessionEvents();

  // 首次启动未配置 API 时引导进入设置页
  useEffect(() => {
    ipc
      .getSettings()
      .then((settings) => {
        if (needsApiConfig(settings)) {
          setView("settings");
          dispatch({ type: "SET_ERROR", error: NEED_CONFIG });
        }
      })
      .catch(() => {});
  }, [dispatch, ipc]);

  // 回填当前会话（重载窗口后恢复）
  useEffect(() => {
    ipc
      .getCurrentSession()
      .then((detail) => {
        if (!detail) return;
        dispatch({ type: "SET_STATUS", status: detail.snapshot.meta.status });
        dispatch({ type: "SET_SESSION", session: detail.snapshot.meta });
        dispatch({ type: "SET_SEGMENTS", segments: detail.segments });
        if (detail.snapshot.summary_markdown) {
          dispatch({ type: "SET_SUMMARY", markdown: detail.snapshot.summary_markdown });
        }
      })
      .catch(() => {});
  }, [dispatch, ipc]);

  const handleStart = useCallback(async () => {
    try {
      const settings = await ipc.getSettings();
      if (needsApiConfig(settings)) {
        dispatch({ type: "SET_ERROR", error: NEED_CONFIG });
        setView("settings");
        return;
      }
    } catch {
      // 读不到配置时交给后端校验
    }
    dispatch({ type: "RESET" });
    try {
      await ipc.startClass();
    } catch (err) {
      dispatch({ type: "SET_ERROR", error: formatError(err) });
    }
  }, [dispatch, ipc]);

  const handleEnd = useCallback(async () => {
    try {
      await ipc.endClass();
    } catch (err) {
      dispatch({ type: "SET_ERROR", error: formatError(err) });
    }
  }, [dispatch, ipc]);

  const handleReset = useCallback(async () => {
    try {
      await ipc.resetSession();
      dispatch({ type: "RESET" });
    } catch (err) {
      dispatch({ type: "SET_ERROR", error: formatError(err) });
    }
  }, [dispatch, ipc]);

  const handleRevealDataDir = useCallback(() => {
    ipc.revealDataDir().catch((err) => {
      dispatch({ type: "SET_ERROR", error: formatError(err) });
    });
  }, [dispatch, ipc]);

  return (
    <div className="flex h-full flex-col gap-3 p-4">
      <div className="rounded-xl border border-slate-200 bg-white">
        <TopBar
          view={view}
          onOpenSettings={() => setView("settings")}
          onBackToMain={() => setView("main")}
          onRevealDataDir={handleRevealDataDir}
        />
        {view === "main" ? (
          <>
            <ControlBar
              status={state.status}
              onStart={handleStart}
              onEnd={handleEnd}
              onReset={handleReset}
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
      <ErrorBanner
        error={state.error}
        onDismiss={() => dispatch({ type: "SET_ERROR", error: null })}
      />
    </div>
  );
}
