const DEFAULT_RESULT_TIMEOUT_MS = 5_000

const RESULT_TIMEOUTS_MS: Record<string, number> = {
  send_prompt: 12_000,
  list_conversations: 12_000,
  request_attachment_upload: 12_000,
  list_model_options: 8_000,
  list_composer_tools: 8_000,
  list_navigation: 8_000,
}

export const LOCAL_AI_RESULT_POLL_INTERVAL_MS = 200

export function localAiAdapterResultTimeoutMs(action: string): number {
  return RESULT_TIMEOUTS_MS[action] ?? DEFAULT_RESULT_TIMEOUT_MS
}

export function localAiAdapterResultAttempts(action: string): number {
  return Math.ceil(localAiAdapterResultTimeoutMs(action) / LOCAL_AI_RESULT_POLL_INTERVAL_MS)
}
