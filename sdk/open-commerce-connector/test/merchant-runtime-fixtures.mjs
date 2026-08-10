import { createHmac } from 'node:crypto'

import {
  createMemoryMerchantRuntimeIdempotencyStore,
  createMerchantRuntime,
} from '../src/index.js'

export const merchantId = 'merchant-demo'
export const runtimeKeyId = 'OPEN_COMMERCE_RUNTIME_SECRET_DEMO'
export const runtimeSecret = 'test-runtime-secret-that-is-at-least-32-bytes'
export const nowUnix = 1_786_345_200

export function runtimeEnvelope(overrides = {}) {
  return {
    schema: 'merchant_runtime.invoke.v1',
    invocation_id: 'invocation-demo-1',
    merchant_id: merchantId,
    capability_key: 'catalog.search',
    requester_user_id: 'user-demo-1',
    requester_app_id: 'consumer-app-demo',
    credential_environment: 'sandbox',
    credential_id: 'credential-demo-1',
    grant_id: null,
    action_confirmation_id: null,
    idempotency_key: 'idempotency-demo-1',
    issued_at_unix: nowUnix,
    input: { query: 'latte' },
    ...overrides,
  }
}

export function signedInvocation(overrides = {}, signing = {}) {
  const envelope = runtimeEnvelope(overrides)
  const body = signing.body ?? JSON.stringify(envelope)
  const timestamp = signing.timestamp ?? envelope.issued_at_unix
  const secret = signing.secret ?? runtimeSecret
  const signature = createHmac('sha256', secret)
    .update(String(timestamp), 'utf8')
    .update('.')
    .update(body)
    .digest('hex')
  return {
    body,
    headers: {
      'x-yilong-runtime-key-id': signing.keyId ?? runtimeKeyId,
      'x-yilong-runtime-timestamp': String(timestamp),
      'x-yilong-runtime-signature': signing.signature ?? `v1=${signature}`,
    },
    nowUnix: signing.nowUnix ?? timestamp,
  }
}

export function createTestRuntime(options = {}) {
  const capabilities = options.capabilities ?? [{
    key: 'catalog.search',
    access: 'public',
    input_schema: { type: 'object' },
  }]
  const handlers = options.handlers ?? {
    async 'catalog.search'(input) {
      return { items: [{ id: 'product-1', query: input.query ?? null }] }
    },
  }
  return createMerchantRuntime({
    merchantId,
    keyId: runtimeKeyId,
    secret: runtimeSecret,
    capabilities,
    handlers,
    idempotencyStore: options.idempotencyStore
      ?? createMemoryMerchantRuntimeIdempotencyStore(),
    maxClockSkewSeconds: options.maxClockSkewSeconds,
  })
}

export function deferred() {
  let resolve
  let reject
  const promise = new Promise((resolvePromise, rejectPromise) => {
    resolve = resolvePromise
    reject = rejectPromise
  })
  return { promise, resolve, reject }
}
