import { useEffect, useMemo, useState } from 'react'
import { fetchNodeAgentVersion } from './nodeHelpers'
import type { NodeAgentVersion } from './types'
import { buildWinClientUpdateState } from './winClientUpdateModel'

export function useWinClientUpdateCheck(
  localVersion: string | null | undefined,
  enabled: boolean,
  localGitSha?: string | null,
) {
  const [latest, setLatest] = useState<NodeAgentVersion | null>(null)

  useEffect(() => {
    if (!enabled) {
      setLatest(null)
      return
    }
    let canceled = false
    async function load() {
      try {
        const data = await fetchNodeAgentVersion()
        if (!canceled) setLatest(data)
      } catch {
        if (!canceled) setLatest(null)
      }
    }
    load()
    const timer = window.setInterval(load, 120_000)
    return () => {
      canceled = true
      window.clearInterval(timer)
    }
  }, [enabled])

  return useMemo(
    () => buildWinClientUpdateState(localVersion, localGitSha, latest),
    [localVersion, localGitSha, latest],
  )
}
