import { api } from '../../api/client'
import {
  type ComputeAttemptConsumerReviewReceipt,
  type ComputeAttemptTerminalCandidateReceipt,
  type ReviewComputeAttemptTerminalCandidateBody,
} from '../compute-attempt/terminalContracts'

export type {
  ComputeAttemptConsumerReviewReceipt,
  ComputeAttemptTerminalCandidateReceipt,
  ComputeConsumerReviewDecision,
  ReviewComputeAttemptTerminalCandidateBody,
} from '../compute-attempt/terminalContracts'

interface PendingConsumerReviewResponse {
  terminal_candidates: ComputeAttemptTerminalCandidateReceipt[]
}

function candidateBase(leaseId: string) {
  return `/api/me/compute/attempt-leases/${encodeURIComponent(leaseId)}/terminal-candidate`
}

export const computeConsumerReviewApi = {
  listPending: (limit = 100) =>
    api.get<PendingConsumerReviewResponse>(
      `/api/me/compute/attempt-terminal-candidates/pending-consumer-review?limit=${limit}`,
    ).then((response) => response.terminal_candidates),
  review: (
    candidate: ComputeAttemptTerminalCandidateReceipt,
    body: ReviewComputeAttemptTerminalCandidateBody,
  ) => api.post<ComputeAttemptConsumerReviewReceipt>(
    `${candidateBase(candidate.lease_id)}/consumer-review`,
    body,
  ),
}
