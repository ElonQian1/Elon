import { createHash } from 'node:crypto'

export const VERIFIED_ERP_HANDOFF_SOURCE_SCHEMA =
  'open_commerce.verified_erp_handoff_source.v1'
export const VERIFIED_ERP_HANDOFF_READBACK_SCHEMA =
  'open_commerce.verified_erp_handoff_readback.v1'

const EVIDENCE_SCHEMA = 'open_commerce.merchant_business_evidence.v1'
const BUSINESS_RECEIPT_SCHEMA = 'open_commerce.merchant_business_receipt.v1'
const SOURCE_DIGEST_DOMAIN = 'open-commerce/verified-erp-handoff-source/v1\0'
const IDENTIFIER = /^[A-Za-z0-9._:-]+$/
const BUSINESS_TOKEN = /^[a-z0-9][a-z0-9._-]*$/

export class VerifiedErpHandoffError extends Error {
  constructor(code, message, path = '') {
    super(message)
    this.name = 'VerifiedErpHandoffError'
    this.code = code
    this.path = path
  }
}

export function createVerifiedErpHandoffHandler(options) {
  expectObject(options, 'options')
  if (typeof options.apply !== 'function') {
    fail('missing_apply', 'options.apply must be a function', 'options.apply')
  }
  if (typeof options.readBack !== 'function') {
    fail('missing_readback', 'options.readBack must be a function', 'options.readBack')
  }

  return async function verifiedErpHandoffHandler(task, context) {
    const source = buildSource(task, context)
    throwIfAborted(context.signal)
    const writeResult = await options.apply(
      Object.freeze({ source, result: task.result }),
      pluginContext(context),
    )
    throwIfAborted(context.signal)
    const targetReference = normalizeWriteResult(writeResult)
    const readBack = await options.readBack(
      Object.freeze({ targetReference, source }),
      pluginContext(context),
    )
    throwIfAborted(context.signal)
    verifyReadBack(readBack, targetReference, source)
    return Object.freeze({ status: 'applied', targetReference })
  }
}

function buildSource(task, context) {
  expectObject(task, 'task')
  if (!Object.hasOwn(task, 'result')) {
    fail('invalid_task', 'task.result is required', 'task.result')
  }
  expectObject(context, 'context')
  expectObject(context.claim, 'context.claim')
  const claim = context.claim
  if (
    claim.schema !== 'open_commerce.adapter_business_handoff_claim.v1'
    || claim.status !== 'active'
  ) {
    fail('invalid_claim', 'context.claim must be an active V1 handoff claim', 'context.claim')
  }
  const projectId = expectIdentifier(claim.project_id, 'context.claim.project_id', 3, 160)
  const merchantId = expectIdentifier(claim.merchant_id, 'context.claim.merchant_id', 3, 160)
  const invocationId = expectIdentifier(claim.invocation_id, 'context.claim.invocation_id', 3, 160)
  const integrationId = expectIdentifier(claim.integration_id, 'context.claim.integration_id', 3, 160)
  const adapterCredentialId = expectIdentifier(
    claim.adapter_credential_id,
    'context.claim.adapter_credential_id',
    3,
    160,
  )
  if (!Number.isInteger(claim.adapter_credential_version) || claim.adapter_credential_version < 1) {
    fail(
      'invalid_claim',
      'context.claim.adapter_credential_version must be a positive integer',
      'context.claim.adapter_credential_version',
    )
  }
  if (
    !Number.isInteger(claim.attempt_no)
    || claim.attempt_no < 1
    || context.attemptNo !== claim.attempt_no
  ) {
    fail(
      'claim_binding_mismatch',
      'context.attemptNo must equal the positive claim attempt number',
      'context.attemptNo',
    )
  }
  if (context.idempotencyKey !== invocationId) {
    fail(
      'claim_binding_mismatch',
      'context.idempotencyKey must equal the claimed Invocation',
      'context.idempotencyKey',
    )
  }

  const evidence = normalizeEvidence(task.evidence, { merchantId, invocationId })
  validateResultReceipt(task.result, evidence.businessReceipt)
  const stableIdentity = {
    schema: VERIFIED_ERP_HANDOFF_SOURCE_SCHEMA,
    projectId,
    merchantId,
    invocationId,
    integrationId,
    idempotencyKey: invocationId,
    evidenceResultSha256: evidence.resultSha256,
    businessReceipt: evidence.businessReceipt,
    fundsMoved: false,
  }
  const source = {
    ...stableIdentity,
    adapterCredential: {
      id: adapterCredentialId,
      version: claim.adapter_credential_version,
    },
    sourceDigest: digestSource(stableIdentity),
  }
  return deepFreeze(source)
}

function validateResultReceipt(result, projectedReceipt) {
  expectObject(result, 'task.result')
  const embedded = normalizeBusinessReceipt(result._yilong_business_receipt)
  if (stableJson(embedded) !== stableJson(projectedReceipt)) {
    fail(
      'business_receipt_mismatch',
      'task result business receipt does not match the evidence projection',
      'task.result._yilong_business_receipt',
    )
  }
}

function normalizeEvidence(value, binding) {
  expectObject(value, 'task.evidence')
  if (value.schema !== EVIDENCE_SCHEMA) {
    fail('invalid_evidence', `task.evidence.schema must equal ${EVIDENCE_SCHEMA}`, 'task.evidence.schema')
  }
  if (value.invocation_id !== binding.invocationId || value.merchant_id !== binding.merchantId) {
    fail(
      'claim_binding_mismatch',
      'task evidence does not match the claimed merchant and Invocation',
      'task.evidence',
    )
  }
  if (
    value.status !== 'succeeded'
    || value.receipt_state !== 'valid'
    || value.result_available !== true
    || value.source_authority !== 'merchant_runtime_asserted'
    || value.funds_moved !== false
  ) {
    fail(
      'invalid_evidence',
      'task evidence must be a succeeded merchant-runtime result with a valid receipt and no funds moved',
      'task.evidence',
    )
  }
  const resultSha256 = expectSha256(value.result_sha256, 'task.evidence.result_sha256')
  const businessReceipt = normalizeBusinessReceipt(value.business_receipt)
  return Object.freeze({ resultSha256, businessReceipt })
}

function normalizeBusinessReceipt(value) {
  expectObject(value, 'task.evidence.business_receipt')
  if (value.schema !== BUSINESS_RECEIPT_SCHEMA) {
    fail(
      'invalid_business_receipt',
      `business receipt schema must equal ${BUSINESS_RECEIPT_SCHEMA}`,
      'task.evidence.business_receipt.schema',
    )
  }
  const entityType = expectBusinessToken(
    value.entity_type,
    'task.evidence.business_receipt.entity_type',
  )
  const referenceId = expectReference(
    value.reference_id,
    'task.evidence.business_receipt.reference_id',
  )
  const state = expectBusinessToken(value.state, 'task.evidence.business_receipt.state')
  const occurredAt = expectTimestamp(
    value.occurred_at,
    'task.evidence.business_receipt.occurred_at',
  )
  const amountPresent = value.amount_minor !== undefined && value.amount_minor !== null
  const currencyPresent = value.currency !== undefined && value.currency !== null
  if (amountPresent !== currencyPresent) {
    fail(
      'invalid_business_receipt',
      'business receipt amount and currency must be provided together',
      'task.evidence.business_receipt',
    )
  }
  let amountMinor
  let currency
  if (amountPresent) {
    if (!Number.isSafeInteger(value.amount_minor) || value.amount_minor < 0) {
      fail(
        'invalid_business_receipt',
        'business receipt amount must be a non-negative safe integer',
        'task.evidence.business_receipt.amount_minor',
      )
    }
    if (typeof value.currency !== 'string' || !/^[A-Z]{3,8}$/.test(value.currency)) {
      fail(
        'invalid_business_receipt',
        'business receipt currency must contain 3-8 uppercase ASCII letters',
        'task.evidence.business_receipt.currency',
      )
    }
    amountMinor = value.amount_minor
    currency = value.currency
  }
  return deepFreeze({
    schema: BUSINESS_RECEIPT_SCHEMA,
    entityType,
    referenceId,
    state,
    occurredAt,
    ...(amountPresent ? { amountMinor, currency } : {}),
  })
}

function normalizeWriteResult(value) {
  expectObject(value, 'apply result')
  return expectIdentifier(value.targetReference, 'apply result.targetReference', 1, 160)
}

function verifyReadBack(value, targetReference, source) {
  expectObject(value, 'readBack result')
  if (value.schema !== VERIFIED_ERP_HANDOFF_READBACK_SCHEMA) {
    fail(
      'invalid_readback',
      `readBack result.schema must equal ${VERIFIED_ERP_HANDOFF_READBACK_SCHEMA}`,
      'readBack result.schema',
    )
  }
  const returnedReference = expectIdentifier(
    value.targetReference,
    'readBack result.targetReference',
    1,
    160,
  )
  if (returnedReference !== targetReference) {
    mismatch('readBack target reference does not match the ERP write result', 'targetReference')
  }
  expectObject(value.source, 'readBack result.source')
  if (stableJson(value.source) !== stableJson(source)) {
    mismatch('readBack source binding does not match the claimed business evidence', 'source')
  }
}

function pluginContext(context) {
  return Object.freeze({
    signal: context.signal,
    idempotencyKey: context.idempotencyKey,
    attemptNo: context.attemptNo,
  })
}

function digestSource(value) {
  return createHash('sha256')
    .update(SOURCE_DIGEST_DOMAIN, 'utf8')
    .update(stableJson(value), 'utf8')
    .digest('hex')
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

function deepFreeze(value) {
  if (!value || typeof value !== 'object' || Object.isFrozen(value)) return value
  Object.values(value).forEach(deepFreeze)
  return Object.freeze(value)
}

function expectObject(value, path) {
  if (!value || Array.isArray(value) || typeof value !== 'object') {
    fail('invalid_object', `${path} must be an object`, path)
  }
}

function expectIdentifier(value, path, min, max) {
  if (
    typeof value !== 'string'
    || value.trim().length < min
    || value.trim().length > max
    || !IDENTIFIER.test(value.trim())
  ) {
    fail('invalid_identifier', `${path} must be a ${min}-${max} character identifier`, path)
  }
  return value.trim()
}

function expectBusinessToken(value, path) {
  if (typeof value !== 'string' || value.length > 64 || !BUSINESS_TOKEN.test(value)) {
    fail('invalid_business_receipt', `${path} is invalid`, path)
  }
  return value
}

function expectReference(value, path) {
  if (
    typeof value !== 'string'
    || value.trim().length < 1
    || value.trim().length > 160
    || /[\u0000-\u001f\u007f]/.test(value)
  ) {
    fail('invalid_business_receipt', `${path} is invalid`, path)
  }
  return value.trim()
}

function expectTimestamp(value, path) {
  if (
    typeof value !== 'string'
    || !/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$/.test(value)
    || !Number.isFinite(Date.parse(value))
  ) {
    fail('invalid_timestamp', `${path} must be an RFC3339 timestamp`, path)
  }
  return value
}

function expectSha256(value, path) {
  if (typeof value !== 'string' || !/^[a-f0-9]{64}$/.test(value)) {
    fail('invalid_sha256', `${path} must be a lowercase SHA-256 digest`, path)
  }
  return value
}

function throwIfAborted(signal) {
  if (!signal?.aborted) return
  const error = new Error(signal.reason ? String(signal.reason) : 'operation aborted')
  error.name = 'AbortError'
  throw error
}

function mismatch(message, path) {
  fail('readback_mismatch', message, `readBack result.${path}`)
}

function fail(code, message, path = '') {
  throw new VerifiedErpHandoffError(code, message, path)
}
