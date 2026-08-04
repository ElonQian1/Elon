import { type ComputeSettlementChallengeResolutionReceipt } from './settlementChallengeResolutionContracts'
import { type ComputeSettlementChallengeReceipt } from './settlementChallengeContracts'
import { type ComputeAttemptSettlementReceipt } from './settlementContracts'

export interface ComputePendingSettlementCorrectionCandidate {
  settlement: ComputeAttemptSettlementReceipt
  challenge: ComputeSettlementChallengeReceipt
  resolution: ComputeSettlementChallengeResolutionReceipt
  balance_effect: 'read_only_no_balance_change'
  settlement_release_effect: 'blocked_until_v199_correction'
  external_payment_effect: 'none'
}

export interface CorrectComputeAttemptSettlementBody {
  expected_challenge_id: string
  expected_challenge_event_digest: string
  expected_resolution_id: string
  expected_resolution_event_digest: string
  expected_settlement_receipt_id: string
  expected_settlement_event_digest: string
  corrected_consumer_charge_fen: number
  corrected_provider_payable_micros: number
  corrected_platform_margin_micros: number
  statement: string
  evidence_refs: string[]
  idempotency_key: string
  confirm_consumer_refund_and_pending_reversal: true
}

export interface ComputeSettlementCorrectionReceipt {
  schema: string
  correction_id: string
  challenge_id: string
  challenge_event_digest: string
  resolution_id: string
  resolution_event_digest: string
  settlement_receipt_id: string
  settlement_event_digest: string
  lease_id: string
  consumer_account_id: string
  provider_account_id: string
  platform_account_id: string
  currency: 'CNY'
  original_consumer_charge_fen: number
  original_consumer_charge_micros: number
  corrected_consumer_charge_fen: number
  corrected_consumer_charge_micros: number
  consumer_refund_fen: number
  consumer_refund_micros: number
  original_provider_payable_micros: number
  corrected_provider_payable_micros: number
  provider_reversal_micros: number
  original_platform_margin_micros: number
  corrected_platform_margin_micros: number
  platform_reversal_micros: number
  consumer_balance_after_fen: number
  provider_pending_balance_after_micros: number
  provider_account_revision_after: number
  platform_pending_balance_after_micros: number
  platform_account_revision_after: number
  statement: string
  evidence_refs: string[]
  evidence_refs_digest: string
  policy_id: string
  policy_version: number
  posting_id: string
  posting_digest: string
  request_digest: string
  event_digest: string
  corrected_by_user_id: string
  corrected_at: string
  balance_effect: 'consumer_refunded_provider_and_platform_pending_reversed'
  settlement_release_effect: 'accepted_challenge_corrected_release_net_amounts'
  replayed: boolean
}
