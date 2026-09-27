// API 配置类型（与 Rust domain::settings 镜像）
export type ProviderKind = "mock" | "mimo";

export interface ProviderConfig {
  provider: ProviderKind;
  api_key: string;
  base_url: string;
  model: string;
}

// 关闭按钮行为：exit 退出软件，tray 最小化到托盘
export type CloseBehavior = "exit" | "tray";

export interface AppSettings {
  asr: ProviderConfig;
  summarizer: ProviderConfig;
  // 总结是否开启深度思考（true = 深度思考，false = 最快输出）
  thinking: boolean;
  close_behavior: CloseBehavior;
  // 输入设备名（空 = 系统默认输入）
  input_device: string;
  // 音量过低时自动增益
  auto_gain: boolean;
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
    base_url: "https://api.xiaomimimo.com/v1",
    model: "mimo-v2.5-asr",
  },
  summarizer: {
    provider: "mimo",
    api_key: "",
    base_url: "https://api.xiaomimimo.com/v1",
    model: "mimo-v2.6-flash",
  },
  thinking: true,
  close_behavior: "tray",
  input_device: "",
  auto_gain: true,
};
