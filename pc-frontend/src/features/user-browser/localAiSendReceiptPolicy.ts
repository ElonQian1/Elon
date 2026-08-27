import type { LocalAiCommandResult } from './localAiBrowserApi'

export type LocalAiSendReceiptDecision = 'accepted' | 'rejected' | 'reconcile' | 'restore'

export function localAiSendReceiptDecision({
  commandQueued,
  requestId,
  receipt,
}: {
  commandQueued: boolean
  requestId: string
  receipt?: LocalAiCommandResult | null
}): LocalAiSendReceiptDecision {
  if (!commandQueued) return 'restore'
  if (receipt?.action !== 'send_prompt' || receipt.requestId !== requestId) return 'reconcile'
  return receipt.ok ? 'accepted' : 'rejected'
}
