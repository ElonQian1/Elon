import {
  type ComputeAttemptConsumerReviewReceipt,
  type ComputeAttemptTerminalCandidateReceipt,
} from './terminalContracts'
import {
  type ComputeAttemptPlatformObservationReceipt,
  type ComputeAttemptUsageDeclarationReceipt,
  type ComputeMeterReading,
} from './platformObservationContracts'

export interface ComputePendingAttemptVerificationCandidate {
  terminal_candidate: ComputeAttemptTerminalCandidateReceipt
  provider_usage: ComputeAttemptUsageDeclarationReceipt
  consumer_review: ComputeAttemptConsumerReviewReceipt
  platform_observation: ComputeAttemptPlatformObservationReceipt
}

export type ComputeVerificationDecision = 'accepted' | 'rejected' | 'disputed'

export interface DecideComputeAttemptVerificationBody {
  expected_terminal_candidate_id: string
  expected_terminal_candidate_event_digest: string
  expected_consumer_review_id: string
  expected_consumer_review_event_digest: string
  expected_platform_observation_id: string
  expected_platform_observation_event_digest: string
  policy_id: 'conservative_min_v1'
  policy_version: 1
  decision: ComputeVerificationDecision
  reason_codes: string[]
  decision_ref: string
  idempotency_key: string
  confirm_no_state_or_settlement_effect: true
}

export interface ComputeAttemptVerificationDecisionReceipt {
  schema: string
  verification_decision_id: string
  terminal_candidate_id: string
  terminal_candidate_event_digest: string
  consumer_review_id: string
  consumer_review_event_digest: string
  platform_observation_id: string
  platform_observation_event_digest: string
  lease_id: string
  provider_id: string
  consumer_account_id: string
  source_lease_revision: number
  source_lease_digest: string
  fencing_generation: number
  job_id: string
  job_revision: number
  job_digest: string
  reservation_id: string
  reservation_revision: number
  reservation_digest: string
  capacity_claim_id: string
  capacity_claim_revision: number
  capacity_claim_digest: string
  final_usage_snapshot_id: string
  final_usage_sequence_no: number
  final_provider_usage_digest: string
  platform_observed_usage_digest: string
  candidate_outcome: string
  consumer_decision: string
  observed_outcome: string
  policy_id: string
  policy_version: number
  decision: ComputeVerificationDecision
  reason_codes: string[]
  reason_codes_digest: string
  decision_ref: string
  verified_usage: ComputeMeterReading[]
  verified_usage_digest: string
  compensable_usage: ComputeMeterReading[]
  compensable_usage_digest: string
  request_digest: string
  event_digest: string
  decided_by_user_id: string
  decided_at: string
  verification_effect: string
  execution_receipt_effect: string
  lease_effect: string
  job_effect: string
  capacity_effect: string
  reservation_effect: string
  money_effect: string
  replayed: boolean
}
