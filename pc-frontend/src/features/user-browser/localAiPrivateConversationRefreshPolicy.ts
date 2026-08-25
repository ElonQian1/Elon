import type { LocalAiMessageSnapshot, LocalAiVisibleMessage } from './localAiBrowserApi'
import {
  localAiAssistantExtractionIncomplete,
  localAiAssistantHasRendererPlaceholder,
} from './localAiAssistantContentQuality'
import { lastMatchingLocalAiUserIndex, normalizeLocalAiResponsePrompt } from './localAiResponseTracking'

export const PRIVATE_CONVERSATION_REFRESH_GRACE_MS = 1_800
export const PRIVATE_CONVERSATION_STREAM_STALL_MS = 6_000

interface LocalAiPrivateConversationRefreshInput {
  providerId: string
  snapshot: LocalAiMessageSnapshot | null
  expectedPrompt: string
  elapsedMs: number
  attempted: boolean
}

export function shouldRequestLocalAiPrivateConversationRefresh({
  providerId,
  snapshot,
  expectedPrompt,
  elapsedMs,
  attempted,
}: LocalAiPrivateConversationRefreshInput): boolean {
  if (providerId !== 'chatgpt' || attempted || !snapshot
    || elapsedMs < PRIVATE_CONVERSATION_REFRESH_GRACE_MS
    || !isChatGptConversationUrl(snapshot.url)) return false
  const expected = normalizeLocalAiResponsePrompt(expectedPrompt)
  if (!expected) return false
  const userIndex = lastMatchingLocalAiUserIndex(snapshot.messages, expected)
  if (userIndex < 0) return false
  const assistant = snapshot.messages.slice(userIndex + 1)
    .find((message) => message.role === 'assistant')
  if (!assistant) return !snapshot.streaming || elapsedMs >= PRIVATE_CONVERSATION_STREAM_STALL_MS
  if (assistant.state === 'streaming' || snapshot.streaming) {
    return elapsedMs >= PRIVATE_CONVERSATION_STREAM_STALL_MS
  }
  return assistantNeedsPrivateRefresh(assistant)
}

function assistantNeedsPrivateRefresh(message: LocalAiVisibleMessage): boolean {
  return localAiAssistantExtractionIncomplete(message)
    || localAiAssistantHasRendererPlaceholder(message)
}

function isChatGptConversationUrl(value: string): boolean {
  try {
    const path = new URL(value).pathname
    return /^\/c\/[A-Za-z0-9_-]{1,160}$/.test(path)
      || /^\/g\/g-p-[A-Za-z0-9_-]{1,160}\/c\/[A-Za-z0-9_-]{1,160}$/.test(path)
  } catch {
    return false
  }
}
