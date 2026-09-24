import { useState } from "react";
import { DEFAULT_SETTINGS, type AppSettings, type ProviderConfig } from "../types/settings";

const input =
  "w-full rounded-lg border border-slate-200 px-3 py-2 text-sm text-slate-800 focus:border-indigo-400 focus:outline-none";
const btn = "rounded-lg bg-indigo-600 px-4 py-2 text-sm font-medium text-white hover:bg-indigo-500";
const btnGhost = "rounded-lg border border-slate-200 bg-white px-4 py-2 text-sm text-slate-700 hover:bg-slate-50";

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
              placeholder="请输入你自己的 API Key"
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
            placeholder="接口地址（默认留空即可）"
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
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const [message, setMessage] = useState<string | null>(null);

  return (
    <div className="mx-auto max-w-3xl space-y-4 px-6 py-6">
      <div className="rounded-lg bg-indigo-50 px-4 py-3 text-sm text-indigo-700">
        目前仅支持小米 MiMo API。每位使用者请填写自己的 API 密钥，密钥只保存在本机，不会上传。
      </div>
      <ProviderSection
        title="语音识别（小米 MiMo v2.5 ASR）"
        hint="上课时把老师讲话实时转成文字"
        value={settings.asr}
        onChange={(next) => setSettings({ ...settings, asr: next })}
        onTest={() => setMessage("（骨架演示）测试连接：等步骤 13 接入真实配置")}
      />
      <ProviderSection
        title="知识点总结（小米 MiMo v2.6 flash）"
        hint="下课后自动生成本节课的知识点总结"
        value={settings.summarizer}
        onChange={(next) => setSettings({ ...settings, summarizer: next })}
        onTest={() => setMessage("（骨架演示）测试连接：等步骤 13 接入真实配置")}
      />
      <div className="flex items-center gap-3">
        <button className={btn} onClick={() => setMessage("（骨架演示）已保存：等步骤 13 写入 config.json")}>
          保存
        </button>
        <button
          className={btnGhost}
          onClick={() => {
            setSettings(DEFAULT_SETTINGS);
            setMessage("已恢复默认（API Key 已清空）");
          }}
        >
          恢复默认
        </button>
        {message && <span className="text-sm text-slate-500">{message}</span>}
      </div>
    </div>
  );
}
