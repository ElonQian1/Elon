import { createHash } from 'node:crypto'

export const SUI_ADAPTER_HANDOFF_SCHEMA = 'task_economy.sui_adapter_handoff.v1'
export const SUI_PREFLIGHT_REPORT_SCHEMA = 'task_economy.sui_preflight_report.v1'
export const SUI_PREFLIGHT_MAX_RESPONSE_BYTES = 256 * 1024

const PACKAGE_KINDS = new Set(['standard', 'correction'])
const TARGET_NETWORKS = new Set(['devnet', 'testnet', 'mainnet'])
const REPORT_OUTCOMES = new Set(['passed', 'rejected'])
const HANDOFF_KEYS = new Set([
  'schema',
  'package_kind',
  'project_id',
  'projection_package_id',
  'source_id',
  'target_network',
  'package_schema',
  'projection_digest',
  'source_digest',
  'envelope',
  'shadow_only',
  'atomic_bundle',
  'network_submission',
  'submission_attempts',
  'package_created_at',
  'constraints',
  'handoff_digest',
])
const CONSTRAINT_KEYS = new Set([
  'allowed_adapter_action',
  'signature_present',
  'transaction_broadcast',
  'finality_verified',
  'funds_moved',
])

export class SuiPreflightContractError extends Error {
  constructor(code, message, status = undefined) {
    super(message)
    this.name = 'SuiPreflightContractError'
    this.code = code
    this.status = status
  }
}

export function verifySuiAdapterHandoff(bundle) {
  expectObject(bundle, 'handoff')
  expectExactKeys(bundle, HANDOFF_KEYS, 'handoff')
  expectExact(bundle.schema, SUI_ADAPTER_HANDOFF_SCHEMA, 'handoff.schema')
  expectOneOf(bundle.package_kind, PACKAGE_KINDS, 'handoff.package_kind')
  expectIdentifier(bundle.project_id, 'handoff.project_id', 3, 160)
  expectIdentifier(bundle.projection_package_id, 'handoff.projection_package_id', 8, 160)
  expectIdentifier(bundle.source_id, 'handoff.source_id', 3, 160)
  expectOneOf(bundle.target_network, TARGET_NETWORKS, 'handoff.target_network')
  expectIdentifier(bundle.package_schema, 'handoff.package_schema', 3, 160)
  expectDigest(bundle.projection_digest, 'handoff.projection_digest')
  expectDigest(bundle.source_digest, 'handoff.source_digest')
  expectObject(bundle.envelope, 'handoff.envelope')
  expectExact(bundle.shadow_only, true, 'handoff.shadow_only')
  expectExact(
    bundle.atomic_bundle,
    bundle.package_kind === 'correction',
    'handoff.atomic_bundle',
  )
  expectExact(bundle.network_submission, 'not_submitted', 'handoff.network_submission')
  expectExact(bundle.submission_attempts, 0, 'handoff.submission_attempts')
  expectTimestamp(bundle.package_created_at, 'handoff.package_created_at')
  verifyOfflineConstraints(bundle.constraints)
  const suppliedDigest = expectDigest(bundle.handoff_digest, 'handoff.handoff_digest')
  const computedDigest = sha256Hex(JSON.stringify(handoffPayload(bundle)))
  if (computedDigest !== suppliedDigest) {
    fail('handoff_digest_mismatch', 'handoff digest does not match its deterministic payload')
  }
  return Object.freeze({
    schema: SUI_ADAPTER_HANDOFF_SCHEMA,
    projectId: bundle.project_id,
    packageKind: bundle.package_kind,
    projectionPackageId: bundle.projection_package_id,
    targetNetwork: bundle.target_network,
    projectionDigest: bundle.projection_digest,
    handoffDigest: suppliedDigest,
    atomicBundle: bundle.atomic_bundle,
  })
}

export function createSuiPreflightClient(options) {
  expectObject(options, 'options')
  const baseUrl = normalizeBaseUrl(options.baseUrl)
  const token = expectToken(options.token)
  const fetchImpl = options.fetch ?? globalThis.fetch
  if (typeof fetchImpl !== 'function') {
    fail('missing_fetch', 'options.fetch or globalThis.fetch is required')
  }
  return Object.freeze({
    async report(handoff, input, { signal } = {}) {
      const verified = verifySuiAdapterHandoff(handoff)
      expectObject(input, 'report')
      const body = {
        package_kind: verified.packageKind,
        projection_package_id: verified.projectionPackageId,
        handoff_digest: verified.handoffDigest,
        outcome: expectOneOf(input.outcome, REPORT_OUTCOMES, 'report.outcome'),
        summary: expectText(input.summary, 'report.summary', 4, 500),
        tool_version: expectText(input.toolVersion, 'report.toolVersion', 1, 100),
        idempotency_key: expectIdentifier(
          input.idempotencyKey,
          'report.idempotencyKey',
          8,
          128,
        ),
      }
      const response = await fetchImpl(
        new URL('/api/economy/sui-preflight/reports', `${baseUrl}/`),
        {
          method: 'POST',
          headers: {
            authorization: `Bearer ${token}`,
            'content-type': 'application/json',
          },
          body: JSON.stringify(body),
          signal,
        },
      )
      return readReportResponse(response)
    },
  })
}

function handoffPayload(bundle) {
  return {
    schema: bundle.schema,
    package_kind: bundle.package_kind,
    project_id: bundle.project_id,
    projection_package_id: bundle.projection_package_id,
    source_id: bundle.source_id,
    target_network: bundle.target_network,
    package_schema: bundle.package_schema,
    projection_digest: bundle.projection_digest,
    source_digest: bundle.source_digest,
    envelope: bundle.envelope,
    shadow_only: bundle.shadow_only,
    atomic_bundle: bundle.atomic_bundle,
    network_submission: bundle.network_submission,
    submission_attempts: bundle.submission_attempts,
    package_created_at: bundle.package_created_at,
    constraints: {
      allowed_adapter_action: bundle.constraints.allowed_adapter_action,
      signature_present: bundle.constraints.signature_present,
      transaction_broadcast: bundle.constraints.transaction_broadcast,
      finality_verified: bundle.constraints.finality_verified,
      funds_moved: bundle.constraints.funds_moved,
    },
  }
}

function verifyOfflineConstraints(constraints) {
  expectObject(constraints, 'handoff.constraints')
  expectExactKeys(constraints, CONSTRAINT_KEYS, 'handoff.constraints')
  expectExact(
    constraints.allowed_adapter_action,
    'offline_preflight_only',
    'handoff.constraints.allowed_adapter_action',
  )
  for (const field of [
    'signature_present',
    'transaction_broadcast',
    'finality_verified',
    'funds_moved',
  ]) {
    expectExact(constraints[field], false, `handoff.constraints.${field}`)
  }
}

async function readReportResponse(response) {
  const contentLength = Number(response.headers?.get?.('content-length'))
  if (Number.isFinite(contentLength) && contentLength > SUI_PREFLIGHT_MAX_RESPONSE_BYTES) {
    fail('response_too_large', 'preflight report response exceeds 256 KiB', response.status)
  }
  const text = await response.text()
  if (Buffer.byteLength(text, 'utf8') > SUI_PREFLIGHT_MAX_RESPONSE_BYTES) {
    fail('response_too_large', 'preflight report response exceeds 256 KiB', response.status)
  }
  let payload
  try {
    payload = text ? JSON.parse(text) : null
  } catch {
    fail('invalid_json_response', 'preflight report API returned invalid JSON', response.status)
  }
  if (!response.ok) {
    const message = typeof payload?.error === 'string'
      ? payload.error.slice(0, 240)
      : `preflight report API returned HTTP ${response.status}`
    fail('preflight_http_error', message, response.status)
  }
  if (payload?.schema !== SUI_PREFLIGHT_REPORT_SCHEMA) {
    fail('invalid_report_response', 'preflight report response schema is unsupported')
  }
  expectDigest(payload.report_digest, 'response.report_digest')
  return payload
}

function normalizeBaseUrl(value) {
  if (typeof value !== 'string' || !value.trim()) {
    fail('invalid_base_url', 'options.baseUrl is required')
  }
  let url
  try {
    url = new URL(value.trim())
  } catch {
    fail('invalid_base_url', 'options.baseUrl must be an absolute HTTP URL')
  }
  if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password || url.search || url.hash) {
    fail('invalid_base_url', 'options.baseUrl must be an HTTP origin without credentials, query, or hash')
  }
  if (url.protocol === 'http:' && !['localhost', '127.0.0.1', '::1'].includes(url.hostname)) {
    fail('insecure_base_url', 'non-local preflight traffic must use HTTPS')
  }
  return url.toString().replace(/\/$/, '')
}

function expectToken(value) {
  if (typeof value !== 'string' || !value.startsWith('sui_preflight_') || value.length < 52 || /\s/.test(value)) {
    fail('invalid_token', 'Sui preflight token is missing or malformed')
  }
  return value
}

function expectObject(value, path) {
  if (!value || Array.isArray(value) || typeof value !== 'object') {
    fail('invalid_object', `${path} must be an object`)
  }
}

function expectExactKeys(value, expected, path) {
  const actual = Object.keys(value)
  if (actual.length !== expected.size || actual.some((key) => !expected.has(key))) {
    fail('unexpected_field', `${path} contains missing or unsupported fields`)
  }
}

function expectExact(value, expected, path) {
  if (value !== expected) fail('invalid_value', `${path} must be ${JSON.stringify(expected)}`)
  return value
}

function expectOneOf(value, allowed, path) {
  if (!allowed.has(value)) fail('invalid_value', `${path} is unsupported`)
  return value
}

function expectIdentifier(value, path, min, max) {
  const text = expectText(value, path, min, max)
  if (!/^[A-Za-z0-9._:-]+$/.test(text)) {
    fail('invalid_identifier', `${path} contains unsupported characters`)
  }
  return text
}

function expectText(value, path, min, max) {
  if (typeof value !== 'string') fail('invalid_text', `${path} must be text`)
  const text = value.trim()
  if (text.length < min || text.length > max) {
    fail('invalid_text', `${path} must contain ${min}-${max} characters`)
  }
  return text
}

function expectTimestamp(value, path) {
  if (typeof value !== 'string' || !Number.isFinite(Date.parse(value))) {
    fail('invalid_timestamp', `${path} must be an RFC3339 timestamp`)
  }
  return value
}

function expectDigest(value, path) {
  if (typeof value !== 'string' || !/^[a-f0-9]{64}$/.test(value)) {
    fail('invalid_digest', `${path} must be a lowercase SHA-256 digest`)
  }
  return value
}

function sha256Hex(value) {
  return createHash('sha256').update(value, 'utf8').digest('hex')
}

function fail(code, message, status = undefined) {
  throw new SuiPreflightContractError(code, message, status)
}
