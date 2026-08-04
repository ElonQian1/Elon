import { api } from '../../api/client'
import {
  type ComputePendingSettlementChallengeCandidate,
  type ComputeSettlementChallengeReceipt,
  type OpenComputeSettlementChallengeBody,
} from '../compute-attempt/settlementChallengeContracts'

export type {
  ComputePendingSettlementChallengeCandidate,
  ComputeSettlementChallengeReasonCode,
  ComputeSettlementChallengeReceipt,
  OpenComputeSettlementChallengeBody,
} from '../compute-attempt/settlementChallengeContracts'

interface PendingSettlementChallengeResponse {
  challenge_candidates: ComputePendingSettlementChallengeCandidate[]
}

function challengeBase(leaseId: string) {
  return `/api/me/compute/attempt-leases/${encodeURIComponent(leaseId)}/settlement-challenge`
}

export const computeSettlementChallengeApi = {
  listPending: (limit = 100) => api.get<PendingSettlementChallengeResponse>(
    `/api/me/compute/settlement-receipts/pending-challenge?limit=${limit}`,
  ).then((response) => response.challenge_candidates),
  open: (
    candidate: ComputePendingSettlementChallengeCandidate,
    body: OpenComputeSettlementChallengeBody,
  ) => api.post<ComputeSettlementChallengeReceipt>(
    challengeBase(candidate.settlement.lease_id),
    body,
  ),
  get: (leaseId: string) => api.get<ComputeSettlementChallengeReceipt>(challengeBase(leaseId)),
}
