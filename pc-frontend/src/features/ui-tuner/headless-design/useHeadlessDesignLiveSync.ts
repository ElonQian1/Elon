import { useEffect, useState } from 'react'

export function useHeadlessDesignLiveSync(
  active: boolean,
  reload: () => Promise<unknown>,
) {
  const [lastSyncedAt, setLastSyncedAt] = useState('')
  const [error, setError] = useState('')

  useEffect(() => {
    if (!active) return
    let cancelled = false
    let timer = 0
    const tick = async () => {
      try {
        await reload()
        if (!cancelled) {
          setLastSyncedAt(new Date().toISOString())
          setError('')
        }
      } catch (reason) {
        if (!cancelled) setError(reason instanceof Error ? reason.message : '画布自动跟随失败')
      } finally {
        if (!cancelled) timer = window.setTimeout(() => { void tick() }, 1500)
      }
    }
    void tick()
    return () => {
      cancelled = true
      window.clearTimeout(timer)
    }
  }, [active, reload])

  return { active, lastSyncedAt, error }
}
