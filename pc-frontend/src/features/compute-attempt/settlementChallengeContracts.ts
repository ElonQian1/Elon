import { type ComputeAttemptSettlementReceipt } from './settlementContracts'

export type ComputeSettlementChallengeReasonCode =
  | 'amount'
  | 'metering'
  | 'price_snapshot'
  | 'execution_evidence'
  | 'provider_identity'
  | 'other'

export interface ComputePendingSettlementChallengeCandidate {
  settlement: ComputeAttemptSettlementReceipt
  challenge_deadline: string
  balance_effect: 'provider_and_platform_pending_unchanged'
  settlement_release_effect: 'blocked_by_open_challenge'
  external_payment_effect: 'none'
}

export interface OpenComputeSettlementChallengeBody {
  expected_settlement_receipt_id: string
  expected_settlement_event_digest: string
  expected_posting_id: string
  expected_posting_digest: string
  reason_code: ComputeSettlementChallengeReasonCode
  summary: string
  evidence_refs: string[]
  idempotency_key: string
  confirm_pending_release_block: true
}

export interface ComputeSettlementChallengeReceipt {
  schema: string
  challenge_id: string
  settlement_receipt_id: string
  settlement_event_digest: string
  lease_id: string
  consumer_account_id: string
  provider_account_id: string
  posting_id: string
  posting_digest: string
  policy_id: string
  policy_version: number
  challenge_deadline: string
  status: 'open'
  reason_code: ComputeSettlementChallengeReasonCode
  summary: string
  evidence_refs: string[]
  evidence_refs_digest: string
  request_digest: string
  event_digest: string
  opened_by_user_id: string
  opened_at: string
  balance_effect: 'provider_and_platform_pending_unchanged'
  settlement_release_effect: 'blocked_by_open_challenge'
  replayed: boolean
}
