export interface ComputeDeclaredResultArtifactInput {
  artifact_id: string
  digest_algorithm: 'sha256'
  digest: string
  media_type: string
  size_bytes: number
  location_ref: string
  encryption_profile: string | null
}

export interface ComputeAttemptTerminalCandidateReceipt {
  schema: string
  terminal_candidate_id: string
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
  final_cumulative_usage_digest: string
  executor_terminal_ref: string
  outcome: 'succeeded' | 'failed' | 'canceled'
  reason_code: string
  diagnostic_ref: string | null
  output_digest: string | null
  result_artifacts: ComputeDeclaredResultArtifactInput[]
  result_artifacts_digest: string
  request_digest: string
  event_digest: string
  declared_by_user_id: string
  declared_at: string
  verification_status: string
  execution_effect: string
  lease_effect: string
  job_effect: string
  capacity_effect: string
  reservation_effect: string
  money_effect: string
  replayed: boolean
}

export type ComputeConsumerReviewDecision = 'accepted' | 'rejected' | 'disputed'

export interface ReviewComputeAttemptTerminalCandidateBody {
  expected_terminal_candidate_id: string
  expected_terminal_candidate_event_digest: string
  decision: ComputeConsumerReviewDecision
  reason_code: string
  consumer_review_ref: string
  evidence_refs: string[]
  idempotency_key: string
  confirm_consumer_attestation_only: true
}

export interface ComputeAttemptConsumerReviewReceipt {
  schema: string
  consumer_review_id: string
  terminal_candidate_id: string
  terminal_candidate_event_digest: string
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
  final_cumulative_usage_digest: string
  candidate_outcome: string
  decision: ComputeConsumerReviewDecision
  reason_code: string
  consumer_review_ref: string
  evidence_refs: string[]
  evidence_refs_digest: string
  request_digest: string
  event_digest: string
  reviewed_by_user_id: string
  reviewed_at: string
  evidence_status: string
  review_effect: string
  verification_effect: string
  lease_effect: string
  job_effect: string
  capacity_effect: string
  reservation_effect: string
  money_effect: string
  replayed: boolean
}
