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
}
