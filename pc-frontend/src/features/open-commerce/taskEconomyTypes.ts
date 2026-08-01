export interface TaskEconomyProjectSetting {
  project_id: string
  enabled: boolean
  shadow_only: true
  updated_by_user_id?: string
  updated_at?: string
}

export interface UsageReceipt {
  id: string
  project_id: string
  subject_type: string
  subject_id: string
  source_type: string
  source_id: string
  source_digest: string
  consumer_user_id: string
  provider_user_id?: string
  units: number
  amount_micros: number
  provider_amount_micros: number
  currency: string
  billing_source: string
  source_status: string
  occurred_at: string
  created_at: string
}

export interface SettlementIntent {
  id: string
  project_id: string
  matter_id?: string
  assignment_id?: string
  payer_user_id: string
  payee_user_id?: string
  idempotency_key: string
  policy_version: string
  policy_digest: string
  status: 'pending' | 'posted' | 'voided'
  shadow_only: true
  created_at: string
  updated_at: string
}

export interface SettlementReceipt {
  id: string
  project_id: string
  intent_id: string
  posting_key: string
  status: 'reconciled' | 'voided'
  compute_amount_micros: number
  provider_amount_micros: number
  platform_amount_micros: number
  outcome_reward_micros: number
  review_reward_micros: number
  currency: string
  shadow_only: true
  accepted_matter_id?: string
  reason: string
  receipt_kind: 'standard' | 'correction_reversal' | 'correction_replacement'
  correction_id?: string
  created_at: string
}

export interface LedgerEntry {
  id: string
  transaction_id: string
  account_key: string
  user_id?: string
  side: 'debit' | 'credit'
  amount_micros: number
  currency: string
  created_at: string
}

export interface LedgerTransaction {
  id: string
  project_id: string
  settlement_receipt_id: string
  posting_key: string
  description: string
  created_at: string
  entries: LedgerEntry[]
}

export interface TaskEconomyOverview {
  schema: string
  project_id: string
  runtime_enabled: boolean
  project_setting: TaskEconomyProjectSetting
  shadow_only: true
  totals: {
    usage_receipts: number
    pending_intents: number
    posted_intents: number
    voided_intents: number
    settlement_receipts: number
    compute_amount_micros: number
    provider_amount_micros: number
    platform_amount_micros: number
  }
  usage_receipts: UsageReceipt[]
  intents: SettlementIntent[]
  settlement_receipts: SettlementReceipt[]
}

export interface SettlementReceiptDetail {
  receipt: SettlementReceipt
  intent: SettlementIntent
  usage_receipts: UsageReceipt[]
  ledger_transaction?: LedgerTransaction
}

export interface SuiSettlementEnvelope {
  schema: string
  source_receipt_id: string
  source_posting_key: string
  project_object_key: string
  intent_object_key: string
  receipt_object_key: string
  amount_micros: number
  provider_amount_micros: number
  platform_amount_micros: number
  currency: string
  shadow_only: true
  ptb_steps: string[]
  network_submission: 'not_submitted'
}

export type SuiTargetNetwork = 'devnet' | 'testnet' | 'mainnet'

export interface SuiProjectionPackage {
  id: string
  project_id: string
  settlement_receipt_id: string
  target_network: SuiTargetNetwork
  package_schema: string
  projection_digest: string
  source_receipt_digest: string
  envelope: SuiSettlementEnvelope
  integrity_status: 'verified' | 'conflict'
  submission_readiness: 'adapter_required' | 'integrity_conflict' | 'dispute_blocked'
  network_submission: 'not_submitted'
  submission_attempts: number
  last_error?: string
  created_by_user_id: string
  verified_at: string
  created_at: string
  updated_at: string
}

export type SettlementDisputeStatus = 'open' | 'accepted' | 'rejected' | 'withdrawn'
export type SettlementDisputeReason =
  | 'amount'
  | 'provider_allocation'
  | 'policy'
  | 'source_evidence'
  | 'other'

export interface SettlementDispute {
  id: string
  project_id: string
  settlement_receipt_id: string
  status: SettlementDisputeStatus
  reason_code: SettlementDisputeReason
  summary: string
  evidence_ref?: string
  opened_by_user_id: string
  resolved_by_user_id?: string
  resolution_note?: string
  opened_at: string
  resolved_at?: string
  updated_at: string
}

export interface SettlementDisputeEvent {
  id: string
  dispute_id: string
  action: 'opened' | 'accepted' | 'rejected' | 'withdrawn'
  previous_status?: SettlementDisputeStatus
  next_status: SettlementDisputeStatus
  actor_user_id: string
  note?: string
  created_at: string
}

export interface SettlementDisputeDetail {
  dispute: SettlementDispute
  events: SettlementDisputeEvent[]
  blocks_projection: boolean
}

export type SettlementCorrectionStatus = 'matter_pending' | 'posted' | 'canceled'

export interface SettlementCorrection {
  id: string
  project_id: string
  dispute_id: string
  original_settlement_receipt_id: string
  correction_matter_id: string
  status: SettlementCorrectionStatus
  corrected_compute_amount_micros: number
  corrected_provider_amount_micros: number
  corrected_platform_amount_micros: number
  summary: string
  evidence_ref?: string
  created_by_user_id: string
  posted_by_user_id?: string
  reversal_receipt_id?: string
  replacement_receipt_id?: string
  matter_status: string
  matter_final_decision?: string
  created_at: string
  posted_at?: string
  updated_at: string
}

export interface SettlementCorrectionEvent {
  id: string
  correction_id: string
  action: 'matter_created' | 'posted' | 'canceled'
  previous_status?: SettlementCorrectionStatus
  next_status: SettlementCorrectionStatus
  actor_user_id: string
  note?: string
  created_at: string
}

export interface SettlementCorrectionDetail {
  correction: SettlementCorrection
  events: SettlementCorrectionEvent[]
  original_receipt: SettlementReceipt
  reversal_receipt?: SettlementReceipt
  replacement_receipt?: SettlementReceipt
}
