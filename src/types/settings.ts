// API 配置类型（与 Rust domain::settings 镜像）
export type ProviderKind = "mock" | "mimo";

export interface ProviderConfig {
  provider: ProviderKind;
  api_key: string;
  base_url: string;
  model: string;
}

export interface AppSettings {
  asr: ProviderConfig;
  summarizer: ProviderConfig;
}

export interface TestResult {
  ok: boolean;
  message: string;
}

// 默认配置（仅支持小米 MiMo API）
export const DEFAULT_SETTINGS: AppSettings = {
  asr: {
    provider: "mimo",
    api_key: "",
    base_url: "",
    model: "mimo-v2.5-asr",
  },
  summarizer: {
    provider: "mimo",
    api_key: "",
    base_url: "",
    model: "mimo-v2.6-flash",
  },
};
