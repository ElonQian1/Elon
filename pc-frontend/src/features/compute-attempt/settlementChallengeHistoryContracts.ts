import { type ComputeSettlementChallengeResolutionReceipt } from './settlementChallengeResolutionContracts'
import { type ComputeSettlementChallengeReceipt } from './settlementChallengeContracts'
import { type ComputeSettlementCorrectionReceipt } from './settlementCorrectionContracts'
import { type ComputeAttemptSettlementReceipt } from './settlementContracts'

export type ComputeSettlementChallengeLifecycleStatus =
  | 'open'
  | 'withdrawn'
  | 'rejected'
  | 'accepted_pending_correction'
  | 'accepted_corrected'
  | 'withdrawn_released'
  | 'rejected_released'
  | 'accepted_corrected_released'

export type ComputeSettlementChallengeBalanceStatus =
  | 'pending_blocked'
  | 'release_pending'
  | 'corrected_pending'
  | 'available'
  | 'corrected_available'

export interface ComputeSettlementReleaseHistoryReceipt {
  schema: string
  release_id: string
  settlement_receipt_id: string
  settlement_event_digest: string
  lease_id: string
  provider_released_micros: number
  platform_released_micros: number
  event_digest: string
  released_at: string
  balance_effect: string
  withdrawal_effect: string
}

export interface ComputeSettlementChallengeHistoryItem {
  settlement: ComputeAttemptSettlementReceipt
  challenge: ComputeSettlementChallengeReceipt
  resolution: ComputeSettlementChallengeResolutionReceipt | null
  correction: ComputeSettlementCorrectionReceipt | null
  release: ComputeSettlementReleaseHistoryReceipt | null
  lifecycle_status: ComputeSettlementChallengeLifecycleStatus
  balance_status: ComputeSettlementChallengeBalanceStatus
  external_payment_effect: 'not_proven_by_settlement_challenge_history'
}

export interface ComputeSettlementChallengeHistoryResponse {
  challenge_history: ComputeSettlementChallengeHistoryItem[]
}
