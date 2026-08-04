import { api } from '../../api/client'
import {
  type ComputeAttemptFinalizationReceipt,
  type ComputePendingAttemptFinalizationCandidate,
  type FinalizeComputeAttemptBody,
} from '../compute-attempt/finalizationContracts'

export type {
  ComputeAttemptCapacityTransactionRef,
  ComputeAttemptFinalizationReceipt,
  ComputeAttemptRevisionBinding,
  ComputeCapacityClaimBinding,
  ComputeJobVersionBinding,
  ComputePendingAttemptFinalizationCandidate,
  ComputeReservedCapacity,
  FinalizeComputeAttemptBody,
} from '../compute-attempt/finalizationContracts'

interface PendingAttemptFinalizationResponse {
  trusted_finalization_candidates: ComputePendingAttemptFinalizationCandidate[]
}

function finalizationBase(leaseId: string) {
  return `/api/admin/compute/attempt-leases/${encodeURIComponent(leaseId)}/trusted-finalization`
}

export const computeAttemptFinalizationApi = {
  listPending: (limit = 100) =>
    api.get<PendingAttemptFinalizationResponse>(
      `/api/admin/compute/attempt-execution-receipts/pending-trusted-finalization?limit=${limit}`,
    ).then((response) => response.trusted_finalization_candidates),
  finalize: (
    candidate: ComputePendingAttemptFinalizationCandidate,
    body: FinalizeComputeAttemptBody,
  ) => api.post<ComputeAttemptFinalizationReceipt>(
    finalizationBase(candidate.execution_receipt.receipt.attempt_lease_id),
    body,
  ),
}
