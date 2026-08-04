import { api } from '../../api/client'

export interface PlatformSettlementAccount {
  schema: string
  account_kind: 'platform'
  account_id: string
  currency: 'CNY'
  pending_micros: number
  available_micros: number
  disputed_micros: number
  withdrawn_micros: number
  account_revision: number
  updated_at?: string | null
  settlement_posting_count: number
  gross_margin_credited_micros: number
  correction_posting_count: number
  corrected_margin_micros: number
  release_posting_count: number
  released_margin_micros: number
  projection_digest: string
  audit_status: string
  withdrawal_effect: string
}

export interface SettlementChallengeGate {
  status: string
  blocked: boolean
  correction_required: boolean
}

export interface SettlementReleaseCandidate {
  lease_id: string
  settlement_receipt_id: string
  settled_at: string
  challenge_deadline: string
  challenge_gate: SettlementChallengeGate
  eligible: boolean
  blocked_reason?: string | null
}

export interface SettlementReleaseCandidatePage {
  schema: string
  as_of: string
  limit: number
  candidates: SettlementReleaseCandidate[]
  money_effect: string
  external_transfer_effect: string
}

export interface SettlementReleaseBatchReport {
  schema: string
  scanned: number
  eligible: number
  released: Array<{ release_id: string; lease_id: string; replayed: boolean }>
  skipped: SettlementReleaseCandidate[]
  failed: Array<{ lease_id: string; settlement_receipt_id: string; error: string }>
  transaction_scope: string
  money_effect: string
  external_transfer_effect: string
}

export type WithdrawalStatus =
  | 'all'
  | 'pending'
  | 'cancelled'
  | 'rejected'
  | 'external_paid_attested'

export interface SettlementWithdrawalRequest {
  withdrawal_id: string
  provider_id: string
  provider_account_id: string
  amount_micros: number
  currency: 'CNY'
  destination_kind: string
  destination_ref: string
  request_posting_id: string
  request_posting_digest: string
  event_digest: string
  requested_at: string
}

export type WithdrawalTerminalAction = 'rejected' | 'external_paid_attested'

export type WithdrawalEvidenceKind =
  | 'bank_receipt'
  | 'payment_provider_receipt'
  | 'sui_transaction_digest'
  | 'other_receipt'

export interface TerminalizeSettlementWithdrawalBody {
  expected_withdrawal_event_digest: string
  expected_request_posting_id: string
  expected_request_posting_digest: string
  action: WithdrawalTerminalAction
  reason_code: string
  reason_detail?: string | null
  external_evidence_kind?: WithdrawalEvidenceKind | null
  external_evidence_ref?: string | null
  external_evidence_digest?: string | null
  idempotency_key: string
  confirm_refund_or_attestation_only: boolean
  confirm_external_payment_already_completed: boolean
  confirm_evidence_ref_contains_no_secret: boolean
}

export interface SettlementWithdrawalTerminal {
  action: Exclude<WithdrawalStatus, 'all' | 'pending'>
  reason_code: string
  reason_detail?: string | null
  external_evidence_kind?: WithdrawalEvidenceKind | null
  external_evidence_ref?: string | null
  external_evidence_digest?: string | null
  terminal_at: string
}

export interface SettlementWithdrawalQueueItem {
  status: Exclude<WithdrawalStatus, 'all'>
  request: SettlementWithdrawalRequest
  terminal?: SettlementWithdrawalTerminal | null
}

export interface SettlementWithdrawalQueuePage {
  schema: string
  status_filter: WithdrawalStatus
  limit: number
  items: SettlementWithdrawalQueueItem[]
  external_transfer_effect: string
}

export const computeSettlementApi = {
  platformAccount: () =>
    api.get<PlatformSettlementAccount>('/api/admin/compute/settlement-account'),
  dueReleases: (limit = 50) =>
    api.get<SettlementReleaseCandidatePage>(
      `/api/admin/compute/settlement-releases/due?limit=${limit}`,
    ),
  releaseDue: (limit = 50) =>
    api.post<SettlementReleaseBatchReport>('/api/admin/compute/settlement-releases/due', {
      limit,
      confirm_each_item_uses_v198_internal_release_only: true,
    }),
  withdrawals: (status: WithdrawalStatus, limit = 50) =>
    api.get<SettlementWithdrawalQueuePage>(
      `/api/admin/compute/settlement-withdrawals?status=${encodeURIComponent(status)}&limit=${limit}`,
    ),
  terminalizeWithdrawal: (
    withdrawalId: string,
    body: TerminalizeSettlementWithdrawalBody,
  ) =>
    api.post<SettlementWithdrawalTerminal>(
      `/api/admin/compute/settlement-withdrawals/${encodeURIComponent(withdrawalId)}/terminal`,
      body,
    ),
}
