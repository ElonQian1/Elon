import { useEffect, useRef } from 'react'
import type { PendingLocalAiResponse, PendingLocalAiSend } from './localAiOptimisticSend'
import { LocalAiSendFlightLedger } from './localAiSendFlightLedger'

export default function useLocalAiSendFlightLedger(
  sessionIdentity: string,
  pendingSends: PendingLocalAiSend[],
  pendingResponses: PendingLocalAiResponse[],
): LocalAiSendFlightLedger {
  const ledger = useRef(new LocalAiSendFlightLedger())

  useEffect(() => () => { ledger.current.invalidate() }, [])

  useEffect(() => {
    const retainedSendIds = new Set([
      ...pendingSends.map((pending) => pending.id),
      ...pendingResponses.map((pending) => pending.sendId),
    ])
    const active = ledger.current.activeClaim()
    if (active
      && active.sessionIdentity === sessionIdentity
      && !retainedSendIds.has(active.sendId)) {
      ledger.current.settle(sessionIdentity, active.sendId)
    }
  }, [pendingResponses, pendingSends, sessionIdentity])

  return ledger.current
}
