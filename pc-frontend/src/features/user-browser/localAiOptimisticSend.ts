import type { LocalAiVisibleMessage } from './localAiBrowserApi'
import { localAiAssistantExtractionIncomplete } from './localAiAssistantContentQuality'
import { hasVisibleAiMessageContent } from '../ai/aiMessageVisibility'

export interface PendingLocalAiSend {
  id: string
  prompt: string
  normalizedPrompt: string
  baselineMatchingUserCount: number
}

export interface PendingLocalAiResponse {
  id: string
  sendId: string
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

export function beginPendingLocalAiResponse(
  pending: PendingLocalAiSend,
): PendingLocalAiResponse {
  return {
    id: `${pending.id}:assistant`,
    sendId: pending.id,
    prompt: pending.prompt,
    normalizedPrompt: pending.normalizedPrompt,
    baselineMatchingUserCount: pending.baselineMatchingUserCount,
  }
}

export function pendingLocalAiResponseObserved(
  officialMessages: LocalAiVisibleMessage[],
  pending: PendingLocalAiResponse,
): boolean {
  const assistant = officialAssistantForPendingResponse(officialMessages, pending)
  return Boolean(assistant && (
    assistant.state === 'streaming' || (
      hasSubstantiveAssistantContent(assistant)
      && !localAiAssistantExtractionIncomplete(assistant)
    )
  ))
}

export function mergeOptimisticLocalAiMessages(
  officialMessages: LocalAiVisibleMessage[],
  pendingSends: PendingLocalAiSend[],
  pendingResponses: PendingLocalAiResponse[] = [],
  responseBlocked = false,
): LocalAiVisibleMessage[] {
  const unresolved = pendingSends.filter((pending) => (
    !pendingLocalAiSendObserved(officialMessages, pending)
  ))
  const normalizedOfficial = responseBlocked ? officialMessages : officialMessages.map((message) => {
    const pending = pendingResponses.find((candidate) => (
      officialAssistantForPendingResponse(officialMessages, candidate) === message
    ))
    if (!pending || message.state === 'streaming' || (
      hasSubstantiveAssistantContent(message)
      && !localAiAssistantExtractionIncomplete(message)
    )) {
      return message
    }
    return {
      ...message,
      state: 'streaming' as const,
      content: message.content.filter(isSubstantiveStructuredPart),
    }
  })
  const optimisticUsers = unresolved.map((pending) => ({
    id: pending.id,
    role: 'user' as const,
    state: 'completed' as const,
    content: [{ type: 'text' as const, text: pending.prompt }],
  }))
  const optimisticResponses = (responseBlocked ? [] : pendingResponses)
    .filter((pending) => !officialAssistantForPendingResponse(officialMessages, pending))
    .map((pending) => ({
      id: pending.id,
      role: 'assistant' as const,
      state: 'streaming' as const,
      content: [],
    }))
  return normalizedOfficial.concat(optimisticUsers, optimisticResponses)
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

function officialAssistantForPendingResponse(
  messages: LocalAiVisibleMessage[],
  pending: PendingLocalAiResponse,
): LocalAiVisibleMessage | undefined {
  const userIndex = targetUserIndex(messages, pending)
  if (userIndex < 0) return undefined
  return messages.slice(userIndex + 1).find((message) => message.role === 'assistant')
}

function targetUserIndex(
  messages: LocalAiVisibleMessage[],
  pending: PendingLocalAiResponse,
): number {
  let matchingUsers = 0
  for (let index = 0; index < messages.length; index += 1) {
    const message = messages[index]
    if (message.role !== 'user'
      || normalizeLocalAiPrompt(visibleMessageText(message)) !== pending.normalizedPrompt) continue
    if (matchingUsers >= pending.baselineMatchingUserCount) return index
    matchingUsers += 1
  }
  return -1
}

function hasSubstantiveAssistantContent(message: LocalAiVisibleMessage): boolean {
  return message.content.some((part) => {
    if (part.type === 'text' || part.type === 'markdown') {
      return substantiveText(part.text)
    }
    return isSubstantiveStructuredPart(part)
  })
}

function isSubstantiveStructuredPart(part: LocalAiVisibleMessage['content'][number]): boolean {
  return !['text', 'markdown', 'citation'].includes(part.type)
}

function substantiveText(value: string): boolean {
  return hasVisibleAiMessageContent(value)
}

function normalizeLocalAiPrompt(value: string): string {
  return value.trim().replace(/\s+/g, ' ')
}
