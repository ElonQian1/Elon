import { type ComputeSettlementChallengeReceipt } from './settlementChallengeContracts'

export type ComputeSettlementChallengeResolutionAction = 'accepted' | 'rejected' | 'withdrawn'
export type PlatformSettlementChallengeDecision = Exclude<ComputeSettlementChallengeResolutionAction, 'withdrawn'>

export interface WithdrawComputeSettlementChallengeBody {
  expected_challenge_id: string
  expected_challenge_event_digest: string
  statement: string
  idempotency_key: string
  confirm_balances_unchanged: true
}

export interface ResolveComputeSettlementChallengeBody {
  expected_challenge_id: string
  expected_challenge_event_digest: string
  decision: PlatformSettlementChallengeDecision
  statement: string
  idempotency_key: string
  confirm_no_money_movement: true
}

export interface ComputeSettlementChallengeResolutionReceipt {
  schema: string
  resolution_id: string
  challenge_id: string
  challenge_event_digest: string
  settlement_receipt_id: string
  settlement_event_digest: string
  lease_id: string
  consumer_account_id: string
  provider_account_id: string
  action: ComputeSettlementChallengeResolutionAction
  statement: string
  actor_user_id: string
  actor_role: 'consumer' | 'platform_admin'
  request_digest: string
  event_digest: string
  resolved_at: string
  challenge_status: ComputeSettlementChallengeResolutionAction
  correction_required: boolean
  balance_effect: 'consumer_provider_and_platform_balances_unchanged'
  settlement_release_effect: 'blocked_by_accepted_challenge' | 'challenge_no_longer_blocks_release'
  replayed: boolean
}

export interface OpenSettlementChallengeQueueResponse {
  challenge_candidates: ComputeSettlementChallengeReceipt[]
}
