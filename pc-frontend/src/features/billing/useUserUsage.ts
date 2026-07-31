import { useEffect, useState } from 'react'
import { fetchMyUsageStats, type UserUsageStats } from './usageApi'

export function useUserUsage(userId?: string, token?: string | null) {
  const [usage, setUsage] = useState<UserUsageStats | null>(null)
  const [loading, setLoading] = useState(false)

  useEffect(() => {
    if (!userId || !token) {
      setUsage(null)
      setLoading(false)
      return
    }

    let cancelled = false
    const resolvedUserId = userId

    async function refresh() {
      setLoading(true)
      try {
        const data = await fetchMyUsageStats(resolvedUserId)
        if (!cancelled) setUsage(data)
      } catch {
        if (!cancelled) setUsage(null)
      } finally {
        if (!cancelled) setLoading(false)
      }
    }

    void refresh()
    const timer = window.setInterval(refresh, 60_000)

    return () => {
      cancelled = true
      window.clearInterval(timer)
    }
  }, [userId, token])

  return { usage, loading }
}
