import { useEffect, useState } from 'react'
import { api } from '../../api/client'
import { useAuthStore } from '../../store/auth'

export interface UserPresenceSettings {
  user_id: string
  status: string
  custom_status?: string | null
  activity?: string | null
  updated_at?: string
}

export type PresenceStatus = 'online' | 'idle' | 'dnd' | 'invisible' | 'offline'

export function useMyPresence(enabled = true) {
  const user = useAuthStore((s) => s.user)
  const [presence, setPresence] = useState<UserPresenceSettings | null>(null)

  useEffect(() => {
    let canceled = false
    if (!enabled || !user?.id) {
      setPresence(null)
      return () => { canceled = true }
    }
    api.get<UserPresenceSettings>('/api/me/presence')
      .then((data) => {
        if (!canceled) setPresence(data)
      })
      .catch(() => {
        if (!canceled) setPresence(null)
      })
    return () => { canceled = true }
  }, [enabled, user?.id])

  useEffect(() => {
    if (!enabled || !user?.id) return
    const userId = user.id

    function onPresence(event: Event) {
      const detail = (event as CustomEvent<{
        userId?: string
        status?: string
        customStatus?: string | null
        custom_status?: string | null
        activity?: string | null
        updatedAt?: string
        updated_at?: string
      }>).detail
      if (!detail || detail.userId !== userId) return
      setPresence((current) => ({
        user_id: userId,
        status: detail.status ?? current?.status ?? 'online',
        custom_status: Object.prototype.hasOwnProperty.call(detail, 'customStatus')
          || Object.prototype.hasOwnProperty.call(detail, 'custom_status')
          ? detail.customStatus ?? detail.custom_status ?? null
          : current?.custom_status ?? null,
        activity: Object.prototype.hasOwnProperty.call(detail, 'activity')
          ? detail.activity ?? null
          : current?.activity ?? null,
        updated_at: detail.updatedAt ?? detail.updated_at ?? current?.updated_at,
      }))
    }

    window.addEventListener('elon:presence', onPresence)
    return () => window.removeEventListener('elon:presence', onPresence)
  }, [enabled, user?.id])

  return presence
}

export function normalizePresenceStatus(status?: string | null): PresenceStatus {
  const value = String(status ?? '').trim().toLowerCase()
  if (value === 'idle' || value === 'dnd' || value === 'invisible' || value === 'offline') return value
  return 'online'
}

export function visiblePresenceStatus(status?: string | null): PresenceStatus {
  const normalized = normalizePresenceStatus(status)
  return normalized === 'invisible' ? 'offline' : normalized
}

export function presenceLabel(status?: string | null): string {
  const labels: Record<PresenceStatus, string> = {
    online: '在线',
    idle: '离开',
    dnd: '勿扰',
    invisible: '隐身',
    offline: '离线',
  }
  return labels[normalizePresenceStatus(status)]
}

export function presenceSummary(presence: UserPresenceSettings | null): string {
  const extras = [
    cleanPresenceText(presence?.activity),
    cleanPresenceText(presence?.custom_status),
  ].filter(Boolean)
  return [presenceLabel(presence?.status), ...extras].join(' · ')
}

function cleanPresenceText(value?: string | null): string {
  return String(value ?? '').trim()
}
