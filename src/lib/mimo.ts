// MiMo 接口地址推断（按密钥前缀区分两套计费）
export const PAY_AS_YOU_GO_BASE_URL = "https://api.xiaomimimo.com/v1";
export const TOKEN_PLAN_BASE_URL = "https://token-plan-cn.xiaomimimo.com/v1";

// 是否为已知的默认接口地址（用于判断用户是否手动改过）
export function isKnownBaseUrl(baseUrl: string): boolean {
  return baseUrl === "" || baseUrl === PAY_AS_YOU_GO_BASE_URL || baseUrl === TOKEN_PLAN_BASE_URL;
}

// 按密钥前缀返回接口地址：sk 按量付费，tp/ttp 走 Token Plan
export function resolveBaseUrl(apiKey: string): string {
  const key = apiKey.trim();
  if (key.startsWith("tp-") || key.startsWith("ttp-")) {
    return TOKEN_PLAN_BASE_URL;
  }
  return PAY_AS_YOU_GO_BASE_URL;
}
