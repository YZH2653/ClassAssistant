import { useState } from "react";

import { useIpc } from "../hooks/useIpc";
import { formatError } from "../lib/format";
import { isKnownBaseUrl, resolveBaseUrl } from "../lib/mimo";
import { useSettings } from "../state/SettingsContext";
import {
  DEFAULT_SETTINGS,
  type ProviderConfig,
  type ProviderKind,
} from "../types/settings";

const input =
  "w-full rounded-lg border border-slate-200 px-3 py-2 text-sm text-slate-800 focus:border-indigo-400 focus:outline-none";
const btn = "rounded-lg bg-indigo-600 px-4 py-2 text-sm font-medium text-white hover:bg-indigo-500";
const btnGhost =
  "rounded-lg border border-slate-200 bg-white px-4 py-2 text-sm text-slate-700 hover:bg-slate-50";

interface ProviderSectionProps {
  title: string;
  hint: string;
  value: ProviderConfig;
  onChange: (next: ProviderConfig) => void;
  onTest: () => void;
}

function ProviderSection({ title, hint, value, onChange, onTest }: ProviderSectionProps) {
  const [showKey, setShowKey] = useState(false);
  return (
    <div className="rounded-xl border border-slate-200 bg-white p-5">
      <h3 className="text-sm font-semibold text-slate-800">{title}</h3>
      <p className="mt-1 text-xs text-slate-500">{hint}</p>
      <div className="mt-4 space-y-3">
        <label className="block text-xs text-slate-500">
          API Key
          <div className="mt-1 flex gap-2">
            <input
              className={input}
              type={showKey ? "text" : "password"}
              value={value.api_key}
              placeholder="sk-…（按量付费）或 tp-… / ttp-…（Token Plan）"
              onChange={(e) => onChange({ ...value, api_key: e.currentTarget.value })}
            />
            <button className={btnGhost} onClick={() => setShowKey(!showKey)}>
              {showKey ? "隐藏" : "显示"}
            </button>
          </div>
        </label>
        <label className="block text-xs text-slate-500">
          API Base URL
          <input
            className={`mt-1 ${input}`}
            value={value.base_url}
            placeholder="按密钥类型自动选择，可手动覆盖"
            onChange={(e) => onChange({ ...value, base_url: e.currentTarget.value })}
          />
        </label>
        <label className="block text-xs text-slate-500">
          模型名
          <input
            className={`mt-1 ${input}`}
            value={value.model}
            onChange={(e) => onChange({ ...value, model: e.currentTarget.value })}
          />
        </label>
        <button className={btnGhost} onClick={onTest}>
          测试连接
        </button>
      </div>
    </div>
  );
}

export function SettingsPage() {
  const ipc = useIpc();
  const { settings, loading, message, setMessage, update, save } = useSettings();

  const setProvider = (provider: ProviderKind) => {
    update(
      {
        ...settings,
        asr: { ...settings.asr, provider },
        summarizer: { ...settings.summarizer, provider },
      },
      { persist: false },
    );
  };

  // 改配置项：密钥变化时自动带出接口地址（用户手动改过地址则不覆盖）
  const changeProvider = (kind: "asr" | "summarizer", next: ProviderConfig) => {
    const prev = settings[kind];
    const autoUrl = next.api_key !== prev.api_key && isKnownBaseUrl(prev.base_url);
    const merged = autoUrl ? { ...next, base_url: resolveBaseUrl(next.api_key) } : next;
    update({ ...settings, [kind]: merged }, { persist: false });
    if (autoUrl) {
      setMessage("已按密钥类型自动选择接口地址");
    }
  };

  const handleSave = async () => {
    await save();
    setMessage("已保存到本机 config.json");
  };

  const handleTest = async (kind: "asr" | "summarizer") => {
    try {
      const result = await ipc.testProviderConnection(kind);
      setMessage(`${kind === "asr" ? "ASR" : "总结"}：${result.message}`);
    } catch (err) {
      setMessage(formatError(err));
    }
  };

  const handleReset = () => {
    update(DEFAULT_SETTINGS);
    setMessage("已恢复默认（API Key 已清空）");
  };

  const isMock = settings.asr.provider === "mock";
  const choice = (active: boolean) =>
    `rounded-lg px-4 py-2 text-sm ${
      active
        ? "bg-indigo-600 text-white"
        : "border border-slate-200 bg-white text-slate-700 hover:bg-slate-50"
    }`;

  return (
    <div className="mx-auto max-w-3xl space-y-4 px-6 py-6">
      <div className="rounded-lg bg-indigo-50 px-4 py-3 text-sm text-indigo-700">
        目前仅支持小米 MiMo API（按量付费 sk- 开头 / Token Plan tp-、ttp- 开头）。每位使用者请填写自己的
        API 密钥，密钥只保存在本机，不会上传。
      </div>

      <div className="rounded-xl border border-slate-200 bg-white p-5">
        <h3 className="text-sm font-semibold text-slate-800">识别引擎</h3>
        <div className="mt-3 flex gap-2">
          <button className={choice(!isMock)} onClick={() => setProvider("mimo")}>
            小米 MiMo
          </button>
          <button className={choice(isMock)} onClick={() => setProvider("mock")}>
            本地演示（Mock）
          </button>
        </div>
        <p className="mt-2 text-xs text-slate-500">
          {isMock
            ? "演示模式：不需要密钥，用于体验完整流程。"
            : "使用小米 MiMo 云端 API，请填写下方密钥。"}
        </p>
      </div>

      {!isMock && (
        <>
          <ProviderSection
            title="语音识别（小米 MiMo v2.5 ASR）"
            hint="上课时把老师讲话实时转成文字"
            value={settings.asr}
            onChange={(next) => changeProvider("asr", next)}
            onTest={() => handleTest("asr")}
          />
          <ProviderSection
            title="知识点总结（小米 MiMo v2.6 flash）"
            hint="下课后自动生成本节课的知识点总结"
            value={settings.summarizer}
            onChange={(next) => changeProvider("summarizer", next)}
            onTest={() => handleTest("summarizer")}
          />
        </>
      )}

      <div className="flex items-center gap-3">
        <button className={btn} onClick={handleSave} disabled={loading}>
          保存
        </button>
        <button className={btnGhost} onClick={handleReset} disabled={loading}>
          恢复默认
        </button>
        {message && <span className="text-sm text-slate-500">{message}</span>}
      </div>
    </div>
  );
}
