import { useEffect, useMemo, useRef, useState } from 'react'
import {
  getCachedLocalAiWebSessionState,
  getLocalAiWebSessionState,
  type LocalAiWebProvider,
  type LocalAiWebSessionState,
} from './localAiBrowserApi'
import {
  initializeLocalAiProviderActivity,
  updateLocalAiProviderActivity,
  type LocalAiProviderActivity,
} from './localAiProviderActivity'

const ACTIVE_BACKGROUND_DELAY_MS = 1_500
const IDLE_BACKGROUND_DELAY_MS = 8_000
const HIDDEN_BACKGROUND_DELAY_MS = 15_000

interface LocalAiProviderActivityOptions {
  enabled: boolean
  providers: LocalAiWebProvider[]
  selectedProviderId?: string
  ownerKey: string
  selectedState: LocalAiWebSessionState | null
}

export default function useLocalAiProviderActivity({
  enabled,
  providers,
  selectedProviderId,
  ownerKey,
  selectedState,
}: LocalAiProviderActivityOptions): Record<string, LocalAiProviderActivity> {
  const providerIds = useMemo(() => providers.map((item) => item.id), [providers])
  const providerKey = providerIds.join('|')
  const [activities, setActivities] = useState<Record<string, LocalAiProviderActivity>>({})
  const activitiesRef = useRef(activities)
  activitiesRef.current = activities

  useEffect(() => {
    const initial = Object.fromEntries(providerIds.map((providerId) => [
      providerId,
      initializeLocalAiProviderActivity(getCachedLocalAiWebSessionState(providerId, ownerKey)),
    ]))
    activitiesRef.current = initial
    setActivities(initial)
  }, [ownerKey, providerKey])

  useEffect(() => {
    if (!selectedProviderId) return
    setActivities((current) => {
      const updated = updateActivity(current, selectedProviderId, selectedState, true)
      activitiesRef.current = updated
      return updated
    })
  }, [selectedProviderId, selectedState])

  useEffect(() => {
    if (!enabled || !ownerKey || providerIds.length < 2) return
    let active = true
    let polling = false
    let timer = 0

    const schedule = () => {
      if (!active) return
      const inactiveActivities = providerIds
        .filter((providerId) => providerId !== selectedProviderId)
        .map((providerId) => activitiesRef.current[providerId])
      const delay = document.visibilityState === 'hidden'
        ? HIDDEN_BACKGROUND_DELAY_MS
        : inactiveActivities.some((item) => item?.phase === 'streaming')
          ? ACTIVE_BACKGROUND_DELAY_MS
          : IDLE_BACKGROUND_DELAY_MS
      window.clearTimeout(timer)
      timer = window.setTimeout(() => void poll(), delay)
    }
    const poll = async () => {
      if (!active || polling) return
      polling = true
      window.clearTimeout(timer)
      const inactiveProviderIds = providerIds.filter((providerId) => providerId !== selectedProviderId)
      const results = await Promise.all(inactiveProviderIds.map(async (providerId) => {
        try {
          return [providerId, await getLocalAiWebSessionState(providerId, ownerKey)] as const
        } catch {
          return [providerId, null] as const
        }
      }))
      if (active) {
        setActivities((current) => {
          const updated = results.reduce(
            (next, [providerId, state]) => updateActivity(next, providerId, state, false),
            current,
          )
          activitiesRef.current = updated
          return updated
        })
      }
      polling = false
      schedule()
    }
    const refreshWhenVisible = () => {
      if (document.visibilityState !== 'visible' || polling) return
      window.clearTimeout(timer)
      void poll()
    }

    document.addEventListener('visibilitychange', refreshWhenVisible)
    window.addEventListener('focus', refreshWhenVisible)
    void poll()
    return () => {
      active = false
      window.clearTimeout(timer)
      document.removeEventListener('visibilitychange', refreshWhenVisible)
      window.removeEventListener('focus', refreshWhenVisible)
    }
  }, [enabled, ownerKey, providerKey, selectedProviderId])

  return activities
}

function updateActivity(
  current: Record<string, LocalAiProviderActivity>,
  providerId: string,
  state: LocalAiWebSessionState | null,
  selected: boolean,
): Record<string, LocalAiProviderActivity> {
  const alignedState = state?.providerId === providerId ? state : null
  const previous = current[providerId] ?? initializeLocalAiProviderActivity(alignedState)
  const next = updateLocalAiProviderActivity(previous, alignedState, selected)
  if (sameActivity(previous, next)) return current
  const updated = { ...current, [providerId]: next }
  return updated
}

function sameActivity(left: LocalAiProviderActivity, right: LocalAiProviderActivity): boolean {
  return left.phase === right.phase
    && left.label === right.label
    && left.unread === right.unread
    && left.observedSemanticAtMs === right.observedSemanticAtMs
    && left.lastAssistantId === right.lastAssistantId
}
