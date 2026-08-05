import { api } from '../../api/client'
import {
  type ComputeActivationEvidenceRequest,
  type ComputeActivationPreflightReport,
} from '../compute-supply/computeActivationApi'

export type ActivationRequestStatus =
  | 'submitted'
  | 'changes_requested'
  | 'approved'
  | 'activated'
  | 'rejected'
  | 'canceled'
  | 'superseded'

export type ActivationReviewDecision = 'approved' | 'changes_requested' | 'rejected'

interface ActivationRequestListResponse {
  activation_evidence_requests: ComputeActivationEvidenceRequest[]
}

export interface ActivationReviewReceipt {
  request: ComputeActivationEvidenceRequest
  activation_effect: 'none'
}

export interface ComputeProviderEndpointRef {
  endpoint_id: string
  transport: string
  address_hint: string | null
  gateway_id: string | null
  credential_ref: string | null
}

export interface ComputeProviderAdapterRef {
  adapter_id: string
  adapter_version: string
  config_revision: number
  config_digest: string
}

export interface ComputeActivationPlan {
  schema: string
  plan_id: string
  request_id: string
  provider_id: string
  pool_id: string
  expected_request_digest: string
  target_provider_policy_revision: number
  target_provider_digest: string
  target_provider: {
    status: string
    trust_tier: string
    policy_revision: number
    endpoint: ComputeProviderEndpointRef | null
  }
  endpoint_digest: string
  status: string
  plan_digest: string
  prepared_by_user_id: string
  prepared_at: string
  applied_at: string | null
  superseded_at: string | null
}

export interface ComputeActivationPlanPreflight {
  schema: string
  plan_id: string
  request_id: string
  provider_id: string
  pool_id: string
  plan_status: string
  checked_at: string
  ready_for_apply: boolean
  plan_review_present: boolean
  plan_review_digest_matches: boolean
  plan_review_separation_valid: boolean
  blockers: string[]
  activation_effect: 'none'
}

export interface ComputeActivationPlanReview {
  schema: string
  review_id: string
  plan_id: string
  request_id: string
  provider_id: string
  pool_id: string
  plan_digest: string
  prepared_by_user_id: string
  reviewed_by_user_id: string
  review_note: string | null
  request_digest: string
  review_digest: string
  reviewed_at: string
  replayed: boolean
  activation_effect: 'none'
}

export interface ComputeActivationApplication {
  schema: string
  application_id: string
  plan_id: string
  request_id: string
  provider_id: string
  pool_id: string
  plan_digest: string
  target_provider_policy_revision: number
  target_provider_digest: string
  pool_lifecycle_event_id: string
  application_digest: string
  applied_by_user_id: string
  applied_at: string
  replayed: boolean
  activation_effect: 'provider_and_pool_active'
  offer_effect: 'none'
}

export interface ComputeActivationQuarantine {
  schema: string
  quarantine_id: string
  application_id: string
  request_id: string
  provider_id: string
  pool_id: string
  application_digest: string
  previous_provider_policy_revision: number
  previous_provider_digest: string
  quarantined_provider_policy_revision: number
  quarantined_provider_digest: string
  capacity_epoch: number
  pool_lifecycle_event_id: string
  reason: string
  quarantine_digest: string
  quarantined_by_user_id: string
  quarantined_at: string
  replayed: boolean
  provider_effect: 'quarantined'
  pool_effect: 'quarantined'
  offer_effect: 'none_direct'
}

export interface ComputeActivationRecoveryPlan {
  schema: string
  recovery_plan_id: string
  quarantine_id: string
  application_id: string
  request_id: string
  provider_id: string
  pool_id: string
  expected_quarantine_digest: string
  target_provider_policy_revision: number
  target_provider_digest: string
  target_provider: {
    status: string
    trust_tier: string
    policy_revision: number
    endpoint: ComputeProviderEndpointRef | null
    adapter: ComputeProviderAdapterRef | null
  }
  routing_digest: string
  remediation_summary: string
  evidence_refs: string[]
  evidence_refs_digest: string
  status: string
  plan_digest: string
  prepared_by_user_id: string
  prepared_at: string
  applied_at: string | null
  superseded_at: string | null
}

export interface ComputeActivationRecoveryPreflight {
  schema: string
  recovery_plan_id: string
  request_id: string
  provider_id: string
  pool_id: string
  plan_status: string
  checked_at: string
  active_offer_count: number
  active_offers_drained: boolean
  plan_review_present: boolean
  plan_review_digest_matches: boolean
  plan_review_separation_valid: boolean
  ready_for_apply: boolean
  blockers: string[]
  recovery_effect: 'none'
}

export interface ComputeActivationRecoveryReview {
  schema: string
  recovery_review_id: string
  recovery_plan_id: string
  request_id: string
  plan_digest: string
  prepared_by_user_id: string
  reviewed_by_user_id: string
  review_note: string | null
  request_digest: string
  review_digest: string
  reviewed_at: string
  replayed: boolean
  recovery_effect: 'none'
}

export interface ComputeActivationRecoveryApplication {
  schema: string
  recovery_application_id: string
  recovery_plan_id: string
  recovery_review_id: string
  quarantine_id: string
  request_id: string
  provider_id: string
  pool_id: string
  plan_digest: string
  review_digest: string
  recovered_provider_policy_revision: number
  recovered_provider_digest: string
  capacity_epoch: number
  pool_lifecycle_event_id: string
  application_digest: string
  applied_by_user_id: string
  applied_at: string
  replayed: boolean
  provider_effect: 'active'
  pool_effect: 'active'
  offer_effect: 'none_active_offers_required'
  node_effect: 'none'
  money_effect: 'none'
}

export interface ComputeActivationRecoverySupersession {
  schema: string
  recovery_supersession_id: string
  recovery_plan_id: string
  quarantine_id: string
  request_id: string
  provider_id: string
  pool_id: string
  plan_digest: string
  reason: string
  request_digest: string
  supersession_digest: string
  superseded_by_user_id: string
  superseded_at: string
  replayed: boolean
  recovery_effect: 'plan_superseded'
  provider_effect: 'none'
  pool_effect: 'none'
  offer_effect: 'none'
  node_effect: 'none'
  money_effect: 'none'
}

export interface PrepareActivationRecoveryPlanBody {
  idempotency_key: string
  expected_quarantine_digest: string
  endpoint: ComputeProviderEndpointRef | null
  adapter: ComputeProviderAdapterRef | null
  verified_hardware_digest: string
  trust_tier: string
  verified_at: string
  remediation_summary: string
  evidence_refs: string[]
  confirm_prepare: true
}

export interface PrepareActivationPlanBody {
  idempotency_key: string
  expected_request_digest: string
  endpoint: ComputeProviderEndpointRef
  verified_hardware_digest: string
  trust_tier: string
  verified_at: string
  confirm_prepare: true
}

interface ActivationPlanResponse { activation_plan: ComputeActivationPlan | null; activation_effect: 'none' }
interface ActivationPlanReceipt { plan: ComputeActivationPlan; replayed: boolean; activation_effect: 'none' }
interface ActivationPlanReviewResponse { activation_plan_review: ComputeActivationPlanReview | null; activation_effect: 'none' }
interface ActivationApplicationResponse { activation_application: ComputeActivationApplication | null }
interface ActivationQuarantineResponse { activation_quarantine: ComputeActivationQuarantine | null }
interface ActivationRecoveryPlanResponse { activation_recovery_plan: ComputeActivationRecoveryPlan | null }
interface ActivationRecoveryPlanReceipt { plan: ComputeActivationRecoveryPlan; replayed: boolean }
interface ActivationRecoveryReviewResponse { activation_recovery_review: ComputeActivationRecoveryReview | null }
interface ActivationRecoveryApplicationResponse { activation_recovery_application: ComputeActivationRecoveryApplication | null }
interface ActivationRecoverySupersessionResponse { activation_recovery_supersession: ComputeActivationRecoverySupersession | null }

function activationBase(requestId: string) {
  return `/api/admin/compute/activation-evidence-requests/${encodeURIComponent(requestId)}`
}

export const computeActivationAdminApi = {
  list: (status: ActivationRequestStatus, limit = 50) =>
    api.get<ActivationRequestListResponse>(
      `/api/admin/compute/activation-evidence-requests?status=${encodeURIComponent(status)}&limit=${limit}`,
    ),
  preflight: (requestId: string) =>
    api.get<ComputeActivationPreflightReport>(
      `/api/admin/compute/activation-evidence-requests/${encodeURIComponent(requestId)}/preflight`,
    ),
  review: (
    requestId: string,
    expectedRequestDigest: string,
    decision: ActivationReviewDecision,
    reviewNote: string | null,
  ) => api.post<ActivationReviewReceipt>(
    `/api/admin/compute/activation-evidence-requests/${encodeURIComponent(requestId)}/review`,
    {
      expected_request_digest: expectedRequestDigest,
      decision,
      review_note: reviewNote,
      confirm_review: true,
    },
  ),
  supersede: (requestId: string, expectedRequestDigest: string, reason: string) =>
    api.post<ComputeActivationEvidenceRequest>(`${activationBase(requestId)}/supersede`, {
      expected_request_digest: expectedRequestDigest,
      reason,
      confirm_supersede: true,
    }),
  plan: (requestId: string) =>
    api.get<ActivationPlanResponse>(`${activationBase(requestId)}/activation-plan`),
  preparePlan: (requestId: string, body: PrepareActivationPlanBody) =>
    api.post<ActivationPlanReceipt>(`${activationBase(requestId)}/activation-plan`, body),
  planPreflight: (requestId: string) =>
    api.get<ComputeActivationPlanPreflight>(`${activationBase(requestId)}/activation-plan/preflight`),
  planReview: (requestId: string) =>
    api.get<ActivationPlanReviewResponse>(`${activationBase(requestId)}/activation-plan/review`),
  reviewPlan: (requestId: string, idempotencyKey: string, expectedPlanDigest: string, reviewNote: string | null) =>
    api.post<ComputeActivationPlanReview>(`${activationBase(requestId)}/activation-plan/review`, {
      idempotency_key: idempotencyKey,
      expected_plan_digest: expectedPlanDigest,
      review_note: reviewNote,
      confirm_review: true,
    }),
  application: (requestId: string) =>
    api.get<ActivationApplicationResponse>(`${activationBase(requestId)}/activation-plan/application`),
  applyPlan: (requestId: string, idempotencyKey: string, expectedPlanDigest: string) =>
    api.post<ComputeActivationApplication>(`${activationBase(requestId)}/activation-plan/application`, {
      idempotency_key: idempotencyKey,
      expected_plan_digest: expectedPlanDigest,
      confirm_apply: true,
    }),
  quarantine: (requestId: string) =>
    api.get<ActivationQuarantineResponse>(`${activationBase(requestId)}/activation-plan/application/quarantine`),
  quarantineApplication: (
    requestId: string,
    idempotencyKey: string,
    expectedApplicationDigest: string,
    reason: string,
  ) => api.post<ComputeActivationQuarantine>(
    `${activationBase(requestId)}/activation-plan/application/quarantine`,
    {
      idempotency_key: idempotencyKey,
      expected_application_digest: expectedApplicationDigest,
      reason,
      confirm_quarantine: true,
    },
  ),
  recoveryPlan: (requestId: string) =>
    api.get<ActivationRecoveryPlanResponse>(`${activationBase(requestId)}/activation-recovery-plan`),
  prepareRecoveryPlan: (requestId: string, body: PrepareActivationRecoveryPlanBody) =>
    api.post<ActivationRecoveryPlanReceipt>(`${activationBase(requestId)}/activation-recovery-plan`, body),
  recoveryPreflight: (requestId: string) =>
    api.get<ComputeActivationRecoveryPreflight>(`${activationBase(requestId)}/activation-recovery-plan/preflight`),
  recoverySupersession: (requestId: string) =>
    api.get<ActivationRecoverySupersessionResponse>(`${activationBase(requestId)}/activation-recovery-plan/supersession`),
  supersedeRecoveryPlan: (
    requestId: string,
    idempotencyKey: string,
    expectedPlanDigest: string,
    reason: string,
  ) => api.post<ComputeActivationRecoverySupersession>(
    `${activationBase(requestId)}/activation-recovery-plan/supersession`,
    {
      idempotency_key: idempotencyKey,
      expected_plan_digest: expectedPlanDigest,
      reason,
      confirm_supersede: true,
    },
  ),
  recoveryReview: (requestId: string) =>
    api.get<ActivationRecoveryReviewResponse>(`${activationBase(requestId)}/activation-recovery-plan/review`),
  reviewRecoveryPlan: (
    requestId: string,
    idempotencyKey: string,
    expectedPlanDigest: string,
    reviewNote: string | null,
  ) => api.post<ComputeActivationRecoveryReview>(`${activationBase(requestId)}/activation-recovery-plan/review`, {
    idempotency_key: idempotencyKey,
    expected_plan_digest: expectedPlanDigest,
    review_note: reviewNote,
    confirm_review: true,
  }),
  recoveryApplication: (requestId: string) =>
    api.get<ActivationRecoveryApplicationResponse>(`${activationBase(requestId)}/activation-recovery-plan/application`),
  applyRecoveryPlan: (requestId: string, idempotencyKey: string, expectedPlanDigest: string) =>
    api.post<ComputeActivationRecoveryApplication>(`${activationBase(requestId)}/activation-recovery-plan/application`, {
      idempotency_key: idempotencyKey,
      expected_plan_digest: expectedPlanDigest,
      confirm_apply: true,
    }),
}
