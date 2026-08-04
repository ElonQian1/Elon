import {
  type ComputeAttemptExecutionReceiptEnvelope,
} from './executionReceiptContracts'
import {
  type ComputeAttemptFinalizationReceipt,
  type ComputeJobVersionBinding,
} from './finalizationContracts'
import { type MyComputePriceSnapshotView } from '../compute-supply/computePriceSnapshotApi'

export interface ComputeSettlementAmounts {
  consumer_charge_micros: number
  provider_payable_micros: number
  platform_margin_micros: number
  third_party_cost_micros: number
  transfer_fee_micros: number
  storage_fee_micros: number
  verification_fee_micros: number
  availability_bonus_micros: number
  acceptance_bonus_micros: number
  delivery_penalty_micros: number
  refund_micros: number
}

export interface ComputePendingAttemptSettlementPreview {
  currency: 'CNY'
  budget_reserved_fen: number
  consumer_charge_fen: number
  consumer_refund_fen: number
  amounts: ComputeSettlementAmounts
  reason_codes: string[]
}

export interface ComputePendingAttemptSettlementCandidate {
  finalization: ComputeAttemptFinalizationReceipt
  execution_receipt: ComputeAttemptExecutionReceiptEnvelope
  expected_job: ComputeJobVersionBinding
  expected_budget_reservation_id: string
  price_snapshot: MyComputePriceSnapshotView['snapshot']
  provider_account_id: string
  preview: ComputePendingAttemptSettlementPreview
  money_effect: string
  provider_balance_effect: string
  external_payment_effect: 'none'
}

export interface SettleComputeAttemptBody {
  expected_finalization_id: string
  expected_finalization_event_digest: string
  expected_execution_receipt_id: string
  expected_execution_receipt_digest: string
  expected_job_revision: number
  expected_job_digest: string
  expected_budget_reservation_id: string
  expected_price_snapshot_id: string
  expected_price_snapshot_digest: string
  idempotency_key: string
  confirm_consumer_capture_and_provider_pending: true
}

export interface ComputeSettlementReceipt {
  schema: string
  settlement_receipt_id: string
  settlement_receipt_digest: string
  execution_receipt_id: string
  execution_receipt_digest: string
  reservation_id: string
  price_snapshot_id: string
  price_snapshot_digest: string
  consumer_account_id: string
  provider_account_id: string
  currency: string
  amounts: ComputeSettlementAmounts
  verified_usage_digest: string
  compensable_usage_digest: string
  balance_state: string
  correction_of_receipt_id: string | null
  ledger_posting_ref: string | null
  reason_codes: string[]
  created_at: string
  available_at: string | null
}

export interface ComputeAttemptSettlementReceipt {
  schema: string
  settlement: ComputeSettlementReceipt
  lease_id: string
  finalization_id: string
  finalization_event_digest: string
  budget_reservation_id: string
  budget_reserved_fen: number
  consumer_charged_fen: number
  consumer_refunded_fen: number
  consumer_balance_after_fen: number
  provider_policy_revision: number
  provider_digest: string
  source_job: ComputeJobVersionBinding
  terminal_job: ComputeJobVersionBinding
  posting_id: string
  posting_digest: string
  provider_pending_balance_micros: number
  platform_pending_balance_micros: number
  request_digest: string
  event_digest: string
  settled_by_user_id: string
  settled_at: string
  money_effect: string
  provider_balance_effect: string
  replayed: boolean
}
