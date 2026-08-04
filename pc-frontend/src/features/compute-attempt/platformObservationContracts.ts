import { type ComputeAttemptTerminalCandidateReceipt } from './terminalContracts'

export interface ComputeMeterReading {
  meter: string
  quantity: number
  source_kind: string
  source_id: string
  reading_digest: string
  observed_at: string
}

export interface ComputeAttemptUsageDeclarationReceipt {
  schema: string
  snapshot_id: string
  lease_id: string
  provider_id: string
  consumer_account_id: string
  sequence_no: number
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
  executor_usage_ref: string
  cumulative_declared_usage: ComputeMeterReading[]
  cumulative_usage_digest: string
  reserved_contract: Array<{ meter: string; reserved_quantity: number }>
  reserved_contract_digest: string
  overage_meters: string[]
  request_digest: string
  event_digest: string
  declared_by_user_id: string
  declared_at: string
  verification_status: string
  execution_effect: string
  capacity_effect: string
  reservation_effect: string
  money_effect: string
  replayed: boolean
}

export interface ComputePendingPlatformObservationCandidate {
  terminal_candidate: ComputeAttemptTerminalCandidateReceipt
  provider_usage: ComputeAttemptUsageDeclarationReceipt
}

export type ComputeObservationSource = 'control_plane' | 'transport_gateway' | 'server_metering'
export type ComputeObservedOutcome = 'succeeded' | 'failed' | 'canceled' | 'indeterminate'

export interface ObserveComputeAttemptTerminalCandidateBody {
  expected_terminal_candidate_id: string
  expected_terminal_candidate_event_digest: string
  observation_source: ComputeObservationSource
  observer_ref: string
  observed_outcome: ComputeObservedOutcome
  cumulative_observed_usage: Array<{ meter: string; cumulative_quantity: number }>
  evidence_refs: string[]
  idempotency_key: string
  confirm_platform_observation_only: true
}

export interface ComputeAttemptPlatformObservationReceipt {
  schema: string
  platform_observation_id: string
  terminal_candidate_id: string
  terminal_candidate_event_digest: string
  lease_id: string
  provider_id: string
  consumer_account_id: string
  job_id: string
  final_usage_snapshot_id: string
  final_usage_sequence_no: number
  final_provider_usage_digest: string
  candidate_outcome: string
  observation_source: ComputeObservationSource
  observer_ref: string
  observed_outcome: ComputeObservedOutcome
  cumulative_observed_usage: ComputeMeterReading[]
  cumulative_observed_usage_digest: string
  variance_meters: string[]
  variance_meters_digest: string
  evidence_refs: string[]
  evidence_refs_digest: string
  request_digest: string
  event_digest: string
  observed_by_user_id: string
  observed_at: string
  evidence_status: string
  observation_effect: string
  verification_effect: string
  lease_effect: string
  job_effect: string
  capacity_effect: string
  reservation_effect: string
  money_effect: string
  replayed: boolean
}
