// 会话事件订阅：后端事件 → UI 状态
import { useEffect } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import { useSessionDispatch } from "../state/SessionContext";
import { EVT } from "../types/ipc";
import type {
  AppErrorEvent,
  AsrPartialEvent,
  AsrSegmentEvent,
  AudioLevelEvent,
  SessionStateEvent,
  SummaryProgressEvent,
  SummaryReadyEvent,
} from "../types/session";

export function useSessionEvents() {
  const dispatch = useSessionDispatch();

  useEffect(() => {
    let unlisteners: UnlistenFn[] = [];
    let cancelled = false;

    const subscribe = async () => {
      const subs: UnlistenFn[] = [];
      subs.push(
        await listen<SessionStateEvent>(EVT.SessionState, (event) => {
          if (event.payload.status === "idle") {
            dispatch({ type: "RESET" });
          } else {
            dispatch({ type: "SET_STATUS", status: event.payload.status });
          }
        }),
      );
      subs.push(
        await listen<AsrPartialEvent>(EVT.AsrPartial, (event) => {
          dispatch({ type: "SET_PARTIAL", text: event.payload.text });
        }),
      );
      subs.push(
        await listen<AsrSegmentEvent>(EVT.AsrSegment, (event) => {
          dispatch({ type: "SET_PARTIAL", text: null });
          dispatch({
            type: "APPEND_SEGMENT",
            seg: {
              text: event.payload.text,
              start_ms: event.payload.start_ms,
              end_ms: event.payload.end_ms,
            },
          });
        }),
      );
      subs.push(
        await listen<SummaryProgressEvent>(EVT.SummaryProgress, (event) => {
          dispatch({ type: "SET_SUMMARY_STAGE", stage: event.payload.stage });
        }),
      );
      subs.push(
        await listen<SummaryReadyEvent>(EVT.SummaryReady, (event) => {
          dispatch({ type: "SET_SUMMARY", markdown: event.payload.summary_markdown });
          dispatch({ type: "SET_SESSION", session: event.payload.meta });
          dispatch({ type: "SET_SUMMARY_STAGE", stage: null });
        }),
      );
      subs.push(
        await listen<AudioLevelEvent>(EVT.AudioLevel, (event) => {
          dispatch({ type: "SET_LEVEL", level: event.payload.level });
        }),
      );
      subs.push(
        await listen<AppErrorEvent>(EVT.AppError, (event) => {
          dispatch({ type: "SET_ERROR", error: event.payload.message });
        }),
      );

      if (cancelled) {
        subs.forEach((unlisten) => unlisten());
      } else {
        unlisteners = subs;
      }
    };

    subscribe();
    return () => {
      cancelled = true;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [dispatch]);
}
