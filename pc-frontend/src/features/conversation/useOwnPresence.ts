import { useCallback, useEffect, useState } from 'react'
import { api } from '../../api/client'
import type { User } from '../../store/auth'
import type { UserPresenceSettings } from './types'

interface PresenceEventDetail {
  userId?: string
  status?: string
  customStatus?: string | null
  custom_status?: string | null
  activity?: string | null
  updatedAt?: string
  updated_at?: string
}

export function useOwnPresence(
  user: User | null,
  reloadProjectSpace: () => Promise<void>,
) {
  const [myPresence, setMyPresence] = useState<UserPresenceSettings | null>(null)

  const handlePresenceSaved = useCallback(async (presence: UserPresenceSettings) => {
    setMyPresence(presence)
    await reloadProjectSpace()
  }, [reloadProjectSpace])

  useEffect(() => {
    let canceled = false
    if (!user?.id) {
      setMyPresence(null)
      return () => { canceled = true }
    }
    api.get<UserPresenceSettings>('/api/me/presence')
      .then((presence) => {
        if (!canceled) setMyPresence(presence)
      })
      .catch(() => {
        if (!canceled) setMyPresence(null)
      })
    return () => { canceled = true }
  }, [user?.id])

  useEffect(() => {
    if (!user?.id) return
    const userId = user.id
    function onPresence(event: Event) {
      const detail = (event as CustomEvent<PresenceEventDetail>).detail
      if (!detail || detail.userId !== userId) return
      setMyPresence((prev) => ({
        user_id: userId,
        status: detail.status ?? prev?.status ?? 'online',
        custom_status: Object.prototype.hasOwnProperty.call(detail, 'customStatus')
          || Object.prototype.hasOwnProperty.call(detail, 'custom_status')
          ? detail.customStatus ?? detail.custom_status ?? null
          : prev?.custom_status ?? null,
        activity: Object.prototype.hasOwnProperty.call(detail, 'activity')
          ? detail.activity ?? null
          : prev?.activity ?? null,
        updated_at: detail.updatedAt ?? detail.updated_at ?? prev?.updated_at,
      }))
    }
    window.addEventListener('elon:presence', onPresence)
    return () => window.removeEventListener('elon:presence', onPresence)
  }, [user?.id])

  return { myPresence, handlePresenceSaved }
}
