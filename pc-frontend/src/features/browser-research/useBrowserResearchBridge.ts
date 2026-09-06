import { useEffect, useRef } from 'react'
import { getDesktopInvoke } from '../shell/desktopShell'
import useLocalAiOwnerIdentity from '../user-browser/useLocalAiOwnerIdentity'
import { claimResearchAction, pendingResearchActions, postResearchReceipt } from './browserResearchApi'
import { createResearchExecutor } from './browserResearchExecutor'

export function useBrowserResearchBridge() {
  const identity = useLocalAiOwnerIdentity()
  const owner = useRef('')
  // Never use a temporary random browser identity to select persisted research material.
  owner.current = !identity.checking && !identity.ownerKey.startsWith('anonymous-session:') ? identity.ownerKey : ''
  useEffect(() => {
    const invoke = getDesktopInvoke()
    if (!invoke) return
    const executor = createResearchExecutor({
      pending: pendingResearchActions,
      claim: claimResearchAction,
      receipt: postResearchReceipt,
      invoke: (projectKey, ownerKey, command) => invoke('run_browser_research', { projectKey, ownerKey, command }),
      owner: () => owner.current,
      now: Date.now,
    })
    void executor.poll()
    const timer = window.setInterval(() => { void executor.poll() }, 1800)
    return () => { window.clearInterval(timer); executor.dispose() }
  }, [])
}
