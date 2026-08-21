const READ_SCHEMA = 'compute_federation.core_historical_causal_reference.read.v1'
const CARRIER_SCHEMA = 'compute_federation.core_historical_causal_reference.v1'
const CARRIER_CANONICALIZATION = 'rfc8785_jcs'
const CARRIER_DIGEST_ALGORITHM = 'sha256'
const CARRIER_DIGEST_DOMAIN = 'ELON-COMPUTE-CORE-HISTORICAL-LINEAGE-V1'
const CARRIER_MAX_JSON_BYTES = 262_144

export type FederationHistoricalLineageKind = 'execution_source_v1' | 'settlement_source_v1'
export type FederationHistoricalLineageScope = 'participant' | 'admin'

export interface FederationHistoricalLineageReadResponse {
  schema: typeof READ_SCHEMA
  lineage_kind: FederationHistoricalLineageKind
  lineage_digest: string
  canonical_carrier_json: string
  read_effect: 'none'
}

interface ProviderVersionRef {
  provider_id: string
  policy_revision: number
  provider_digest: string
}

interface CapacityPoolVersionRef {
  pool_id: string
  capacity_epoch: number
  pool_revision: number
  pool_digest: string
}

interface OfferVersionRef {
  provider_id: string
  offer_id: string
  offer_version: number
  offer_digest: string
}

interface PriceSnapshotRef {
  price_snapshot_id: string
  price_snapshot_digest: string
}

interface JobVersionRef {
  job_id: string
  job_revision: number
  job_digest: string
}

interface ReservationVersionRef {
  reservation_id: string
  reservation_revision: number
  reservation_digest: string
}

interface CapacityClaimVersionRef {
  claim_id: string
  claim_revision: number
  claim_digest: string
}

interface AttemptLeaseSourceRef {
  lease_id: string
  lease_revision: number
  lease_digest: string
  fencing_generation: number
}

interface ExecutionReceiptRef {
  execution_receipt_id: string
  execution_receipt_digest: string
}

interface FinalizationRef {
  finalization_id: string
  finalization_event_digest: string
}

interface AttemptSettlementRef {
  settlement_receipt_id: string
  settlement_receipt_digest: string
  settlement_event_digest: string
}

interface ExecutionSourceLineageV1 {
  execution_receipt: ExecutionReceiptRef
  provider: ProviderVersionRef
  capacity_pool: CapacityPoolVersionRef
  offer: OfferVersionRef
  price_snapshot: PriceSnapshotRef
  job: JobVersionRef
  reservation: ReservationVersionRef
  capacity_claim: CapacityClaimVersionRef
  attempt_lease_source: AttemptLeaseSourceRef
}

interface SettlementSourceLineageV1 {
  attempt_settlement: AttemptSettlementRef
  execution_receipt: ExecutionReceiptRef
  execution_lineage_digest: string
  finalization: FinalizationRef
  price_snapshot: PriceSnapshotRef
  provider: ProviderVersionRef
  source_job: JobVersionRef
  terminal_job: JobVersionRef
  terminal_reservation: ReservationVersionRef
}

export interface ExecutionSourceCarrierV1 {
  schema: typeof CARRIER_SCHEMA
  lineage_kind: 'execution_source_v1'
  lineage_digest: string
  canonicalization: typeof CARRIER_CANONICALIZATION
  digest_algorithm: typeof CARRIER_DIGEST_ALGORITHM
  lineage: ExecutionSourceLineageV1
}

export interface SettlementSourceCarrierV1 {
  schema: typeof CARRIER_SCHEMA
  lineage_kind: 'settlement_source_v1'
  lineage_digest: string
  canonicalization: typeof CARRIER_CANONICALIZATION
  digest_algorithm: typeof CARRIER_DIGEST_ALGORITHM
  lineage: SettlementSourceLineageV1
}

export type FederationHistoricalLineageCarrierV1 =
  | ExecutionSourceCarrierV1
  | SettlementSourceCarrierV1

export interface ValidatedFederationHistoricalLineageRead {
  response: FederationHistoricalLineageReadResponse
  carrier: FederationHistoricalLineageCarrierV1
}

const READ_RESPONSE_KEYS = [
  'schema',
  'lineage_kind',
  'lineage_digest',
  'canonical_carrier_json',
  'read_effect',
] as const

const CARRIER_KEYS = [
  'schema',
  'lineage_kind',
  'lineage_digest',
  'canonicalization',
  'digest_algorithm',
  'lineage',
] as const

const EXECUTION_LINEAGE_KEYS = [
  'execution_receipt',
  'provider',
  'capacity_pool',
  'offer',
  'price_snapshot',
  'job',
  'reservation',
  'capacity_claim',
  'attempt_lease_source',
] as const

const SETTLEMENT_LINEAGE_KEYS = [
  'attempt_settlement',
  'execution_receipt',
  'execution_lineage_digest',
  'finalization',
  'price_snapshot',
  'provider',
  'source_job',
  'terminal_job',
  'terminal_reservation',
] as const

export async function validateFederationHistoricalLineageReadResponse(
  value: unknown,
  expectedKind: FederationHistoricalLineageKind,
): Promise<ValidatedFederationHistoricalLineageRead> {
  const response = exactObject(value, READ_RESPONSE_KEYS, '历史因果读取响应')
  expectLiteral(response.schema, READ_SCHEMA, '响应 schema')
  expectLiteral(response.lineage_kind, expectedKind, '响应 lineage_kind')
  const responseLineageDigest = expectString(response.lineage_digest, '响应 lineage_digest')
  expectSha256(responseLineageDigest, '响应 lineage_digest')
  expectLiteral(response.read_effect, 'none', '响应 read_effect')
  const canonicalCarrierJson = expectString(
    response.canonical_carrier_json,
    '响应 canonical_carrier_json',
  )
  const canonicalCarrierBytes = new TextEncoder().encode(canonicalCarrierJson)
  if (canonicalCarrierBytes.byteLength > CARRIER_MAX_JSON_BYTES) {
    throw new Error('历史因果 Carrier 超过 262144 字节上限')
  }

  const parsed = parseCarrier(canonicalCarrierJson)
  const carrier = validateCarrierShape(parsed, expectedKind)
  expectLiteral(carrier.lineage_kind, response.lineage_kind as string, '内外 lineage_kind')
  expectLiteral(carrier.lineage_digest, response.lineage_digest as string, '内外 lineage_digest')

  const canonicalBytes = new TextEncoder().encode(canonicalizeJcs(carrier))
  if (canonicalBytes.byteLength !== canonicalCarrierBytes.byteLength
    || canonicalBytes.some((byte, index) => byte !== canonicalCarrierBytes[index])) {
    throw new Error('历史因果 Carrier 不是逐字 RFC 8785 canonical JSON')
  }
  const digestMaterial = canonicalizeJcs({ ...carrier, lineage_digest: '' })
  const recomputedDigest = await sha256Hex(`${CARRIER_DIGEST_DOMAIN}\u0000${digestMaterial}`)
  expectLiteral(recomputedDigest, carrier.lineage_digest, 'Carrier domain digest')

  return {
    response: {
      schema: READ_SCHEMA,
      lineage_kind: expectedKind,
      lineage_digest: responseLineageDigest,
      canonical_carrier_json: canonicalCarrierJson,
      read_effect: 'none',
    },
    carrier,
  }
}

export function validateFederationHistoricalLineagePair(
  execution: ValidatedFederationHistoricalLineageRead,
  settlement: ValidatedFederationHistoricalLineageRead,
) {
  if (execution.response.lineage_kind !== 'execution_source_v1'
    || execution.carrier.lineage_kind !== 'execution_source_v1') {
    throw new Error('Execution 响应没有返回 execution_source_v1')
  }
  if (settlement.response.lineage_kind !== 'settlement_source_v1'
    || settlement.carrier.lineage_kind !== 'settlement_source_v1') {
    throw new Error('Settlement 响应没有返回 settlement_source_v1')
  }
  expectLiteral(
    settlement.carrier.lineage.execution_lineage_digest,
    execution.response.lineage_digest,
    'Settlement→Execution lineage digest',
  )
  expectLiteral(
    settlement.carrier.lineage.execution_receipt.execution_receipt_id,
    execution.carrier.lineage.execution_receipt.execution_receipt_id,
    'Settlement→Execution receipt ID',
  )
  expectLiteral(
    settlement.carrier.lineage.execution_receipt.execution_receipt_digest,
    execution.carrier.lineage.execution_receipt.execution_receipt_digest,
    'Settlement→Execution receipt digest',
  )
}

function parseCarrier(canonicalCarrierJson: string): unknown {
  try {
    return JSON.parse(canonicalCarrierJson) as unknown
  } catch {
    throw new Error('历史因果 Carrier 不是合法 JSON')
  }
}

function validateCarrierShape(
  value: unknown,
  expectedKind: FederationHistoricalLineageKind,
): FederationHistoricalLineageCarrierV1 {
  const carrier = exactObject(value, CARRIER_KEYS, '历史因果 Carrier')
  expectLiteral(carrier.schema, CARRIER_SCHEMA, 'Carrier schema')
  expectLiteral(carrier.lineage_kind, expectedKind, 'Carrier lineage_kind')
  expectSha256(carrier.lineage_digest, 'Carrier lineage_digest')
  expectLiteral(carrier.canonicalization, CARRIER_CANONICALIZATION, 'Carrier canonicalization')
  expectLiteral(carrier.digest_algorithm, CARRIER_DIGEST_ALGORITHM, 'Carrier digest_algorithm')
  if (expectedKind === 'execution_source_v1') {
    return {
      schema: CARRIER_SCHEMA,
      lineage_kind: expectedKind,
      lineage_digest: carrier.lineage_digest as string,
      canonicalization: CARRIER_CANONICALIZATION,
      digest_algorithm: CARRIER_DIGEST_ALGORITHM,
      lineage: validateExecutionLineage(carrier.lineage),
    }
  }
  return {
    schema: CARRIER_SCHEMA,
    lineage_kind: expectedKind,
    lineage_digest: carrier.lineage_digest as string,
    canonicalization: CARRIER_CANONICALIZATION,
    digest_algorithm: CARRIER_DIGEST_ALGORITHM,
    lineage: validateSettlementLineage(carrier.lineage),
  }
}

function validateExecutionLineage(value: unknown): ExecutionSourceLineageV1 {
  const lineage = exactObject(value, EXECUTION_LINEAGE_KEYS, 'execution_source_v1 lineage')
  return {
    execution_receipt: executionReceiptRef(lineage.execution_receipt),
    provider: providerRef(lineage.provider),
    capacity_pool: capacityPoolRef(lineage.capacity_pool),
    offer: offerRef(lineage.offer),
    price_snapshot: priceSnapshotRef(lineage.price_snapshot),
    job: jobRef(lineage.job),
    reservation: reservationRef(lineage.reservation),
    capacity_claim: capacityClaimRef(lineage.capacity_claim),
    attempt_lease_source: attemptLeaseRef(lineage.attempt_lease_source),
  }
}

function validateSettlementLineage(value: unknown): SettlementSourceLineageV1 {
  const lineage = exactObject(value, SETTLEMENT_LINEAGE_KEYS, 'settlement_source_v1 lineage')
  expectSha256(lineage.execution_lineage_digest, 'execution_lineage_digest')
  return {
    attempt_settlement: attemptSettlementRef(lineage.attempt_settlement),
    execution_receipt: executionReceiptRef(lineage.execution_receipt),
    execution_lineage_digest: lineage.execution_lineage_digest as string,
    finalization: finalizationRef(lineage.finalization),
    price_snapshot: priceSnapshotRef(lineage.price_snapshot),
    provider: providerRef(lineage.provider),
    source_job: jobRef(lineage.source_job),
    terminal_job: jobRef(lineage.terminal_job),
    terminal_reservation: reservationRef(lineage.terminal_reservation),
  }
}

function providerRef(value: unknown): ProviderVersionRef {
  const ref = exactObject(value, ['provider_id', 'policy_revision', 'provider_digest'], 'Provider ref')
  return {
    provider_id: expectString(ref.provider_id, 'provider_id'),
    policy_revision: positiveSafeInteger(ref.policy_revision, 'policy_revision'),
    provider_digest: expectString(ref.provider_digest, 'provider_digest'),
  }
}

function capacityPoolRef(value: unknown): CapacityPoolVersionRef {
  const ref = exactObject(value, ['pool_id', 'capacity_epoch', 'pool_revision', 'pool_digest'], 'Pool ref')
  return {
    pool_id: expectString(ref.pool_id, 'pool_id'),
    capacity_epoch: positiveSafeInteger(ref.capacity_epoch, 'capacity_epoch'),
    pool_revision: positiveSafeInteger(ref.pool_revision, 'pool_revision'),
    pool_digest: expectString(ref.pool_digest, 'pool_digest'),
  }
}

function offerRef(value: unknown): OfferVersionRef {
  const ref = exactObject(value, ['provider_id', 'offer_id', 'offer_version', 'offer_digest'], 'Offer ref')
  return {
    provider_id: expectString(ref.provider_id, 'offer provider_id'),
    offer_id: expectString(ref.offer_id, 'offer_id'),
    offer_version: positiveSafeInteger(ref.offer_version, 'offer_version'),
    offer_digest: expectString(ref.offer_digest, 'offer_digest'),
  }
}

function priceSnapshotRef(value: unknown): PriceSnapshotRef {
  const ref = exactObject(value, ['price_snapshot_id', 'price_snapshot_digest'], 'Snapshot ref')
  return {
    price_snapshot_id: expectString(ref.price_snapshot_id, 'price_snapshot_id'),
    price_snapshot_digest: expectString(ref.price_snapshot_digest, 'price_snapshot_digest'),
  }
}

function jobRef(value: unknown): JobVersionRef {
  const ref = exactObject(value, ['job_id', 'job_revision', 'job_digest'], 'Job ref')
  return {
    job_id: expectString(ref.job_id, 'job_id'),
    job_revision: positiveSafeInteger(ref.job_revision, 'job_revision'),
    job_digest: expectString(ref.job_digest, 'job_digest'),
  }
}

function reservationRef(value: unknown): ReservationVersionRef {
  const ref = exactObject(value, ['reservation_id', 'reservation_revision', 'reservation_digest'], 'Reservation ref')
  return {
    reservation_id: expectString(ref.reservation_id, 'reservation_id'),
    reservation_revision: positiveSafeInteger(ref.reservation_revision, 'reservation_revision'),
    reservation_digest: expectString(ref.reservation_digest, 'reservation_digest'),
  }
}

function capacityClaimRef(value: unknown): CapacityClaimVersionRef {
  const ref = exactObject(value, ['claim_id', 'claim_revision', 'claim_digest'], 'Claim ref')
  return {
    claim_id: expectString(ref.claim_id, 'claim_id'),
    claim_revision: positiveSafeInteger(ref.claim_revision, 'claim_revision'),
    claim_digest: expectString(ref.claim_digest, 'claim_digest'),
  }
}

function attemptLeaseRef(value: unknown): AttemptLeaseSourceRef {
  const ref = exactObject(value, ['lease_id', 'lease_revision', 'lease_digest', 'fencing_generation'], 'Lease ref')
  return {
    lease_id: expectString(ref.lease_id, 'lease_id'),
    lease_revision: positiveSafeInteger(ref.lease_revision, 'lease_revision'),
    lease_digest: expectString(ref.lease_digest, 'lease_digest'),
    fencing_generation: positiveSafeInteger(ref.fencing_generation, 'fencing_generation'),
  }
}

function executionReceiptRef(value: unknown): ExecutionReceiptRef {
  const ref = exactObject(value, ['execution_receipt_id', 'execution_receipt_digest'], 'Execution Receipt ref')
  return {
    execution_receipt_id: expectString(ref.execution_receipt_id, 'execution_receipt_id'),
    execution_receipt_digest: expectString(ref.execution_receipt_digest, 'execution_receipt_digest'),
  }
}

function finalizationRef(value: unknown): FinalizationRef {
  const ref = exactObject(value, ['finalization_id', 'finalization_event_digest'], 'Finalization ref')
  return {
    finalization_id: expectString(ref.finalization_id, 'finalization_id'),
    finalization_event_digest: expectString(ref.finalization_event_digest, 'finalization_event_digest'),
  }
}

function attemptSettlementRef(value: unknown): AttemptSettlementRef {
  const ref = exactObject(
    value,
    ['settlement_receipt_id', 'settlement_receipt_digest', 'settlement_event_digest'],
    'Settlement ref',
  )
  return {
    settlement_receipt_id: expectString(ref.settlement_receipt_id, 'settlement_receipt_id'),
    settlement_receipt_digest: expectString(ref.settlement_receipt_digest, 'settlement_receipt_digest'),
    settlement_event_digest: expectString(ref.settlement_event_digest, 'settlement_event_digest'),
  }
}

function exactObject<const Keys extends readonly string[]>(
  value: unknown,
  keys: Keys,
  label: string,
): Record<Keys[number], unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new Error(`${label} 必须是 object`)
  }
  const actualKeys = Object.keys(value)
  const expectedKeys = [...keys].sort()
  if (actualKeys.length !== expectedKeys.length || actualKeys.sort().some((key, index) => key !== expectedKeys[index])) {
    throw new Error(`${label} key set 不符合冻结 ABI`)
  }
  return value as Record<Keys[number], unknown>
}

function expectString(value: unknown, label: string) {
  if (typeof value !== 'string' || hasUnpairedSurrogate(value)) {
    throw new Error(`${label} 必须是 I-JSON string`)
  }
  return value
}

function expectSha256(value: unknown, label: string) {
  const digest = expectString(value, label)
  if (!/^[0-9a-f]{64}$/.test(digest)) throw new Error(`${label} 必须是 64 位 lowercase SHA-256`)
}

function expectLiteral(value: unknown, expected: string, label: string) {
  if (value !== expected) throw new Error(`${label} 不符合冻结 ABI`)
}

function positiveSafeInteger(value: unknown, label: string) {
  if (typeof value !== 'number' || !Number.isSafeInteger(value) || value < 1) {
    throw new Error(`${label} 必须是正 JSON safe integer`)
  }
  return value
}

function hasUnpairedSurrogate(value: string) {
  for (let index = 0; index < value.length; index += 1) {
    const code = value.charCodeAt(index)
    if (code >= 0xd800 && code <= 0xdbff) {
      const next = value.charCodeAt(index + 1)
      if (next < 0xdc00 || next > 0xdfff) return true
      index += 1
    } else if (code >= 0xdc00 && code <= 0xdfff) {
      return true
    }
  }
  return false
}

function canonicalizeJcs(value: unknown): string {
  if (value === null) return 'null'
  if (typeof value === 'string') {
    if (hasUnpairedSurrogate(value)) throw new Error('JCS string 包含未配对 surrogate')
    return JSON.stringify(value)
  }
  if (typeof value === 'number') {
    if (!Number.isSafeInteger(value)) throw new Error('Carrier 只允许 JSON safe integer')
    return JSON.stringify(value)
  }
  if (typeof value === 'boolean') return value ? 'true' : 'false'
  if (Array.isArray(value)) return `[${value.map(canonicalizeJcs).join(',')}]`
  if (typeof value === 'object') {
    const record = value as Record<string, unknown>
    return `{${Object.keys(record).sort().map((key) => (
      `${canonicalizeJcs(key)}:${canonicalizeJcs(record[key])}`
    )).join(',')}}`
  }
  throw new Error('Carrier 含有不可 canonicalize 的 JSON 值')
}

async function sha256Hex(value: string) {
  if (!globalThis.crypto?.subtle) throw new Error('当前浏览器不支持 Web Crypto SHA-256')
  const digest = await globalThis.crypto.subtle.digest('SHA-256', new TextEncoder().encode(value))
  return Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, '0')).join('')
}
