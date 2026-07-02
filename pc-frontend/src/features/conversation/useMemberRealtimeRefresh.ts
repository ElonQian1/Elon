import { useEffect } from 'react'
import { useProjectStore } from './useProjectStore'

const MEMBER_REFRESH_DEBOUNCE_MS = 220
const PRESENCE_RELOAD_DEBOUNCE_MS = 900

interface ProjectMembersUpdatedEvent extends CustomEvent {
  detail: {
    projectId?: string
    action?: string
    targetUserId?: string
    actorUserId?: string
  }
}

interface PresenceEvent extends CustomEvent {
  detail: {
    userId?: string
    isOnline?: boolean
  }
}

export function useMemberRealtimeRefresh() {
  useEffect(() => {
    let refreshTimer: ReturnType<typeof setTimeout> | null = null

    function clearRefreshTimer() {
      if (refreshTimer) {
        clearTimeout(refreshTimer)
        refreshTimer = null
      }
    }

    function scheduleProjectSpaceRefresh(delay = MEMBER_REFRESH_DEBOUNCE_MS) {
      clearRefreshTimer()
      refreshTimer = setTimeout(() => {
        const { activeProjectId, reloadProjectSpace } = useProjectStore.getState()
        if (!activeProjectId) return
        reloadProjectSpace().catch(() => {})
      }, delay)
    }

    function onProjectMembersUpdated(event: ProjectMembersUpdatedEvent) {
      const { activeProjectId } = useProjectStore.getState()
      const projectId = event.detail?.projectId ?? ''
      if (!activeProjectId || projectId !== activeProjectId) return
      scheduleProjectSpaceRefresh()
    }

    function onPresence(event: PresenceEvent) {
      const userId = event.detail?.userId ?? ''
      if (!userId) return
      const { activeProjectId, members, applyMemberPresence } = useProjectStore.getState()
      if (!activeProjectId || !members.some((member) => member.user_id === userId)) return
      applyMemberPresence(userId, !!event.detail?.isOnline)
      scheduleProjectSpaceRefresh(PRESENCE_RELOAD_DEBOUNCE_MS)
    }

    window.addEventListener('elon:project-members-updated', onProjectMembersUpdated as EventListener)
    window.addEventListener('elon:presence', onPresence as EventListener)
    return () => {
      clearRefreshTimer()
      window.removeEventListener('elon:project-members-updated', onProjectMembersUpdated as EventListener)
      window.removeEventListener('elon:presence', onPresence as EventListener)
    }
  }, [])
}
