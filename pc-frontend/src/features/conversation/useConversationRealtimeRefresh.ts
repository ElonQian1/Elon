import { useCallback, useMemo, useRef } from 'react'
import type { Dispatch, SetStateAction } from 'react'
import { realtimeResources } from '../realtime/resourceKeys'
import { REALTIME_SERVER_TYPES, type RealtimeEvent } from '../realtime/realtimeEvents'
import { useRealtimeResourceRefresh } from '../realtime/useRealtimeResourceRefresh'
import { useProjectStore } from './useProjectStore'
import {
  listMemberConversationMessages,
  listMemberConversations,
} from './memberConversationApi'
import type { MemberConversationEntry } from './memberConversationApi'
import { hasRunningTask, messageTaskId } from './messageFlow'
import { sameMessageList } from './messageListCompare'
import type { Message } from './types'

interface Options {
  activeProjectId: string
  activeConversationTargetId: string
  sessionView: string | 'new' | null
  aiDevelopmentChannelId: string
  activeChannelId: string
  displayMessages: Message[]
  loadTaskMessages: (projectId: string, channelId: string, force?: boolean) => Promise<Message[]>
  writeConversationCache: (
    projectId: string,
    targetUserId: string,
    conversationId: string,
    nextMessages: Message[],
    nextTaskMessages: Message[],
  ) => void
  setConvMessages: Dispatch<SetStateAction<Message[]>>
  setSessionTaskMessages: Dispatch<SetStateAction<Message[]>>
  setMemberConversations: Dispatch<SetStateAction<MemberConversationEntry[]>>
}

export function useConversationRealtimeRefresh({
  activeProjectId,
  activeConversationTargetId,
  sessionView,
  aiDevelopmentChannelId,
  activeChannelId,
  displayMessages,
  loadTaskMessages,
  writeConversationCache,
  setConvMessages,
  setSessionTaskMessages,
  setMemberConversations,
}: Options) {
  const activeConversationId = typeof sessionView === 'string' && sessionView !== 'new' ? sessionView : ''
  const refreshSeqRef = useRef(0)
  const visibleTaskIds = useMemo(() => {
    const ids = new Set<string>()
    for (const message of displayMessages) {
      const taskId = messageTaskId(message)
      if (taskId) ids.add(taskId)
    }
    return ids
  }, [displayMessages])
  const running = useMemo(() => hasRunningTask(displayMessages), [displayMessages])
  const resourceKeys = useMemo(() => {
    if (!activeProjectId || !activeConversationTargetId || !activeConversationId) return []
    return [
      realtimeResources.projectSpace(activeProjectId),
      realtimeResources.conversationAny(activeProjectId, activeConversationId),
      realtimeResources.conversationMessages(activeProjectId, activeConversationTargetId, activeConversationId),
      realtimeResources.conversationList(activeProjectId, activeConversationTargetId),
      ...Array.from(visibleTaskIds, (taskId) => realtimeResources.taskTimeline(activeProjectId, taskId)),
    ]
  }, [activeProjectId, activeConversationTargetId, activeConversationId, visibleTaskIds])

  const refreshConversation = useCallback(async () => {
    if (!activeProjectId || !activeConversationTargetId || !activeConversationId) return
    const requestSeq = refreshSeqRef.current + 1
    refreshSeqRef.current = requestSeq
    const taskMessagesPromise = aiDevelopmentChannelId
      ? loadTaskMessages(activeProjectId, aiDevelopmentChannelId, true)
      : Promise.resolve([] as Message[])
    const channelMessagesPromise = activeChannelId
      ? useProjectStore.getState().loadMessages(activeProjectId, activeChannelId)
      : Promise.resolve()
    try {
      const [conversationMessages, taskMessages, conversations] = await Promise.all([
        listMemberConversationMessages(
          activeProjectId,
          activeConversationTargetId,
          activeConversationId,
        ) as Promise<Message[]>,
        taskMessagesPromise,
        listMemberConversations(activeProjectId, activeConversationTargetId),
        channelMessagesPromise,
      ]).then(([conversationMessages, taskMessages, conversations]) => [
        conversationMessages,
        taskMessages,
        conversations,
      ] as const)
      if (refreshSeqRef.current !== requestSeq) return
      writeConversationCache(activeProjectId, activeConversationTargetId, activeConversationId, conversationMessages, taskMessages)
      setConvMessages((prev) => sameMessageList(prev, conversationMessages) ? prev : conversationMessages)
      setSessionTaskMessages((prev) => sameMessageList(prev, taskMessages) ? prev : taskMessages)
      setMemberConversations(conversations)
    } catch (err) {
      console.warn('[ConversationRealtime] refresh failed:', err)
    }
  }, [
    activeProjectId,
    activeConversationTargetId,
    activeConversationId,
    aiDevelopmentChannelId,
    activeChannelId,
    loadTaskMessages,
    writeConversationCache,
    setConvMessages,
    setSessionTaskMessages,
    setMemberConversations,
  ])

  const matchesCurrentConversation = useCallback((event: RealtimeEvent) => {
    if (!activeConversationId) return false
    if (event.projectId && event.projectId !== activeProjectId) return false
    if (event.conversationId === activeConversationId) return true
    if (event.taskId && visibleTaskIds.has(event.taskId)) return true
    if (event.type === REALTIME_SERVER_TYPES.projectTaskDone) return true
    return !!event.channelId && event.channelId === aiDevelopmentChannelId && running
  }, [
    activeProjectId,
    activeConversationId,
    aiDevelopmentChannelId,
    visibleTaskIds,
    running,
  ])

  useRealtimeResourceRefresh({
    enabled: !!activeConversationId,
    running,
    debounceMs: 120,
    resourceKeys,
    refresh: refreshConversation,
    shouldRefreshEvent: matchesCurrentConversation,
  })
}
