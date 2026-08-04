import { api } from '../../api/client'
import {
  type ComputeAttemptExecutionReceiptEnvelope,
  type ComputePendingExecutionReceiptCandidate,
  type IssueComputeAttemptExecutionReceiptBody,
} from '../compute-attempt/executionReceiptContracts'

export type {
  ComputeAttemptExecutionReceiptEnvelope,
  ComputeAttestationEvidence,
  ComputeExecutionReceipt,
  ComputeExecutionUsage,
  ComputePendingExecutionReceiptCandidate,
  ComputeReceiptVerificationDecision,
  IssueComputeAttemptExecutionReceiptBody,
} from '../compute-attempt/executionReceiptContracts'

interface PendingExecutionReceiptResponse {
  execution_receipt_candidates: ComputePendingExecutionReceiptCandidate[]
}

function receiptBase(leaseId: string) {
  return `/api/admin/compute/attempt-leases/${encodeURIComponent(leaseId)}/execution-receipt`
}

export const computeExecutionReceiptApi = {
  listPending: (limit = 100) =>
    api.get<PendingExecutionReceiptResponse>(
      `/api/admin/compute/attempt-verifications/pending-execution-receipt?limit=${limit}`,
    ).then((response) => response.execution_receipt_candidates),
  issue: (
    candidate: ComputePendingExecutionReceiptCandidate,
    body: IssueComputeAttemptExecutionReceiptBody,
  ) => api.post<ComputeAttemptExecutionReceiptEnvelope>(
    receiptBase(candidate.terminal_candidate.lease_id),
    body,
  ),
}
