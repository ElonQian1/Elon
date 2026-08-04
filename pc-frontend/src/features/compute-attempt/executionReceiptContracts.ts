import {
  type ComputeAttemptTerminalCandidateReceipt,
  type ComputeDeclaredResultArtifactInput,
} from './terminalContracts'
import { type ComputeMeterReading } from './platformObservationContracts'
import { type ComputeAttemptVerificationDecisionReceipt } from './verificationContracts'

export interface ComputePendingExecutionReceiptCandidate {
  verification_decision: ComputeAttemptVerificationDecisionReceipt
  terminal_candidate: ComputeAttemptTerminalCandidateReceipt
}

export interface IssueComputeAttemptExecutionReceiptBody {
  expected_verification_decision_id: string
  expected_verification_event_digest: string
  idempotency_key: string
  confirm_execution_receipt_only: true
}

export interface ComputeExecutionUsage {
  declared_usage: ComputeMeterReading[]
  observed_usage: ComputeMeterReading[]
  verified_usage: ComputeMeterReading[]
  compensable_usage: ComputeMeterReading[]
}

export interface ComputeAttestationEvidence {
  evidence_kind: string
  issuer: string
  evidence_digest: string
  artifact_ref: string | null
  observed_at: string
}

export interface ComputeReceiptVerificationDecision {
  status: string
  policy_id: string
  policy_version: number
  reason_codes: string[]
  duplicate_receipt_ids: string[]
  challenge_receipt_ids: string[]
  decision_digest: string
  decided_at: string | null
}

export interface ComputeExecutionReceipt {
  schema: string
  receipt_id: string
  receipt_digest: string
  job_id: string
  reservation_id: string
  attempt_lease_id: string
  attempt_no: number
  fencing_generation: number
  provider_id: string
  executor_id: string
  offer_id: string
  offer_version: number
  offer_digest: string
  plugin_digest: string | null
  runner_digest: string
  model_digest: string | null
  tokenizer_digest: string | null
  input_digest: string
  output_digest: string | null
  result_artifacts: ComputeDeclaredResultArtifactInput[]
  execution_status: string
  usage: ComputeExecutionUsage
  attestations: ComputeAttestationEvidence[]
  verification: ComputeReceiptVerificationDecision
  started_at: string
  finished_at: string
  created_at: string
}

export interface ComputeAttemptExecutionReceiptEnvelope {
  receipt: ComputeExecutionReceipt
  verification_decision_id: string
  verification_event_digest: string
  request_digest: string
  issued_by_user_id: string
  issued_at: string
  execution_effect: string
  lease_effect: string
  job_effect: string
  capacity_effect: string
  reservation_effect: string
  money_effect: string
  replayed: boolean
}
