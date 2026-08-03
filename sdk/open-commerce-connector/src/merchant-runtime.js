import { createHash, createHmac, timingSafeEqual } from 'node:crypto'

export const MERCHANT_RUNTIME_REQUEST_SCHEMA = 'merchant_runtime.invoke.v1'
export const MERCHANT_RUNTIME_RESULT_SCHEMA = 'merchant_runtime.result.v1'
export const MERCHANT_RUNTIME_ERROR_SCHEMA = 'merchant_runtime.error.v1'
export const MERCHANT_RUNTIME_MANIFEST_SCHEMA = 'merchant_runtime.manifest.v1'
export const MERCHANT_RUNTIME_MAX_BODY_BYTES = 512 * 1024

const IDENTIFIER = /^[A-Za-z0-9._:-]+$/

export class MerchantRuntimeError extends Error {
  constructor(status, errorCode, message) {
    super(message)
    this.name = 'MerchantRuntimeError'
    this.status = status
    this.errorCode = errorCode
  }
}

export function createMerchantRuntime(options) {
  expectObject(options, 'options')
  const merchantId = expectIdentifier(options.merchantId, 'options.merchantId', 2, 160)
  const keyId = expectIdentifier(options.keyId, 'options.keyId', 8, 128)
  const secret = expectSecret(options.secret)
  const maxClockSkewSeconds = boundedInteger(
    options.maxClockSkewSeconds ?? 300,
    30,
    900,
    'options.maxClockSkewSeconds',
  )
  const handlers = normalizeHandlers(options.handlers)
  const definitions = normalizeDefinitions(options.capabilities, handlers)
  const authorized = new Set(
    definitions.filter((item) => item.access === 'authorized').map((item) => item.key),
  )
  const idempotencyStore = options.idempotencyStore
  expectIdempotencyStore(idempotencyStore)
  const manifest = Object.freeze({
    schema: MERCHANT_RUNTIME_MANIFEST_SCHEMA,
    merchant_id: merchantId,
    protocol: 'yilong.open_commerce.runtime.v1',
    money_format: 'integer_minor_units',
    result_extensions: {
      business_receipt: {
        field: '_yilong_business_receipt',
        schema: 'open_commerce.merchant_business_receipt.v1',
        authority: 'merchant_runtime_asserted',
        capabilities: definitions
          .map((item) => item.key)
          .filter((key) => key.startsWith('order.')),
      },
    },
    capabilities: definitions,
  })
  const manifestSha256 = digest(stableJson(manifest))

  async function handleInvoke({ headers, body, nowUnix = Math.floor(Date.now() / 1_000) }) {
    let claimInput
    try {
      const bytes = bodyBytes(body)
      verifySignature(headers, bytes, { keyId, secret, nowUnix, maxClockSkewSeconds })
      const envelope = parseEnvelope(bytes, merchantId)
      const signedTimestamp = Number(header(headers, 'x-yilong-runtime-timestamp'))
      if (envelope.issued_at_unix !== signedTimestamp) {
        reject(401, 'signature_rejected', 'signed timestamp does not match the runtime envelope')
      }
      if (authorized.has(envelope.capability_key) && !nonEmpty(envelope.grant_id)) {
        reject(401, 'signature_rejected', 'authorized capability requires a platform grant')
      }
      if (envelope.capability_key === 'order.commit') validateOrderConfirmation(envelope.input)

      claimInput = idempotencyInput(envelope)
      const claim = await idempotencyStore.claim(claimInput)
      if (claim?.status === 'conflict') {
        reject(409, 'idempotency_conflict', 'the same idempotency key was used for different input')
      }
      if (claim?.status === 'busy') {
        reject(409, 'idempotency_busy', 'the same idempotent invocation is still processing')
      }
      if (claim?.status === 'replayed') {
        expectResult(claim.result)
        return successEnvelope(envelope, claim.result)
      }
      if (claim?.status !== 'claimed') {
        reject(500, 'idempotency_store_error', 'idempotency store returned an unsupported claim state')
      }

      const result = envelope.capability_key === 'system.health'
        ? { merchant_id: merchantId, status: 'ok', manifest_sha256: manifestSha256 }
        : await invokeHandler(handlers, envelope)
      expectResult(result)
      const completed = await idempotencyStore.complete(claimInput, result)
      if (completed !== true) {
        reject(409, 'idempotency_ownership_lost', 'idempotent invocation ownership changed before completion')
      }
      return successEnvelope(envelope, result)
    } catch (error) {
      if (claimInput) {
        try {
          await idempotencyStore.release(claimInput)
        } catch {
          // The durable store owns stale-claim recovery; never hide the original failure.
        }
      }
      return errorEnvelope(error)
    }
  }

  return Object.freeze({
    manifest: () => manifest,
    manifestSha256: () => manifestSha256,
    handleInvoke,
  })
}

async function invokeHandler(handlers, envelope) {
  const handler = handlers.get(envelope.capability_key)
  if (!handler) reject(404, 'capability_not_found', 'merchant capability is not published')
  return handler(envelope.input, Object.freeze({
    invocationId: envelope.invocation_id,
    merchantId: envelope.merchant_id,
    capabilityKey: envelope.capability_key,
    requesterUserId: envelope.requester_user_id,
    requesterAppId: envelope.requester_app_id,
    grantId: envelope.grant_id,
    idempotencyKey: envelope.idempotency_key,
    issuedAtUnix: envelope.issued_at_unix,
  }))
}

function parseEnvelope(bytes, merchantId) {
  let value
  try {
    value = JSON.parse(bytes.toString('utf8'))
  } catch {
    reject(400, 'invalid_request', 'runtime body is not valid JSON')
  }
  if (!value || Array.isArray(value) || typeof value !== 'object') {
    reject(400, 'invalid_request', 'runtime envelope must be an object')
  }
  if (value.schema !== MERCHANT_RUNTIME_REQUEST_SCHEMA) {
    reject(400, 'invalid_request', 'runtime envelope schema is unsupported')
  }
  if (value.merchant_id !== merchantId) {
    reject(400, 'merchant_mismatch', 'runtime envelope merchant does not match this runtime')
  }
  for (const [field, min, max] of [
    ['invocation_id', 3, 160],
    ['capability_key', 2, 128],
    ['requester_user_id', 2, 160],
    ['requester_app_id', 2, 160],
    ['idempotency_key', 8, 120],
  ]) {
    try {
      expectIdentifier(value[field], `runtime envelope.${field}`, min, max)
    } catch {
      reject(400, 'invalid_request', `runtime envelope ${field} is invalid`)
    }
  }
  if (!Number.isInteger(value.issued_at_unix)) {
    reject(400, 'invalid_request', 'runtime envelope issued_at_unix must be an integer')
  }
  if (value.grant_id !== null && value.grant_id !== undefined) {
    try {
      expectIdentifier(value.grant_id, 'runtime envelope.grant_id', 2, 160)
    } catch {
      reject(400, 'invalid_request', 'runtime envelope grant_id is invalid')
    }
  }
  if (!value.input || Array.isArray(value.input) || typeof value.input !== 'object') {
    reject(400, 'invalid_request', 'runtime envelope input must be an object')
  }
  return value
}

function idempotencyInput(envelope) {
  const logical = {
    merchant_id: envelope.merchant_id,
    capability_key: envelope.capability_key,
    requester_user_id: envelope.requester_user_id,
    requester_app_id: envelope.requester_app_id,
    grant_id: envelope.grant_id ?? null,
    input: envelope.input,
  }
  return Object.freeze({
    merchantId: envelope.merchant_id,
    requesterAppId: envelope.requester_app_id,
    capabilityKey: envelope.capability_key,
    idempotencyKey: envelope.idempotency_key,
    invocationId: envelope.invocation_id,
    requestHash: digest(stableJson(logical)),
  })
}

function verifySignature(headers, body, config) {
  const actualKeyId = header(headers, 'x-yilong-runtime-key-id')
  if (actualKeyId !== config.keyId) reject(401, 'signature_rejected', 'runtime key id mismatch')
  const timestamp = header(headers, 'x-yilong-runtime-timestamp')
  const issuedAt = Number(timestamp)
  if (!Number.isInteger(issuedAt)
    || Math.abs(config.nowUnix - issuedAt) > config.maxClockSkewSeconds) {
    reject(401, 'signature_rejected', 'runtime request is outside the replay window')
  }
  const encoded = header(headers, 'x-yilong-runtime-signature')
  if (!/^v1=[a-f0-9]{64}$/i.test(encoded)) {
    reject(401, 'signature_rejected', 'runtime signature encoding is invalid')
  }
  const expected = createHmac('sha256', config.secret)
    .update(timestamp, 'utf8')
    .update('.')
    .update(body)
    .digest()
  const actual = Buffer.from(encoded.slice(3), 'hex')
  if (actual.length !== expected.length || !timingSafeEqual(actual, expected)) {
    reject(401, 'signature_rejected', 'runtime signature verification failed')
  }
}

function successEnvelope(envelope, result) {
  return {
    status: 200,
    body: {
      schema: MERCHANT_RUNTIME_RESULT_SCHEMA,
      invocation_id: envelope.invocation_id,
      capability_key: envelope.capability_key,
      result: structuredClone(result),
    },
  }
}

function errorEnvelope(error) {
  const runtimeError = error instanceof MerchantRuntimeError
    ? error
    : new MerchantRuntimeError(500, 'internal_error', 'merchant runtime handler failed')
  return {
    status: runtimeError.status,
    body: {
      schema: MERCHANT_RUNTIME_ERROR_SCHEMA,
      error_code: runtimeError.errorCode,
      message: runtimeError.message,
    },
  }
}

function normalizeHandlers(value) {
  expectObject(value, 'options.handlers')
  const handlers = new Map()
  for (const [key, handler] of Object.entries(value)) {
    expectIdentifier(key, 'handler capability key', 2, 128)
    if (typeof handler !== 'function') throw new TypeError(`handler ${key} must be a function`)
    handlers.set(key, handler)
  }
  return handlers
}

function normalizeDefinitions(value, handlers) {
  if (!Array.isArray(value) || value.length !== handlers.size) {
    throw new TypeError('options.capabilities must define every handler exactly once')
  }
  const seen = new Set()
  const definitions = value.map((item, index) => {
    expectObject(item, `options.capabilities[${index}]`)
    const key = expectIdentifier(item.key, `options.capabilities[${index}].key`, 2, 128)
    if (!handlers.has(key) || seen.has(key)) {
      throw new TypeError(`capability definition ${key} is missing a handler or is duplicated`)
    }
    seen.add(key)
    if (!['public', 'authorized'].includes(item.access)) {
      throw new TypeError(`capability definition ${key} has unsupported access`)
    }
    expectObject(item.input_schema, `options.capabilities[${index}].input_schema`)
    return structuredClone({ key, access: item.access, input_schema: item.input_schema })
  })
  return definitions.sort((left, right) => left.key.localeCompare(right.key))
}

function expectIdempotencyStore(store) {
  expectObject(store, 'options.idempotencyStore')
  for (const method of ['claim', 'complete', 'release']) {
    if (typeof store[method] !== 'function') {
      throw new TypeError(`options.idempotencyStore.${method} is required`)
    }
  }
}

function validateOrderConfirmation(input) {
  if (input.confirmed_by_user !== true
    || typeof input.confirmation_id !== 'string'
    || input.confirmation_id.trim().length < 8
    || input.confirmation_id.trim().length > 120) {
    reject(400, 'confirmation_required', 'order.commit requires explicit user confirmation')
  }
}

function expectResult(result) {
  expectObject(result, 'merchant runtime result')
  const bytes = Buffer.byteLength(JSON.stringify(result), 'utf8')
  if (bytes > MERCHANT_RUNTIME_MAX_BODY_BYTES) {
    reject(500, 'result_too_large', 'merchant runtime result exceeds 512 KiB')
  }
}

function bodyBytes(value) {
  if (!(typeof value === 'string' || Buffer.isBuffer(value) || value instanceof Uint8Array)) {
    reject(400, 'invalid_request', 'merchant runtime body must be raw UTF-8 bytes or a string')
  }
  const bytes = Buffer.isBuffer(value) ? value : Buffer.from(value)
  if (bytes.length > MERCHANT_RUNTIME_MAX_BODY_BYTES) {
    reject(400, 'request_too_large', 'merchant runtime request exceeds 512 KiB')
  }
  return bytes
}

function header(headers, name) {
  const value = typeof headers?.get === 'function'
    ? headers.get(name)
    : Object.entries(headers ?? {}).find(([key]) => key.toLowerCase() === name)?.[1]
  const normalized = Array.isArray(value) ? value[0] : value
  if (typeof normalized !== 'string' || !normalized.trim()) {
    reject(401, 'signature_rejected', 'runtime signature headers are incomplete')
  }
  return normalized.trim()
}

function expectSecret(value) {
  if (typeof value !== 'string' || value.length < 32 || value.length > 512) {
    throw new TypeError('options.secret must contain 32-512 characters')
  }
  return value
}

function expectObject(value, path) {
  if (!value || Array.isArray(value) || typeof value !== 'object') {
    throw new TypeError(`${path} must be an object`)
  }
}

function expectIdentifier(value, path, min, max) {
  if (typeof value !== 'string' || value.trim().length < min || value.trim().length > max) {
    throw new TypeError(`${path} must contain ${min}-${max} characters`)
  }
  if (!IDENTIFIER.test(value.trim())) throw new TypeError(`${path} contains unsupported characters`)
  return value.trim()
}

function boundedInteger(value, min, max, path) {
  if (!Number.isInteger(value) || value < min || value > max) {
    throw new TypeError(`${path} must be an integer between ${min} and ${max}`)
  }
  return value
}

function nonEmpty(value) {
  return typeof value === 'string' && value.trim().length > 0
}

function digest(value) {
  return createHash('sha256').update(value, 'utf8').digest('hex')
}

function stableJson(value) {
  return JSON.stringify(sortValue(value))
}

function sortValue(value) {
  if (Array.isArray(value)) return value.map(sortValue)
  if (!value || typeof value !== 'object') return value
  return Object.fromEntries(Object.keys(value).sort().map((key) => [key, sortValue(value[key])]))
}

function reject(status, errorCode, message) {
  throw new MerchantRuntimeError(status, errorCode, message)
}
