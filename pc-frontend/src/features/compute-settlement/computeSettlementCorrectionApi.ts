import { api } from '../../api/client'
import {
  type ComputePendingSettlementCorrectionCandidate,
  type ComputeSettlementCorrectionReceipt,
  type CorrectComputeAttemptSettlementBody,
} from '../compute-attempt/settlementCorrectionContracts'

export type {
  ComputePendingSettlementCorrectionCandidate,
  ComputeSettlementCorrectionReceipt,
  CorrectComputeAttemptSettlementBody,
} from '../compute-attempt/settlementCorrectionContracts'

interface PendingSettlementCorrectionResponse {
  correction_candidates: ComputePendingSettlementCorrectionCandidate[]
}

function adminBase(leaseId: string) {
  return `/api/admin/compute/attempt-leases/${encodeURIComponent(leaseId)}/settlement-correction`
}

function participantBase(leaseId: string) {
  return `/api/me/compute/attempt-leases/${encodeURIComponent(leaseId)}/settlement-correction`
}

export const computeSettlementCorrectionApi = {
  listPending: (limit = 100) => api.get<PendingSettlementCorrectionResponse>(
    `/api/admin/compute/settlement-challenges/accepted/pending-correction?limit=${limit}`,
  ).then((response) => response.correction_candidates),
  correct: (
    candidate: ComputePendingSettlementCorrectionCandidate,
    body: CorrectComputeAttemptSettlementBody,
  ) => api.post<ComputeSettlementCorrectionReceipt>(
    adminBase(candidate.settlement.lease_id),
    body,
  ),
  getAdmin: (leaseId: string) => api.get<ComputeSettlementCorrectionReceipt>(adminBase(leaseId)),
  getParticipant: (leaseId: string) => api.get<ComputeSettlementCorrectionReceipt>(
    participantBase(leaseId),
  ),
}
