import {
  type ComputeAttemptExecutionReceiptEnvelope,
} from './executionReceiptContracts'
import {
  type ComputeAttemptTerminalCandidateReceipt,
} from './terminalContracts'
import { type ComputeMeterReading } from './platformObservationContracts'

export interface ComputeAttemptRevisionBinding {
  revision: number
  digest: string
}

export interface ComputeJobVersionBinding {
  job_id: string
  job_revision: number
  job_digest: string
}

export interface ComputeCapacityClaimBinding {
  claim_id: string
  claim_revision: number
  claim_digest: string
}

export interface ComputeReservedCapacity {
  meter: string
  quantity: number
}

export interface ComputePendingAttemptFinalizationCandidate {
  execution_receipt: ComputeAttemptExecutionReceiptEnvelope
  terminal_candidate: ComputeAttemptTerminalCandidateReceipt
  expected_lease: ComputeAttemptRevisionBinding
  expected_fencing_generation: number
  expected_job: ComputeJobVersionBinding
  expected_reservation: ComputeAttemptRevisionBinding
  expected_claim: ComputeCapacityClaimBinding
  compensable_usage: ComputeMeterReading[]
  lease_effect: string
  job_effect: string
  reservation_effect: string
  capacity_effect: string
  money_effect: string
  settlement_effect: string
}

export interface FinalizeComputeAttemptBody {
  expected_execution_receipt_id: string
  expected_execution_receipt_digest: string
  expected_lease_revision: number
  expected_lease_digest: string
  expected_fencing_generation: number
  expected_job_revision: number
  expected_job_digest: string
  expected_reservation_revision: number
  expected_reservation_digest: string
  expected_claim_revision: number
  expected_claim_digest: string
  idempotency_key: string
  confirm_trusted_terminal_and_capacity: true
}

export interface ComputeAttemptCapacityTransactionRef {
  transaction_id: string
  transaction_digest: string
  ledger_sequence: number
  event_kind: string
}

export interface ComputeAttemptFinalizationReceipt {
  schema: string
  finalization_id: string
  execution_receipt_id: string
  execution_receipt_digest: string
  lease_id: string
  provider_id: string
  consumer_account_id: string
  outcome: string
  reason_code: string
  source_lease: ComputeAttemptRevisionBinding
  terminal_lease: ComputeAttemptRevisionBinding
  source_job: ComputeJobVersionBinding
  terminal_job: ComputeJobVersionBinding
  source_reservation: ComputeAttemptRevisionBinding
  terminal_reservation: ComputeAttemptRevisionBinding
  source_claim: ComputeCapacityClaimBinding
  terminal_claim: ComputeCapacityClaimBinding
  compensable_usage: ComputeReservedCapacity[]
  capacity_consumed: ComputeReservedCapacity[]
  capacity_returned: ComputeReservedCapacity[]
  capacity_transactions: ComputeAttemptCapacityTransactionRef[]
  request_digest: string
  event_digest: string
  finalized_by_user_id: string
  effective_at: string
  finalized_at: string
  execution_effect: string
  lease_effect: string
  job_effect: string
  capacity_effect: string
  reservation_effect: string
  money_effect: string
  settlement_effect: string
  replayed: boolean
}
