import { api } from '../../api/client'

export interface ComputeActivationEvidenceRequest {
  schema: string
  request_id: string
  provider_id: string
  pool_id: string
  owner_user_id: string
  expected_provider_policy_revision: number
  expected_provider_digest: string
  expected_capacity_epoch: number
  expected_pool_revision: number
  expected_pool_digest: string
  node_binding_ref: string
  ready_capability_digest: string
  route_proof_digest: string
  hardware_observation_digest: string
  ledger_audit_digest: string
  status: string
  request_digest: string
  requested_at: string
  reviewed_at: string | null
  reviewed_by_user_id: string | null
  review_note: string | null
  canceled_at: string | null
  superseded_at: string | null
  superseded_by_user_id: string | null
  supersede_reason: string | null
  created_at: string
  updated_at: string
}

export interface SubmitActivationEvidenceBody {
  idempotency_key: string
  node_binding_ref: string
  ready_capability_digest: string
  route_proof_digest: string
  hardware_observation_digest: string
  confirm_evidence_submission: true
}

export interface ComputeActivationPreflightReport {
  schema: string
  request_id: string
  provider_id: string
  pool_id: string
  request_status: string
  checked_at: string
  provider_ownership_matches: boolean
  provider_version_matches: boolean
  provider_status_registering: boolean
  provider_has_routing: boolean
  provider_has_verified_hardware: boolean
  provider_has_verified_at: boolean
  provider_trust_tier: string
  provider_regions_present: boolean
  pool_provider_matches: boolean
  pool_version_matches: boolean
  pool_status_registering: boolean
  ledger_audit_healthy: boolean
  ledger_audit_digest_matches: boolean
  ready_for_activation: boolean
  blockers: string[]
  activation_effect: 'none'
}

interface ActivationEvidenceListResponse {
  activation_evidence_requests: ComputeActivationEvidenceRequest[]
}

interface ActivationEvidenceReceipt {
  request: ComputeActivationEvidenceRequest
  replayed: boolean
  activation_effect: 'none'
}

function base(providerId: string, poolId: string) {
  return `/api/me/compute/providers/${encodeURIComponent(providerId)}/capacity-pools/${encodeURIComponent(poolId)}/activation-evidence-requests`
}

export const computeActivationApi = {
  list: (providerId: string, poolId: string, limit = 20) =>
    api.get<ActivationEvidenceListResponse>(`${base(providerId, poolId)}?limit=${limit}`),
  submit: (providerId: string, poolId: string, body: SubmitActivationEvidenceBody) =>
    api.post<ActivationEvidenceReceipt>(base(providerId, poolId), body),
  cancel: (providerId: string, poolId: string, requestId: string, requestDigest: string) =>
    api.post<ComputeActivationEvidenceRequest>(
      `${base(providerId, poolId)}/${encodeURIComponent(requestId)}/cancel`,
      { expected_request_digest: requestDigest, confirm_cancel: true },
    ),
  preflight: (providerId: string, poolId: string, requestId: string) =>
    api.get<ComputeActivationPreflightReport>(
      `${base(providerId, poolId)}/${encodeURIComponent(requestId)}/preflight`,
    ),
}
