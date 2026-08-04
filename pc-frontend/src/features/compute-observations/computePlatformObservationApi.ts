import { api } from '../../api/client'
import {
  type ComputeAttemptPlatformObservationReceipt,
  type ComputePendingPlatformObservationCandidate,
  type ObserveComputeAttemptTerminalCandidateBody,
} from '../compute-attempt/platformObservationContracts'

export type {
  ComputeAttemptPlatformObservationReceipt,
  ComputeObservationSource,
  ComputeObservedOutcome,
  ComputePendingPlatformObservationCandidate,
  ObserveComputeAttemptTerminalCandidateBody,
} from '../compute-attempt/platformObservationContracts'

interface PendingObservationResponse {
  observation_candidates: ComputePendingPlatformObservationCandidate[]
}

function observationBase(leaseId: string) {
  return `/api/admin/compute/attempt-leases/${encodeURIComponent(leaseId)}/terminal-candidate/platform-observation`
}

export const computePlatformObservationApi = {
  listPending: (limit = 100) =>
    api.get<PendingObservationResponse>(
      `/api/admin/compute/attempt-terminal-candidates/pending-platform-observation?limit=${limit}`,
    ).then((response) => response.observation_candidates),
  observe: (
    candidate: ComputePendingPlatformObservationCandidate,
    body: ObserveComputeAttemptTerminalCandidateBody,
  ) => api.post<ComputeAttemptPlatformObservationReceipt>(
    observationBase(candidate.terminal_candidate.lease_id),
    body,
  ),
}
