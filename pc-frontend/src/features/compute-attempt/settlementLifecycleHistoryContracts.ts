import {
  type ComputeSettlementReleaseHistoryReceipt,
} from './settlementChallengeHistoryContracts'
import { type ComputeSettlementChallengeResolutionReceipt } from './settlementChallengeResolutionContracts'
import { type ComputeSettlementChallengeReceipt } from './settlementChallengeContracts'
import { type ComputeSettlementCorrectionReceipt } from './settlementCorrectionContracts'
import { type ComputeAttemptSettlementReceipt } from './settlementContracts'

export type ComputeSettlementLifecycleStatus =
  | 'unchallenged_pending'
  | 'unchallenged_released'
  | 'open'
  | 'withdrawn'
  | 'rejected'
  | 'accepted_pending_correction'
  | 'accepted_corrected'
  | 'withdrawn_released'
  | 'rejected_released'
  | 'accepted_corrected_released'

export type ComputeSettlementLifecycleBalanceStatus =
  | 'pending'
  | 'available'
  | 'pending_blocked'
  | 'release_pending'
  | 'corrected_pending'
  | 'corrected_available'

export interface ComputeSettlementLifecycleHistoryItem {
  settlement: ComputeAttemptSettlementReceipt
  challenge: ComputeSettlementChallengeReceipt | null
  resolution: ComputeSettlementChallengeResolutionReceipt | null
  correction: ComputeSettlementCorrectionReceipt | null
  release: ComputeSettlementReleaseHistoryReceipt | null
  lifecycle_status: ComputeSettlementLifecycleStatus
  balance_status: ComputeSettlementLifecycleBalanceStatus
  external_payment_effect: 'not_proven_by_settlement_lifecycle_history'
}

export interface ComputeSettlementLifecycleHistoryResponse {
  settlement_history: ComputeSettlementLifecycleHistoryItem[]
}
