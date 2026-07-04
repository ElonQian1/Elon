/**
 * 频道消息实时刷新。
 *
 * `/ws/app` 收到项目消息更新后派发统一 realtime 事件，本 hook 只刷新当前可见频道。
 * 多条连续 AI 事件会被合并成一次短延迟刷新，避免打字流式输出时反复请求。
 */
import { useCallback, useMemo } from 'react'
import { realtimeResources } from '../realtime/resourceKeys'
import { REALTIME_SERVER_TYPES, type RealtimeEvent } from '../realtime/realtimeEvents'
import { useRealtimeResourceRefresh } from '../realtime/useRealtimeResourceRefresh'
import { useProjectStore } from './useProjectStore'
import { hasRunningTask } from './messageFlow'

export function useChannelAutoRefresh() {
  const messages = useProjectStore((state) => state.messages)
  const activeProjectId = useProjectStore((state) => state.activeProjectId)
  const activeChannelId = useProjectStore((state) => state.activeChannelId)
  const running = hasRunningTask(messages)
  const resourceKeys = useMemo(() => {
    if (!activeProjectId || !activeChannelId) return []
    return [
      realtimeResources.projectSpace(activeProjectId),
      realtimeResources.channelMessages(activeProjectId, activeChannelId),
    ]
  }, [activeProjectId, activeChannelId])

  const refreshActiveChannel = useCallback(() => {
    if (!activeProjectId || !activeChannelId) return
    useProjectStore.getState().loadMessages(activeProjectId, activeChannelId).catch(() => {})
  }, [activeProjectId, activeChannelId])

  const currentChannelMatches = useCallback((event: RealtimeEvent) => {
    if (!activeProjectId || !activeChannelId) return false
    if (event.projectId && event.projectId !== activeProjectId) return false
    if (event.type === REALTIME_SERVER_TYPES.projectMessageUpdated) {
      if (!event.channelId && event.conversationId) return false
      return !event.channelId || event.channelId === activeChannelId
    }
    return event.type === REALTIME_SERVER_TYPES.projectTaskDone || event.type === REALTIME_SERVER_TYPES.projectAiMatterEvent
  }, [activeProjectId, activeChannelId])

  useRealtimeResourceRefresh({
    enabled: !!activeProjectId && !!activeChannelId,
    running,
    resourceKeys,
    refresh: refreshActiveChannel,
    shouldRefreshEvent: currentChannelMatches,
  })
}
