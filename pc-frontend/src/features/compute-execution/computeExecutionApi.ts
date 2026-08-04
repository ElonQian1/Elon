import { api } from '../../api/client'
import {
  type ComputeCapacityClaimBinding,
  type ComputeJobVersionBinding,
  type ComputeReservationReceipt,
} from '../compute-market/computeMarketApi'

export interface ComputeAttemptLease {
  schema: string
  lease_id: string
  job_id: string
  reservation_id: string
  attempt_no: number
  shard_id: string | null
  provider_id: string
  executor_id: string
  status: string
  fencing_generation: number
  lease_credential_ref: string
  lease_credential_hint: string
  latest_checkpoint: unknown | null
  issued_at: string
  last_heartbeat_at: string | null
  expires_at: string
  hard_deadline_at: string
  terminal_reason_code: string | null
}

export interface ActivateComputeAttemptBody {
  lease_id: string
  reservation_id: string
  executor_id: string
  shard_id: string | null
  attempt_no: 1
  fencing_generation: 1
  executor_acceptance_ref: string
  lease_credential_ref: string
  lease_credential_hint: string
  expected_job_revision: number
  expected_job_digest: string
  expected_reservation_revision: number
  expected_reservation_digest: string
  expected_claim_revision: number
  expected_claim_digest: string
  expires_at: string
  hard_deadline_at: string
  idempotency_key: string
  confirm_executor_accepted: true
}

export interface ComputeAttemptActivationReceipt {
  lease: ComputeAttemptLease
  lease_digest: string
  request_digest: string
  executor_acceptance_ref: string
  source_job: ComputeJobVersionBinding
  running_job: ComputeJobVersionBinding
  source_reservation_revision: number
  source_reservation_digest: string
  active_reservation_revision: number
  active_reservation_digest: string
  source_claim: ComputeCapacityClaimBinding
  active_claim: ComputeCapacityClaimBinding
  budget_reservation_id: string
  budget_reserved_fen: number
  activated_by_user_id: string
  activated_at: string
  execution_effect: 'none'
  money_effect: 'preauthorization_unchanged'
  replayed: boolean
}

export interface ComputeAttemptLeaseStateReceipt {
  schema: string
  lease: ComputeAttemptLease
  lease_revision: number
  lease_digest: string
  updated_by_user_id: string
  updated_at: string
}

export interface ComputeAttemptLeaseRenewalReceipt {
  schema: string
  renewal_id: string
  previous_lease_revision: number
  previous_lease_digest: string
  state: ComputeAttemptLeaseStateReceipt
  executor_heartbeat_ref: string
  event_digest: string
  renewed_at: string
  execution_effect: string
  capacity_effect: string
  reservation_effect: string
  money_effect: string
  replayed: boolean
}

export interface RenewComputeAttemptLeaseBody {
  expected_lease_revision: number
  expected_lease_digest: string
  expected_fencing_generation: number
  executor_heartbeat_ref: string
  expires_at: string
  idempotency_key: string
  confirm_executor_alive: true
}

export interface AbortComputeAttemptBody {
  expected_lease_revision: number
  expected_lease_digest: string
  expected_fencing_generation: number
  expected_job_revision: number
  expected_job_digest: string
  expected_reservation_revision: number
  expected_reservation_digest: string
  expected_claim_revision: number
  expected_claim_digest: string
  executor_abort_ref: string
  reason_code: string
  idempotency_key: string
  confirm_no_execution_started: true
}

export interface ComputeAttemptAbortReceipt {
  abort_id: string
  terminal_lease: ComputeAttemptLease
  terminal_lease_revision: number
  terminal_lease_digest: string
  terminal_job: ComputeJobVersionBinding
  terminal_reservation_revision: number
  terminal_reservation_digest: string
  returned_claim: ComputeCapacityClaimBinding
  budget_refunded_fen: number
  budget_terminal_status: string
  aborted_at: string
  execution_effect: string
  money_effect: string
  replayed: boolean
}

export interface ComputeOutputContract {
  media_type: string
  max_output_bytes: number
  streaming: boolean
  result_artifact_required: boolean
  deterministic_digest_expected: boolean
}

export interface ComputeAttemptUsageTemplateReceipt {
  schema: string
  lease_id: string
  provider_id: string
  lease_revision: number
  lease_digest: string
  fencing_generation: number
  task_kind: string
  output_contract: ComputeOutputContract
  next_sequence_no: number
  meters: Array<{ meter: string; reserved_quantity: number; previous_cumulative_quantity: number }>
  latest_snapshot: { snapshot_id: string; sequence_no: number; cumulative_usage_digest: string } | null
  read_effect: 'none'
}

export interface DeclareComputeAttemptUsageBody {
  expected_lease_revision: number
  expected_lease_digest: string
  expected_fencing_generation: number
  sequence_no: number
  executor_usage_ref: string
  cumulative_declared_usage: Array<{ meter: string; cumulative_quantity: number }>
  idempotency_key: string
  confirm_provider_declaration_only: true
}

export interface ComputeAttemptUsageDeclarationReceipt {
  snapshot_id: string
  lease_id: string
  sequence_no: number
  cumulative_declared_usage: Array<{ meter: string; quantity: number; reading_digest: string; observed_at: string }>
  cumulative_usage_digest: string
  overage_meters: string[]
  event_digest: string
  declared_at: string
  verification_status: string
  execution_effect: string
  money_effect: string
  replayed: boolean
}

export interface ComputeDeclaredResultArtifactInput {
  artifact_id: string
  digest_algorithm: 'sha256'
  digest: string
  media_type: string
  size_bytes: number
  location_ref: string
  encryption_profile: string | null
}

export interface DeclareComputeAttemptTerminalCandidateBody {
  expected_lease_revision: number
  expected_lease_digest: string
  expected_fencing_generation: number
  final_usage_snapshot_id: string
  final_usage_sequence_no: number
  final_cumulative_usage_digest: string
  executor_terminal_ref: string
  outcome: 'succeeded' | 'failed' | 'canceled'
  reason_code: string
  diagnostic_ref: string | null
  output_digest: string | null
  result_artifacts: ComputeDeclaredResultArtifactInput[]
  idempotency_key: string
  confirm_provider_declaration_only: true
}

export interface ComputeAttemptTerminalCandidateReceipt {
  terminal_candidate_id: string
  lease_id: string
  final_usage_snapshot_id: string
  final_usage_sequence_no: number
  final_cumulative_usage_digest: string
  executor_terminal_ref: string
  outcome: string
  reason_code: string
  diagnostic_ref: string | null
  output_digest: string | null
  result_artifacts: ComputeDeclaredResultArtifactInput[]
  event_digest: string
  declared_at: string
  verification_status: string
  execution_effect: string
  lease_effect: string
  money_effect: string
  replayed: boolean
}

interface ActivationCandidateResponse {
  attempt_activation_candidates: ComputeReservationReceipt[]
}

interface AttemptLeaseListResponse {
  attempt_leases: ComputeAttemptLeaseStateReceipt[]
}

function providerBase(providerId: string) {
  return `/api/me/compute/providers/${encodeURIComponent(providerId)}`
}

function leaseBase(leaseId: string) {
  return `/api/me/compute/attempt-leases/${encodeURIComponent(leaseId)}`
}

export const computeExecutionApi = {
  candidates: (providerId: string, limit = 100) =>
    api.get<ActivationCandidateResponse>(`${providerBase(providerId)}/attempt-activations?limit=${limit}`)
      .then((response) => response.attempt_activation_candidates),
  leases: (providerId: string, limit = 100) =>
    api.get<AttemptLeaseListResponse>(`${providerBase(providerId)}/attempt-leases?limit=${limit}`)
      .then((response) => response.attempt_leases),
  activate: (providerId: string, body: ActivateComputeAttemptBody) =>
    api.post<ComputeAttemptActivationReceipt>(`${providerBase(providerId)}/attempt-activations`, body),
  activation: (leaseId: string) =>
    api.get<ComputeAttemptActivationReceipt>(`${leaseBase(leaseId)}/activation`),
  leaseState: (leaseId: string) =>
    api.get<ComputeAttemptLeaseStateReceipt>(`${leaseBase(leaseId)}/state`),
  renew: (providerId: string, lease: ComputeAttemptLeaseStateReceipt, body: RenewComputeAttemptLeaseBody) =>
    api.post<ComputeAttemptLeaseRenewalReceipt>(`${providerBase(providerId)}/attempt-leases/${encodeURIComponent(lease.lease.lease_id)}/renewals`, body),
  abort: (providerId: string, leaseId: string, body: AbortComputeAttemptBody) =>
    api.post<ComputeAttemptAbortReceipt>(`${providerBase(providerId)}/attempt-leases/${encodeURIComponent(leaseId)}/abort`, body),
  usageTemplate: (providerId: string, leaseId: string) =>
    api.get<ComputeAttemptUsageTemplateReceipt>(`${providerBase(providerId)}/attempt-leases/${encodeURIComponent(leaseId)}/declared-usage`),
  declareUsage: (providerId: string, leaseId: string, body: DeclareComputeAttemptUsageBody) =>
    api.post<ComputeAttemptUsageDeclarationReceipt>(`${providerBase(providerId)}/attempt-leases/${encodeURIComponent(leaseId)}/declared-usage`, body),
  latestUsage: (leaseId: string) =>
    api.get<ComputeAttemptUsageDeclarationReceipt>(`${leaseBase(leaseId)}/declared-usage/latest`),
  terminalCandidate: (leaseId: string) =>
    api.get<ComputeAttemptTerminalCandidateReceipt>(`${leaseBase(leaseId)}/terminal-candidate`),
  declareTerminalCandidate: (providerId: string, leaseId: string, body: DeclareComputeAttemptTerminalCandidateBody) =>
    api.post<ComputeAttemptTerminalCandidateReceipt>(`${providerBase(providerId)}/attempt-leases/${encodeURIComponent(leaseId)}/terminal-candidate`, body),
}
