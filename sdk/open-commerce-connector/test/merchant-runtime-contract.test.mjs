import assert from 'node:assert/strict'
import test from 'node:test'

import { MERCHANT_RUNTIME_MAX_BODY_BYTES } from '../src/index.js'
import {
  createTestRuntime,
  merchantId,
  signedInvocation,
} from './merchant-runtime-fixtures.mjs'

test('manifest ordering and digest are deterministic and deeply immutable', () => {
  const first = manifestRuntime(['z.inventory', 'a.catalog'])
  const second = manifestRuntime(['a.catalog', 'z.inventory'])

  assert.deepEqual(
    first.manifest().capabilities.map((item) => item.key),
    ['a.catalog', 'z.inventory'],
  )
  assert.equal(first.manifestSha256(), second.manifestSha256())
  assert.throws(() => { first.manifest().capabilities[0].key = 'changed' }, TypeError)
  assert.throws(() => { first.manifest().capabilities[0].input_schema.type = 'array' }, TypeError)
})

test('reserved system health is available without a published handler', async () => {
  const runtime = createTestRuntime()
  const response = await runtime.handleInvoke(signedInvocation({
    capability_key: 'system.health',
    input: {},
    credential_environment: 'platform',
    credential_id: null,
    idempotency_key: 'health-idempotency-1',
  }))

  assert.equal(response.status, 200)
  assert.deepEqual(response.body.result, {
    merchant_id: merchantId,
    status: 'ok',
    manifest_sha256: runtime.manifestSha256(),
  })
  assert.equal(runtime.manifest().capabilities.some((item) => item.key === 'system.health'), false)
})

test('runtime normalizes signed identity fields before dispatch and idempotency', async () => {
  let context
  const runtime = createTestRuntime({
    handlers: {
      async 'catalog.search'(_input, current) {
        context = current
        return {}
      },
    },
  })
  const response = await runtime.handleInvoke(signedInvocation({
    requester_user_id: ' user-demo-1 ',
    requester_app_id: ' consumer-app-demo ',
    credential_environment: ' sandbox ',
    credential_id: ' credential-demo-1 ',
  }))

  assert.equal(response.status, 200)
  assert.equal(context.requesterUserId, 'user-demo-1')
  assert.equal(context.requesterAppId, 'consumer-app-demo')
  assert.equal(context.credentialEnvironment, 'sandbox')
  assert.equal(context.credentialId, 'credential-demo-1')
})

test('runtime fails closed on missing provenance, unknown capabilities, and oversized results', async () => {
  const runtime = createTestRuntime({
    handlers: {
      async 'catalog.search'(input) {
        return input.large ? { value: 'x'.repeat(MERCHANT_RUNTIME_MAX_BODY_BYTES) } : {}
      },
    },
  })
  const missingProvenance = await runtime.handleInvoke(signedInvocation({
    credential_environment: undefined,
  }))
  const unknown = await runtime.handleInvoke(signedInvocation({
    capability_key: 'catalog.unknown',
    idempotency_key: 'unknown-idempotency-1',
  }))
  const oversized = await runtime.handleInvoke(signedInvocation({
    input: { large: true },
    idempotency_key: 'large-result-idempotency-1',
  }))

  assert.equal(missingProvenance.status, 400)
  assert.equal(missingProvenance.body.error_code, 'invalid_request')
  assert.equal(unknown.status, 404)
  assert.equal(unknown.body.error_code, 'capability_not_found')
  assert.equal(oversized.status, 500)
  assert.equal(oversized.body.error_code, 'result_too_large')
})

test('runtime preserves action metadata and rejects ambiguous action declarations', () => {
  const runtime = createTestRuntime({
    capabilities: [{
      key: 'inventory.reserve',
      access: 'authorized',
      action: true,
      input_schema: { type: 'object' },
    }],
    handlers: { async 'inventory.reserve'() { return {} } },
  })

  assert.equal(runtime.manifest().capabilities[0].action, true)
  assert.throws(
    () => createTestRuntime({
      capabilities: [{
        key: 'inventory.reserve',
        access: 'authorized',
        action: 'yes',
        input_schema: { type: 'object' },
      }],
      handlers: { async 'inventory.reserve'() { return {} } },
    }),
    /action must be a boolean/,
  )
})

function manifestRuntime(keys) {
  const capabilities = keys.map((key) => ({
    key,
    access: 'public',
    input_schema: key.startsWith('a.')
      ? { properties: { query: { type: 'string' } }, type: 'object' }
      : { type: 'object' },
  }))
  const handlers = Object.fromEntries(keys.map((key) => [key, async () => ({})]))
  return createTestRuntime({ capabilities, handlers })
}
