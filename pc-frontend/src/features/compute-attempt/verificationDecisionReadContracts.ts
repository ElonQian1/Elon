import {
  exactObject,
  expectLiteral,
  expectSha256,
  expectString,
} from './federationHistoricalLineageContracts'
import { type ValidatedExecutionVerificationLineageRead } from './federationHistoricalVerificationLineageContracts'

export const VERIFICATION_DECISION_READ_SCHEMA =
  'compute_federation.attempt_verification_decision.v1' as const

const VERIFICATION_DECISION_READ_KEYS = [
  'schema',
  'verification_decision_id',
  'terminal_candidate_id',
  'terminal_candidate_event_digest',
  'consumer_review_id',
  'consumer_review_event_digest',
  'platform_observation_id',
  'platform_observation_event_digest',
  'lease_id',
  'provider_id',
  'consumer_account_id',
  'source_lease_revision',
  'source_lease_digest',
  'fencing_generation',
  'job_id',
  'job_revision',
  'job_digest',
  'reservation_id',
  'reservation_revision',
  'reservation_digest',
  'capacity_claim_id',
  'capacity_claim_revision',
  'capacity_claim_digest',
  'final_usage_snapshot_id',
  'final_usage_sequence_no',
  'final_provider_usage_digest',
  'platform_observed_usage_digest',
  'candidate_outcome',
  'consumer_decision',
  'observed_outcome',
  'policy_id',
  'policy_version',
  'decision',
  'reason_codes',
  'reason_codes_digest',
  'decision_ref',
  'verified_usage',
  'verified_usage_digest',
  'compensable_usage',
  'compensable_usage_digest',
  'request_digest',
  'event_digest',
  'decided_by_user_id',
  'decided_at',
  'verification_effect',
  'execution_receipt_effect',
  'lease_effect',
  'job_effect',
  'capacity_effect',
  'reservation_effect',
  'money_effect',
  'replayed',
] as const

const METER_READING_KEYS = [
  'meter',
  'quantity',
  'source_kind',
  'source_id',
  'reading_digest',
  'observed_at',
] as const

const CANDIDATE_OUTCOMES = ['succeeded', 'failed', 'canceled'] as const
const CONSUMER_DECISIONS = ['accepted', 'rejected', 'disputed'] as const
const OBSERVED_OUTCOMES = ['succeeded', 'failed', 'canceled', 'indeterminate'] as const
const VERIFICATION_DECISIONS = ['accepted', 'rejected', 'disputed'] as const
const POLICY_ID = 'conservative_min_v1' as const
const POLICY_VERSION = 1 as const
const POLICY_SOURCE_ID = 'conservative_min_v1@1' as const

type CandidateOutcome = (typeof CANDIDATE_OUTCOMES)[number]
type ConsumerDecision = (typeof CONSUMER_DECISIONS)[number]
type ObservedOutcome = (typeof OBSERVED_OUTCOMES)[number]
export type RetainedVerificationDecision = (typeof VERIFICATION_DECISIONS)[number]
type VerificationEffect = 'verified_usage_recorded' | 'rejection_recorded' | 'dispute_recorded'
type VerificationMeterSourceKind = 'verification_policy' | 'compensation_policy'

export interface ValidatedVerificationMeterReading {
  meter: string
  quantity: number
  source_kind: VerificationMeterSourceKind
  source_id: typeof POLICY_SOURCE_ID
  reading_digest: string
  observed_at: string
}

export interface ValidatedVerificationDecisionRead {
  schema: typeof VERIFICATION_DECISION_READ_SCHEMA
  verification_decision_id: string
  terminal_candidate_id: string
  terminal_candidate_event_digest: string
  consumer_review_id: string
  consumer_review_event_digest: string
  platform_observation_id: string
  platform_observation_event_digest: string
  lease_id: string
  provider_id: string
  consumer_account_id: string
  source_lease_revision: number
  source_lease_digest: string
  fencing_generation: number
  job_id: string
  job_revision: number
  job_digest: string
  reservation_id: string
  reservation_revision: number
  reservation_digest: string
  capacity_claim_id: string
  capacity_claim_revision: number
  capacity_claim_digest: string
  final_usage_snapshot_id: string
  final_usage_sequence_no: number
  final_provider_usage_digest: string
  platform_observed_usage_digest: string
  candidate_outcome: CandidateOutcome
  consumer_decision: ConsumerDecision
  observed_outcome: ObservedOutcome
  policy_id: typeof POLICY_ID
  policy_version: typeof POLICY_VERSION
  decision: RetainedVerificationDecision
  reason_codes: string[]
  reason_codes_digest: string
  decision_ref: string
  verified_usage: ValidatedVerificationMeterReading[]
  verified_usage_digest: string
  compensable_usage: ValidatedVerificationMeterReading[]
  compensable_usage_digest: string
  request_digest: string
  event_digest: string
  decided_by_user_id: string
  decided_at: string
  verification_effect: VerificationEffect
  execution_receipt_effect: 'none'
  lease_effect: 'unchanged'
  job_effect: 'unchanged'
  capacity_effect: 'unchanged'
  reservation_effect: 'unchanged'
  money_effect: 'preauthorization_unchanged'
  replayed: false
}

export function validateVerificationDecisionRead(
  value: unknown,
): ValidatedVerificationDecisionRead {
  const receipt = exactObject(value, VERIFICATION_DECISION_READ_KEYS, 'v192 retained Verification 回执')
  expectLiteral(receipt.schema, VERIFICATION_DECISION_READ_SCHEMA, 'v192 schema')
  const decision = enumValue(receipt.decision, VERIFICATION_DECISIONS, 'Verification decision')
  const candidateOutcome = enumValue(receipt.candidate_outcome, CANDIDATE_OUTCOMES, 'candidate_outcome')
  const consumerDecision = enumValue(receipt.consumer_decision, CONSUMER_DECISIONS, 'consumer_decision')
  const observedOutcome = enumValue(receipt.observed_outcome, OBSERVED_OUTCOMES, 'observed_outcome')
  const decidedAt = safeText(receipt.decided_at, 'decided_at', 100)
  const reasonCodes = validateReasonCodes(receipt.reason_codes)
  const verifiedUsage = validateMeterReadings(
    receipt.verified_usage,
    'verification_policy',
    decidedAt,
  )
  const compensableUsage = validateMeterReadings(
    receipt.compensable_usage,
    'compensation_policy',
    decidedAt,
  )
  validatePolicyResult(
    decision,
    candidateOutcome,
    consumerDecision,
    observedOutcome,
    verifiedUsage,
    compensableUsage,
  )

  const policyVersion = positiveSafeInteger(receipt.policy_version, 'policy_version')
  if (policyVersion !== POLICY_VERSION) throw new Error('policy_version 不符合 conservative_min_v1@1')
  expectLiteral(receipt.policy_id, POLICY_ID, 'policy_id')
  const expectedVerificationEffect: Record<RetainedVerificationDecision, VerificationEffect> = {
    accepted: 'verified_usage_recorded',
    rejected: 'rejection_recorded',
    disputed: 'dispute_recorded',
  }
  expectLiteral(
    receipt.verification_effect,
    expectedVerificationEffect[decision],
    'verification_effect',
  )
  expectLiteral(receipt.execution_receipt_effect, 'none', 'execution_receipt_effect')
  expectLiteral(receipt.lease_effect, 'unchanged', 'lease_effect')
  expectLiteral(receipt.job_effect, 'unchanged', 'job_effect')
  expectLiteral(receipt.capacity_effect, 'unchanged', 'capacity_effect')
  expectLiteral(receipt.reservation_effect, 'unchanged', 'reservation_effect')
  expectLiteral(receipt.money_effect, 'preauthorization_unchanged', 'money_effect')
  if (receipt.replayed !== false) throw new Error('historical v192 read 的 replayed 必须为 false')

  return {
    schema: VERIFICATION_DECISION_READ_SCHEMA,
    verification_decision_id: id(receipt.verification_decision_id, 'verification_decision_id'),
    terminal_candidate_id: id(receipt.terminal_candidate_id, 'terminal_candidate_id'),
    terminal_candidate_event_digest: digest(
      receipt.terminal_candidate_event_digest,
      'terminal_candidate_event_digest',
    ),
    consumer_review_id: id(receipt.consumer_review_id, 'consumer_review_id'),
    consumer_review_event_digest: digest(
      receipt.consumer_review_event_digest,
      'consumer_review_event_digest',
    ),
    platform_observation_id: id(receipt.platform_observation_id, 'platform_observation_id'),
    platform_observation_event_digest: digest(
      receipt.platform_observation_event_digest,
      'platform_observation_event_digest',
    ),
    lease_id: id(receipt.lease_id, 'lease_id'),
    provider_id: id(receipt.provider_id, 'provider_id'),
    consumer_account_id: id(receipt.consumer_account_id, 'consumer_account_id'),
    source_lease_revision: positiveSafeInteger(receipt.source_lease_revision, 'source_lease_revision'),
    source_lease_digest: digest(receipt.source_lease_digest, 'source_lease_digest'),
    fencing_generation: positiveSafeInteger(receipt.fencing_generation, 'fencing_generation'),
    job_id: id(receipt.job_id, 'job_id'),
    job_revision: positiveSafeInteger(receipt.job_revision, 'job_revision'),
    job_digest: digest(receipt.job_digest, 'job_digest'),
    reservation_id: id(receipt.reservation_id, 'reservation_id'),
    reservation_revision: positiveSafeInteger(receipt.reservation_revision, 'reservation_revision'),
    reservation_digest: digest(receipt.reservation_digest, 'reservation_digest'),
    capacity_claim_id: id(receipt.capacity_claim_id, 'capacity_claim_id'),
    capacity_claim_revision: positiveSafeInteger(
      receipt.capacity_claim_revision,
      'capacity_claim_revision',
    ),
    capacity_claim_digest: digest(receipt.capacity_claim_digest, 'capacity_claim_digest'),
    final_usage_snapshot_id: id(receipt.final_usage_snapshot_id, 'final_usage_snapshot_id'),
    final_usage_sequence_no: positiveSafeInteger(
      receipt.final_usage_sequence_no,
      'final_usage_sequence_no',
    ),
    final_provider_usage_digest: digest(
      receipt.final_provider_usage_digest,
      'final_provider_usage_digest',
    ),
    platform_observed_usage_digest: digest(
      receipt.platform_observed_usage_digest,
      'platform_observed_usage_digest',
    ),
    candidate_outcome: candidateOutcome,
    consumer_decision: consumerDecision,
    observed_outcome: observedOutcome,
    policy_id: POLICY_ID,
    policy_version: POLICY_VERSION,
    decision,
    reason_codes: reasonCodes,
    reason_codes_digest: digest(receipt.reason_codes_digest, 'reason_codes_digest'),
    decision_ref: safeText(receipt.decision_ref, 'decision_ref', 1_000),
    verified_usage: verifiedUsage,
    verified_usage_digest: digest(receipt.verified_usage_digest, 'verified_usage_digest'),
    compensable_usage: compensableUsage,
    compensable_usage_digest: digest(
      receipt.compensable_usage_digest,
      'compensable_usage_digest',
    ),
    request_digest: digest(receipt.request_digest, 'request_digest'),
    event_digest: digest(receipt.event_digest, 'event_digest'),
    decided_by_user_id: id(receipt.decided_by_user_id, 'decided_by_user_id'),
    decided_at: decidedAt,
    verification_effect: expectedVerificationEffect[decision],
    execution_receipt_effect: 'none',
    lease_effect: 'unchanged',
    job_effect: 'unchanged',
    capacity_effect: 'unchanged',
    reservation_effect: 'unchanged',
    money_effect: 'preauthorization_unchanged',
    replayed: false,
  }
}

export function validateVerificationDecisionReadForLease(value: unknown, expectedLeaseId: string) {
  const receipt = validateVerificationDecisionRead(value)
  expectSame(receipt.lease_id, id(expectedLeaseId, 'expected lease_id'), 'requested Lease→v192 lease_id')
  return receipt
}

export function validateVerificationDecisionLineage(
  decision: ValidatedVerificationDecisionRead,
  verification: ValidatedExecutionVerificationLineageRead,
) {
  expectLiteral(decision.decision, 'accepted', 'execution_verification_source_v1 的 native v192 decision')
  const lineage = verification.carrier.lineage
  expectSame(decision.final_usage_snapshot_id, lineage.provider_declared_usage.usage_snapshot_id, 'v192→v188 snapshot ID')
  expectSame(decision.final_usage_sequence_no, lineage.provider_declared_usage.usage_sequence_no, 'v192→v188 sequence')
  expectSame(decision.final_provider_usage_digest, lineage.provider_declared_usage.cumulative_usage_digest, 'v192→v188 usage digest')
  expectSame(decision.terminal_candidate_id, lineage.terminal_candidate.terminal_candidate_id, 'v192→v189 candidate ID')
  expectSame(decision.terminal_candidate_event_digest, lineage.terminal_candidate.terminal_candidate_event_digest, 'v192→v189 event digest')
  expectSame(decision.consumer_review_id, lineage.consumer_review.consumer_review_id, 'v192→v190 review ID')
  expectSame(decision.consumer_review_event_digest, lineage.consumer_review.consumer_review_event_digest, 'v192→v190 event digest')
  expectSame(decision.platform_observation_id, lineage.platform_observation.platform_observation_id, 'v192→v191 observation ID')
  expectSame(decision.platform_observation_event_digest, lineage.platform_observation.platform_observation_event_digest, 'v192→v191 event digest')
  expectSame(decision.platform_observed_usage_digest, lineage.platform_observation.cumulative_observed_usage_digest, 'v192→v191 usage digest')
  expectSame(decision.verification_decision_id, lineage.verification_decision.verification_decision_id, 'v192 Verification ID')
  expectSame(decision.event_digest, lineage.verification_decision.verification_event_digest, 'v192 Verification event digest')
  expectSame(decision.verified_usage_digest, lineage.verification_decision.verified_usage_digest, 'v192 verified usage digest')
  expectSame(decision.compensable_usage_digest, lineage.verification_decision.compensable_usage_digest, 'v192 compensable usage digest')
}

function validateReasonCodes(value: unknown) {
  if (!Array.isArray(value) || value.length < 1 || value.length > 16) {
    throw new Error('reason_codes 必须包含 1 至 16 项')
  }
  const result = value.map((reason, index) => {
    const code = safeText(reason, `reason_codes[${index}]`, 100)
    if (code !== asciiLower(code)) throw new Error('reason_codes 必须保留 v192 lowercase normalization')
    return code
  })
  for (let index = 1; index < result.length; index += 1) {
    if (compareUtf8(result[index - 1], result[index]) >= 0) {
      throw new Error('reason_codes 必须按 UTF-8 严格递增且无重复')
    }
  }
  return result
}

function validateMeterReadings(
  value: unknown,
  sourceKind: VerificationMeterSourceKind,
  decidedAt: string,
) {
  if (!Array.isArray(value) || value.length < 1 || value.length > 64) {
    throw new Error(`${sourceKind} readings 必须包含 1 至 64 项`)
  }
  const readings = value.map((entry, index): ValidatedVerificationMeterReading => {
    const reading = exactObject(entry, METER_READING_KEYS, `${sourceKind}[${index}]`)
    expectLiteral(reading.source_kind, sourceKind, `${sourceKind}[${index}].source_kind`)
    expectLiteral(reading.source_id, POLICY_SOURCE_ID, `${sourceKind}[${index}].source_id`)
    expectLiteral(reading.observed_at, decidedAt, `${sourceKind}[${index}].observed_at`)
    return {
      meter: safeText(reading.meter, `${sourceKind}[${index}].meter`, 120),
      quantity: nonNegativeSafeInteger(reading.quantity, `${sourceKind}[${index}].quantity`),
      source_kind: sourceKind,
      source_id: POLICY_SOURCE_ID,
      reading_digest: digest(reading.reading_digest, `${sourceKind}[${index}].reading_digest`),
      observed_at: decidedAt,
    }
  })
  for (let index = 1; index < readings.length; index += 1) {
    if (compareUtf8(readings[index - 1].meter, readings[index].meter) >= 0) {
      throw new Error(`${sourceKind} meter 必须按 UTF-8 严格递增且无重复`)
    }
  }
  return readings
}

function validatePolicyResult(
  decision: RetainedVerificationDecision,
  candidateOutcome: CandidateOutcome,
  consumerDecision: ConsumerDecision,
  observedOutcome: ObservedOutcome,
  verified: ValidatedVerificationMeterReading[],
  compensable: ValidatedVerificationMeterReading[],
) {
  if (
    decision === 'accepted'
    && (consumerDecision !== 'accepted'
      || observedOutcome === 'indeterminate'
      || candidateOutcome !== observedOutcome)
  ) {
    throw new Error('accepted v192 不符合 conservative_min_v1 outcome gate')
  }
  if (verified.length !== compensable.length) {
    throw new Error('verified_usage 与 compensable_usage meter 集合不一致')
  }
  for (let index = 0; index < verified.length; index += 1) {
    if (verified[index].meter !== compensable[index].meter) {
      throw new Error('verified_usage 与 compensable_usage meter 集合不一致')
    }
    if (compensable[index].quantity > verified[index].quantity) {
      throw new Error('compensable_usage 不能超过 verified_usage')
    }
    if (decision !== 'accepted' && (verified[index].quantity !== 0 || compensable[index].quantity !== 0)) {
      throw new Error('rejected/disputed v192 的 policy usage 必须为零')
    }
  }
}

// Rust `str::trim` follows Unicode White_Space and deliberately does not trim U+FEFF.
const RUST_TRIM_EDGE =
  /(?:^[\u0009-\u000d\u0020\u0085\u00a0\u1680\u2000-\u200a\u2028\u2029\u202f\u205f\u3000])|(?:[\u0009-\u000d\u0020\u0085\u00a0\u1680\u2000-\u200a\u2028\u2029\u202f\u205f\u3000]$)/u
const CONTROL_CHARACTER = /[\u0000-\u001f\u007f-\u009f]/u

function id(value: unknown, label: string) {
  const result = safeText(value, label, 200)
  if (CONTROL_CHARACTER.test(result)) throw new Error(`${label} 不能包含控制字符`)
  return result
}

function safeText(value: unknown, label: string, maxBytes: number) {
  const result = expectString(value, label)
  if (
    !result
    || RUST_TRIM_EDGE.test(result)
    || new TextEncoder().encode(result).byteLength > maxBytes
  ) {
    throw new Error(`${label} 必须是有界 nonempty trim-stable string`)
  }
  return result
}

function digest(value: unknown, label: string) {
  const result = expectString(value, label)
  expectSha256(result, label)
  return result
}

function positiveSafeInteger(value: unknown, label: string) {
  if (typeof value !== 'number' || !Number.isSafeInteger(value) || value < 1) {
    throw new Error(`${label} 必须是正 JSON safe integer`)
  }
  return value
}

function nonNegativeSafeInteger(value: unknown, label: string) {
  if (typeof value !== 'number' || !Number.isSafeInteger(value) || value < 0) {
    throw new Error(`${label} 必须是非负 JSON safe integer`)
  }
  return value
}

function enumValue<const Values extends readonly string[]>(
  value: unknown,
  allowed: Values,
  label: string,
): Values[number] {
  const result = expectString(value, label)
  if (!allowed.some((candidate) => candidate === result)) {
    throw new Error(`${label} 不符合冻结枚举`)
  }
  return result as Values[number]
}

function expectSame(actual: string | number, expected: string | number, label: string) {
  if (actual !== expected) throw new Error(`${label} 跨响应等式不成立`)
}

function asciiLower(value: string) {
  return value.replace(/[A-Z]/g, (character) => character.toLowerCase())
}

function compareUtf8(left: string, right: string) {
  const leftBytes = new TextEncoder().encode(left)
  const rightBytes = new TextEncoder().encode(right)
  const sharedLength = Math.min(leftBytes.length, rightBytes.length)
  for (let index = 0; index < sharedLength; index += 1) {
    if (leftBytes[index] !== rightBytes[index]) return leftBytes[index] - rightBytes[index]
  }
  return leftBytes.length - rightBytes.length
}
