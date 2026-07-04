import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import type { Dispatch, SetStateAction } from 'react'
import { useProjectStore } from './useProjectStore'
import {
  listMemberConversationMessages,
  listMemberConversations,
} from './memberConversationApi'
import type { MemberConversationEntry } from './memberConversationApi'
import { hasRunningTask, messageTaskId } from './messageFlow'
import { sameMessageList } from './messageListCompare'
import type { Message } from './types'

const EVENT_REFRESH_DEBOUNCE_MS = 120
const RUNNING_POLL_MS = 1800
const SETTLE_POLL_MS = 2600
const SETTLE_REFRESH_WINDOW_MS = 12_000

interface ProjectRealtimeDetail {
  projectId?: string
  channelId?: string
  conversationId?: string
  taskId?: string
  kind?: string
}

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
  const refreshTimerRef = useRef<number | undefined>()
  const refreshSeqRef = useRef(0)
  const [settleUntil, setSettleUntil] = useState(0)
  const visibleTaskIds = useMemo(() => {
    const ids = new Set<string>()
    for (const message of displayMessages) {
      const taskId = messageTaskId(message)
      if (taskId) ids.add(taskId)
    }
    return ids
  }, [displayMessages])
  const running = useMemo(() => hasRunningTask(displayMessages), [displayMessages])

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

  useEffect(() => {
    function clearRefreshTimer() {
      if (refreshTimerRef.current) {
        window.clearTimeout(refreshTimerRef.current)
        refreshTimerRef.current = undefined
      }
    }

    function scheduleRefresh(delay = EVENT_REFRESH_DEBOUNCE_MS) {
      clearRefreshTimer()
      setSettleUntil(Date.now() + SETTLE_REFRESH_WINDOW_MS)
      refreshTimerRef.current = window.setTimeout(() => {
        refreshTimerRef.current = undefined
        refreshConversation()
      }, delay)
    }

    function matchesCurrentConversation(detail: ProjectRealtimeDetail | undefined) {
      if (!detail || !activeConversationId) return false
      if (detail.projectId && detail.projectId !== activeProjectId) return false
      if (detail.conversationId === activeConversationId) return true
      if (detail.taskId && visibleTaskIds.has(detail.taskId)) return true
      return !!detail.channelId && detail.channelId === aiDevelopmentChannelId && running
    }

    function onProjectMessageUpdated(e: Event) {
      const detail = (e as CustomEvent<ProjectRealtimeDetail>).detail
      if (matchesCurrentConversation(detail)) scheduleRefresh()
    }

    function onProjectTaskDone(e: Event) {
      const detail = (e as CustomEvent<ProjectRealtimeDetail>).detail
      if (matchesCurrentConversation(detail)) scheduleRefresh(0)
    }

    window.addEventListener('elon:project-message-updated', onProjectMessageUpdated)
    window.addEventListener('elon:project-task-done', onProjectTaskDone)
    return () => {
      clearRefreshTimer()
      window.removeEventListener('elon:project-message-updated', onProjectMessageUpdated)
      window.removeEventListener('elon:project-task-done', onProjectTaskDone)
    }
  }, [
    activeProjectId,
    activeConversationId,
    aiDevelopmentChannelId,
    visibleTaskIds,
    running,
    refreshConversation,
  ])

  useEffect(() => {
    if (!activeConversationId) return
    if (!running && Date.now() >= settleUntil) return
    const intervalMs = running ? RUNNING_POLL_MS : SETTLE_POLL_MS
    const timer = window.setInterval(() => {
      if (!running && Date.now() >= settleUntil) {
        setSettleUntil(0)
        return
      }
      refreshConversation()
    }, intervalMs)
    return () => window.clearInterval(timer)
  }, [activeConversationId, running, settleUntil, refreshConversation])

  useEffect(() => {
    if (!activeConversationId) return
    function refreshVisiblePage() {
      if (document.visibilityState === 'hidden') return
      setSettleUntil(Date.now() + SETTLE_REFRESH_WINDOW_MS)
      refreshConversation()
    }
    window.addEventListener('focus', refreshVisiblePage)
    document.addEventListener('visibilitychange', refreshVisiblePage)
    return () => {
      window.removeEventListener('focus', refreshVisiblePage)
      document.removeEventListener('visibilitychange', refreshVisiblePage)
    }
  }, [activeConversationId, refreshConversation])
}
