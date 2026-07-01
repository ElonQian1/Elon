/**
 * 频道消息实时刷新。
 *
 * `/ws/app` 收到项目消息更新后派发前端事件，本 hook 只刷新当前可见频道。
 * 多条连续 AI 事件会被合并成一次短延迟刷新，避免打字流式输出时反复请求。
 */
import { useEffect } from 'react'
import { useProjectStore } from './useProjectStore'

const REFRESH_DEBOUNCE_MS = 160

interface ProjectMessageUpdatedEvent extends CustomEvent {
  detail: {
    projectId?: string
    channelId?: string
    conversationId?: string
    kind?: string
  }
}

interface TaskDoneEvent extends CustomEvent {
  detail: { projectId?: string; conversationId?: string }
}

interface GroupAiMatterEvent extends CustomEvent {
  detail: { projectId?: string; matterId?: string; matterEventType?: string }
}

export function useChannelAutoRefresh() {
  useEffect(() => {
    let refreshTimer: ReturnType<typeof setTimeout> | null = null

    function clearRefreshTimer() {
      if (refreshTimer) {
        clearTimeout(refreshTimer)
        refreshTimer = null
      }
    }

    function scheduleChannelRefresh(delay = REFRESH_DEBOUNCE_MS) {
      clearRefreshTimer()
      refreshTimer = setTimeout(() => {
        const { activeProjectId, activeChannelId, loadMessages } = useProjectStore.getState()
        if (!activeProjectId || !activeChannelId) return
        loadMessages(activeProjectId, activeChannelId).catch(() => {})
      }, delay)
    }

    function currentChannelMatches(projectId?: string, channelId?: string) {
      const { activeProjectId, activeChannelId } = useProjectStore.getState()
      if (!activeProjectId || !activeChannelId) return false
      if (projectId && projectId !== activeProjectId) return false
      if (channelId && channelId !== activeChannelId) return false
      return true
    }

    function onProjectMessageUpdated(e: ProjectMessageUpdatedEvent) {
      const { projectId, channelId, conversationId } = e.detail ?? {}
      if (!channelId && conversationId) return
      if (currentChannelMatches(projectId, channelId)) scheduleChannelRefresh()
    }

    function onTaskDone(e: TaskDoneEvent) {
      const eventProjectId = e.detail?.projectId ?? ''
      if (currentChannelMatches(eventProjectId, undefined)) scheduleChannelRefresh(0)
    }

    function onGroupAiMatterEvent(e: GroupAiMatterEvent) {
      const eventProjectId = e.detail?.projectId ?? ''
      if (currentChannelMatches(eventProjectId, undefined)) scheduleChannelRefresh()
    }

    window.addEventListener('elon:project-message-updated', onProjectMessageUpdated as EventListener)
    window.addEventListener('elon:project-task-done', onTaskDone as EventListener)
    window.addEventListener('elon:project-ai-matter-event', onGroupAiMatterEvent as EventListener)
    return () => {
      clearRefreshTimer()
      window.removeEventListener('elon:project-message-updated', onProjectMessageUpdated as EventListener)
      window.removeEventListener('elon:project-task-done', onTaskDone as EventListener)
      window.removeEventListener('elon:project-ai-matter-event', onGroupAiMatterEvent as EventListener)
    }
  }, [])
}
