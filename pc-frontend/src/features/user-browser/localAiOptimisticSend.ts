import type { LocalAiVisibleMessage } from './localAiBrowserApi'

export interface PendingLocalAiSend {
  id: string
  prompt: string
  normalizedPrompt: string
  baselineMatchingUserCount: number
}

export function beginOptimisticLocalAiSend(
  officialMessages: LocalAiVisibleMessage[],
  pendingSends: PendingLocalAiSend[],
  prompt: string,
  id: string,
): PendingLocalAiSend | null {
  const normalizedPrompt = normalizeLocalAiPrompt(prompt)
  if (!normalizedPrompt || !id.trim()) return null
  const unresolvedMatching = pendingSends.filter((pending) => (
    pending.normalizedPrompt === normalizedPrompt
    && !pendingLocalAiSendObserved(officialMessages, pending)
  )).length
  return {
    id,
    prompt: prompt.trim(),
    normalizedPrompt,
    baselineMatchingUserCount: matchingUserCount(officialMessages, normalizedPrompt)
      + unresolvedMatching,
  }
}

export function pendingLocalAiSendObserved(
  officialMessages: LocalAiVisibleMessage[],
  pending: PendingLocalAiSend,
): boolean {
  return matchingUserCount(officialMessages, pending.normalizedPrompt)
    > pending.baselineMatchingUserCount
}

export function mergeOptimisticLocalAiMessages(
  officialMessages: LocalAiVisibleMessage[],
  pendingSends: PendingLocalAiSend[],
): LocalAiVisibleMessage[] {
  const unresolved = pendingSends.filter((pending) => (
    !pendingLocalAiSendObserved(officialMessages, pending)
  ))
  if (unresolved.length === 0) return officialMessages
  return officialMessages.concat(unresolved.map((pending) => ({
    id: pending.id,
    role: 'user' as const,
    state: 'completed' as const,
    content: [{ type: 'text' as const, text: pending.prompt }],
  })))
}

function matchingUserCount(messages: LocalAiVisibleMessage[], normalizedPrompt: string): number {
  return messages.filter((message) => (
    message.role === 'user' && normalizeLocalAiPrompt(visibleMessageText(message)) === normalizedPrompt
  )).length
}

function visibleMessageText(message: LocalAiVisibleMessage): string {
  return message.content
    .filter((part) => part.type === 'text' || part.type === 'markdown')
    .map((part) => part.text ?? '')
    .join('\n')
}

function normalizeLocalAiPrompt(value: string): string {
  return value.trim().replace(/\s+/g, ' ')
}
