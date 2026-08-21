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
  type ValidatedFederationHistoricalLineageRead,
} from "./federationHistoricalLineageContracts";

export type ExecutionVerificationLineageKind =
  "execution_verification_source_v1";

interface ExecutionReceiptRef {
  execution_receipt_id: string;
  execution_receipt_digest: string;
}

interface ProviderDeclaredUsageRef {
  usage_snapshot_id: string;
  usage_sequence_no: number;
  cumulative_usage_digest: string;
  usage_event_digest: string;
}

interface TerminalCandidateRef {
  terminal_candidate_id: string;
  terminal_candidate_event_digest: string;
}

interface ConsumerReviewRef {
  consumer_review_id: string;
  consumer_review_event_digest: string;
}

interface PlatformObservationRef {
  platform_observation_id: string;
  platform_observation_event_digest: string;
  cumulative_observed_usage_digest: string;
}

interface VerificationDecisionRef {
  verification_decision_id: string;
  verification_event_digest: string;
  verified_usage_digest: string;
  compensable_usage_digest: string;
}

interface ExecutionVerificationSourceLineageV1 {
  execution_receipt: ExecutionReceiptRef;
  execution_lineage_digest: string;
  provider_declared_usage: ProviderDeclaredUsageRef;
  terminal_candidate: TerminalCandidateRef;
  consumer_review: ConsumerReviewRef;
  platform_observation: PlatformObservationRef;
  verification_decision: VerificationDecisionRef;
}

export interface ExecutionVerificationSourceCarrierV1 {
  schema: typeof CARRIER_SCHEMA;
  lineage_kind: ExecutionVerificationLineageKind;
  lineage_digest: string;
  canonicalization: typeof CARRIER_CANONICALIZATION;
  digest_algorithm: typeof CARRIER_DIGEST_ALGORITHM;
  lineage: ExecutionVerificationSourceLineageV1;
}

export interface ExecutionVerificationLineageReadResponse {
  schema: typeof READ_SCHEMA;
  lineage_kind: ExecutionVerificationLineageKind;
  lineage_digest: string;
  canonical_carrier_json: string;
  read_effect: "none";
}

export interface ValidatedExecutionVerificationLineageRead {
  response: ExecutionVerificationLineageReadResponse;
  carrier: ExecutionVerificationSourceCarrierV1;
}

const READ_KEYS = [
  "schema",
  "lineage_kind",
  "lineage_digest",
  "canonical_carrier_json",
  "read_effect",
] as const;

const CARRIER_KEYS = [
  "schema",
  "lineage_kind",
  "lineage_digest",
  "canonicalization",
  "digest_algorithm",
  "lineage",
] as const;

const LINEAGE_KEYS = [
  "execution_receipt",
  "execution_lineage_digest",
  "provider_declared_usage",
  "terminal_candidate",
  "consumer_review",
  "platform_observation",
  "verification_decision",
] as const;

// Rust `str::trim` follows Unicode White_Space and deliberately does not trim U+FEFF.
const RUST_TRIM_EDGE =
  /(?:^[\u0009-\u000d\u0020\u0085\u00a0\u1680\u2000-\u200a\u2028\u2029\u202f\u205f\u3000])|(?:[\u0009-\u000d\u0020\u0085\u00a0\u1680\u2000-\u200a\u2028\u2029\u202f\u205f\u3000]$)/u;

export async function validateExecutionVerificationLineageReadResponse(
  value: unknown,
): Promise<ValidatedExecutionVerificationLineageRead> {
  const response = exactObject(value, READ_KEYS, "执行验证因果读取响应");
  expectLiteral(response.schema, READ_SCHEMA, "响应 schema");
  expectLiteral(
    response.lineage_kind,
    "execution_verification_source_v1",
    "响应 lineage_kind",
  );
  const responseDigest = digest(response.lineage_digest, "响应 lineage_digest");
  expectLiteral(response.read_effect, "none", "响应 read_effect");
  const canonicalJson = expectString(
    response.canonical_carrier_json,
    "响应 canonical_carrier_json",
  );
  const inputBytes = new TextEncoder().encode(canonicalJson);
  if (inputBytes.byteLength > CARRIER_MAX_JSON_BYTES) {
    throw new Error("执行验证因果 Carrier 超过 262144 字节上限");
  }

  const carrier = validateCarrier(parseJson(canonicalJson));
  expectLiteral(
    carrier.lineage_kind,
    response.lineage_kind as string,
    "内外 lineage_kind",
  );
  expectLiteral(carrier.lineage_digest, responseDigest, "内外 lineage_digest");

  const canonicalBytes = new TextEncoder().encode(canonicalizeJcs(carrier));
  if (
    canonicalBytes.byteLength !== inputBytes.byteLength ||
    canonicalBytes.some((byte, index) => byte !== inputBytes[index])
  ) {
    throw new Error("执行验证因果 Carrier 不是逐字 RFC 8785 canonical JSON");
  }
  const material = canonicalizeJcs({ ...carrier, lineage_digest: "" });
  const recomputed = await sha256Hex(
    `${CARRIER_DIGEST_DOMAIN}\u0000${material}`,
  );
  expectLiteral(
    recomputed,
    carrier.lineage_digest,
    "Verification Carrier domain digest",
  );

  return {
    response: {
      schema: READ_SCHEMA,
      lineage_kind: "execution_verification_source_v1",
      lineage_digest: responseDigest,
      canonical_carrier_json: canonicalJson,
      read_effect: "none",
    },
    carrier,
  };
}

export function validateExecutionVerificationLineagePair(
  execution: ValidatedFederationHistoricalLineageRead,
  verification: ValidatedExecutionVerificationLineageRead,
) {
  if (
    execution.response.lineage_kind !== "execution_source_v1" ||
    execution.carrier.lineage_kind !== "execution_source_v1"
  ) {
    throw new Error("Execution 响应没有返回 execution_source_v1");
  }
  expectLiteral(
    verification.carrier.lineage.execution_lineage_digest,
    execution.response.lineage_digest,
    "Verification→Execution lineage digest",
  );
  expectLiteral(
    verification.carrier.lineage.execution_receipt.execution_receipt_id,
    execution.carrier.lineage.execution_receipt.execution_receipt_id,
    "Verification→Execution receipt ID",
  );
  expectLiteral(
    verification.carrier.lineage.execution_receipt.execution_receipt_digest,
    execution.carrier.lineage.execution_receipt.execution_receipt_digest,
    "Verification→Execution receipt digest",
  );
}

function validateCarrier(value: unknown): ExecutionVerificationSourceCarrierV1 {
  const carrier = exactObject(value, CARRIER_KEYS, "执行验证因果 Carrier");
  expectLiteral(carrier.schema, CARRIER_SCHEMA, "Carrier schema");
  expectLiteral(
    carrier.lineage_kind,
    "execution_verification_source_v1",
    "Carrier lineage_kind",
  );
  const lineageDigest = digest(
    carrier.lineage_digest,
    "Carrier lineage_digest",
  );
  expectLiteral(
    carrier.canonicalization,
    CARRIER_CANONICALIZATION,
    "Carrier canonicalization",
  );
  expectLiteral(
    carrier.digest_algorithm,
    CARRIER_DIGEST_ALGORITHM,
    "Carrier digest_algorithm",
  );
  return {
    schema: CARRIER_SCHEMA,
    lineage_kind: "execution_verification_source_v1",
    lineage_digest: lineageDigest,
    canonicalization: CARRIER_CANONICALIZATION,
    digest_algorithm: CARRIER_DIGEST_ALGORITHM,
    lineage: validateLineage(carrier.lineage),
  };
}

function validateLineage(value: unknown): ExecutionVerificationSourceLineageV1 {
  const lineage = exactObject(
    value,
    LINEAGE_KEYS,
    "execution_verification_source_v1 lineage",
  );
  return {
    execution_receipt: executionReceiptRef(lineage.execution_receipt),
    execution_lineage_digest: digest(
      lineage.execution_lineage_digest,
      "execution_lineage_digest",
    ),
    provider_declared_usage: providerDeclaredUsageRef(
      lineage.provider_declared_usage,
    ),
    terminal_candidate: terminalCandidateRef(lineage.terminal_candidate),
    consumer_review: consumerReviewRef(lineage.consumer_review),
    platform_observation: platformObservationRef(lineage.platform_observation),
    verification_decision: verificationDecisionRef(
      lineage.verification_decision,
    ),
  };
}

function executionReceiptRef(value: unknown): ExecutionReceiptRef {
  const ref = exactObject(
    value,
    ["execution_receipt_id", "execution_receipt_digest"],
    "Execution Receipt ref",
  );
  return {
    execution_receipt_id: id(ref.execution_receipt_id, "execution_receipt_id"),
    execution_receipt_digest: digest(
      ref.execution_receipt_digest,
      "execution_receipt_digest",
    ),
  };
}

function providerDeclaredUsageRef(value: unknown): ProviderDeclaredUsageRef {
  const ref = exactObject(
    value,
    [
      "usage_snapshot_id",
      "usage_sequence_no",
      "cumulative_usage_digest",
      "usage_event_digest",
    ],
    "Provider declared usage ref",
  );
  return {
    usage_snapshot_id: id(ref.usage_snapshot_id, "usage_snapshot_id"),
    usage_sequence_no: positiveSafeInteger(
      ref.usage_sequence_no,
      "usage_sequence_no",
    ),
    cumulative_usage_digest: digest(
      ref.cumulative_usage_digest,
      "cumulative_usage_digest",
    ),
    usage_event_digest: digest(ref.usage_event_digest, "usage_event_digest"),
  };
}

function terminalCandidateRef(value: unknown): TerminalCandidateRef {
  const ref = exactObject(
    value,
    ["terminal_candidate_id", "terminal_candidate_event_digest"],
    "Terminal candidate ref",
  );
  return {
    terminal_candidate_id: id(
      ref.terminal_candidate_id,
      "terminal_candidate_id",
    ),
    terminal_candidate_event_digest: digest(
      ref.terminal_candidate_event_digest,
      "terminal_candidate_event_digest",
    ),
  };
}

function consumerReviewRef(value: unknown): ConsumerReviewRef {
  const ref = exactObject(
    value,
    ["consumer_review_id", "consumer_review_event_digest"],
    "Consumer review ref",
  );
  return {
    consumer_review_id: id(ref.consumer_review_id, "consumer_review_id"),
    consumer_review_event_digest: digest(
      ref.consumer_review_event_digest,
      "consumer_review_event_digest",
    ),
  };
}

function platformObservationRef(value: unknown): PlatformObservationRef {
  const ref = exactObject(
    value,
    [
      "platform_observation_id",
      "platform_observation_event_digest",
      "cumulative_observed_usage_digest",
    ],
    "Platform observation ref",
  );
  return {
    platform_observation_id: id(
      ref.platform_observation_id,
      "platform_observation_id",
    ),
    platform_observation_event_digest: digest(
      ref.platform_observation_event_digest,
      "platform_observation_event_digest",
    ),
    cumulative_observed_usage_digest: digest(
      ref.cumulative_observed_usage_digest,
      "cumulative_observed_usage_digest",
    ),
  };
}

function verificationDecisionRef(value: unknown): VerificationDecisionRef {
  const ref = exactObject(
    value,
    [
      "verification_decision_id",
      "verification_event_digest",
      "verified_usage_digest",
      "compensable_usage_digest",
    ],
    "Verification decision ref",
  );
  return {
    verification_decision_id: id(
      ref.verification_decision_id,
      "verification_decision_id",
    ),
    verification_event_digest: digest(
      ref.verification_event_digest,
      "verification_event_digest",
    ),
    verified_usage_digest: digest(
      ref.verified_usage_digest,
      "verified_usage_digest",
    ),
    compensable_usage_digest: digest(
      ref.compensable_usage_digest,
      "compensable_usage_digest",
    ),
  };
}

function id(value: unknown, label: string) {
  const result = expectString(value, label);
  if (
    !result ||
    RUST_TRIM_EDGE.test(result) ||
    /[\u0000-\u001f\u007f-\u009f]/u.test(result)
  ) {
    throw new Error(`${label} 必须是 nonempty trim-stable string`);
  }
  return result;
}

function digest(value: unknown, label: string) {
  const result = expectString(value, label);
  expectSha256(result, label);
  return result;
}

function positiveSafeInteger(value: unknown, label: string) {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 1) {
    throw new Error(`${label} 必须是正 JSON safe integer`);
  }
  return value;
}

function parseJson(value: string): unknown {
  try {
    return JSON.parse(value) as unknown;
  } catch {
    throw new Error("执行验证因果 Carrier 不是合法 JSON");
  }
}
