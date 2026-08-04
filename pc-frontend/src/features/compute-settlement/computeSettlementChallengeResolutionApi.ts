import { api } from '../../api/client'
import { type ComputeSettlementChallengeReceipt } from '../compute-attempt/settlementChallengeContracts'
import {
  type ComputeSettlementChallengeResolutionReceipt,
  type OpenSettlementChallengeQueueResponse,
  type ResolveComputeSettlementChallengeBody,
  type WithdrawComputeSettlementChallengeBody,
} from '../compute-attempt/settlementChallengeResolutionContracts'

export type {
  ComputeSettlementChallengeResolutionAction,
  ComputeSettlementChallengeResolutionReceipt,
  PlatformSettlementChallengeDecision,
  ResolveComputeSettlementChallengeBody,
  WithdrawComputeSettlementChallengeBody,
} from '../compute-attempt/settlementChallengeResolutionContracts'
export type { ComputeSettlementChallengeReceipt } from '../compute-attempt/settlementChallengeContracts'

function participantBase(leaseId: string) {
  return `/api/me/compute/attempt-leases/${encodeURIComponent(leaseId)}/settlement-challenge`
}

function adminBase(leaseId: string) {
  return `/api/admin/compute/attempt-leases/${encodeURIComponent(leaseId)}/settlement-challenge`
}

export const computeSettlementChallengeResolutionApi = {
  listConsumerOpen: (limit = 100) => api.get<OpenSettlementChallengeQueueResponse>(
    `/api/me/compute/settlement-challenges/open?limit=${limit}`,
  ).then((response) => response.challenge_candidates),
  listAdminOpen: (limit = 100) => api.get<OpenSettlementChallengeQueueResponse>(
    `/api/admin/compute/settlement-challenges/open?limit=${limit}`,
  ).then((response) => response.challenge_candidates),
  withdraw: (
    challenge: ComputeSettlementChallengeReceipt,
    body: WithdrawComputeSettlementChallengeBody,
  ) => api.post<ComputeSettlementChallengeResolutionReceipt>(
    `${participantBase(challenge.lease_id)}/withdrawal`,
    body,
  ),
  resolve: (
    challenge: ComputeSettlementChallengeReceipt,
    body: ResolveComputeSettlementChallengeBody,
  ) => api.post<ComputeSettlementChallengeResolutionReceipt>(
    `${adminBase(challenge.lease_id)}/resolution`,
    body,
  ),
  getParticipant: (leaseId: string) => api.get<ComputeSettlementChallengeResolutionReceipt>(
    `${participantBase(leaseId)}/resolution`,
  ),
  getAdmin: (leaseId: string) => api.get<ComputeSettlementChallengeResolutionReceipt>(
    `${adminBase(leaseId)}/resolution`,
  ),
}
