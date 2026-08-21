import {
  CARRIER_CANONICALIZATION,
  CARRIER_DIGEST_ALGORITHM,
  CARRIER_DIGEST_DOMAIN,
  CARRIER_MAX_JSON_BYTES,
  CARRIER_SCHEMA,
  READ_SCHEMA,
  canonicalizeJcs,
  exactObject,
  expectLiteral,
  expectSha256,
  expectString,
  sha256Hex,
  type AttemptSettlementRef,
  type ValidatedFederationHistoricalLineageRead,
  validateFederationHistoricalLineagePair,
} from './federationHistoricalLineageContracts'

export type SettlementReleaseLineageKind = 'settlement_release_source_v1'

interface SettlementPostingRef {
  settlement_posting_id: string
  settlement_posting_digest: string
}

interface SettlementReleaseRef {
  settlement_release_id: string
  settlement_release_event_digest: string
}

interface SettlementReleasePostingRef {
  settlement_release_posting_id: string
  settlement_release_posting_digest: string
}

interface SettlementChallengeRef {
  settlement_challenge_id: string
  settlement_challenge_event_digest: string
}

interface SettlementChallengeResolutionRef {
  settlement_challenge_resolution_id: string
  settlement_challenge_resolution_event_digest: string
}

interface SettlementCorrectionRef {
  settlement_correction_id: string
  settlement_correction_event_digest: string
}

interface SettlementCorrectionPostingRef {
  settlement_correction_posting_id: string
  settlement_correction_posting_digest: string
}

type SettlementReleaseGateV1 =
  | {
      gate_kind: 'no_challenge'
      challenge_gate_digest: string
    }
  | {
      gate_kind: 'resolved_challenge'
      challenge_gate_digest: string
      resolution_action: 'rejected' | 'withdrawn'
      challenge: SettlementChallengeRef
      resolution: SettlementChallengeResolutionRef
    }
  | {
      gate_kind: 'accepted_corrected'
      challenge_gate_digest: string
      challenge: SettlementChallengeRef
      resolution: SettlementChallengeResolutionRef
      correction: SettlementCorrectionRef
      correction_posting: SettlementCorrectionPostingRef
    }

interface SettlementReleaseSourceLineageV1 {
  attempt_settlement: AttemptSettlementRef
  settlement_lineage_digest: string
  source_settlement_posting: SettlementPostingRef
  release_gate: SettlementReleaseGateV1
  settlement_release: SettlementReleaseRef
  release_posting: SettlementReleasePostingRef
}

export interface SettlementReleaseSourceCarrierV1 {
  schema: typeof CARRIER_SCHEMA
  lineage_kind: SettlementReleaseLineageKind
  lineage_digest: string
  canonicalization: typeof CARRIER_CANONICALIZATION
  digest_algorithm: typeof CARRIER_DIGEST_ALGORITHM
  lineage: SettlementReleaseSourceLineageV1
}

export interface SettlementReleaseLineageReadResponse {
  schema: typeof READ_SCHEMA
  lineage_kind: SettlementReleaseLineageKind
  lineage_digest: string
  canonical_carrier_json: string
  read_effect: 'none'
}

export interface ValidatedSettlementReleaseLineageRead {
  response: SettlementReleaseLineageReadResponse
  carrier: SettlementReleaseSourceCarrierV1
}

const READ_KEYS = [
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

const LINEAGE_KEYS = [
  'attempt_settlement',
  'settlement_lineage_digest',
  'source_settlement_posting',
  'release_gate',
  'settlement_release',
  'release_posting',
] as const

// Rust `str::trim` follows Unicode White_Space and deliberately does not trim U+FEFF.
const RUST_TRIM_EDGE =
  /(?:^[\u0009-\u000d\u0020\u0085\u00a0\u1680\u2000-\u200a\u2028\u2029\u202f\u205f\u3000])|(?:[\u0009-\u000d\u0020\u0085\u00a0\u1680\u2000-\u200a\u2028\u2029\u202f\u205f\u3000]$)/u

export async function validateSettlementReleaseLineageReadResponse(
  value: unknown,
): Promise<ValidatedSettlementReleaseLineageRead> {
  const response = exactObject(value, READ_KEYS, '结算释放因果读取响应')
  expectLiteral(response.schema, READ_SCHEMA, '响应 schema')
  expectLiteral(response.lineage_kind, 'settlement_release_source_v1', '响应 lineage_kind')
  const responseDigest = digest(response.lineage_digest, '响应 lineage_digest')
  expectLiteral(response.read_effect, 'none', '响应 read_effect')
  const canonicalJson = expectString(response.canonical_carrier_json, '响应 canonical_carrier_json')
  const inputBytes = new TextEncoder().encode(canonicalJson)
  if (inputBytes.byteLength > CARRIER_MAX_JSON_BYTES) {
    throw new Error('结算释放因果 Carrier 超过 262144 字节上限')
  }

  const carrier = validateCarrier(parseJson(canonicalJson))
  expectLiteral(carrier.lineage_kind, response.lineage_kind as string, '内外 lineage_kind')
  expectLiteral(carrier.lineage_digest, responseDigest, '内外 lineage_digest')

  const canonicalBytes = new TextEncoder().encode(canonicalizeJcs(carrier))
  if (canonicalBytes.byteLength !== inputBytes.byteLength
    || canonicalBytes.some((byte, index) => byte !== inputBytes[index])) {
    throw new Error('结算释放因果 Carrier 不是逐字 RFC 8785 canonical JSON')
  }
  const material = canonicalizeJcs({ ...carrier, lineage_digest: '' })
  const recomputed = await sha256Hex(`${CARRIER_DIGEST_DOMAIN}\u0000${material}`)
  expectLiteral(recomputed, carrier.lineage_digest, 'Release Carrier domain digest')

  return {
    response: {
      schema: READ_SCHEMA,
      lineage_kind: 'settlement_release_source_v1',
      lineage_digest: responseDigest,
      canonical_carrier_json: canonicalJson,
      read_effect: 'none',
    },
    carrier,
  }
}

export function validateFederationHistoricalLineageTriple(
  execution: ValidatedFederationHistoricalLineageRead,
  settlement: ValidatedFederationHistoricalLineageRead,
  release: ValidatedSettlementReleaseLineageRead,
) {
  validateFederationHistoricalLineagePair(execution, settlement)
  if (settlement.carrier.lineage_kind !== 'settlement_source_v1') {
    throw new Error('Settlement 响应没有返回 settlement_source_v1')
  }
  expectLiteral(
    release.carrier.lineage.settlement_lineage_digest,
    settlement.response.lineage_digest,
    'Release→Settlement lineage digest',
  )
  const expected = settlement.carrier.lineage.attempt_settlement
  const actual = release.carrier.lineage.attempt_settlement
  expectLiteral(actual.settlement_receipt_id, expected.settlement_receipt_id, 'Release→Settlement receipt ID')
  expectLiteral(
    actual.settlement_receipt_digest,
    expected.settlement_receipt_digest,
    'Release→Settlement receipt digest',
  )
  expectLiteral(
    actual.settlement_event_digest,
    expected.settlement_event_digest,
    'Release→Settlement event digest',
  )
}

function validateCarrier(value: unknown): SettlementReleaseSourceCarrierV1 {
  const carrier = exactObject(value, CARRIER_KEYS, '结算释放因果 Carrier')
  expectLiteral(carrier.schema, CARRIER_SCHEMA, 'Carrier schema')
  expectLiteral(carrier.lineage_kind, 'settlement_release_source_v1', 'Carrier lineage_kind')
  const lineageDigest = digest(carrier.lineage_digest, 'Carrier lineage_digest')
  expectLiteral(carrier.canonicalization, CARRIER_CANONICALIZATION, 'Carrier canonicalization')
  expectLiteral(carrier.digest_algorithm, CARRIER_DIGEST_ALGORITHM, 'Carrier digest_algorithm')
  return {
    schema: CARRIER_SCHEMA,
    lineage_kind: 'settlement_release_source_v1',
    lineage_digest: lineageDigest,
    canonicalization: CARRIER_CANONICALIZATION,
    digest_algorithm: CARRIER_DIGEST_ALGORITHM,
    lineage: validateLineage(carrier.lineage),
  }
}

function validateLineage(value: unknown): SettlementReleaseSourceLineageV1 {
  const lineage = exactObject(value, LINEAGE_KEYS, 'settlement_release_source_v1 lineage')
  return {
    attempt_settlement: attemptSettlementRef(lineage.attempt_settlement),
    settlement_lineage_digest: digest(lineage.settlement_lineage_digest, 'settlement_lineage_digest'),
    source_settlement_posting: settlementPostingRef(lineage.source_settlement_posting),
    release_gate: releaseGate(lineage.release_gate),
    settlement_release: settlementReleaseRef(lineage.settlement_release),
    release_posting: settlementReleasePostingRef(lineage.release_posting),
  }
}

function releaseGate(value: unknown): SettlementReleaseGateV1 {
  const tag = objectTag(value, 'gate_kind', 'Release gate')
  if (tag === 'no_challenge') {
    const gate = exactObject(value, ['gate_kind', 'challenge_gate_digest'], 'no_challenge gate')
    return { gate_kind: tag, challenge_gate_digest: digest(gate.challenge_gate_digest, 'challenge_gate_digest') }
  }
  if (tag === 'resolved_challenge') {
    const gate = exactObject(
      value,
      ['gate_kind', 'challenge_gate_digest', 'resolution_action', 'challenge', 'resolution'],
      'resolved_challenge gate',
    )
    if (gate.resolution_action !== 'rejected' && gate.resolution_action !== 'withdrawn') {
      throw new Error('resolved_challenge action 必须是 rejected 或 withdrawn')
    }
    return {
      gate_kind: tag,
      challenge_gate_digest: digest(gate.challenge_gate_digest, 'challenge_gate_digest'),
      resolution_action: gate.resolution_action,
      challenge: challengeRef(gate.challenge),
      resolution: resolutionRef(gate.resolution),
    }
  }
  if (tag === 'accepted_corrected') {
    const gate = exactObject(
      value,
      ['gate_kind', 'challenge_gate_digest', 'challenge', 'resolution', 'correction', 'correction_posting'],
      'accepted_corrected gate',
    )
    return {
      gate_kind: tag,
      challenge_gate_digest: digest(gate.challenge_gate_digest, 'challenge_gate_digest'),
      challenge: challengeRef(gate.challenge),
      resolution: resolutionRef(gate.resolution),
      correction: correctionRef(gate.correction),
      correction_posting: correctionPostingRef(gate.correction_posting),
    }
  }
  throw new Error('Release gate kind 不符合冻结 ABI')
}

function attemptSettlementRef(value: unknown): AttemptSettlementRef {
  const ref = exactObject(
    value,
    ['settlement_receipt_id', 'settlement_receipt_digest', 'settlement_event_digest'],
    'Attempt Settlement ref',
  )
  return {
    settlement_receipt_id: id(ref.settlement_receipt_id, 'settlement_receipt_id'),
    settlement_receipt_digest: digest(ref.settlement_receipt_digest, 'settlement_receipt_digest'),
    settlement_event_digest: digest(ref.settlement_event_digest, 'settlement_event_digest'),
  }
}

function settlementPostingRef(value: unknown): SettlementPostingRef {
  const ref = exactObject(value, ['settlement_posting_id', 'settlement_posting_digest'], 'Settlement Posting ref')
  return {
    settlement_posting_id: id(ref.settlement_posting_id, 'settlement_posting_id'),
    settlement_posting_digest: digest(ref.settlement_posting_digest, 'settlement_posting_digest'),
  }
}

function settlementReleaseRef(value: unknown): SettlementReleaseRef {
  const ref = exactObject(value, ['settlement_release_id', 'settlement_release_event_digest'], 'Release ref')
  return {
    settlement_release_id: id(ref.settlement_release_id, 'settlement_release_id'),
    settlement_release_event_digest: digest(ref.settlement_release_event_digest, 'settlement_release_event_digest'),
  }
}

function settlementReleasePostingRef(value: unknown): SettlementReleasePostingRef {
  const ref = exactObject(
    value,
    ['settlement_release_posting_id', 'settlement_release_posting_digest'],
    'Release Posting ref',
  )
  return {
    settlement_release_posting_id: id(ref.settlement_release_posting_id, 'settlement_release_posting_id'),
    settlement_release_posting_digest: digest(ref.settlement_release_posting_digest, 'settlement_release_posting_digest'),
  }
}

function challengeRef(value: unknown): SettlementChallengeRef {
  const ref = exactObject(
    value,
    ['settlement_challenge_id', 'settlement_challenge_event_digest'],
    'Challenge ref',
  )
  return {
    settlement_challenge_id: id(ref.settlement_challenge_id, 'settlement_challenge_id'),
    settlement_challenge_event_digest: digest(ref.settlement_challenge_event_digest, 'settlement_challenge_event_digest'),
  }
}

function resolutionRef(value: unknown): SettlementChallengeResolutionRef {
  const ref = exactObject(
    value,
    ['settlement_challenge_resolution_id', 'settlement_challenge_resolution_event_digest'],
    'Resolution ref',
  )
  return {
    settlement_challenge_resolution_id: id(
      ref.settlement_challenge_resolution_id,
      'settlement_challenge_resolution_id',
    ),
    settlement_challenge_resolution_event_digest: digest(
      ref.settlement_challenge_resolution_event_digest,
      'settlement_challenge_resolution_event_digest',
    ),
  }
}

function correctionRef(value: unknown): SettlementCorrectionRef {
  const ref = exactObject(
    value,
    ['settlement_correction_id', 'settlement_correction_event_digest'],
    'Correction ref',
  )
  return {
    settlement_correction_id: id(ref.settlement_correction_id, 'settlement_correction_id'),
    settlement_correction_event_digest: digest(ref.settlement_correction_event_digest, 'settlement_correction_event_digest'),
  }
}

function correctionPostingRef(value: unknown): SettlementCorrectionPostingRef {
  const ref = exactObject(
    value,
    ['settlement_correction_posting_id', 'settlement_correction_posting_digest'],
    'Correction Posting ref',
  )
  return {
    settlement_correction_posting_id: id(
      ref.settlement_correction_posting_id,
      'settlement_correction_posting_id',
    ),
    settlement_correction_posting_digest: digest(
      ref.settlement_correction_posting_digest,
      'settlement_correction_posting_digest',
    ),
  }
}

function objectTag(value: unknown, key: string, label: string) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error(`${label} 必须是 object`)
  return expectString((value as Record<string, unknown>)[key], `${label} ${key}`)
}

function id(value: unknown, label: string) {
  const result = expectString(value, label)
  if (!result || RUST_TRIM_EDGE.test(result) || /[\u0000-\u001f\u007f-\u009f]/u.test(result)) {
    throw new Error(`${label} 必须是 nonempty trim-stable string`)
  }
  return result
}

function digest(value: unknown, label: string) {
  const result = expectString(value, label)
  expectSha256(result, label)
  return result
}

function parseJson(value: string): unknown {
  try {
    return JSON.parse(value) as unknown
  } catch {
    throw new Error('结算释放因果 Carrier 不是合法 JSON')
  }
}
