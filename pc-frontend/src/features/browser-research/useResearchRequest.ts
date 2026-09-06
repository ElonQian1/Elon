import { useCallback, useEffect, useRef, useState } from 'react'
import { runResearchCommand } from './browserResearchApi'
import { createResearchEpoch, researchErrorMessage } from './browserResearchModel'
import type { ResearchCommand, ResearchResult } from './types'

export function useResearchRequest(projectRoot: string, scope = '') {
  const epoch = useRef(createResearchEpoch())
  const abort = useRef<AbortController | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const cancel = useCallback(() => {
    epoch.current.next()
    abort.current?.abort()
    abort.current = null
    setBusy(false)
  }, [])
  useEffect(() => { setError(''); return cancel }, [cancel, projectRoot, scope])
  const run = useCallback(async (command: ResearchCommand): Promise<ResearchResult | null> => {
    abort.current?.abort()
    const ticket = epoch.current.next()
    const controller = new AbortController()
    abort.current = controller
    setBusy(true)
    setError('')
    try {
      const result = await runResearchCommand(projectRoot, command, controller.signal)
      return epoch.current.current(ticket) ? result : null
    } catch (reason) {
      if (epoch.current.current(ticket)) setError(researchErrorMessage(reason))
      return null
    } finally {
      if (epoch.current.current(ticket)) { abort.current = null; setBusy(false) }
    }
  }, [projectRoot])
  return { run, busy, error, cancel }
}
