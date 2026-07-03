import { useEffect, useState } from 'react'
import { fetchMyProgression, type UserProgressionSummary } from './progressionApi'

export function useUserProgression(userId?: string, token?: string | null) {
  const [progression, setProgression] = useState<UserProgressionSummary | null>(null)

  useEffect(() => {
    if (!userId || !token) {
      setProgression(null)
      return
    }

    let cancelled = false

    async function refresh() {
      try {
        const data = await fetchMyProgression()
        if (!cancelled) setProgression(data)
      } catch {
        if (!cancelled) setProgression(null)
      }
    }

    void refresh()
    const timer = window.setInterval(refresh, 60_000)

    return () => {
      cancelled = true
      window.clearInterval(timer)
    }
  }, [userId, token])

  return progression
}
