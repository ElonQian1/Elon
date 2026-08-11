import { api } from '../../api/client'

export type OnboardingStatus = 'submitted' | 'approved' | 'changes_requested' | 'rejected' | 'canceled' | 'applied'
export type ReleaseStatus = 'submitted' | 'approved' | 'changes_requested' | 'rejected' | 'staged'
export type GovernanceDecision = 'approved' | 'changes_requested' | 'rejected'

export interface OnboardingRequestReceipt {
  schema: string
  request_id: string
  request_digest: string
  provider_id: string
  provider_owner_account_id: string
  target_provider_digest: string
  status: OnboardingStatus
  credential_ref_present: boolean
  credential_hint: string | null
  requested_at: string
  updated_at: string
  canceled_at: string | null
  replayed: boolean
  onboarding_effect: 'none'
}

export interface OnboardingReviewReceipt {
  schema: string
  review_id: string
  review_digest: string
  request_id: string
  request_digest: string
  provider_id: string
  provider_owner_account_id: string
  decision: GovernanceDecision
  review_reason: string | null
  reviewed_by_user_id: string
  reviewed_at: string
  replayed: boolean
  onboarding_effect: 'none'
}

export interface OnboardingApplicationReceipt {
  schema: string
  application_id: string
  application_digest: string
  request_id: string
  request_digest: string
  review_id: string
  review_digest: string
  provider_id: string
  provider_digest: string
  approved_by_user_id: string
  reviewed_by_user_id: string
  applied_by_user_id: string
  applied_at: string
  replayed: boolean
  onboarding_effect: 'provider_registered_only'
}

export interface OnboardingDetail {
  request: OnboardingRequestReceipt
  review: OnboardingReviewReceipt | null
  application: OnboardingApplicationReceipt | null
}

export interface OnboardingPreflight {
  schema: string
  request_id: string
  request_digest: string
  provider_id: string
  provider_owner_account_id: string
  request_status: OnboardingStatus
  checked_at: string
  review_present: boolean
  application_present: boolean
  provider_conflict: boolean
  owner_cancel_allowed: boolean
  admin_review_allowed: boolean
  admin_apply_allowed: boolean
  blockers: string[]
  onboarding_effect: 'none'
}

export interface SubmitOnboardingBody {
  request_id: string
  idempotency_key: string
  submitted_at: string
  provider_id: string
  display_name: string
  home_region: string
  task_kinds: string[]
  accelerator_kinds: string[]
  regions: string[]
  allowed_data_classes: string[]
  supports_streaming: boolean
  supports_checkpointing: boolean
  declared_hardware_digest: string | null
  adapter_intent: {
    expected_adapter_id: string
    expected_release_version: string
    expected_config_revision: number
    expected_config_digest: string
  }
  credential_intent: {
    non_bearer_credential_ref: string | null
    credential_hint: string | null
  }
  external_evidence_ref: string | null
  external_evidence_sha256: string | null
  owner_note: string
  confirm_submission: true
}

export interface AdapterCapability {
  capability_id: string
  capability_revision: number
}

export interface ReleaseRequestReceipt {
  schema: string
  request_id: string
  request_digest: string
  request_material_digest: string
  adapter_id: string
  release_version: string
  status: ReleaseStatus
  submitted_by_admin_user_id: string
  submitted_at: string
  updated_at: string
  replayed: boolean
  release_effect: 'none'
}

export interface ReleaseReviewReceipt {
  schema: string
  review_id: string
  review_digest: string
  request_id: string
  request_digest: string
  request_material_digest: string
  adapter_id: string
  release_version: string
  decision: GovernanceDecision
  reviewed_by_admin_user_id: string
  reviewed_at: string
  replayed: boolean
  release_effect: 'none'
}

export interface ReleaseAdmissionReceipt {
  schema: string
  admission_id: string
  admission_digest: string
  request_id: string
  request_digest: string
  request_material_digest: string
  review_id: string
  review_digest: string
  adapter_id: string
  release_version: string
  submitted_by_admin_user_id: string
  reviewed_by_admin_user_id: string
  applied_by_admin_user_id: string
  status: 'staged'
  applied_at: string
  replayed: boolean
  release_effect: 'staged_admission_only'
}

export interface ReleaseDetail {
  request: ReleaseRequestReceipt
  review: ReleaseReviewReceipt | null
  admission: ReleaseAdmissionReceipt | null
}

export interface ReleasePreflight {
  schema: string
  request_id: string
  request_digest: string
  adapter_id: string
  release_version: string
  submitted_by_admin_user_id: string
  request_status: ReleaseStatus
  checked_at: string
  review_present: boolean
  admission_present: boolean
  admin_review_allowed: boolean
  admin_stage_allowed: boolean
  blockers: string[]
  release_effect: 'none'
}

export interface SubmitReleaseBody {
  idempotency_key: string
  adapter_id: string
  release_version: string
  candidate_artifact_ref: string
  declared_implementation_sha256: string
  supported_capabilities: AdapterCapability[]
  expected_credential_verifier: {
    verification_kind: string
    verifier_id: string
    verifier_revision: number
    verifier_digest: string
  }
  submission_note: string
  confirm_submission: true
}

interface OnboardingListResponse { onboarding_requests: OnboardingDetail[] }
interface ReleaseListResponse { adapter_release_requests: ReleaseDetail[] }

const ownerBase = '/api/me/compute/external-pool-onboarding-requests'
const adminBase = '/api/admin/compute/external-pool-onboarding-requests'
const releaseBase = '/api/admin/compute/external-pool-adapter-releases'

function listPath(base: string, status: string, limit = 50) {
  const query = new URLSearchParams({ limit: String(limit) })
  if (status) query.set('status', status)
  return `${base}?${query.toString()}`
}

function itemPath(base: string, id: string, suffix = '') {
  return `${base}/${encodeURIComponent(id)}${suffix}`
}

export const externalPoolApi = {
  listMine: (status: OnboardingStatus) => api.get<OnboardingListResponse>(listPath(ownerBase, status)).then((response) => response.onboarding_requests),
  getMine: (id: string) => api.get<OnboardingDetail>(itemPath(ownerBase, id)),
  preflightMine: (id: string) => api.get<OnboardingPreflight>(itemPath(ownerBase, id, '/preflight')),
  submitMine: (body: SubmitOnboardingBody) => api.post<OnboardingRequestReceipt>(ownerBase, body),
  cancelMine: (request: OnboardingRequestReceipt) => api.post<OnboardingRequestReceipt>(itemPath(ownerBase, request.request_id, '/cancel'), {
    expected_request_digest: request.request_digest,
    confirm_cancel: true,
  }),

  listOnboardingAdmin: (status: OnboardingStatus) => api.get<OnboardingListResponse>(listPath(adminBase, status)).then((response) => response.onboarding_requests),
  getOnboardingAdmin: (id: string) => api.get<OnboardingDetail>(itemPath(adminBase, id)),
  preflightOnboardingAdmin: (id: string) => api.get<OnboardingPreflight>(itemPath(adminBase, id, '/preflight')),
  reviewOnboarding: (detail: OnboardingDetail, decision: GovernanceDecision, reason: string | null) => api.post<OnboardingReviewReceipt>(itemPath(adminBase, detail.request.request_id, '/review'), {
    idempotency_key: `external-pool-onboarding-review:${detail.request.request_digest}:${decision}`,
    expected_request_digest: detail.request.request_digest,
    decision,
    review_reason: reason,
    confirm_review: true,
  }),
  applyOnboarding: (detail: OnboardingDetail) => {
    if (!detail.review) throw new Error('当前申请缺少独立复核回执')
    return api.post<OnboardingApplicationReceipt>(itemPath(adminBase, detail.request.request_id, '/application'), {
      idempotency_key: `external-pool-onboarding-apply:${detail.review.review_digest}`,
      expected_request_digest: detail.request.request_digest,
      expected_review_digest: detail.review.review_digest,
      confirm_application: true,
    })
  },

  listReleases: (status: ReleaseStatus) => api.get<ReleaseListResponse>(listPath(releaseBase, status)).then((response) => response.adapter_release_requests),
  getRelease: (id: string) => api.get<ReleaseDetail>(itemPath(releaseBase, id)),
  preflightRelease: (id: string) => api.get<ReleasePreflight>(itemPath(releaseBase, id, '/preflight')),
  submitRelease: (body: SubmitReleaseBody) => api.post<ReleaseRequestReceipt>(releaseBase, body),
  reviewRelease: (detail: ReleaseDetail, decision: GovernanceDecision, note: string | null) => api.post<ReleaseReviewReceipt>(itemPath(releaseBase, detail.request.request_id, '/review'), {
    idempotency_key: `external-pool-release-review:${detail.request.request_digest}:${decision}`,
    expected_request_digest: detail.request.request_digest,
    expected_request_material_digest: detail.request.request_material_digest,
    decision,
    review_note: note,
    confirm_review: true,
  }),
  stageRelease: (detail: ReleaseDetail, note: string) => {
    if (!detail.review) throw new Error('当前 release 缺少独立复核回执')
    return api.post<ReleaseAdmissionReceipt>(itemPath(releaseBase, detail.request.request_id, '/stage'), {
      idempotency_key: `external-pool-release-stage:${detail.review.review_digest}`,
      expected_request_digest: detail.request.request_digest,
      expected_request_material_digest: detail.request.request_material_digest,
      expected_review_digest: detail.review.review_digest,
      apply_note: note,
      confirm_stage: true,
    })
  },
}
