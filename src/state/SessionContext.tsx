// 会话 UI 状态（useReducer + Context）
import { createContext, useContext, useMemo, useReducer, type Dispatch, type ReactNode } from "react";
import type { Segment, SessionMeta, SessionStatus } from "../types/session";

export interface UiState {
  status: SessionStatus;
  session: SessionMeta | null;
  segments: Segment[];
  partial: string | null;
  summaryMarkdown: string | null;
  summaryStage: string | null;
  error: string | null;
  dataDir: string | null;
}

export type UiAction =
  | { type: "SET_STATUS"; status: SessionStatus }
  | { type: "SET_SESSION"; session: SessionMeta | null }
  | { type: "SET_SEGMENTS"; segments: Segment[] }
  | { type: "APPEND_SEGMENT"; seg: Segment }
  | { type: "SET_PARTIAL"; text: string | null }
  | { type: "SET_SUMMARY"; markdown: string | null }
  | { type: "SET_SUMMARY_STAGE"; stage: string | null }
  | { type: "SET_ERROR"; error: string | null }
  | { type: "RESET" };

export const initialUiState: UiState = {
  status: "idle",
  session: null,
  segments: [],
  partial: null,
  summaryMarkdown: null,
  summaryStage: null,
  error: null,
  dataDir: null,
};

function uiReducer(state: UiState, action: UiAction): UiState {
  switch (action.type) {
    case "SET_STATUS":
      return { ...state, status: action.status };
    case "SET_SESSION":
      return { ...state, session: action.session };
    case "SET_SEGMENTS":
      return { ...state, segments: action.segments };
    case "APPEND_SEGMENT":
      return { ...state, segments: [...state.segments, action.seg] };
    case "SET_PARTIAL":
      return { ...state, partial: action.text };
    case "SET_SUMMARY":
      return { ...state, summaryMarkdown: action.markdown };
    case "SET_SUMMARY_STAGE":
      return { ...state, summaryStage: action.stage };
    case "SET_ERROR":
      return { ...state, error: action.error };
    case "RESET":
      return initialUiState;
    default:
      return state;
  }
}

const StateCtx = createContext<UiState>(initialUiState);
const DispatchCtx = createContext<Dispatch<UiAction>>(() => {});

export function SessionProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(uiReducer, initialUiState);
  const value = useMemo(() => state, [state]);
  return (
    <StateCtx.Provider value={value}>
      <DispatchCtx.Provider value={dispatch}>{children}</DispatchCtx.Provider>
    </StateCtx.Provider>
  );
}

export function useSessionState(): UiState {
  return useContext(StateCtx);
}

export function useSessionDispatch(): Dispatch<UiAction> {
  return useContext(DispatchCtx);
}
