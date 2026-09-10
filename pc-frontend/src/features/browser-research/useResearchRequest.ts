import { useCallback, useEffect, useRef, useState } from 'react'
import { heartbeatResearchHost, runResearchCommand } from './browserResearchApi'
import { createResearchEpoch, researchErrorMessage, ResearchError } from './browserResearchModel'
import type { ResearchCommand, ResearchResult } from './types'
import useLocalAiOwnerIdentity from '../user-browser/useLocalAiOwnerIdentity'
import { parseResearchHost } from './browserResearchHost'
import { getDesktopInvoke } from '../shell/desktopShell'

export function useResearchRequest(projectRoot: string, scope = '') {
  const identity = useLocalAiOwnerIdentity()
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
  useEffect(() => { setError(''); return cancel }, [cancel, projectRoot, scope, identity.ownerKey, identity.checking])
  const run = useCallback(async (command: ResearchCommand): Promise<ResearchResult | null> => {
    abort.current?.abort()
    const ticket = epoch.current.next()
    const controller = new AbortController()
    abort.current = controller
    setBusy(true)
    setError('')
    try {
      const invoke = getDesktopInvoke()
      if (!invoke || identity.checking || !identity.ownerKey || identity.ownerKey.startsWith('anonymous-session:')) {
        throw new ResearchError('host_unavailable')
      }
      // Resolve directly so a freshly loaded page does not race the bridge's first poll.
      let value: unknown
      try { value = await invoke('browser_research_host', { ownerKey: identity.ownerKey }) }
      catch { throw new ResearchError('unsupported') }
      const host = parseResearchHost(value)
      if (!epoch.current.current(ticket) || controller.signal.aborted) return null
      await heartbeatResearchHost(host)
      if (!epoch.current.current(ticket) || controller.signal.aborted) return null
      const result = await runResearchCommand(projectRoot, { ...command, instance_id: host.instance_id }, controller.signal)
      return epoch.current.current(ticket) ? result : null
    } catch (reason) {
      if (epoch.current.current(ticket)) setError(researchErrorMessage(reason))
      return null
    } finally {
      if (epoch.current.current(ticket)) { abort.current = null; setBusy(false) }
    }
  }, [projectRoot, identity.checking, identity.ownerKey])
  return { run, busy, error, cancel }
}
