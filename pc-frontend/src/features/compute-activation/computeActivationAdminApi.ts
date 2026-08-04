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
  blockers: string[]
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
interface ActivationApplicationResponse { activation_application: ComputeActivationApplication | null }

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
  plan: (requestId: string) =>
    api.get<ActivationPlanResponse>(`${activationBase(requestId)}/activation-plan`),
  preparePlan: (requestId: string, body: PrepareActivationPlanBody) =>
    api.post<ActivationPlanReceipt>(`${activationBase(requestId)}/activation-plan`, body),
  planPreflight: (requestId: string) =>
    api.get<ComputeActivationPlanPreflight>(`${activationBase(requestId)}/activation-plan/preflight`),
  application: (requestId: string) =>
    api.get<ActivationApplicationResponse>(`${activationBase(requestId)}/activation-plan/application`),
  applyPlan: (requestId: string, idempotencyKey: string, expectedPlanDigest: string) =>
    api.post<ComputeActivationApplication>(`${activationBase(requestId)}/activation-plan/application`, {
      idempotency_key: idempotencyKey,
      expected_plan_digest: expectedPlanDigest,
      confirm_apply: true,
    }),
}
