// 全局 API 设置：启动加载，修改即存 config.json（主界面与设置页共用）
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";

import { useIpc } from "../hooks/useIpc";
import { formatError } from "../lib/format";
import { DEFAULT_SETTINGS, type AppSettings } from "../types/settings";

interface SettingsContextValue {
  settings: AppSettings;
  loading: boolean;
  message: string | null;
  setMessage: (text: string | null) => void;
  // 更新设置；persist 默认立即写盘，表单连续编辑时传 false
  update: (next: AppSettings, options?: { persist?: boolean }) => void;
  save: () => Promise<void>;
}

const SettingsContext = createContext<SettingsContextValue | null>(null);

export function SettingsProvider({ children }: { children: ReactNode }) {
  const ipc = useIpc();
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const [loading, setLoading] = useState(true);
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    ipc
      .getSettings()
      .then((loaded) => setSettings(loaded))
      .catch((err) => setMessage(formatError(err)))
      .finally(() => setLoading(false));
  }, [ipc]);

  const persist = useCallback(
    async (next: AppSettings) => {
      try {
        await ipc.saveSettings(next);
      } catch (err) {
        setMessage(formatError(err));
      }
    },
    [ipc],
  );

  const update = useCallback(
    (next: AppSettings, options?: { persist?: boolean }) => {
      setSettings(next);
      if (options?.persist !== false) {
        void persist(next);
      }
    },
    [persist],
  );

  const save = useCallback(() => persist(settings), [persist, settings]);

  const value = useMemo(
    () => ({ settings, loading, message, setMessage, update, save }),
    [settings, loading, message, update, save],
  );

  return <SettingsContext.Provider value={value}>{children}</SettingsContext.Provider>;
}

export function useSettings(): SettingsContextValue {
  const ctx = useContext(SettingsContext);
  if (!ctx) {
    throw new Error("useSettings 必须在 SettingsProvider 内使用");
  }
  return ctx;
}
