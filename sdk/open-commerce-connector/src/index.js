import { createHash } from 'node:crypto'

export {
  ADAPTER_HANDOFF_CLAIM_POLL_SCHEMA,
  ADAPTER_HANDOFF_CLAIM_RELEASE_SCHEMA,
  ADAPTER_HANDOFF_CLAIM_RENEW_SCHEMA,
  ADAPTER_HANDOFF_CLAIM_SCHEMA,
  ADAPTER_HANDOFF_MAX_RESPONSE_BYTES,
  AdapterHandoffClientError,
  createAdapterHandoffClient,
} from './adapter-handoff-client.js'

export {
  ADAPTER_HANDOFF_WORKER_SCHEMA,
  AdapterHandoffRejectError,
  AdapterHandoffReleaseError,
  createAdapterHandoffWorker,
} from './adapter-handoff-worker.js'

export {
  SUI_ADAPTER_HANDOFF_SCHEMA,
  SUI_PREFLIGHT_MAX_RESPONSE_BYTES,
  SUI_PREFLIGHT_REPORT_SCHEMA,
  SuiPreflightContractError,
  createSuiPreflightClient,
  verifySuiAdapterHandoff,
} from './sui-preflight.js'

export {
  SUI_PREFLIGHT_JOB_COMPLETE_SCHEMA,
  SUI_PREFLIGHT_JOB_POLL_SCHEMA,
  SUI_PREFLIGHT_JOB_RELEASE_SCHEMA,
  SUI_PREFLIGHT_JOB_RENEW_SCHEMA,
  createSuiPreflightJobClient,
} from './sui-preflight-job-client.js'

export { createMemoryMerchantRuntimeIdempotencyStore } from './merchant-runtime-memory-store.js'

export {
  MERCHANT_IDENTITY_ALGORITHM,
  MERCHANT_IDENTITY_PROOF_PROTOCOL,
  createMerchantIdentityProof,
  merchantIdentityProofMessage,
  merchantIdentityPublicKeyId,
  verifyMerchantIdentityProof,
} from './merchant-identity.js'

export {
  CONSUMER_PORTABILITY_SIGNATURE_ALGORITHM,
  CONSUMER_PORTABILITY_SIGNATURE_SCHEMA,
  consumerPortabilityPublicKeyId,
  consumerPortabilitySignatureMessage,
  signConsumerPortabilityPackage,
  verifyConsumerPortabilityPackageSignature,
} from './portability-signature.js'

export {
  CONSUMER_PORTABILITY_ARCHIVE_ITERATIONS,
  CONSUMER_PORTABILITY_ARCHIVE_SCHEMA,
  decryptConsumerPortabilityArchive,
  encryptConsumerPortabilityArchive,
} from './portability-archive.js'

export {
  DEVELOPER_WEBHOOK_EVENT_SCHEMA,
  DEVELOPER_WEBHOOK_MAX_CLOCK_SKEW_SECONDS,
  DEVELOPER_WEBHOOK_VERIFICATION_SCHEMA,
  DeveloperWebhookSignatureError,
  createDeveloperWebhookVerificationResponse,
  developerWebhookSignatureMessage,
  verifyDeveloperWebhookSignature,
} from './webhook-signature.js'

export {
  MERCHANT_RUNTIME_ERROR_SCHEMA,
  MERCHANT_RUNTIME_MANIFEST_SCHEMA,
  MERCHANT_RUNTIME_MAX_BODY_BYTES,
  MERCHANT_RUNTIME_REQUEST_SCHEMA,
  MERCHANT_RUNTIME_RESULT_SCHEMA,
  MerchantRuntimeError,
  createMerchantRuntime,
} from './merchant-runtime.js'

export const CONNECTOR_SCHEMA = 'open_commerce.connector.v1'
export const CONNECTOR_CONTRACT_VERSION = '1.0'
export const MAX_SYNC_PAGE_RECORDS = 500
export const MAX_RECEIPT_KEY_LENGTH = 128

const CONNECTION_MODES = new Set([
  'official_api',
  'merchant_export',
  'local_adapter',
  'manual_import',
])
const SYNC_KINDS = new Set(['full', 'incremental', 'health_check'])
const SYNC_STATUSES = new Set(['succeeded', 'partial', 'failed'])
const HEALTH_STATUSES = new Set(['ready', 'degraded', 'unavailable'])
const SENSITIVE_KEY = /(secret|password|passwd|token|credential|cookie|authorization|api[_-]?key)/i

export class ConnectorContractError extends Error {
  constructor(code, message, path = '') {
    super(message)
    this.name = 'ConnectorContractError'
    this.code = code
    this.path = path
  }
}

export function defineConnector(connector) {
  validateConnector(connector)
  return Object.freeze(connector)
}

export function validateConnector(connector) {
  if (!connector || typeof connector !== 'object') {
    fail('invalid_connector', 'connector must be an object')
  }
  if (typeof connector.describe !== 'function') {
    fail('missing_method', 'connector.describe() is required', 'describe')
  }
  if (typeof connector.health !== 'function') {
    fail('missing_method', 'connector.health() is required', 'health')
  }
  if (typeof connector.sync !== 'function') {
    fail('missing_method', 'connector.sync() is required', 'sync')
  }
  validateManifest(connector.describe())
  return connector
}

export function validateManifest(manifest) {
  expectObject(manifest, 'manifest')
  expectExact(manifest.schema, CONNECTOR_SCHEMA, 'manifest.schema')
  expectExact(manifest.contractVersion, CONNECTOR_CONTRACT_VERSION, 'manifest.contractVersion')
  expectIdentifier(manifest.connectorKey, 'manifest.connectorKey', 3, 96)
  expectIdentifier(manifest.providerKey, 'manifest.providerKey', 2, 96)
  expectText(manifest.displayName, 'manifest.displayName', 2, 80)
  if (!CONNECTION_MODES.has(manifest.connectionMode)) {
    fail('invalid_connection_mode', 'manifest.connectionMode is unsupported', 'manifest.connectionMode')
  }
  expectStringList(manifest.scopes, 'manifest.scopes', 32)
  expectStringList(manifest.dataDomains, 'manifest.dataDomains', 32)
  if (!manifest.dataDomains.length) {
    fail('missing_data_domain', 'manifest.dataDomains must not be empty', 'manifest.dataDomains')
  }
  assertNoSensitiveKeys(manifest, 'manifest')
  return manifest
}

export function validateHealth(result) {
  expectObject(result, 'health')
  if (!HEALTH_STATUSES.has(result.status)) {
    fail('invalid_health_status', 'health.status is unsupported', 'health.status')
  }
  expectTimestamp(result.observedAt, 'health.observedAt')
  expectIdentifier(result.evidenceCode, 'health.evidenceCode', 2, 96)
  if (result.message !== undefined) {
    expectText(result.message, 'health.message', 1, 240)
  }
  assertNoSensitiveKeys(result, 'health')
  return result
}

export function validateSyncRequest(request) {
  expectObject(request, 'request')
  expectIdentifier(request.integrationId, 'request.integrationId', 2, 128)
  expectIdentifier(request.runKey, 'request.runKey', 3, MAX_RECEIPT_KEY_LENGTH)
  if (!SYNC_KINDS.has(request.syncKind)) {
    fail('invalid_sync_kind', 'request.syncKind is unsupported', 'request.syncKind')
  }
  expectStringList(request.dataDomains, 'request.dataDomains', 32)
  const limit = request.limit ?? MAX_SYNC_PAGE_RECORDS
  if (!Number.isInteger(limit) || limit < 1 || limit > MAX_SYNC_PAGE_RECORDS) {
    fail(
      'invalid_page_limit',
      `request.limit must be between 1 and ${MAX_SYNC_PAGE_RECORDS}`,
      'request.limit',
    )
  }
  if (request.cursor !== undefined && typeof request.cursor !== 'string') {
    fail('invalid_cursor', 'request.cursor must be a string', 'request.cursor')
  }
  return { ...request, limit }
}

export function validateSyncPage(page, request) {
  expectObject(page, 'page')
  expectIdentifier(page.receiptKey, 'page.receiptKey', 3, MAX_RECEIPT_KEY_LENGTH)
  if (page.receiptKey !== request.runKey) {
    fail('receipt_key_mismatch', 'page.receiptKey must equal request.runKey', 'page.receiptKey')
  }
  if (!SYNC_KINDS.has(page.syncKind) || page.syncKind !== request.syncKind) {
    fail('sync_kind_mismatch', 'page.syncKind must equal request.syncKind', 'page.syncKind')
  }
  if (!SYNC_STATUSES.has(page.status)) {
    fail('invalid_sync_status', 'page.status is unsupported', 'page.status')
  }
  if (!Array.isArray(page.changes)) {
    fail('invalid_changes', 'page.changes must be an array', 'page.changes')
  }
  if (page.changes.length > request.limit || page.changes.length > MAX_SYNC_PAGE_RECORDS) {
    fail('page_too_large', 'page.changes exceeds the bounded page limit', 'page.changes')
  }
  page.changes.forEach((change, index) => validateChange(change, `page.changes[${index}]`))
  expectTimestamp(page.startedAt, 'page.startedAt')
  expectTimestamp(page.completedAt, 'page.completedAt')
  if (Date.parse(page.completedAt) < Date.parse(page.startedAt)) {
    fail('invalid_time_range', 'page.completedAt cannot precede page.startedAt', 'page.completedAt')
  }
  if (page.nextCursor !== undefined && typeof page.nextCursor !== 'string') {
    fail('invalid_cursor', 'page.nextCursor must be a string', 'page.nextCursor')
  }
  if (page.errorCode !== undefined) {
    expectIdentifier(page.errorCode, 'page.errorCode', 2, 96)
  }
  return page
}

export function createSyncReceipt(request, page) {
  const normalizedRequest = validateSyncRequest(request)
  validateSyncPage(page, normalizedRequest)
  const recordsChanged = page.changes.filter((change) => change.operation !== 'unchanged').length
  const receipt = {
    integration_id: normalizedRequest.integrationId,
    receipt_key: page.receiptKey,
    sync_kind: page.syncKind,
    status: page.status,
    records_seen: page.changes.length,
    records_changed: recordsChanged,
    cursor_digest: page.nextCursor ? digest(page.nextCursor) : undefined,
    error_code: page.errorCode,
    started_at: new Date(page.startedAt).toISOString(),
    completed_at: new Date(page.completedAt).toISOString(),
  }
  assertNoSensitiveKeys(receipt, 'receipt')
  return receipt
}

export async function runConnectorCompatibility(connector, options) {
  validateConnector(connector)
  expectObject(options, 'options')
  const request = validateSyncRequest(options.request)
  const health = validateHealth(await connector.health(options.context ?? {}))
  const firstPage = validateSyncPage(await connector.sync(request), request)
  const firstReceipt = createSyncReceipt(request, firstPage)

  let replayReceipt
  if (options.verifyReplay !== false) {
    const replayPage = validateSyncPage(await connector.sync(request), request)
    replayReceipt = createSyncReceipt(request, replayPage)
    if (stableJson(firstReceipt) !== stableJson(replayReceipt)) {
      fail(
        'non_idempotent_replay',
        'replaying the same runKey produced a different bounded receipt',
        'sync',
      )
    }
  }

  return {
    schema: 'open_commerce.connector.compatibility.v1',
    compatible: true,
    manifest: connector.describe(),
    health,
    receipt: firstReceipt,
    replayVerified: Boolean(replayReceipt),
  }
}

function validateChange(change, path) {
  expectObject(change, path)
  expectIdentifier(change.recordId, `${path}.recordId`, 1, 160)
  expectIdentifier(change.dataDomain, `${path}.dataDomain`, 2, 96)
  if (!['upsert', 'delete', 'unchanged'].includes(change.operation)) {
    fail('invalid_operation', 'change.operation is unsupported', `${path}.operation`)
  }
  if (change.version !== undefined && typeof change.version !== 'string') {
    fail('invalid_version', 'change.version must be a string', `${path}.version`)
  }
  if (change.operation !== 'delete') {
    expectObject(change.value, `${path}.value`)
  }
}

function assertNoSensitiveKeys(value, path) {
  if (Array.isArray(value)) {
    value.forEach((item, index) => assertNoSensitiveKeys(item, `${path}[${index}]`))
    return
  }
  if (!value || typeof value !== 'object') return
  for (const [key, nested] of Object.entries(value)) {
    if (SENSITIVE_KEY.test(key)) {
      fail('sensitive_field', `sensitive field "${key}" is forbidden`, `${path}.${key}`)
    }
    assertNoSensitiveKeys(nested, `${path}.${key}`)
  }
}

function digest(value) {
  return `sha256:${createHash('sha256').update(value, 'utf8').digest('hex')}`
}

function stableJson(value) {
  return JSON.stringify(sortValue(value))
}

function sortValue(value) {
  if (Array.isArray(value)) return value.map(sortValue)
  if (!value || typeof value !== 'object') return value
  return Object.fromEntries(
    Object.keys(value)
      .sort()
      .map((key) => [key, sortValue(value[key])]),
  )
}

function expectObject(value, path) {
  if (!value || Array.isArray(value) || typeof value !== 'object') {
    fail('invalid_object', `${path} must be an object`, path)
  }
}

function expectText(value, path, min, max) {
  if (typeof value !== 'string' || value.trim().length < min || value.trim().length > max) {
    fail('invalid_text', `${path} must contain ${min}-${max} characters`, path)
  }
}

function expectIdentifier(value, path, min, max) {
  expectText(value, path, min, max)
  if (!/^[A-Za-z0-9._:-]+$/.test(value)) {
    fail('invalid_identifier', `${path} contains unsupported characters`, path)
  }
}

function expectStringList(value, path, maxItems) {
  if (!Array.isArray(value) || value.length > maxItems) {
    fail('invalid_list', `${path} must be an array with at most ${maxItems} items`, path)
  }
  value.forEach((item, index) => expectIdentifier(item, `${path}[${index}]`, 1, 96))
}

function expectTimestamp(value, path) {
  if (typeof value !== 'string' || !Number.isFinite(Date.parse(value))) {
    fail('invalid_timestamp', `${path} must be an RFC3339 timestamp`, path)
  }
}

function expectExact(value, expected, path) {
  if (value !== expected) {
    fail('version_mismatch', `${path} must equal ${expected}`, path)
  }
}

function fail(code, message, path = '') {
  throw new ConnectorContractError(code, message, path)
}
