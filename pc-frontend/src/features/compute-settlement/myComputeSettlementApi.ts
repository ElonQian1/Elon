import { api } from '../../api/client'
import {
  type SettlementWithdrawalQueuePage,
  type SettlementWithdrawalRequest,
  type SettlementWithdrawalTerminal,
  type WithdrawalStatus,
} from './computeSettlementApi'
import {
  type ComputeSettlementChallengeHistoryResponse,
} from '../compute-attempt/settlementChallengeHistoryContracts'

export type {
  ComputeSettlementChallengeHistoryItem,
} from '../compute-attempt/settlementChallengeHistoryContracts'

export interface MyComputeProvider {
  provider_id: string
  provider_kind: string
  display_name: string
  status: string
  trust_tier: string
  home_region?: string | null
  policy_revision: number
  has_routing: boolean
  provider_digest: string
  capabilities?: {
    task_kinds: string[]
    accelerator_kinds: string[]
    regions: string[]
    allowed_data_classes: string[]
    supports_streaming: boolean
    supports_checkpointing: boolean
  }
}

export type MyComputeProviderKind = 'user_node' | 'managed_cluster'

export type ComputeDataClass = 'public' | 'low_sensitivity' | 'restricted'

export interface CreateMyComputeProviderBody {
  provider_id: string
  provider_kind: MyComputeProviderKind
  display_name: string
  home_region?: string | null
  task_kinds: string[]
  accelerator_kinds: string[]
  regions: string[]
  allowed_data_classes: ComputeDataClass[]
  supports_streaming: boolean
  supports_checkpointing: boolean
  declared_hardware_digest?: string | null
}

export interface ProviderSettlementAccount {
  schema: string
  provider_id: string
  provider_policy_revision: number
  provider_digest: string
  provider_account_id: string
  currency: 'CNY'
  pending_micros: number
  available_micros: number
  disputed_micros: number
  withdrawn_micros: number
  account_revision: number
  updated_at?: string | null
  withdrawal_request_count: number
  pending_terminal_count: number
  pending_terminal_micros: number
  external_paid_attested_count: number
  returned_to_available_micros: number
  projection_digest: string
  audit_status: string
}

export type WithdrawalDestinationKind =
  | 'bank_account_vault_ref'
  | 'digital_wallet_vault_ref'
  | 'sui_address_ref'
  | 'other_vault_ref'

export interface CreateMyWithdrawalBody {
  amount_micros: number
  destination_kind: WithdrawalDestinationKind
  destination_ref: string
  idempotency_key: string
  confirm_internal_reserve_only: boolean
  confirm_destination_ref_contains_no_secret: boolean
}

export interface CancelMyWithdrawalBody {
  expected_withdrawal_event_digest: string
  expected_request_posting_id: string
  expected_request_posting_digest: string
  reason_code: string
  reason_detail?: string | null
  idempotency_key: string
  confirm_internal_refund_only: boolean
}

export const myComputeSettlementApi = {
  providers: (limit = 100) =>
    api.get<MyComputeProvider[]>(`/api/me/compute/providers?limit=${limit}`),
  createProvider: (body: CreateMyComputeProviderBody) =>
    api.post<MyComputeProvider>('/api/me/compute/providers', body),
  account: (providerId: string) =>
    api.get<ProviderSettlementAccount>(
      `/api/me/compute/providers/${encodeURIComponent(providerId)}/settlement-account`,
    ),
  withdrawals: (providerId: string, status: WithdrawalStatus, limit = 50) =>
    api.get<SettlementWithdrawalQueuePage>(
      `/api/me/compute/providers/${encodeURIComponent(providerId)}/settlement-withdrawal-queue?status=${encodeURIComponent(status)}&limit=${limit}`,
    ),
  challengeHistory: (providerId: string, limit = 100) =>
    api.get<ComputeSettlementChallengeHistoryResponse>(
      `/api/me/compute/providers/${encodeURIComponent(providerId)}/settlement-challenges/history?limit=${limit}`,
    ).then((response) => response.challenge_history),
  createWithdrawal: (providerId: string, body: CreateMyWithdrawalBody) =>
    api.post<SettlementWithdrawalRequest>(
      `/api/me/compute/providers/${encodeURIComponent(providerId)}/settlement-withdrawals`,
      body,
    ),
  cancelWithdrawal: (
    providerId: string,
    withdrawalId: string,
    body: CancelMyWithdrawalBody,
  ) =>
    api.post<SettlementWithdrawalTerminal>(
      `/api/me/compute/providers/${encodeURIComponent(providerId)}/settlement-withdrawals/${encodeURIComponent(withdrawalId)}/cancellation`,
      body,
    ),
}
