import { api } from '../../api/client'
import {
  type ComputeAttemptSettlementReceipt,
  type ComputePendingAttemptSettlementCandidate,
  type SettleComputeAttemptBody,
} from '../compute-attempt/settlementContracts'

export type {
  ComputeAttemptSettlementReceipt,
  ComputePendingAttemptSettlementCandidate,
  ComputePendingAttemptSettlementPreview,
  ComputeSettlementAmounts,
  ComputeSettlementReceipt,
  SettleComputeAttemptBody,
} from '../compute-attempt/settlementContracts'

interface PendingAttemptSettlementResponse {
  settlement_candidates: ComputePendingAttemptSettlementCandidate[]
}

function settlementBase(leaseId: string) {
  return `/api/admin/compute/attempt-leases/${encodeURIComponent(leaseId)}/settlement-receipt`
}

export const computeSettlementIssuanceApi = {
  listPending: (limit = 100) =>
    api.get<PendingAttemptSettlementResponse>(
      `/api/admin/compute/attempt-finalizations/pending-settlement-receipt?limit=${limit}`,
    ).then((response) => response.settlement_candidates),
  settle: (
    candidate: ComputePendingAttemptSettlementCandidate,
    body: SettleComputeAttemptBody,
  ) => api.post<ComputeAttemptSettlementReceipt>(
    settlementBase(candidate.finalization.lease_id),
    body,
  ),
}
