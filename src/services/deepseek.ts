import { invoke } from "@tauri-apps/api/core";

export interface DeepSeekConfigStatus {
  configured: boolean;
  maskedHint?: string;
}

export function getDeepSeekConfigStatus() {
  return invoke<DeepSeekConfigStatus>("get_deepseek_config_status");
}

export function saveDeepSeekApiKey(apiKey: string) {
  return invoke<DeepSeekConfigStatus>("save_deepseek_api_key", {
    input: { apiKey },
  });
}

export function deleteDeepSeekApiKey() {
  return invoke<void>("delete_deepseek_api_key");
}

export function testDeepSeekConnection() {
  return invoke<void>("test_deepseek_connection");
}
