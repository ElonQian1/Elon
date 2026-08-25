import type { PendingLocalAiSend } from './localAiOptimisticSend'

export interface QueuedLocalAiSend {
  prompt: string
  expectedDraft: string
  pending: PendingLocalAiSend
  sessionIdentity: string
}

export const RESPONSE_REFRESH_DELAYS_MS = [400, 800, 1_500, 2_500, 4_000, 6_000, 8_000, 10_000] as const
export const RESPONSE_STREAMING_WATCHDOG_DELAYS_MS = [6_000, 12_000, 20_000, 30_000] as const

// The official DOM can declare the text turn complete before a private-stream
// visualization patch is merged into the semantic snapshot. Keep a short,
// active settle window so late charts replace the DOM placeholder immediately
// instead of waiting for the normal idle poll.
export const RESPONSE_COMPLETION_REFRESH_MS = 600
export const RESPONSE_COMPLETION_SETTLE_MS = 5_000

export type LocalAiResponseRefreshPhase = 'initial' | 'streaming_watchdog' | 'completed'

export function localAiResponseRefreshPhase({
  providerId,
  current,
  assistantObserved,
  streaming,
  completed,
}: {
  providerId: string
  current: LocalAiResponseRefreshPhase
  assistantObserved: boolean
  streaming: boolean
  completed: boolean
}): LocalAiResponseRefreshPhase {
  if (completed) return 'completed'
  if (providerId === 'google-ai-mode' && current === 'initial' && assistantObserved && streaming) {
    return 'streaming_watchdog'
  }
  return current
}

export function localAiResponseRefreshDelay(
  phase: LocalAiResponseRefreshPhase,
  index: number,
): number | undefined {
  if (phase === 'completed') return RESPONSE_COMPLETION_REFRESH_MS
  const delays = phase === 'streaming_watchdog'
    ? RESPONSE_STREAMING_WATCHDOG_DELAYS_MS
    : RESPONSE_REFRESH_DELAYS_MS
  return delays[index]
}

// 后台会话轮询本身已经按 15 秒节奏检查一次是否仍然 closed，这里只再限一个总次数上限，
// 避免真正打不开时无休止地重建 WebView2。
export const BACKGROUND_RECONNECT_MAX_ATTEMPTS = 3

// 新建会话后等待官网给出可信实时快照的上限；超过就不再无限期把输入框清空。
export const GOOGLE_NEW_CONVERSATION_RELOAD_DELAY_MS = 2_000
// 与 APK ChatGptNewConversationRecoveryCoordinator 保持同一恢复窗口：
// 官网命令三秒后仍未形成可信空白会话时，主动离开旧会话或重载首页。
export const CHATGPT_NEW_CONVERSATION_RECOVERY_DELAY_MS = 3_000
export const NEW_CONVERSATION_RECOVERY_TIMEOUT_MS = 24_000
