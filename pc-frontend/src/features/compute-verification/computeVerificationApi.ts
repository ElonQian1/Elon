import { api } from '../../api/client'
import {
  type ComputeAttemptVerificationDecisionReceipt,
  type ComputePendingAttemptVerificationCandidate,
  type DecideComputeAttemptVerificationBody,
} from '../compute-attempt/verificationContracts'
import {
  type ValidatedVerificationDecisionRead,
  validateVerificationDecisionReadForLease,
} from '../compute-attempt/verificationDecisionReadContracts'

export type {
  ComputeAttemptVerificationDecisionReceipt,
  ComputePendingAttemptVerificationCandidate,
  ComputeVerificationDecision,
  DecideComputeAttemptVerificationBody,
} from '../compute-attempt/verificationContracts'
export type { ValidatedVerificationDecisionRead } from '../compute-attempt/verificationDecisionReadContracts'

interface PendingVerificationResponse {
  verification_candidates: ComputePendingAttemptVerificationCandidate[]
}

function verificationBase(leaseId: string) {
  return `/api/admin/compute/attempt-leases/${encodeURIComponent(leaseId)}/verification-decision`
}

export const computeVerificationApi = {
  listPending: (limit = 100) =>
    api.get<PendingVerificationResponse>(
      `/api/admin/compute/attempt-terminal-candidates/pending-verification?limit=${limit}`,
    ).then((response) => response.verification_candidates),
  decide: (
    candidate: ComputePendingAttemptVerificationCandidate,
    body: DecideComputeAttemptVerificationBody,
  ) => api.post<ComputeAttemptVerificationDecisionReceipt>(
    verificationBase(candidate.terminal_candidate.lease_id),
    body,
  ),
  readRetained: async (leaseId: string): Promise<ValidatedVerificationDecisionRead> => {
    const value = await api.get<unknown>(verificationBase(leaseId))
    return validateVerificationDecisionReadForLease(value, leaseId)
  },
}
