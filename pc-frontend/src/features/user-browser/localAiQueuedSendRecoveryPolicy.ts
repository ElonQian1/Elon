import type { QueuedLocalAiSend } from './localAiWebChatControllerConfig'

export type LocalAiQueuedSendRecoveryAction = 'ignore' | 'wait' | 'dispatch' | 'restore' | 'discard'

export function localAiQueuedSendRecoveryAction({
  queuedSend,
  requestedSessionIdentity,
  canSend,
  busy,
  expired,
}: {
  queuedSend: QueuedLocalAiSend | null
  requestedSessionIdentity: string
  canSend: boolean
  busy: boolean
  expired: boolean
}): LocalAiQueuedSendRecoveryAction {
  if (!queuedSend || queuedSend.queueReason !== 'session_resume') return 'ignore'
  if (queuedSend.sessionIdentity !== requestedSessionIdentity) return 'discard'
  if (expired) return 'restore'
  if (canSend && !busy) return 'dispatch'
  return 'wait'
}
