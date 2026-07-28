import { useCallback, useRef } from 'react'
import { api } from '../../api/client'
import type { ChannelMessagesResponse, Message } from './types'

export const CONVERSATION_CACHE_FRESH_MS = 12000
export const CACHED_CONVERSATION_REFRESH_DELAY_MS = 450

const TASK_MESSAGE_CACHE_FRESH_MS = 4000

interface ConversationMessageCacheEntry {
  messages: Message[]
  taskMessages: Message[]
  loadedAt: number
}

interface TaskMessageCacheEntry {
  messages: Message[]
  loadedAt: number
}

export function useConversationMessageCache() {
  const conversationMessageCacheRef = useRef<Map<string, ConversationMessageCacheEntry>>(new Map())
  const taskMessageCacheRef = useRef<Map<string, TaskMessageCacheEntry>>(new Map())

  const clearConversationCaches = useCallback(() => {
    conversationMessageCacheRef.current.clear()
    taskMessageCacheRef.current.clear()
  }, [])

  const getConversationCache = useCallback((
    projectId: string,
    targetUserId: string,
    conversationId: string,
  ): ConversationMessageCacheEntry | undefined => {
    return conversationMessageCacheRef.current.get(conversationMessageCacheKey(projectId, targetUserId, conversationId))
  }, [])

  const writeConversationCache = useCallback((
    projectId: string,
    targetUserId: string,
    conversationId: string,
    nextMessages: Message[],
    nextTaskMessages: Message[],
  ) => {
    conversationMessageCacheRef.current.set(
      conversationMessageCacheKey(projectId, targetUserId, conversationId),
      { messages: nextMessages, taskMessages: nextTaskMessages, loadedAt: Date.now() },
    )
  }, [])

  const touchConversationCache = useCallback((
    projectId: string,
    targetUserId: string,
    conversationId: string,
    cached: ConversationMessageCacheEntry,
  ) => {
    conversationMessageCacheRef.current.set(
      conversationMessageCacheKey(projectId, targetUserId, conversationId),
      { ...cached, loadedAt: Date.now() },
    )
  }, [])

  const loadCachedTaskMessages = useCallback(async (
    projectId: string,
    channelId: string,
    force = false,
  ): Promise<Message[]> => {
    if (!projectId || !channelId) return []
    const key = taskMessageCacheKey(projectId, channelId)
    const cached = taskMessageCacheRef.current.get(key)
    if (!force && cached && Date.now() - cached.loadedAt < TASK_MESSAGE_CACHE_FRESH_MS) {
      return cached.messages
    }
    const messages = await loadAiDevelopmentTaskMessages(projectId, channelId)
    taskMessageCacheRef.current.set(key, { messages, loadedAt: Date.now() })
    return messages
  }, [])

  return {
    clearConversationCaches,
    getConversationCache,
    loadCachedTaskMessages,
    touchConversationCache,
    writeConversationCache,
  }
}

async function loadAiDevelopmentTaskMessages(projectId: string, channelId: string): Promise<Message[]> {
  if (!projectId || !channelId) return []
  const data = await api.get<ChannelMessagesResponse>(
    `/api/projects/${encodeURIComponent(projectId)}/channels/${encodeURIComponent(channelId)}/messages?limit=200`,
  )
  return data.messages ?? []
}

function conversationMessageCacheKey(projectId: string, targetUserId: string, conversationId: string): string {
  return `${projectId}::${targetUserId}::${conversationId}`
}

function taskMessageCacheKey(projectId: string, channelId: string): string {
  return `${projectId}::${channelId}`
}
