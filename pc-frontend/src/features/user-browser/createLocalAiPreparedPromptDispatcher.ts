import type { MutableRefObject } from 'react'
import type { LocalAiWebProvider, LocalAiWebSessionState } from './localAiBrowserApi'
import { dispatchPreparedLocalAiPrompt } from './dispatchPreparedLocalAiPrompt'
import type { LocalAiSendFlightLedger } from './localAiSendFlightLedger'
import type { QueuedLocalAiSend } from './localAiWebChatControllerConfig'

interface LocalAiPreparedPromptDispatcherOptions {
  provider: LocalAiWebProvider | undefined
  ownerKey: string
  requestedSessionIdentity: string
  activeSessionIdentity: MutableRefObject<string>
  ledger: LocalAiSendFlightLedger
  restore: (prepared: QueuedLocalAiSend) => void
  onBusyAction: (action: string) => void
  onMessage: (message: string) => void
  onState: (state: LocalAiWebSessionState) => void
  onResponseRefresh: (prompt: string, baselineMatchingUserCount: number) => void
}

export function createLocalAiPreparedPromptDispatcher({
  provider,
  ownerKey,
  requestedSessionIdentity,
  activeSessionIdentity,
  ledger,
  restore,
  onBusyAction,
  onMessage,
  onState,
  onResponseRefresh,
}: LocalAiPreparedPromptDispatcherOptions) {
  return (prepared: QueuedLocalAiSend): Promise<LocalAiWebSessionState | null> => {
    const claim = ledger.current(prepared.sessionIdentity, prepared.pending.id)
    if (!claim) return Promise.resolve(null)
    const ownsCurrentSession = () => activeSessionIdentity.current === claim.sessionIdentity
    return dispatchPreparedLocalAiPrompt({
      provider,
      ownerKey,
      requestedSessionIdentity,
      prepared,
      restore,
      onBusyAction,
      onMessage,
      onState,
      onResponseRefresh,
      isCurrent: () => ownsCurrentSession() && ledger.isCurrent(claim),
      isGenerationCurrent: () => ownsCurrentSession() && ledger.isGenerationCurrent(claim),
    })
  }
}
